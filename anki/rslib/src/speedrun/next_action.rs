// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! The single recommended "next block" for the Today console hero.
//!
//! Turns the ranked points-at-stake topics (the F5 display view, see
//! `crate::speedrun::focus`) into one concrete, right-sized study block: take
//! the highest points-at-stake topics that actually have cards studyable right
//! now, and recommend a capped block across them. Read-only: it reads the F1
//! topic store + card->topic crosswalk (`crate::speedrun::store`) and the live
//! cards table, and never mutates the collection.
//!
//! Definitions (frozen with the wave-2 contract):
//! - **studyable now ("due")**: a mapped card that still exists and whose
//!   `queue` is one of the active study queues — `New`, `Learn` (intraday
//!   learning), `DayLearn` (interday learning), or `Review`. Suspended, buried,
//!   and filtered-deck preview cards are not studyable and are excluded. This
//!   mirrors how the mastery query inspects `card.queue`
//!   (`crate::speedrun::mastery`); it is a queue-membership check, not a
//!   due-date computation.
//! - **candidate topics**: the points-at-stake topics with `points > 0` (i.e.
//!   `blueprint_weight * weakness > 0` — both weak *and* on the blueprint),
//!   already ordered highest-points-first by `Collection::points_at_stake`.
//! - **block_size**: `min(studyable cards across the chosen topics,
//!   BLOCK_CAP)`. We walk candidate topics highest-points-first, skip any with
//!   no studyable cards, and add topics until their cumulative studyable count
//!   reaches `BLOCK_CAP` (the block is "full") or we run out of candidates —
//!   i.e. the smallest set of top topics that fills the block. Chosen topic ids
//!   are returned highest-points-first.
//! - **abstain (honesty bar)**: when there is no signal we return `abstained =
//!   true` with a clear reason and an empty `headline`/`topic_ids` and
//!   `block_size = 0` — never a fabricated block. No signal means either no
//!   candidate topic (nothing both weak and on the blueprint) or no candidate
//!   topic has a single card studyable right now.

use std::collections::HashMap;

use anki_proto::speedrun::NextAction;

use crate::card::CardQueue;
use crate::prelude::*;

/// Largest number of cards we recommend in a single "next" block. A focused,
/// finishable block beats an intimidating backlog dump — ~20 cards is one
/// sitting, and keeps the Today hero honest about what "next" means.
const BLOCK_CAP: u32 = 20;

impl Collection {
    /// The one recommended next study block for the "Today" console hero.
    ///
    /// See the module docs for the exact `studyable`, `block_size`, and
    /// `abstain` definitions. Read-only; never mutates the collection.
    pub(crate) fn next_action(&self) -> Result<NextAction> {
        // Highest points-at-stake first (F5 display view). Only topics that are
        // both weak and on the blueprint (points > 0) can drive a block; a
        // topic with nothing at stake is never worth recommending.
        let candidates: Vec<_> = self
            .points_at_stake()?
            .topics
            .into_iter()
            .filter(|t| t.points > 0.0)
            .collect();
        if candidates.is_empty() {
            return Ok(abstain(
                "no weak topics with exam weight yet - import QBank results or set \
                 topic weakness to get a recommendation",
            ));
        }

        // Group the crosswalk by topic once so we can count studyable cards per
        // candidate without rescanning it.
        let mut cards_by_topic: HashMap<String, Vec<CardId>> = HashMap::new();
        for (card_id, topic_id) in self.speedrun_card_topics()? {
            cards_by_topic.entry(topic_id).or_default().push(card_id);
        }

        let mut topic_ids: Vec<String> = Vec::new();
        let mut names: Vec<String> = Vec::new();
        let mut studyable_total: u32 = 0;
        // The leading chosen topic (highest points that actually has due cards)
        // drives the human-readable reason.
        let mut driver_name = String::new();
        let mut driver_points = 0.0f64;

        for topic in &candidates {
            // Stop once the block is full: this yields the smallest set of top
            // topics that fills the block.
            if studyable_total >= BLOCK_CAP {
                break;
            }
            let count = match cards_by_topic.get(&topic.topic_id) {
                Some(card_ids) => self.studyable_card_count(card_ids)?,
                None => 0,
            };
            if count == 0 {
                // Nothing due here right now: it can't help fill the block and
                // listing it would be misleading, so skip it.
                continue;
            }
            if topic_ids.is_empty() {
                driver_name = topic.name.clone();
                driver_points = topic.points;
            }
            topic_ids.push(topic.topic_id.clone());
            names.push(topic.name.clone());
            studyable_total += count;
        }

        if topic_ids.is_empty() {
            // Weak, weighted topics exist, but none has a card studyable right
            // now (all suspended/buried, or none mapped) — abstain rather than
            // invent a block (honesty bar).
            return Ok(abstain(
                "your highest points-at-stake topics have no cards due right now",
            ));
        }

        let block_size = studyable_total.min(BLOCK_CAP);
        let headline = format!("Study {} cards across {}", block_size, names.join(", "));
        let reason = format!(
            "{} leads points-at-stake ({:.2}); {} card{} due across your weakest topic{}",
            driver_name,
            driver_points,
            block_size,
            if block_size == 1 { "" } else { "s" },
            if topic_ids.len() == 1 { "" } else { "s" },
        );

        Ok(NextAction {
            abstained: false,
            headline,
            block_size,
            topic_ids,
            reason,
        })
    }

