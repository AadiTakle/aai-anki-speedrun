// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

mod burying;
mod gathering;
pub(crate) mod intersperser;
pub(crate) mod sized_chain;
mod sorting;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::VecDeque;

use intersperser::Intersperser;
use sized_chain::SizedChain;

use super::BuryMode;
use super::CardQueues;
use super::Counts;
use super::LearningQueueEntry;
use super::MainQueueEntry;
use super::MainQueueEntryKind;
use crate::deckconfig::NewCardGatherPriority;
use crate::deckconfig::NewCardSortOrder;
use crate::deckconfig::ReviewCardOrder;
use crate::deckconfig::ReviewMix;
use crate::decks::limits::LimitTreeMap;
use crate::prelude::*;
use crate::scheduler::states::load_balancer::LoadBalancer;
use crate::scheduler::timing::SchedTimingToday;

/// Temporary holder for review cards that will be built into a queue.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DueCard {
    pub id: CardId,
    pub note_id: NoteId,
    pub mtime: TimestampSecs,
    pub due: i32,
    pub current_deck_id: DeckId,
    pub original_deck_id: DeckId,
    pub kind: DueCardKind,
    pub reps: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DueCardKind {
    Review,
    Learning,
}

/// Temporary holder for new cards that will be built into a queue.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NewCard {
    pub id: CardId,
    pub note_id: NoteId,
    pub mtime: TimestampSecs,
    pub current_deck_id: DeckId,
    pub original_deck_id: DeckId,
    pub template_index: u32,
    pub hash: u64,
}

impl From<DueCard> for MainQueueEntry {
    fn from(c: DueCard) -> Self {
        MainQueueEntry {
            id: c.id,
            mtime: c.mtime,
            kind: match c.kind {
                DueCardKind::Review => MainQueueEntryKind::Review,
                DueCardKind::Learning => MainQueueEntryKind::InterdayLearning,
            },
        }
    }
}

impl From<NewCard> for MainQueueEntry {
    fn from(c: NewCard) -> Self {
        MainQueueEntry {
            id: c.id,
            mtime: c.mtime,
            kind: MainQueueEntryKind::New,
        }
    }
}

