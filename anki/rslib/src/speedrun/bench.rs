// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! F13 — benchmark harness support for the Speedrun engine features (challenge
//! 7h skeleton).
//!
//! Compiled only under the `bench` feature (mirroring
//! [`crate::card_rendering::anki_directive_benchmark`]), so it never touches the
//! normal library build or its public API. It provides:
//!
//! - [`build_synthetic_collection`]: a synthetic large-deck generator that
//!   builds an in-memory [`Collection`] with `card_count` due review cards
//!   spread round-robin across [`SYNTHETIC_TOPIC_COUNT`] topics, seeds the
//!   blueprint weights + per-topic weakness + the card->topic crosswalk, and
//!   selects the points-at-stake review order on the study deck. Enough revlog
//!   rows and full blueprint coverage are seeded so [`Collection::memory_score`]
//!   exercises its real scoring path rather than the early abstain return.
//! - Thin `run_*` wrappers exposing the three `pub(crate)` engine entry points
//!   ([`Collection::build_queues`] under `PointsAtStake`,
//!   [`Collection::topic_mastery`], [`Collection::memory_score`]) to the
//!   separate `benches/` crate, which can only see `pub` items.
//!
//! The generator is deterministic (no RNG), so repeated benchmark runs measure
//! the same workload.

use std::collections::HashMap;

use fsrs::FSRS5_DEFAULT_DECAY;

use crate::card::CardQueue;
use crate::card::CardType;
use crate::card::FsrsMemoryState;
use crate::collection::CollectionBuilder;
use crate::deckconfig::ReviewCardOrder;
use crate::prelude::*;
use crate::speedrun::store::TopicInfo;

/// Number of distinct taxonomy topics the synthetic deck spreads its cards
/// over (~20, matching the product's Step 2 CK blueprint granularity).
pub const SYNTHETIC_TOPIC_COUNT: usize = 20;

/// Default deck size the `benches/benchmark.rs` harness uses. 10k keeps a full
/// criterion run quick on a laptop while still being an order of magnitude
/// larger than any hand-written test; the challenge-7h stretch target is
/// 50_000 (pass a larger `card_count` to [`build_synthetic_collection`] to
/// measure that).
pub const SYNTHETIC_CARD_COUNT: usize = 10_000;

/// The default deck (always id 1) that the synthetic cards live in and that
/// [`run_build_queues_points_at_stake`] builds.
pub const SYNTHETIC_DECK_ID: DeckId = DeckId(1);

/// Revlog rows seeded so `memory_score` clears its `graded_reviews >= 200`
/// abstain threshold and scores for real. Only the row count matters to the
/// score, so these are cheap sentinel rows.
const SYNTHETIC_REVLOG_ROWS: usize = 500;

/// Anki's hard maximum for a deck's daily review limit (`reviews_per_day` is
/// validated to `[0, 9999]`). It is therefore the largest number of review
/// cards a single [`Collection::build_queues`] can gather, so the F5 benchmark
/// measures a full 9,999-card gather+reorder even when the deck is larger. F4
/// mastery and F6 score are unaffected — they iterate the whole crosswalk.
pub const MAX_DAILY_REVIEW_LIMIT: usize = 9_999;

