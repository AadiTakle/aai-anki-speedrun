// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! F6 — **memory** score with an uncertainty range + the give-up/abstain rule.
//!
//! Rolls the F4 per-topic mastery up into a single blueprint-weighted memory
//! score, together with an honest uncertainty band and an explicit abstain
//! ("we can't score yet") rule. Read-only: it never mutates the collection.
//!
//! Definitions (memory's abstain was relaxed after the Wednesday contract so
//! the score appears as soon as reviews produce real FSRS recall — see ABSTAIN
//! RULE):
//! - **graded_reviews (n)**: the total row count of the `revlog` table.
//! - **covered topic**: a taxonomy topic that has `total >= 1` in the F4
//!   [`Collection::topic_mastery`] response (at least one existing mapped
//!   card).
//! - **coverage_pct**: `100 * (Σ blueprint_weight over covered topics) / (Σ
//!   blueprint_weight over ALL taxonomy topics)`; `0.0` when the total taxonomy
//!   weight is `0`. Reported for context — it does NOT gate the score.
//! - **point**: the blueprint-weighted mean recall over recall-backed covered
//!   topics — `100 * (Σ weight*avg_recall) / (Σ weight)` — clamped to `[0,
//!   100]`. A recall-backed topic is a covered topic whose `recall_card_count`
//!   is at least one; covered topics with no FSRS recall data are excluded so
//!   their sentinel `avg_recall` of `0.0` cannot depress the score with an
//!   unbacked number (honesty bar).
//! - **ABSTAIN RULE:** memory abstains ONLY when no covered topic is
//!   recall-backed (no reviewed card carries an FSRS memory state) — the honest
//!   floor: with no recall data there is simply no number to give. It
//!   deliberately does NOT gate on a fixed review count or coverage %: the
//!   moment reviews produce real FSRS recall the score is shown, and the
//!   uncertainty BAND below widens at low `n` so it stays honest about a thin
//!   sample. (The stricter review-volume / whole-blueprint-coverage gates
//!   belong to READINESS — the exam-day projection — not to this descriptive
//!   recall stat.) When abstained, `point/low/high` are all `0.0` (the real
//!   `coverage_pct` is still reported) and `reasons` names why. Otherwise a
//!   non-degenerate band bounded by `0 <= low <= point <= high <= 100`, with
//!   `high` above `low`, is produced.
//! - **band**: the uncertainty half-width shrinks with `sqrt(n)` (with `n`
//!   floored at 1) and with higher coverage, so it is monotonically
//!   non-increasing in `n` (more graded reviews, with coverage & recall fixed,
//!   never widens the band) and strictly positive (never degenerate).
//! - **updated_at**: `TimestampSecs::now().0`.

use anki_proto::speedrun::MemoryScore;

use crate::prelude::*;