impl From<DueCard> for LearningQueueEntry {
    fn from(c: DueCard) -> Self {
        LearningQueueEntry {
            due: TimestampSecs(c.due as i64),
            id: c.id,
            mtime: c.mtime,
            reps: c.reps,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub(super) struct QueueSortOptions {
    pub(super) new_order: NewCardSortOrder,
    pub(super) new_gather_priority: NewCardGatherPriority,
    pub(super) review_order: ReviewCardOrder,
    pub(super) day_learn_mix: ReviewMix,
    pub(super) new_review_mix: ReviewMix,
}

#[derive(Debug)]
pub(super) struct QueueBuilder {
    pub(super) new: Vec<NewCard>,
    pub(super) review: Vec<DueCard>,
    pub(super) learning: Vec<DueCard>,
    pub(super) day_learning: Vec<DueCard>,
    limits: LimitTreeMap,
    load_balancer: Option<LoadBalancer>,
    context: Context,
}

/// Data container and helper for building queues.
#[derive(Debug, Clone)]
struct Context {
    timing: SchedTimingToday,
    config_map: HashMap<DeckConfigId, DeckConfig>,
    root_deck: Deck,
    sort_options: QueueSortOptions,
    seen_note_ids: HashMap<NoteId, BuryMode>,
    deck_map: HashMap<DeckId, Deck>,
    fsrs: bool,
}

impl QueueBuilder {
    pub(super) fn new(col: &mut Collection, deck_id: DeckId) -> Result<Self> {
        let timing = col.timing_for_timestamp(TimestampSecs::now())?;
        let new_cards_ignore_review_limit = col.get_config_bool(BoolKey::NewCardsIgnoreReviewLimit);
        let apply_all_parent_limits = col.get_config_bool(BoolKey::ApplyAllParentLimits);
        let config_map = col.storage.get_deck_config_map()?;
        let root_deck = col.storage.get_deck(deck_id)?.or_not_found(deck_id)?;
        let mut decks = col.storage.child_decks(&root_deck)?;
        decks.insert(0, root_deck.clone());
        if apply_all_parent_limits {
            for parent in col.storage.parent_decks(&root_deck)? {
                decks.insert(0, parent);
            }
        }
        let limits = LimitTreeMap::build(
            &decks,
            &config_map,
            timing.days_elapsed,
            new_cards_ignore_review_limit,
        );
        let sort_options = sort_options(&root_deck, &config_map);
        let deck_map = col.storage.get_decks_map()?;

        let load_balancer = col
            .get_config_bool(BoolKey::LoadBalancerEnabled)
            .then(|| {
                let did_to_dcid = deck_map
                    .values()
                    .filter_map(|deck| Some((deck.id, deck.config_id()?)))
                    .collect::<HashMap<_, _>>();
                LoadBalancer::new(
                    timing.days_elapsed,
                    did_to_dcid,
                    col.timing_today()?.next_day_at,
                    &col.storage,
                )
            })
            .transpose()?;

        Ok(QueueBuilder {
            new: Vec::new(),
            review: Vec::new(),
            learning: Vec::new(),
            day_learning: Vec::new(),
            limits,
            load_balancer,
            context: Context {
                timing,
                config_map,
                root_deck,
                sort_options,
                seen_note_ids: HashMap::new(),
                deck_map,
                fsrs: col.get_config_bool(BoolKey::Fsrs),
            },
        })
    }

    pub(super) fn build(mut self, learn_ahead_secs: i64) -> CardQueues {
        self.sort_new();

        // intraday learning and total learn count
        let intraday_learning = sort_learning(self.learning);
        let now = TimestampSecs::now();
        let cutoff = now.adding_secs(learn_ahead_secs);
        let learn_count =
            intraday_learning.iter().filter(|e| e.due <= cutoff).count() + self.day_learning.len();
        let review_count = self.review.len();
        let new_count = self.new.len();

        // merge interday and new cards into main
        let with_interday_learn = merge_day_learning(
            self.review,
            self.day_learning,
            self.context.sort_options.day_learn_mix,
        );
        let main_iter = merge_new(
            with_interday_learn,
            self.new,
            self.context.sort_options.new_review_mix,
        );

        CardQueues {
            counts: Counts {
                new: new_count,
                review: review_count,
                learning: learn_count,
            },
            main: main_iter.collect(),
            intraday_learning,
            learn_ahead_secs,
            current_day: self.context.timing.days_elapsed,
            build_time: TimestampMillis::now(),
            load_balancer: self.load_balancer,
            current_learning_cutoff: now,
        }
    }
}

fn sort_options(deck: &Deck, config_map: &HashMap<DeckConfigId, DeckConfig>) -> QueueSortOptions {
    deck.config_id()
        .and_then(|config_id| config_map.get(&config_id))
        .map(|config| QueueSortOptions {
            new_order: config.inner.new_card_sort_order(),
            new_gather_priority: config.inner.new_card_gather_priority(),
            review_order: config.inner.review_order(),
            day_learn_mix: config.inner.interday_learning_mix(),
            new_review_mix: config.inner.new_mix(),
        })
        .unwrap_or_else(|| {
            // filtered decks do not space siblings
            QueueSortOptions {
                new_order: NewCardSortOrder::NoSort,
                ..Default::default()
            }
        })
}

fn merge_day_learning(
    reviews: Vec<DueCard>,
    day_learning: Vec<DueCard>,
    mode: ReviewMix,
) -> Box<dyn ExactSizeIterator<Item = MainQueueEntry>> {
    let day_learning_iter = day_learning.into_iter().map(Into::into);
    let reviews_iter = reviews.into_iter().map(Into::into);

    match mode {
        ReviewMix::AfterReviews => Box::new(SizedChain::new(reviews_iter, day_learning_iter)),
        ReviewMix::BeforeReviews => Box::new(SizedChain::new(day_learning_iter, reviews_iter)),
        ReviewMix::MixWithReviews => Box::new(Intersperser::new(reviews_iter, day_learning_iter)),
    }
}

fn merge_new(
    review_iter: impl ExactSizeIterator<Item = MainQueueEntry> + 'static,
    new: Vec<NewCard>,
    mode: ReviewMix,
) -> Box<dyn ExactSizeIterator<Item = MainQueueEntry>> {
    let new_iter = new.into_iter().map(Into::into);

    match mode {
        ReviewMix::BeforeReviews => Box::new(SizedChain::new(new_iter, review_iter)),
        ReviewMix::AfterReviews => Box::new(SizedChain::new(review_iter, new_iter)),
        ReviewMix::MixWithReviews => Box::new(Intersperser::new(review_iter, new_iter)),
    }
}

fn sort_learning(learning: Vec<DueCard>) -> VecDeque<LearningQueueEntry> {
    let mut entries: Vec<LearningQueueEntry> =
        learning.into_iter().map(LearningQueueEntry::from).collect();
    entries.sort_by(|a, b| a.cmp_by_reps_then_due(b));
    entries.into_iter().collect()
}

impl Collection {
    pub(crate) fn build_queues(&mut self, deck_id: DeckId) -> Result<CardQueues> {
        let mut queues = QueueBuilder::new(self, deck_id)?;
        self.storage
            .update_active_decks(&queues.context.root_deck)?;

        queues.gather_cards(self)?;

        // Points-at-stake is a Rust post-reorder of the gathered review cards.
        // Rather than a plain descending sort by
        // blueprint_weight(topic) * weakness(topic) — which would block a whole
        // topic back-to-back — it recency-decay *interleaves* topics so the
        // highest points-at-stake topics come up early and often while still
        // spreading topics out (see `sort_review_by_points_at_stake`). The SQL
        // gather uses the Day order (see storage/card review_order_sql), then we
        // reorder the gathered set here, where the collection/config store is
        // available. Note: this reorders within the already-limit-capped gather
        // window (documented in docs/wednesday_plan.md §8).
        if queues.context.sort_options.review_order
            == crate::deckconfig::ReviewCardOrder::PointsAtStake
        {
            self.sort_review_by_points_at_stake(&mut queues.review)?;
        }

        let queues = queues.build(self.learn_ahead_secs() as i64);

        Ok(queues)
    }

    /// Reorder gathered review cards by **recency-decayed weighted
    /// interleaving** of each card's topic points-at-stake
    /// (`base = blueprint_weight * weakness`).
    ///
    /// A plain descending sort by `base` groups an entire topic into one
    /// back-to-back block; interleaving keeps the highest-`base` topics coming
    /// up early and often while still spreading topics out, so no single topic
    /// dominates a long run. The reorder is deterministic and read-only (it
    /// only permutes the gathered vec — no DB writes, no mutation).
    ///
    /// Algorithm:
    /// 1. Partition cards, preserving gather order, into `positive` (topic
    ///    `base > 0`) and `zero` (`base == 0`: unmapped card, or a topic with a
    ///    missing/zero weight or weakness). `zero` cards keep their gather
    ///    order and are appended last (preserving the prior "unmapped sorts
    ///    last" behavior).
    /// 2. Interleave `positive` by topic. Group its cards into per-topic FIFO
    ///    queues (gather order within a topic). Start every involved topic at
    ///    `recency = 1` (equal start ⇒ the first pick is the highest-`base`
    ///    topic). Then loop until all queues are empty: for each topic with a
    ///    non-empty queue compute `eff = base * recency`, pop the front card of
    ///    the arg-max topic (tie-break: higher `base`, then the smaller gather
    ///    index of the topic's next card — fully deterministic), then increment
    ///    `recency` for every involved topic and reset the just-picked topic's
    ///    `recency` to 0.
    /// 3. Final order = interleaved positives, then the zero cards.
    ///
    /// Uses `f64`; non-finite scores are treated as 0 (so such cards sort
    /// last). Equal-`base` topics with one card each fall through to the
    /// gather-index tie-break, i.e. they keep gather order.
    fn sort_review_by_points_at_stake(&self, review: &mut [DueCard]) -> Result<()> {
        let topics = self.speedrun_topics()?;
        let weakness = self.speedrun_topic_weakness()?;
        let card_topics = self.speedrun_card_topics()?;

        // base(topic) = blueprint_weight * weakness, guarded to a finite,
        // strictly-positive value (anything else -> 0.0).
        let base_for_topic = |topic_id: &str| -> f64 {
            let weight = topics
                .get(topic_id)
                .map(|t| t.blueprint_weight)
                .unwrap_or(0.0);
            let weak = weakness.get(topic_id).copied().unwrap_or(0.0);
            let base = weight * weak;
            if base.is_finite() && base > 0.0 {
                base
            } else {
                0.0
            }
        };

        // Partition, preserving gather order: positive-base cards go into
        // per-topic FIFO queues (of gather indices); zero-base cards are held
        // aside to be appended last. `topic_order` records topics in first-seen
        // order so iteration is deterministic.
        let mut topic_order: Vec<String> = Vec::new();
        let mut queues: HashMap<String, VecDeque<usize>> = HashMap::new();
        let mut base_by_topic: HashMap<String, f64> = HashMap::new();
        let mut zero: Vec<usize> = Vec::new();

        for (idx, card) in review.iter().enumerate() {
            let base = card_topics
                .get(&card.id)
                .map(|topic_id| base_for_topic(topic_id))
                .unwrap_or(0.0);
            if base > 0.0 {
                // base > 0 implies a topic mapping exists.
                let topic_id = card_topics.get(&card.id).unwrap();
                if !queues.contains_key(topic_id) {
                    topic_order.push(topic_id.clone());
                    base_by_topic.insert(topic_id.clone(), base);
                }
                queues.entry(topic_id.clone()).or_default().push_back(idx);
            } else {
                zero.push(idx);
            }
        }

        // Recency-decayed interleave of the positive-base cards.
        let mut recency: HashMap<String, f64> =
            topic_order.iter().map(|t| (t.clone(), 1.0)).collect();
        let mut interleaved: Vec<usize> = Vec::with_capacity(review.len());
        loop {
            // Pick the highest-priority non-empty topic: max eff = base *
            // recency, tie-break on higher base, then the smaller gather index
            // of the topic's next card (globally unique ⇒ fully deterministic).
            let next = topic_order
                .iter()
                .filter_map(|topic| {
                    queues[topic]
                        .front()
                        .map(|&front| (topic, base_by_topic[topic] * recency[topic], front))
                })
                .max_by(|a, b| {
                    a.1.partial_cmp(&b.1)
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| {
                            base_by_topic[a.0]
                                .partial_cmp(&base_by_topic[b.0])
                                .unwrap_or(Ordering::Equal)
                        })
                        // smaller gather index wins the final tie-break
                        .then_with(|| b.2.cmp(&a.2))
                });
            let Some((topic, _eff, _front)) = next else {
                break; // all queues empty
            };
            let topic = topic.clone();

            let idx = queues.get_mut(&topic).unwrap().pop_front().unwrap();
            interleaved.push(idx);

            // Every involved topic ages by 1; the just-picked topic resets to 0.
            for r in recency.values_mut() {
                *r += 1.0;
            }
            recency.insert(topic, 0.0);
        }

        // Final order = interleaved positives, then the zero-base cards (which
        // kept their gather order). Reorder `review` in place to match.
        let mut order = interleaved;
        order.extend(zero);
        let reordered: Vec<DueCard> = order.iter().map(|&i| review[i]).collect();
        review.copy_from_slice(&reordered);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anki_proto::deck_config::deck_config::config::NewCardGatherPriority;
    use anki_proto::deck_config::deck_config::config::NewCardSortOrder;

    use super::*;
    use crate::card::CardQueue;
    use crate::card::CardType;

    impl Collection {
        fn set_deck_gather_order(&mut self, deck: &mut Deck, order: NewCardGatherPriority) {
            let mut conf = DeckConfig::default();
            conf.inner.new_card_gather_priority = order as i32;
            conf.inner.new_card_sort_order = NewCardSortOrder::NoSort as i32;
            self.add_or_update_deck_config(&mut conf).unwrap();
            deck.normal_mut().unwrap().config_id = conf.id.0;
            self.add_or_update_deck(deck).unwrap();
        }

        fn set_deck_new_limit(&mut self, deck: &mut Deck, new_limit: u32) {
            let mut conf = DeckConfig::default();
            conf.inner.new_per_day = new_limit;
            self.add_or_update_deck_config(&mut conf).unwrap();
            deck.normal_mut().unwrap().config_id = conf.id.0;
            self.add_or_update_deck(deck).unwrap();
        }

        fn set_deck_review_limit(&mut self, deck: DeckId, limit: u32) {
            let dcid = self.get_deck(deck).unwrap().unwrap().config_id().unwrap();
            let mut conf = self.get_deck_config(dcid, false).unwrap().unwrap();
            conf.inner.reviews_per_day = limit;
            self.add_or_update_deck_config(&mut conf).unwrap();
        }

        fn queue_as_deck_and_template(&mut self, deck_id: DeckId) -> Vec<(DeckId, u16)> {
            self.build_queues(deck_id)
                .unwrap()
                .iter()
                .map(|entry| {
                    let card = self.storage.get_card(entry.card_id()).unwrap().unwrap();
                    (card.deck_id, card.template_idx)
                })
                .collect()
        }

        fn set_deck_review_order(&mut self, deck: &mut Deck, order: ReviewCardOrder) {
            let mut conf = DeckConfig::default();
            conf.inner.review_order = order as i32;
            self.add_or_update_deck_config(&mut conf).unwrap();
            deck.normal_mut().unwrap().config_id = conf.id.0;
            self.add_or_update_deck(deck).unwrap();
        }

        fn queue_as_due_and_ivl(&mut self, deck_id: DeckId) -> Vec<(i32, u32)> {
            self.build_queues(deck_id)
                .unwrap()
                .iter()
                .map(|entry| {
                    let card = self.storage.get_card(entry.card_id()).unwrap().unwrap();
                    (card.due, card.interval)
                })
                .collect()
        }

        fn queue_as_card_ids(&mut self, deck_id: DeckId) -> Vec<CardId> {
            self.build_queues(deck_id)
                .unwrap()
                .iter()
                .map(|entry| entry.card_id())
                .collect()
        }

        /// Adds a note whose first card is turned into a due review card, and
        /// returns that card's id.
        fn add_due_review_card(&mut self, nt: &Notetype, deck: DeckId) -> CardId {
            let mut note = nt.new_note();
            note.set_field(0, "foo").unwrap();
            note.id.0 = 0;
            self.add_note(&mut note, deck).unwrap();
            let mut card = self
                .storage
                .get_card_by_ordinal(note.id, 0)
                .unwrap()
                .unwrap();
            // All test review cards share the same due/interval so the gather-time
            // SQL ordering is a tie and the points-at-stake post-sort dominates.
            card.interval = 10;
            card.due = 0;
            card.ctype = CardType::Review;
            card.queue = CardQueue::Review;
            let id = card.id;
            self.update_cards_maybe_undoable(vec![card], false).unwrap();
            id
        }

        fn seed_topic_weights(
            &mut self,
            topics: &[(&str, f64)],
            weaknesses: &[(&str, f64)],
            card_topics: &[(CardId, &str)],
        ) {
            use anki_proto::speedrun::CardTopic;
            use anki_proto::speedrun::SetTopicWeightsRequest;
            use anki_proto::speedrun::Topic;
            use anki_proto::speedrun::TopicWeakness;

            use crate::services::SpeedrunService;

            let req = SetTopicWeightsRequest {
                topics: topics
                    .iter()
                    .map(|(id, weight)| Topic {
                        id: (*id).to_string(),
                        name: (*id).to_string(),
                        blueprint_weight: *weight,
                    })
                    .collect(),
                card_topics: card_topics
                    .iter()
                    .map(|(card_id, topic_id)| CardTopic {
                        card_id: card_id.0,
                        topic_id: (*topic_id).to_string(),
                    })
                    .collect(),
                weaknesses: weaknesses
                    .iter()
                    .map(|(topic_id, weakness)| TopicWeakness {
                        topic_id: (*topic_id).to_string(),
                        weakness: *weakness,
                    })
                    .collect(),
            };
            let _ = self.set_topic_weights(req).unwrap();
        }
    }

    /// R1: with topics weighted and per-topic weakness seeded, and due review
    /// cards mapped to topics, the points-at-stake order returns review cards
    /// sorted by `blueprint_weight * weakness` descending.
    #[test]
    fn points_at_stake_orders_by_weight_times_weakness() -> Result<()> {
        let mut col = Collection::new();
        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();

        // points-at-stake: cardio 0.5*0.9=0.45, renal 0.4*0.5=0.20, gi 0.2*0.3=0.06
        let cardio = col.add_due_review_card(&nt, deck.id);
        let renal = col.add_due_review_card(&nt, deck.id);
        let gi = col.add_due_review_card(&nt, deck.id);

        col.seed_topic_weights(
            &[("cardio", 0.5), ("renal", 0.4), ("gi", 0.2)],
            &[("cardio", 0.9), ("renal", 0.5), ("gi", 0.3)],
            &[(cardio, "cardio"), (renal, "renal"), (gi, "gi")],
        );

        col.set_deck_review_order(&mut deck, ReviewCardOrder::PointsAtStake);
        assert_eq!(col.queue_as_card_ids(deck.id), vec![cardio, renal, gi]);

        Ok(())
    }

    /// R2: a due card whose topic has no weight/weakness (or no mapping at all)
    /// scores 0.0 and sorts last; no panic, no skipped cards.
    #[test]
    fn points_at_stake_unmapped_card_sorts_last() -> Result<()> {
        let mut col = Collection::new();
        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();

        let cardio = col.add_due_review_card(&nt, deck.id);
        let renal = col.add_due_review_card(&nt, deck.id);
        // No topic mapping for this card -> score 0.0 -> sorts last.
        let orphan = col.add_due_review_card(&nt, deck.id);

        col.seed_topic_weights(
            &[("cardio", 0.5), ("renal", 0.4)],
            &[("cardio", 0.9), ("renal", 0.5)],
            &[(cardio, "cardio"), (renal, "renal")],
        );

        col.set_deck_review_order(&mut deck, ReviewCardOrder::PointsAtStake);
        let queue = col.queue_as_card_ids(deck.id);
        assert_eq!(queue.len(), 3, "no cards skipped");
        assert_eq!(queue, vec![cardio, renal, orphan]);

        Ok(())
    }

    /// R3: building + answering through the points-at-stake queue, then
    /// undoing, restores prior state and leaves the database uncorrupted.
    #[test]
    fn points_at_stake_answer_then_undo_is_safe() -> Result<()> {
        let mut col = Collection::new();
        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();

        let cardio = col.add_due_review_card(&nt, deck.id);
        let renal = col.add_due_review_card(&nt, deck.id);

        col.seed_topic_weights(
            &[("cardio", 0.5), ("renal", 0.4)],
            &[("cardio", 0.9), ("renal", 0.5)],
            &[(cardio, "cardio"), (renal, "renal")],
        );

        col.set_deck_review_order(&mut deck, ReviewCardOrder::PointsAtStake);

        // The highest points-at-stake card must come out first.
        assert_eq!(col.queue_as_card_ids(deck.id)[0], cardio);

        // Snapshot the top card before answering.
        let before = col.storage.get_card(cardio)?.unwrap();

        col.clear_study_queues();
        let answered = col.answer_good();
        assert_eq!(answered.card_id, cardio);

        // Answering changed the card.
        let after = col.storage.get_card(cardio)?.unwrap();
        assert_ne!(after.reps, before.reps);

        // Undo restores the previous state.
        col.undo()?;
        let restored = col.storage.get_card(cardio)?.unwrap();
        assert_eq!(restored.reps, before.reps);
        assert_eq!(restored.due, before.due);
        assert_eq!(restored.interval, before.interval);
        assert_eq!(restored.ctype, before.ctype);
        assert_eq!(restored.queue, before.queue);

        // No corruption introduced.
        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");

        Ok(())
    }

    /// Deterministic tie-break: cards whose topics have EQUAL points-at-stake
    /// (`weight * weakness`) keep their gather-time order, because the
    /// post-sort is stable. Verified by comparing against the plain gather
    /// order (Day), which is what points-at-stake gathers with before
    /// reordering.
    #[test]
    fn points_at_stake_equal_scores_keep_gather_order() -> Result<()> {
        let mut col = Collection::new();
        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();

        // Three topics with identical points-at-stake: 0.5*0.4 == 0.4*0.5 ==
        // 0.2*1.0 == 0.20, so the sort key is a three-way tie.
        let a = col.add_due_review_card(&nt, deck.id);
        let b = col.add_due_review_card(&nt, deck.id);
        let c = col.add_due_review_card(&nt, deck.id);

        col.seed_topic_weights(
            &[("t_a", 0.5), ("t_b", 0.4), ("t_c", 0.2)],
            &[("t_a", 0.4), ("t_b", 0.5), ("t_c", 1.0)],
            &[(a, "t_a"), (b, "t_b"), (c, "t_c")],
        );

        // Gather order under the plain Day order (what points-at-stake gathers
        // with before the post-sort).
        col.set_deck_review_order(&mut deck, ReviewCardOrder::Day);
        let gather_order = col.queue_as_card_ids(deck.id);

        col.set_deck_review_order(&mut deck, ReviewCardOrder::PointsAtStake);
        let pas_order = col.queue_as_card_ids(deck.id);

        assert_eq!(
            pas_order, gather_order,
            "equal points-at-stake must preserve gather order (stable sort)"
        );

        Ok(())
    }

    /// R4 (product-owner redesign): points-at-stake no longer blocks a whole
    /// topic back-to-back; it recency-decay interleaves topics by
    /// `base = blueprint_weight * weakness`. With three topics of distinct base
    /// (cardio 0.45, nephro 0.20, gastro 0.06) and three cards each, the queued
    /// topic sequence must match the deterministic worked example exactly: the
    /// dominant topic leads and recurs early/often, yet no topic is ever shown
    /// twice in a row.
    #[test]
    fn points_at_stake_interleaves_by_recency() -> Result<()> {
        let mut col = Collection::new();
        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();

        // Three topics, three cards each. base = weight * weakness:
        // cardio 0.5*0.9=0.45, nephro 0.4*0.5=0.20, gastro 0.2*0.3=0.06.
        let topic_ids = ["cardio", "nephro", "gastro"];
        let mut card_topic: HashMap<CardId, &str> = HashMap::new();
        let mut card_topics: Vec<(CardId, &str)> = Vec::new();
        // Round-robin insertion so the gather order is interleaved; the topic
        // sequence below is produced solely by the F5 recency interleave.
        for _ in 0..3 {
            for tid in topic_ids {
                let cid = col.add_due_review_card(&nt, deck.id);
                card_topic.insert(cid, tid);
                card_topics.push((cid, tid));
            }
        }

        col.seed_topic_weights(
            &[("cardio", 0.5), ("nephro", 0.4), ("gastro", 0.2)],
            &[("cardio", 0.9), ("nephro", 0.5), ("gastro", 0.3)],
            &card_topics,
        );

        col.set_deck_review_order(&mut deck, ReviewCardOrder::PointsAtStake);
        let seq: Vec<&str> = col
            .queue_as_card_ids(deck.id)
            .iter()
            .map(|cid| card_topic[cid])
            .collect();

        // Exact deterministic sequence from the product-owner worked example.
        assert_eq!(
            seq,
            vec![
                "cardio", "nephro", "cardio", "gastro", "cardio", "nephro", "gastro", "nephro",
                "gastro"
            ],
            "recency-decayed interleave must match the worked example"
        );
        // No topic is ever shown twice in a row (max consecutive run == 1).
        assert!(
            seq.windows(2).all(|w| w[0] != w[1]),
            "no topic should repeat consecutively: {seq:?}"
        );
        // Each of the three topics appears exactly three times.
        for tid in topic_ids {
            assert_eq!(
                seq.iter().filter(|t| **t == tid).count(),
                3,
                "topic {tid} should appear exactly 3 times"
            );
        }

        Ok(())
    }

    #[test]
    fn should_build_empty_queue_if_limit_is_reached() {
        let mut col = Collection::new();
        CardAdder::new().due_dates(["0"]).add(&mut col);
        col.set_deck_review_limit(DeckId(1), 0);
        assert_eq!(col.queue_as_deck_and_template(DeckId(1)), vec![]);
    }

    #[test]
    fn new_queue_building() -> Result<()> {
        let mut col = Collection::new();

        // parent
        // ┣━━child━━grandchild
        // ┗━━child_2
        let mut parent = DeckAdder::new("parent").add(&mut col);
        let mut child = DeckAdder::new("parent::child").add(&mut col);
        let child_2 = DeckAdder::new("parent::child_2").add(&mut col);
        let grandchild = DeckAdder::new("parent::child::grandchild").add(&mut col);

        // add 2 new cards to each deck
        for deck in [&parent, &child, &child_2, &grandchild] {
            CardAdder::new().siblings(2).deck(deck.id).add(&mut col);
        }

        // set child's new limit to 3, which should affect grandchild
        col.set_deck_new_limit(&mut child, 3);

        // depth-first tree order
        col.set_deck_gather_order(&mut parent, NewCardGatherPriority::Deck);
        let cards = vec![
            (parent.id, 0),
            (parent.id, 1),
            (child.id, 0),
            (child.id, 1),
            (grandchild.id, 0),
            (child_2.id, 0),
            (child_2.id, 1),
        ];
        assert_eq!(col.queue_as_deck_and_template(parent.id), cards);

        // insertion order
        col.set_deck_gather_order(&mut parent, NewCardGatherPriority::LowestPosition);
        let cards = vec![
            (parent.id, 0),
            (parent.id, 1),
            (child.id, 0),
            (child.id, 1),
            (child_2.id, 0),
            (child_2.id, 1),
            (grandchild.id, 0),
        ];
        assert_eq!(col.queue_as_deck_and_template(parent.id), cards);

        // inverted insertion order, but sibling order is preserved
        col.set_deck_gather_order(&mut parent, NewCardGatherPriority::HighestPosition);
        let cards = vec![
            (grandchild.id, 0),
            (grandchild.id, 1),
            (child_2.id, 0),
            (child_2.id, 1),
            (child.id, 0),
            (parent.id, 0),
            (parent.id, 1),
        ];
        assert_eq!(col.queue_as_deck_and_template(parent.id), cards);

        Ok(())
    }

    #[test]
    fn review_queue_building() -> Result<()> {
        let mut col = Collection::new();

        let mut deck = col.get_or_create_normal_deck("Default").unwrap();
        let nt = col.get_notetype_by_name("Basic")?.unwrap();
        let mut cards = vec![];

        // relative overdueness
        let expected_queue = vec![
            (-150, 1),
            (-100, 1),
            (-50, 1),
            (-150, 5),
            (-100, 5),
            (-50, 5),
            (-150, 20),
            (-150, 20),
            (-100, 20),
            (-50, 20),
            (-150, 100),
            (-100, 100),
            (-50, 100),
            (0, 1),
            (0, 5),
            (0, 20),
            (0, 100),
        ];
        for t in expected_queue.iter() {
            let mut note = nt.new_note();
            note.set_field(0, "foo")?;
            note.id.0 = 0;
            col.add_note(&mut note, deck.id)?;
            let mut card = col.storage.get_card_by_ordinal(note.id, 0)?.unwrap();
            card.interval = t.1;
            card.due = t.0;
            card.ctype = CardType::Review;
            card.queue = CardQueue::Review;
            cards.push(card);
        }
        col.update_cards_maybe_undoable(cards, false)?;
        col.set_deck_review_order(&mut deck, ReviewCardOrder::RelativeOverdueness);
        assert_eq!(col.queue_as_due_and_ivl(deck.id), expected_queue);

        Ok(())
    }

    impl Collection {
        fn card_queue_len(&mut self) -> usize {
            self.get_queued_cards(5, false).unwrap().cards.len()
        }
    }

    #[test]
    fn new_card_potentially_burying_review_card() {
        let mut col = Collection::new();
        // add one new and one review card
        CardAdder::new().siblings(2).due_dates(["0"]).add(&mut col);
        // Potentially problematic config: New cards are shown first and would bury
        // review siblings. This poses a problem because we gather review cards first.
        col.update_default_deck_config(|config| {
            config.new_mix = ReviewMix::BeforeReviews as i32;
            config.bury_new = false;
            config.bury_reviews = true;
        });

        let old_queue_len = col.card_queue_len();
        col.answer_easy();
        col.clear_study_queues();

        // The number of cards in the queue must decrease by exactly 1, either because
        // no burying was performed, or the first built queue anticipated it and didn't
        // include the buried card.
        assert_eq!(col.card_queue_len(), old_queue_len - 1);
    }

    #[test]
    fn new_cards_may_ignore_review_limit() {
        let mut col = Collection::new();
        col.set_config_bool(BoolKey::NewCardsIgnoreReviewLimit, true, false)
            .unwrap();
        col.update_default_deck_config(|config| {
            config.reviews_per_day = 0;
        });
        CardAdder::new().add(&mut col);

        // review limit doesn't apply to new card
        assert_eq!(col.card_queue_len(), 1);
    }

    #[test]
    fn reviews_dont_affect_new_limit_before_review_limit_is_reached() {
        let mut col = Collection::new();
        col.update_default_deck_config(|config| {
            config.new_per_day = 1;
        });
        CardAdder::new().siblings(2).due_dates(["0"]).add(&mut col);
        assert_eq!(col.card_queue_len(), 2);
    }

    #[test]
    fn may_apply_parent_limits() {
        let mut col = Collection::new();
        col.set_config_bool(BoolKey::ApplyAllParentLimits, true, false)
            .unwrap();
        col.update_default_deck_config(|config| {
            config.new_per_day = 0;
        });
        let child = DeckAdder::new("Default::child")
            .with_config(|_| ())
            .add(&mut col);
        CardAdder::new().deck(child.id).add(&mut col);
        col.set_current_deck(child.id).unwrap();
        assert_eq!(col.card_queue_len(), 0);
    }
}
