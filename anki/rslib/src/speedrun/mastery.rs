// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! F4 — per-topic **memory** mastery query.
//!
//! Computes, per canonical topic, how many of the topic's cards are
//! "mastered" plus the topic's average current recall, joining the F1
//! card->topic crosswalk (see `crate::speedrun::store`) to the live cards
//! table. Read-only: it never mutates the collection.
//!
//! Definitions (frozen with the Wednesday contract, docs/wednesday_plan.md §5):
//! - **total**: the cards mapped to the topic via the crosswalk that still
//!   exist in the collection (dangling crosswalk ids are skipped).
//! - **mastered**: of those, the cards that are **mature** — in the Review
//!   state with `interval >= 21` days. 21 days is Anki's standard "mature"
//!   threshold (`MATURE_EVL`/`graph_cutoff` upstream), so a card is mastered
//!   exactly when it is a real review card whose current interval has reached
//!   maturity. New/learning/relearning cards are never mastered.
//! - **avg_recall**: the mean of the *current FSRS retrievability* over the
//!   topic's Review-state cards that have an FSRS memory state, computed the
//!   same way as card stats (`current_retrievability_seconds(memory_state,
//!   seconds_since_last_review, decay)`). Review cards without a memory state
//!   (FSRS never run / SM-2 collection) are excluded from the mean; when no
//!   review card has a memory state the topic's `avg_recall` is `0.0`. The
//!   result is clamped to [0, 1].

use std::collections::HashMap;

use anki_proto::speedrun::TopicMastery;
use anki_proto::speedrun::TopicMasteryResponse;
use fsrs::FSRS;
use fsrs::FSRS5_DEFAULT_DECAY;

use crate::card::CardType;
use crate::prelude::*;

/// Anki's standard "mature" threshold in days. A Review-state card whose
/// current interval is at least this many days is mature, and we treat mature
/// as "mastered" for the memory-mastery query.
const MATURE_INTERVAL_DAYS: u32 = 21;