    /// Count how many of the given crosswalk card ids are studyable right now:
    /// the card still exists and sits in an active study queue (see the module
    /// docs). Dangling ids (card deleted) are skipped.
    fn studyable_card_count(&self, card_ids: &[CardId]) -> Result<u32> {
        let mut count = 0u32;
        for &cid in card_ids {
            let Some(card) = self.storage.get_card(cid)? else {
                continue;
            };
            if is_studyable(card.queue) {
                count += 1;
            }
        }
        Ok(count)
    }
}

/// Whether a card in this queue is studyable right now. The active study queues
/// — `New`, intraday `Learn`, interday `DayLearn`, and `Review` — are
/// studyable; suspended, buried, and filtered-deck preview cards are not.
/// Mirrors the active-study convention used by `crate::speedrun::mastery`.
fn is_studyable(queue: CardQueue) -> bool {
    matches!(
        queue,
        CardQueue::New | CardQueue::Learn | CardQueue::DayLearn | CardQueue::Review
    )
}

/// Build an honest abstention: no block, just a reason (honesty bar).
fn abstain(reason: &str) -> NextAction {
    NextAction {
        abstained: true,
        reason: reason.to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use anki_proto::speedrun::TopicWeakness;

    use super::*;
    use crate::card::CardQueue;
    use crate::card::CardType;
    use crate::services::SpeedrunService;

    /// Add a bare card in the given queue and return its id. Only `card.queue`
    /// matters to the "studyable" check, so the card type is set to a sensible
    /// match and everything else is left at defaults (no note needed).
    fn add_card_in_queue(col: &mut Collection, queue: CardQueue) -> CardId {
        let ctype = match queue {
            CardQueue::New => CardType::New,
            CardQueue::Learn | CardQueue::DayLearn | CardQueue::PreviewRepeat => CardType::Learn,
            _ => CardType::Review,
        };
        let mut card = Card {
            ctype,
            queue,
            ..Default::default()
        };
        col.add_card(&mut card).unwrap();
        card.id
    }

    /// Seed the F1 taxonomy (id, name, blueprint_weight), the card->topic
    /// crosswalk, and per-topic weakness in one undo-safe write.
    fn seed(
        col: &mut Collection,
        topics: &[(&str, &str, f64)],
        crosswalk: &[(CardId, &str)],
        weakness: &[(&str, f64)],
    ) {
        let req = SetTopicWeightsRequest {
            topics: topics
                .iter()
                .map(|(id, name, w)| Topic {
                    id: (*id).into(),
                    name: (*name).into(),
                    blueprint_weight: *w,
                })
                .collect(),
            card_topics: crosswalk
                .iter()
                .map(|(cid, tid)| CardTopic {
                    card_id: cid.0,
                    topic_id: (*tid).into(),
                })
                .collect(),
            weaknesses: weakness
                .iter()
                .map(|(id, w)| TopicWeakness {
                    topic_id: (*id).into(),
                    weakness: *w,
                })
                .collect(),
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    /// Empty collection: no topics and no cards -> honest abstain, no block.
    #[test]
    fn abstains_on_empty_collection() -> Result<()> {
        let col = Collection::new();
        let action = col.next_action()?;
        assert!(action.abstained, "no topics/cards -> abstain");
        assert!(action.topic_ids.is_empty(), "abstain lists no topics");
        assert!(action.headline.is_empty(), "abstain has no headline");
        assert_eq!(action.block_size, 0, "abstain recommends no cards");
        assert!(!action.reason.is_empty(), "abstain must explain why");
        Ok(())
    }

    /// Weighted topics with studyable cards but zero weakness everywhere means
    /// nothing is at stake -> abstain (never invent a block from no signal).
    #[test]
    fn abstains_when_no_weakness_signal() -> Result<()> {
        let mut col = Collection::new();
        let cardio = add_card_in_queue(&mut col, CardQueue::Review);
        let renal = add_card_in_queue(&mut col, CardQueue::New);
        seed(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[(cardio, "cardio"), (renal, "renal")],
            &[], // no weakness anywhere -> points all zero
        );
        let action = col.next_action()?;
        assert!(action.abstained, "weakness all zero -> nothing at stake");
        assert!(action.topic_ids.is_empty());
        assert_eq!(action.block_size, 0);
        Ok(())
    }

    /// A weak, weighted topic whose only mapped card is suspended has nothing
    /// studyable right now -> abstain rather than fabricate a block.
    #[test]
    fn abstains_when_weak_topics_have_no_due_cards() -> Result<()> {
        let mut col = Collection::new();
        let suspended = add_card_in_queue(&mut col, CardQueue::Suspended);
        seed(
            &mut col,
            &[("cardio", "Cardiology", 0.5)],
            &[(suspended, "cardio")],
            &[("cardio", 0.9)],
        );
        let action = col.next_action()?;
        assert!(action.abstained, "weak topic but nothing due -> abstain");
        assert!(action.topic_ids.is_empty());
        assert_eq!(action.block_size, 0);
        Ok(())
    }

    /// With signal + due cards, the highest points-at-stake topic is
    /// recommended first, and the block names the driver topic.
    #[test]
    fn recommends_highest_points_topic_first() -> Result<()> {
        let mut col = Collection::new();
        // cardio points = 0.5 * 0.9 = 0.45 (highest); renal = 0.4 * 0.5 = 0.20.
        let cardio = add_card_in_queue(&mut col, CardQueue::Review);
        let renal = add_card_in_queue(&mut col, CardQueue::New);
        seed(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[(cardio, "cardio"), (renal, "renal")],
            &[("cardio", 0.9), ("renal", 0.5)],
        );
        let action = col.next_action()?;
        assert!(!action.abstained, "signal + due cards -> a real block");
        assert_eq!(
            action.topic_ids.first().map(String::as_str),
            Some("cardio"),
            "highest points-at-stake topic comes first"
        );
        assert_eq!(action.block_size, 2, "both due cards fit under the cap");
        assert!(
            action.headline.contains("Cardiology"),
            "headline names the driver topic: {}",
            action.headline
        );
        assert!(
            action.reason.contains("Cardiology"),
            "reason names the driver topic: {}",
            action.reason
        );
        Ok(())
    }

    /// Many due cards in the top topic -> block_size is capped at BLOCK_CAP,
    /// and a single topic fills the whole block.
    #[test]
    fn block_size_capped_when_many_due() -> Result<()> {
        let mut col = Collection::new();
        let mut crosswalk: Vec<(CardId, &str)> = Vec::new();
        for _ in 0..25 {
            let cid = add_card_in_queue(&mut col, CardQueue::Review);
            crosswalk.push((cid, "cardio"));
        }
        seed(
            &mut col,
            &[("cardio", "Cardiology", 0.5)],
            &crosswalk,
            &[("cardio", 0.9)],
        );
        let action = col.next_action()?;
        assert!(!action.abstained);
        assert_eq!(
            action.block_size, 20,
            "capped at BLOCK_CAP even with 25 due"
        );
        assert_eq!(
            action.topic_ids,
            vec!["cardio".to_string()],
            "one high-stakes topic fills the block"
        );
        assert!(
            action.headline.contains("Study 20 cards"),
            "headline reflects the capped size: {}",
            action.headline
        );
        Ok(())
    }

    /// Few due cards spread across topics -> block_size equals the studyable
    /// count (below the cap), suspended cards are excluded, and topic_ids come
    /// out ordered by points-at-stake descending.
    #[test]
    fn block_size_equals_due_count_and_orders_by_points() -> Result<()> {
        let mut col = Collection::new();
        // points: cardio 0.5*0.9=0.45, renal 0.4*0.8=0.32, gi 0.3*0.6=0.18.
        let cardio_a = add_card_in_queue(&mut col, CardQueue::Review);
        let cardio_b = add_card_in_queue(&mut col, CardQueue::Learn);
        // A suspended cardio card must NOT count toward the block.
        let cardio_suspended = add_card_in_queue(&mut col, CardQueue::Suspended);
        let renal = add_card_in_queue(&mut col, CardQueue::New);
        let gi = add_card_in_queue(&mut col, CardQueue::Review);
        seed(
            &mut col,
            &[
                ("cardio", "Cardiology", 0.5),
                ("renal", "Nephrology", 0.4),
                ("gi", "Gastroenterology", 0.3),
            ],
            &[
                (cardio_a, "cardio"),
                (cardio_b, "cardio"),
                (cardio_suspended, "cardio"),
                (renal, "renal"),
                (gi, "gi"),
            ],
            &[("cardio", 0.9), ("renal", 0.8), ("gi", 0.6)],
        );
        let action = col.next_action()?;
        assert!(!action.abstained);
        assert_eq!(
            action.block_size, 4,
            "sum of studyable cards below the cap (suspended excluded)"
        );
        assert_eq!(
            action.topic_ids,
            vec!["cardio".to_string(), "renal".to_string(), "gi".to_string()],
            "topic_ids ordered by points-at-stake descending"
        );
        Ok(())
    }
}