/// Build an in-memory [`Collection`] populated with `card_count` due review
/// cards for benchmarking the Speedrun engine at scale. See the module docs for
/// the shape of the deck. Deterministic: identical for a given `card_count`.
pub fn build_synthetic_collection(card_count: usize) -> Collection {
    let mut col = CollectionBuilder::default()
        .build()
        .expect("in-memory collection");

    // Select the points-at-stake review order on the study deck, with the review
    // limit raised to Anki's maximum so build_queues gathers as many cards as it
    // ever will (`reviews_per_day` is validated to `[0, 9999]`; larger values are
    // silently reset to the 200 default). Sibling burying is off so distinct
    // synthetic notes are never collapsed.
    let mut deck = col
        .get_or_create_normal_deck("Default")
        .expect("default deck");
    let mut conf = DeckConfig::default();
    conf.inner.review_order = ReviewCardOrder::PointsAtStake as i32;
    conf.inner.reviews_per_day = MAX_DAILY_REVIEW_LIMIT as u32;
    conf.inner.new_per_day = MAX_DAILY_REVIEW_LIMIT as u32;
    conf.inner.bury_reviews = false;
    conf.inner.bury_new = false;
    col.add_or_update_deck_config(&mut conf)
        .expect("add deck config");
    deck.normal_mut().expect("normal deck").config_id = conf.id.0;
    col.add_or_update_deck(&mut deck).expect("update deck");

    // Add the review cards, recording each card's topic for the crosswalk.
    let mut crosswalk: HashMap<CardId, String> = HashMap::with_capacity(card_count);
    let now = TimestampSecs::now();
    for i in 0..card_count {
        let topic_id = synthetic_topic_id(i % SYNTHETIC_TOPIC_COUNT);
        // Spread intervals across / around the 21-day maturity threshold and
        // vary the FSRS memory state so mastery and recall aren't degenerate.
        let interval = 1 + (i % 60) as u32;
        let memory = FsrsMemoryState {
            stability: 10.0 + (i % 100) as f32,
            difficulty: 1.0 + (i % 9) as f32,
        };
        let last_review = now.adding_secs(-((i % 60) as i64) * 86_400);
        let mut card = Card {
            note_id: NoteId(i as i64 + 1),
            deck_id: SYNTHETIC_DECK_ID,
            ctype: CardType::Review,
            queue: CardQueue::Review,
            due: 0,
            interval,
            memory_state: Some(memory),
            last_review_time: Some(last_review),
            decay: Some(FSRS5_DEFAULT_DECAY),
            ..Default::default()
        };
        col.add_card(&mut card).expect("add synthetic card");
        crosswalk.insert(card.id, topic_id);
    }

    // Blueprint weights + per-topic weakness for every topic. Distinct values
    // give the points-at-stake sort (weight * weakness) a meaningful ordering,
    // and covering every topic keeps memory-score coverage at 100%.
    let mut topics: HashMap<String, TopicInfo> = HashMap::with_capacity(SYNTHETIC_TOPIC_COUNT);
    let mut weakness: HashMap<String, f64> = HashMap::with_capacity(SYNTHETIC_TOPIC_COUNT);
    for t in 0..SYNTHETIC_TOPIC_COUNT {
        let id = synthetic_topic_id(t);
        topics.insert(
            id.clone(),
            TopicInfo {
                name: id.clone(),
                blueprint_weight: 1.0 + t as f64 * 0.1,
            },
        );
        weakness.insert(id, (t as f64 + 1.0) / SYNTHETIC_TOPIC_COUNT as f64);
    }
    col.set_speedrun_topic_weights(topics, crosswalk, weakness)
        .expect("seed topic weights");

    seed_revlog(&mut col, SYNTHETIC_REVLOG_ROWS);

    col
}

/// Deterministic topic id for topic index `t` (e.g. `topic_07`).
fn synthetic_topic_id(t: usize) -> String {
    format!("topic_{t:02}")
}

/// Insert `n` cheap sentinel rows into the revlog so `memory_score` sees
/// `graded_reviews == n`. Mirrors the score module's test helper: only the row
/// count is read by the score, so the contents are placeholders.
fn seed_revlog(col: &mut Collection, n: usize) {
    for i in 0..n {
        col.storage
            .db
            .execute(
                "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
                 values (?, 1, 0, 3, 10, 10, 2500, 0, 1)",
                [i as i64 + 1],
            )
            .expect("seed revlog row");
    }
}

/// F5 — build the study queue under the points-at-stake review order and return
/// the number of gathered review cards (so the work isn't optimized away).
pub fn run_build_queues_points_at_stake(col: &mut Collection) -> usize {
    let mut queues = col
        .build_queues(SYNTHETIC_DECK_ID)
        .expect("build_queues under points-at-stake");
    queues.counts().review
}

/// F4 — compute per-topic mastery over the whole taxonomy and return the number
/// of topics scored.
pub fn run_topic_mastery(col: &mut Collection) -> usize {
    col.topic_mastery(vec![])
        .expect("topic_mastery over all topics")
        .topics
        .len()
}

/// F6 — compute the blueprint-weighted memory score and return its point
/// estimate.
pub fn run_memory_score(col: &mut Collection) -> f64 {
    col.memory_score().expect("memory_score").point
}