impl Collection {
    /// Compute per-topic memory mastery for the requested topics.
    ///
    /// An empty `topic_ids` means "every topic in the stored taxonomy" (ordered
    /// deterministically by id). Otherwise exactly the requested topics, in the
    /// requested order — one [`TopicMastery`] per topic considered, even when a
    /// topic has no mapped cards (it reports zeros). See the module docs for the
    /// exact `mastered`/`total`/`avg_recall` definitions.
    pub(crate) fn topic_mastery(&mut self, topic_ids: Vec<String>) -> Result<TopicMasteryResponse> {
        let topics = self.speedrun_topics()?;
        let card_topics = self.speedrun_card_topics()?;

        // Which topics to report, and in what order: an empty filter means
        // "every topic in the taxonomy", ordered deterministically by id.
        let requested: Vec<String> = if topic_ids.is_empty() {
            let mut ids: Vec<String> = topics.keys().cloned().collect();
            ids.sort();
            ids
        } else {
            topic_ids
        };

        // Group the crosswalk by topic.
        let mut cards_by_topic: HashMap<String, Vec<CardId>> = HashMap::new();
        for (card_id, topic_id) in card_topics {
            cards_by_topic.entry(topic_id).or_default().push(card_id);
        }

        let timing = self.timing_today()?;
        let fsrs = FSRS::new(None).unwrap();

        let mut out = Vec::with_capacity(requested.len());
        for topic_id in requested {
            let mut total = 0u32;
            let mut mastered = 0u32;
            let mut recall_sum = 0f32;
            let mut recall_count = 0u32;

            if let Some(card_ids) = cards_by_topic.get(&topic_id) {
                for &cid in card_ids {
                    // Skip dangling crosswalk ids (card no longer exists).
                    let Some(card) = self.storage.get_card(cid)? else {
                        continue;
                    };
                    total += 1;
                    if card.ctype == CardType::Review {
                        if card.interval >= MATURE_INTERVAL_DAYS {
                            mastered += 1;
                        }
                        if let Some(state) = card.memory_state {
                            let elapsed = card.seconds_since_last_review(&timing).unwrap_or_default();
                            let r = fsrs.current_retrievability_seconds(
                                state.into(),
                                elapsed,
                                card.decay.unwrap_or(FSRS5_DEFAULT_DECAY),
                            );
                            recall_sum += r;
                            recall_count += 1;
                        }
                    }
                }
            }

            let avg_recall = if recall_count > 0 {
                (recall_sum / recall_count as f32).clamp(0.0, 1.0) as f64
            } else {
                0.0
            };

            out.push(TopicMastery {
                topic_id,
                mastered,
                total,
                avg_recall,
            });
        }

        Ok(TopicMasteryResponse { topics: out })
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::GetTopicMasteryRequest;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use anki_proto::speedrun::TopicMastery;

    use super::*;
    use crate::card::CardQueue;
    use crate::card::CardType;
    use crate::card::FsrsMemoryState;
    use crate::services::SpeedrunService;

    /// Add a card directly in the given state and return its id. No note is
    /// needed because the mastery query only reads the cards table.
    fn add_card(
        col: &mut Collection,
        ctype: CardType,
        interval: u32,
        memory: Option<FsrsMemoryState>,
    ) -> CardId {
        let queue = match ctype {
            CardType::New => CardQueue::New,
            CardType::Learn | CardType::Relearn => CardQueue::Learn,
            CardType::Review => CardQueue::Review,
        };
        let mut card = Card {
            ctype,
            queue,
            interval,
            memory_state: memory,
            // A recent last review keeps retrievability high but is irrelevant
            // to the [0,1] bound the tests assert.
            last_review_time: memory.map(|_| TimestampSecs::now()),
            decay: memory.map(|_| FSRS5_DEFAULT_DECAY),
            ..Default::default()
        };
        col.add_card(&mut card).unwrap();
        card.id
    }

    fn topic(id: &str) -> Topic {
        Topic {
            id: id.into(),
            name: id.into(),
            blueprint_weight: 1.0,
        }
    }

    /// Seed the taxonomy + crosswalk in one undo-safe write.
    fn seed(col: &mut Collection, topics: &[&str], crosswalk: &[(i64, &str)]) {
        let req = SetTopicWeightsRequest {
            topics: topics.iter().map(|t| topic(t)).collect(),
            card_topics: crosswalk
                .iter()
                .map(|(card_id, topic_id)| CardTopic {
                    card_id: *card_id,
                    topic_id: (*topic_id).into(),
                })
                .collect(),
            weaknesses: vec![],
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    fn find<'a>(resp: &'a [TopicMastery], topic_id: &str) -> &'a TopicMastery {
        resp.iter()
            .find(|t| t.topic_id == topic_id)
            .unwrap_or_else(|| panic!("missing topic {topic_id} in response"))
    }

    /// mastered/total: a topic with 3 existing mapped cards where 2 are mature
    /// (Review, interval >= 21) and 1 is young -> mastered=2, total=3.
    /// Dangling crosswalk ids and other topics' cards do not leak in.
    #[test]
    fn mastered_and_total_counts() -> Result<()> {
        let mut col = Collection::new();

        let mature_a = add_card(&mut col, CardType::Review, 30, None);
        let mature_b = add_card(&mut col, CardType::Review, 21, None);
        let young = add_card(&mut col, CardType::Review, 5, None);
        // A different topic's mature card must not be counted under cardio.
        let renal_mature = add_card(&mut col, CardType::Review, 100, None);

        seed(
            &mut col,
            &["cardio", "renal"],
            &[
                (mature_a.0, "cardio"),
                (mature_b.0, "cardio"),
                (young.0, "cardio"),
                // A dangling crosswalk id (card never created) is skipped.
                (999_999, "cardio"),
                (renal_mature.0, "renal"),
            ],
        );

        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest::default())?
            .topics;

        let cardio = find(&resp, "cardio");
        assert_eq!(cardio.total, 3, "dangling id and renal card excluded");
        assert_eq!(cardio.mastered, 2, "interval 30 and 21 are mature; 5 is not");

        let renal = find(&resp, "renal");
        assert_eq!(renal.total, 1);
        assert_eq!(renal.mastered, 1);

        Ok(())
    }