impl Collection {
    /// Compute the blueprint-weighted memory score with its uncertainty band
    /// and the abstain rule. See the module docs for the exact definitions.
    /// This is a read-only query: it never mutates the collection.
    pub(crate) fn memory_score(&mut self) -> Result<MemoryScore> {
        // graded_reviews (n): the total number of rows in the revlog table.
        let graded_reviews: i64 =
            self.storage
                .db
                .query_row("select count(*) from revlog", [], |row| row.get(0))?;

        // Blueprint weights for every taxonomy topic, and the F4 per-topic
        // mastery (covered => total >= 1) it rolls up.
        let topics = self.speedrun_topics()?;
        let total_weight: f64 = topics.values().map(|t| t.blueprint_weight).sum();
        let mastery = self.topic_mastery(vec![])?;

        let mut covered_weight = 0.0f64;
        let mut scored_weight = 0.0f64;
        let mut weighted_recall_sum = 0.0f64;
        let mut covered_topics = 0u32;
        for tm in &mastery.topics {
            if tm.total >= 1 {
                let weight = topics
                    .get(&tm.topic_id)
                    .map(|t| t.blueprint_weight)
                    .unwrap_or(0.0);
                covered_weight += weight;
                covered_topics += 1;
                // Only topics whose recall is actually backed by data contribute
                // to the recall average. A topic with cards but no FSRS memory
                // state reports avg_recall == 0.0 as a *sentinel*, not a real 0%
                // recall (see F4 `recall_card_count`); counting it would depress
                // the score with an unbacked number, violating the honesty bar.
                if tm.recall_card_count > 0 {
                    scored_weight += weight;
                    weighted_recall_sum += weight * tm.avg_recall;
                }
            }
        }

        let coverage_pct = if total_weight > 0.0 {
            100.0 * covered_weight / total_weight
        } else {
            0.0
        };

        let updated_at = TimestampSecs::now().0;

        // ABSTAIN RULE: the only honest floor is "do we have real recall data?".
        // If no covered topic is recall-backed (no reviewed card carries an FSRS
        // memory state) there is no number to give, so we abstain rather than
        // emit an unbacked 0 (honesty bar). We deliberately do NOT gate on a
        // fixed review count or coverage % — the moment reviews produce FSRS
        // recall the score appears, with the band (below) widening at low n to
        // stay honest about a thin sample. (Whole-blueprint coverage / review
        // volume are READINESS's gates, not this descriptive recall stat's.)
        if scored_weight <= 0.0 {
            let reason = if graded_reviews == 0 {
                "no reviews yet — study some cards to build a memory signal".to_string()
            } else {
                "reviewed cards have no FSRS recall data yet (enable FSRS so reviews \
                 record a memory state)"
                    .to_string()
            };
            return Ok(MemoryScore {
                abstained: true,
                point: 0.0,
                low: 0.0,
                high: 0.0,
                coverage_pct,
                reasons: vec![reason],
                updated_at,
            });
        }

        // Recall is averaged only over topics whose recall is backed by data
        // (scored_weight), never the full covered weight, so unbacked topics
        // don't drag the estimate down.
        let weighted_recall = weighted_recall_sum / scored_weight; // 0..1
        let point = (100.0 * weighted_recall).clamp(0.0, 100.0);

        // Uncertainty half-width (in score points). It shrinks with sqrt(n) so
        // it is monotonically non-increasing in the number of graded reviews
        // (coverage & recall fixed), and shrinks further as coverage rises. It
        // is strictly positive, so the [low, high] band is never degenerate.
        let coverage_frac = (coverage_pct / 100.0).clamp(0.0, 1.0);
        // n floored at 1 so the band is well-defined even when recall data exists
        // with very few (or zero) revlog rows; low n => a wide, honest band.
        let n_eff = graded_reviews.max(1) as f64;
        let half_width = 100.0 / n_eff.sqrt() * (1.0 - 0.5 * coverage_frac);

        let low = (point - half_width).max(0.0);
        let high = (point + half_width).min(100.0);

        let reasons = vec![
            format!("{graded_reviews} graded reviews"),
            format!("coverage {coverage_pct:.1}% of blueprint"),
            format!("weighted recall {:.1}%", 100.0 * weighted_recall),
            format!("{covered_topics} covered topics"),
        ];

        Ok(MemoryScore {
            abstained: false,
            point,
            low,
            high,
            coverage_pct,
            reasons,
            updated_at,
        })
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use fsrs::FSRS5_DEFAULT_DECAY;

    use super::*;
    use crate::card::CardQueue;
    use crate::card::CardType;
    use crate::card::FsrsMemoryState;
    use crate::services::SpeedrunService;

    /// Add a Review-state card directly (no note needed; the score only reads
    /// the cards table via the F4 mastery join). `memory`/`last_review` drive
    /// the FSRS recall used for the score's `point`.
    fn add_review_card(
        col: &mut Collection,
        interval: u32,
        memory: Option<FsrsMemoryState>,
        last_review: Option<TimestampSecs>,
    ) -> CardId {
        let mut card = Card {
            ctype: CardType::Review,
            queue: CardQueue::Review,
            interval,
            memory_state: memory,
            last_review_time: last_review,
            decay: memory.map(|_| FSRS5_DEFAULT_DECAY),
            ..Default::default()
        };
        col.add_card(&mut card).unwrap();
        card.id
    }

    /// Seed the taxonomy (topic id -> blueprint weight) and the card->topic
    /// crosswalk in one undo-safe write.
    fn seed_topics(col: &mut Collection, topics: &[(&str, f64)], crosswalk: &[(i64, &str)]) {
        let req = SetTopicWeightsRequest {
            topics: topics
                .iter()
                .map(|(id, weight)| Topic {
                    id: (*id).into(),
                    name: (*id).into(),
                    blueprint_weight: *weight,
                })
                .collect(),
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

    /// Insert `n` cheap rows straight into the revlog so `graded_reviews == n`.
    /// The row contents are irrelevant to the score — only the count matters.
    fn seed_revlog(col: &mut Collection, n: usize) {
        for i in 0..n {
            col.storage
                .db
                .execute(
                    "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type)\
                     values (?, 1, 0, 3, 10, 10, 2500, 0, 1)",
                    [i as i64 + 1],
                )
                .unwrap();
        }
    }

    fn revlog_count(col: &Collection) -> i64 {
        col.storage
            .db
            .query_row("select count(*) from revlog", [], |row| row.get(0))
            .unwrap()
    }

    /// 1. Memory no longer gates on a fixed review count: a covered topic with
    ///    real FSRS recall data scores even with very few graded reviews. The
    ///    band just stays wide (honest about the thin sample) instead of
    ///    abstaining — strictly wider than the same data at high n.
    #[test]
    fn scores_with_few_reviews_when_recall_backed() -> Result<()> {
        let mem = FsrsMemoryState {
            stability: 100.0,
            difficulty: 5.0,
        };

        // Few graded reviews (4), far below the old 200 gate, but the covered
        // topic carries an FSRS memory state -> real recall -> a score.
        let mut col = Collection::new();
        let a = add_review_card(&mut col, 30, Some(mem), Some(TimestampSecs::now()));
        seed_topics(&mut col, &[("cardio", 1.0)], &[(a.0, "cardio")]);
        seed_revlog(&mut col, 4);
        assert_eq!(revlog_count(&col), 4);
        let few = col.memory_score()?;
        assert!(!few.abstained, "few reviews but recall-backed must score");
        assert!(few.high > few.low, "band must be non-degenerate");
        assert!(few.low >= 0.0 && few.high <= 100.0, "band within [0,100]");

        // Same data with many reviews -> a strictly narrower band (sqrt(n)).
        let mut col2 = Collection::new();
        let b = add_review_card(&mut col2, 30, Some(mem), Some(TimestampSecs::now()));
        seed_topics(&mut col2, &[("cardio", 1.0)], &[(b.0, "cardio")]);
        seed_revlog(&mut col2, 400);
        let many = col2.memory_score()?;
        assert!(!many.abstained);
        assert!(
            (few.high - few.low) > (many.high - many.low),
            "low-n band {:.3} should be wider than high-n band {:.3}",
            few.high - few.low,
            many.high - many.low
        );
        Ok(())
    }

    /// 2. Memory no longer gates on 50% blueprint coverage: with real recall
    ///    data on the covered slice it scores, reporting the (low) coverage as
    ///    context rather than abstaining. Whole-blueprint coverage is
    ///    READINESS's gate, not this recall stat's.
    #[test]
    fn scores_at_low_coverage_when_recall_backed() -> Result<()> {
        let mut col = Collection::new();
        let mem = FsrsMemoryState {
            stability: 100.0,
            difficulty: 5.0,
        };
        // cardio (weight 1) covered; renal (weight 3) uncovered -> 25% coverage.
        let c = add_review_card(&mut col, 30, Some(mem), Some(TimestampSecs::now()));
        seed_topics(
            &mut col,
            &[("cardio", 1.0), ("renal", 3.0)],
            &[(c.0, "cardio")],
        );
        seed_revlog(&mut col, 250);

        let score = col.memory_score()?;
        assert!(
            !score.abstained,
            "recall-backed at 25% coverage must now score, not abstain"
        );
        assert!(
            score.coverage_pct < 50.0,
            "coverage {} is still reported as context",
            score.coverage_pct
        );
        assert!(score.point > 0.0, "a real recall-backed point");
        assert!(score.high > score.low, "band must be non-degenerate");
        Ok(())
    }

    /// 3. Enough reviews AND enough coverage -> score, with a valid
    ///    non-degenerate band and a fresh timestamp.
    #[test]
    fn scores_when_enough_data() -> Result<()> {
        let mut col = Collection::new();
        let mem = FsrsMemoryState {
            stability: 100.0,
            difficulty: 5.0,
        };
        let a = add_review_card(&mut col, 30, Some(mem), Some(TimestampSecs::now()));
        let b = add_review_card(&mut col, 40, Some(mem), Some(TimestampSecs::now()));
        seed_topics(
            &mut col,
            &[("cardio", 1.0), ("renal", 1.0)],
            &[(a.0, "cardio"), (b.0, "renal")],
        );
        seed_revlog(&mut col, 250);

        let score = col.memory_score()?;
        assert!(!score.abstained, "n=250 and coverage=100% must score");
        assert!(score.low >= 0.0, "low {} >= 0", score.low);
        assert!(
            score.low <= score.point,
            "low {} <= point {}",
            score.low,
            score.point
        );
        assert!(
            score.point <= score.high,
            "point {} <= high {}",
            score.point,
            score.high
        );
        assert!(score.high <= 100.0, "high {} <= 100", score.high);
        assert!(
            score.high > score.low,
            "band must be non-degenerate: low {} high {}",
            score.low,
            score.high
        );
        assert!(score.coverage_pct >= 50.0);
        assert!(!score.reasons.is_empty());
        assert!(score.updated_at > 0, "updated_at must be set");
        Ok(())
    }

    /// 4. Coverage weighting: two equal-weight topics with only one covered
    ///    gives ~50% coverage; and higher average recall yields a higher point.
    #[test]
    fn coverage_and_weighting() -> Result<()> {
        // Two equal-weight topics, only one covered -> coverage == 50.0.
        let mut col = Collection::new();
        let c = add_review_card(&mut col, 30, None, None);
        seed_topics(
            &mut col,
            &[("cardio", 1.0), ("renal", 1.0)],
            &[(c.0, "cardio")],
        );
        seed_revlog(&mut col, 250);
        let score = col.memory_score()?;
        assert!(
            (score.coverage_pct - 50.0).abs() < 1e-6,
            "one of two equal-weight topics covered -> 50%, got {}",
            score.coverage_pct
        );

        // Same shape, high vs low recall on the covered topic -> higher point.
        let point_high = {
            let mut col = Collection::new();
            let mem = FsrsMemoryState {
                stability: 100.0,
                difficulty: 5.0,
            };
            let a = add_review_card(&mut col, 30, Some(mem), Some(TimestampSecs::now()));
            seed_topics(&mut col, &[("cardio", 1.0)], &[(a.0, "cardio")]);
            seed_revlog(&mut col, 250);
            col.memory_score()?.point
        };
        let point_low = {
            let mut col = Collection::new();
            let mem = FsrsMemoryState {
                stability: 1.0,
                difficulty: 5.0,
            };
            // A long-ago review of a low-stability card -> low retrievability.
            let long_ago = TimestampSecs::now().adding_secs(-100 * 86_400);
            let a = add_review_card(&mut col, 30, Some(mem), Some(long_ago));
            seed_topics(&mut col, &[("cardio", 1.0)], &[(a.0, "cardio")]);
            seed_revlog(&mut col, 250);
            col.memory_score()?.point
        };
        assert!(
            point_high > point_low,
            "high recall {point_high} should exceed low recall {point_low}"
        );
        Ok(())
    }

    /// 5. Read-only + deterministic: two calls on fixed data agree, no undo
    ///    step is created, and the database stays uncorrupted.
    #[test]
    fn read_only_and_deterministic() -> Result<()> {
        let mut col = Collection::new();
        // A memory-less review card gives a recall of exactly 0.0, so the score
        // is fully deterministic (independent of wall-clock elapsed time).
        let a = add_review_card(&mut col, 30, None, None);
        seed_topics(&mut col, &[("cardio", 1.0)], &[(a.0, "cardio")]);
        seed_revlog(&mut col, 250);

        let undo_before = col.undo_status().last_step;
        let first = col.memory_score()?;
        let second = col.memory_score()?;
        let undo_after = col.undo_status().last_step;

        assert_eq!(first.point, second.point);
        assert_eq!(first.low, second.low);
        assert_eq!(first.high, second.high);
        assert_eq!(first.coverage_pct, second.coverage_pct);
        assert_eq!(first.abstained, second.abstained);
        assert_eq!(
            undo_before, undo_after,
            "memory_score must not create an undo step"
        );

        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }

    /// Honesty guard: even with enough reviews AND full coverage, a covered
    /// topic whose cards carry no FSRS memory state has no recall data, so the
    /// score abstains rather than emit an unbacked `0` — and says why.
    #[test]
    fn abstains_when_covered_but_no_recall_data() -> Result<()> {
        let mut col = Collection::new();
        // A due review card with NO memory state -> recall is unbacked.
        let a = add_review_card(&mut col, 30, None, None);
        seed_topics(&mut col, &[("cardio", 1.0)], &[(a.0, "cardio")]);
        seed_revlog(&mut col, 250);

        let score = col.memory_score()?;
        assert!(
            score.abstained,
            "covered but no recall data must abstain, not emit an unbacked 0"
        );
        assert_eq!(score.point, 0.0);
        assert_eq!(score.low, 0.0);
        assert_eq!(score.high, 0.0);
        assert!(
            score.coverage_pct >= 50.0,
            "coverage {} is still reported",
            score.coverage_pct
        );
        assert!(
            score.reasons.iter().any(|r| r.contains("recall data")),
            "a reason must explain the missing recall data: {:?}",
            score.reasons
        );
        Ok(())
    }
}