    /// A non-mature card type with a large interval is still not mastered (only
    /// the Review state counts as mature).
    #[test]
    fn only_review_state_can_be_mastered() -> Result<()> {
        let mut col = Collection::new();
        // Relearn card with a big interval -> not mastered.
        let relearn = add_card(&mut col, CardType::Relearn, 50, None);
        let new = add_card(&mut col, CardType::New, 0, None);

        seed(
            &mut col,
            &["cardio"],
            &[(relearn.0, "cardio"), (new.0, "cardio")],
        );

        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest::default())?
            .topics;
        let cardio = find(&resp, "cardio");
        assert_eq!(cardio.total, 2);
        assert_eq!(cardio.mastered, 0);
        Ok(())
    }

    /// A topic with no mapped cards reports zeros (no panic); an empty taxonomy
    /// yields an empty response.
    #[test]
    fn empty_topics_and_empty_taxonomy() -> Result<()> {
        let mut col = Collection::new();

        // Empty taxonomy + empty filter -> empty response.
        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest::default())?
            .topics;
        assert!(resp.is_empty());

        // Taxonomy with topics but no crosswalk entries -> zeros, no panic.
        seed(&mut col, &["cardio", "renal"], &[]);
        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest::default())?
            .topics;
        assert_eq!(resp.len(), 2);
        for t in &resp {
            assert_eq!(t.total, 0);
            assert_eq!(t.mastered, 0);
            assert_eq!(t.avg_recall, 0.0);
        }
        Ok(())
    }

    /// avg_recall is always within [0,1]: with FSRS memory state present it is a
    /// real retrievability in range; a topic whose review cards lack a memory
    /// state reports exactly 0.0.
    #[test]
    fn avg_recall_within_bounds() -> Result<()> {
        let mut col = Collection::new();

        let memory = FsrsMemoryState {
            stability: 100.0,
            difficulty: 5.0,
        };
        let with_mem_a = add_card(&mut col, CardType::Review, 30, Some(memory));
        let with_mem_b = add_card(&mut col, CardType::Review, 40, Some(memory));
        // Review card with no memory state is excluded from the cardio mean.
        let no_mem = add_card(&mut col, CardType::Review, 25, None);

        // renal: review cards but none have a memory state -> avg_recall 0.0.
        let renal_card = add_card(&mut col, CardType::Review, 30, None);

        seed(
            &mut col,
            &["cardio", "renal"],
            &[
                (with_mem_a.0, "cardio"),
                (with_mem_b.0, "cardio"),
                (no_mem.0, "cardio"),
                (renal_card.0, "renal"),
            ],
        );

        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest::default())?
            .topics;

        for t in &resp {
            assert!(
                (0.0..=1.0).contains(&t.avg_recall),
                "avg_recall {} out of [0,1] for {}",
                t.avg_recall,
                t.topic_id
            );
        }

        let cardio = find(&resp, "cardio");
        assert!(cardio.avg_recall > 0.0, "memory-backed recall should be > 0");

        let renal = find(&resp, "renal");
        assert_eq!(renal.avg_recall, 0.0, "no memory state -> 0.0");

        Ok(())
    }

    /// The topic_ids filter returns exactly the requested topics, in order.
    #[test]
    fn topic_ids_filter() -> Result<()> {
        let mut col = Collection::new();
        let c = add_card(&mut col, CardType::Review, 30, None);
        let r = add_card(&mut col, CardType::Review, 30, None);
        let n = add_card(&mut col, CardType::Review, 30, None);
        seed(
            &mut col,
            &["cardio", "renal", "neuro"],
            &[(c.0, "cardio"), (r.0, "renal"), (n.0, "neuro")],
        );

        let resp = col
            .get_topic_mastery(GetTopicMasteryRequest {
                topic_ids: vec!["renal".into()],
            })?
            .topics;

        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0].topic_id, "renal");
        assert_eq!(resp[0].total, 1);
        Ok(())
    }
}
