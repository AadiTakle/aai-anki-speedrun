// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! **Performance** (application / accuracy) score — the second of the three
//! Speedrun scores, derived from the student's imported QBank question attempts
//! (PRD F2). Same honesty-bar shape as the F6 [`super::score`] memory score: a
//! point estimate with an uncertainty band, a real coverage figure, explicit
//! reasons, and an explicit abstain ("we can't score yet") rule. Read-only: it
//! never mutates the collection.
//!
//! Definitions (frozen contract):
//! - **covered topic**: a taxonomy topic (present in
//!   [`Collection::speedrun_topics`]) that has at least one graded attempt.
//! - **per-topic (`n` attempts, `k` correct)**:
//!   - `accuracy` = the Beta-Binomial posterior mean under a uniform
//!     `Beta(1,1)` prior — the Laplace rule `(k + 1) / (n + 2)`, in `[0, 1]`.
//!   - `[low, high]` = the dependency-free **Wilson score interval at 90%** (`z
//!     = 1.6449`) on `(k, n)`, clamped to `[0, 1]`. The Wilson interval always
//!     brackets both the raw rate `k/n` and the Laplace mean, so `low <=
//!     accuracy <= high`, and its half-width is strictly positive so the band
//!     is never degenerate.
//! - **point**: `100 * (Σ weight*accuracy) / (Σ weight)` over covered topics,
//!   clamped to `[0, 100]`. When the covered weight is `0` there is no honest
//!   basis for a weighted number, so we abstain.
//! - **low / high**: the same blueprint-weighted aggregation of the per-topic
//!   `low` / `high` (`×100`), clamped to `[0, 100]`; because every per-topic
//!   band brackets its accuracy and is strictly positive, the aggregate
//!   satisfies `0 <= low <= point <= high <= 100` with `high > low` when
//!   scored.
//! - **coverage_pct**: `100 * (Σ weight over covered topics) / (Σ weight over
//!   ALL taxonomy topics)`; `0.0` when the total taxonomy weight is `0`.
//! - **ABSTAIN RULE (frozen):** `abstained = (total_attempts < 50)`, where
//!   `total_attempts` is the sum of `n` over covered topics; plus a covered-
//!   weight guard (abstain when the covered weight is `0`, since a weighted
//!   mean is undefined). When abstained, `point/low/high` are all `0.0` while
//!   the real `coverage_pct` and the per-topic `topics` breakdown are still
//!   reported, and `reasons` names the shortfall with numbers.
//! - **updated_at**: `TimestampSecs::now().0`.

use std::collections::BTreeMap;

use anki_proto::speedrun::PerformanceScore;
use anki_proto::speedrun::TopicPerformance;

use crate::prelude::*;

/// Minimum graded attempts (summed over covered topics) before we are willing
/// to emit a performance score. Frozen with the contract.
const MIN_TOTAL_ATTEMPTS: u32 = 50;
/// z-score for a 90% two-sided interval, used by the Wilson score interval.
const WILSON_Z_90: f64 = 1.6449;

impl Collection {
    /// Compute the blueprint-weighted performance (application) score with its
    /// per-topic Beta-Binomial breakdown, uncertainty band, coverage, and the
    /// abstain rule. See the module docs for the exact definitions. This is a
    /// read-only query: it never mutates the collection.
    pub(crate) fn performance_score(&mut self) -> Result<PerformanceScore> {
        let attempts = self.speedrun_question_attempts()?;
        let taxonomy = self.speedrun_topics()?;

        // Group graded attempts by (non-empty) topic id -> (n attempts, k
        // correct). A BTreeMap keeps the per-topic output ordered by topic id so
        // the response is deterministic regardless of import/storage order.
        let mut grouped: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        for a in &attempts {
            if a.topic_id.is_empty() {
                continue;
            }
            let entry = grouped.entry(a.topic_id.clone()).or_insert((0, 0));
            entry.0 += 1;
            if a.correct {
                entry.1 += 1;
            }
        }

        // Per-topic Beta-Binomial breakdown, emitted for every topic that has
        // attempts (taxonomy-mapped or not). Only "covered" topics — those in
        // the taxonomy — feed the blueprint-weighted aggregate.
        let total_weight: f64 = taxonomy.values().map(|t| t.blueprint_weight).sum();
        let mut topics_out: Vec<TopicPerformance> = Vec::with_capacity(grouped.len());
        let mut covered_weight = 0.0f64;
        let mut total_attempts: u32 = 0;
        let mut covered_topics: u32 = 0;
        let mut weighted_accuracy_sum = 0.0f64;
        let mut weighted_low_sum = 0.0f64;
        let mut weighted_high_sum = 0.0f64;

        for (topic_id, (n, k)) in &grouped {
            let (n, k) = (*n, *k);
            // Beta-Binomial posterior mean under a uniform Beta(1,1) prior
            // (Laplace rule), and the dependency-free Wilson 90% credible band.
            let accuracy = (k as f64 + 1.0) / (n as f64 + 2.0);
            let (low, high) = wilson_interval(k, n, WILSON_Z_90);
            topics_out.push(TopicPerformance {
                topic_id: topic_id.clone(),
                attempts: n,
                correct: k,
                accuracy,
                low,
                high,
            });

            if let Some(info) = taxonomy.get(topic_id) {
                let weight = info.blueprint_weight;
                covered_weight += weight;
                total_attempts = total_attempts.saturating_add(n);
                covered_topics += 1;
                weighted_accuracy_sum += weight * accuracy;
                weighted_low_sum += weight * low;
                weighted_high_sum += weight * high;
            }
        }

        let coverage_pct = if total_weight > 0.0 {
            100.0 * covered_weight / total_weight
        } else {
            0.0
        };

        let updated_at = TimestampSecs::now().0;

        // Frozen abstain rule: fewer than MIN_TOTAL_ATTEMPTS graded attempts over
        // covered topics. A covered-weight of 0 also abstains — a weighted mean is
        // undefined there, so emitting a number would be unbacked (honesty bar).
        let abstained = total_attempts < MIN_TOTAL_ATTEMPTS || covered_weight <= 0.0;
        if abstained {
            let mut reasons = Vec::new();
            if total_attempts < MIN_TOTAL_ATTEMPTS {
                reasons.push(format!(
                    "only {total_attempts} graded attempts (< {MIN_TOTAL_ATTEMPTS} required)"
                ));
            }
            if covered_weight <= 0.0 {
                reasons.push(
                    "covered topics carry no blueprint weight (cannot weight a score)".to_string(),
                );
            }
            return Ok(PerformanceScore {
                abstained: true,
                point: 0.0,
                low: 0.0,
                high: 0.0,
                coverage_pct,
                reasons,
                updated_at,
                topics: topics_out,
            });
        }

        // Blueprint-weighted aggregation over covered topics (covered_weight > 0
        // is guaranteed here). Each per-topic band brackets its own accuracy, so
        // mathematically low <= point <= high already holds; the final min/max
        // only guards that invariant against floating-point rounding.
        let point = (100.0 * weighted_accuracy_sum / covered_weight).clamp(0.0, 100.0);
        let low = (100.0 * weighted_low_sum / covered_weight)
            .clamp(0.0, 100.0)
            .min(point);
        let high = (100.0 * weighted_high_sum / covered_weight)
            .clamp(0.0, 100.0)
            .max(point);

        let reasons = vec![
            format!("{total_attempts} graded attempts"),
            format!("coverage {coverage_pct:.1}% of blueprint"),
            format!("weighted accuracy {point:.1}%"),
            format!("{covered_topics} covered topics"),
        ];

        Ok(PerformanceScore {
            abstained: false,
            point,
            low,
            high,
            coverage_pct,
            reasons,
            updated_at,
            topics: topics_out,
        })
    }
}

/// The Wilson score interval at confidence `z` for `k` successes out of `n`
/// trials, clamped to `[0, 1]`. Dependency-free (no new crate): the standard
/// closed form. `n == 0` yields the whole `[0, 1]` interval.
fn wilson_interval(k: u32, n: u32, z: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let n = n as f64;
    let p_hat = k as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denom;
    let margin = (z / denom) * (p_hat * (1.0 - p_hat) / n + z2 / (4.0 * n * n)).sqrt();
    (
        (center - margin).clamp(0.0, 1.0),
        (center + margin).clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;

    use super::*;
    use crate::services::SpeedrunService;
    use crate::speedrun::attempts::StoredQuestionAttempt;

    /// Seed the taxonomy (topic id -> blueprint weight). The performance score
    /// reads QBank attempts, not cards, so no crosswalk/weakness is needed.
    fn seed_topics(col: &mut Collection, topics: &[(&str, f64)]) {
        let req = SetTopicWeightsRequest {
            topics: topics
                .iter()
                .map(|(id, weight)| Topic {
                    id: (*id).into(),
                    name: (*id).into(),
                    blueprint_weight: *weight,
                })
                .collect(),
            card_topics: Vec::<CardTopic>::new(),
            weaknesses: vec![],
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    /// Import `n` graded attempts for `topic`, `k` of them correct. Each
    /// attempt gets a unique `(source, external_id, answered_at)` dedup key
    /// so the F2 store keeps all of them (no silent dedup).
    fn seed_attempts(col: &mut Collection, topic: &str, n: u32, k: u32) {
        assert!(k <= n, "cannot have more correct ({k}) than attempts ({n})");
        let attempts: Vec<StoredQuestionAttempt> = (0..n)
            .map(|i| StoredQuestionAttempt {
                source: "uworld".into(),
                external_id: format!("{topic}-{i}"),
                answered_at: 1_000 + i as i64,
                topic_id: topic.into(),
                correct: i < k,
                seconds: 60,
            })
            .collect();
        let _ = col.import_qbank_data(attempts, vec![]).unwrap();
    }

    fn find_topic<'a>(score: &'a PerformanceScore, topic_id: &str) -> &'a TopicPerformance {
        score
            .topics
            .iter()
            .find(|t| t.topic_id == topic_id)
            .unwrap_or_else(|| panic!("topic {topic_id} missing from {:?}", score.topics))
    }

    /// 1. Fewer than 50 graded attempts across covered topics -> abstain:
    ///    point/low/high all 0, the real coverage and per-topic breakdown are
    ///    still reported, and a reason names the shortfall with numbers.
    #[test]
    fn abstains_when_too_few_attempts() -> Result<()> {
        let mut col = Collection::new();
        seed_topics(&mut col, &[("cardio", 1.0)]);
        seed_attempts(&mut col, "cardio", 12, 8);

        let score = col.performance_score()?;
        assert!(score.abstained, "12 attempts < 50 must abstain");
        assert_eq!(score.point, 0.0);
        assert_eq!(score.low, 0.0);
        assert_eq!(score.high, 0.0);
        // Coverage is still reported: cardio is the only taxonomy topic and it
        // is covered, so coverage is 100%.
        assert!(
            (score.coverage_pct - 100.0).abs() < 1e-6,
            "coverage should still be reported (100%), got {}",
            score.coverage_pct
        );
        // The per-topic breakdown is emitted even while abstaining.
        assert!(
            !score.topics.is_empty(),
            "topics must be reported even when abstaining"
        );
        let cardio = find_topic(&score, "cardio");
        assert_eq!(cardio.attempts, 12);
        assert_eq!(cardio.correct, 8);
        // A reason must name the attempts shortfall with numbers.
        assert!(
            score
                .reasons
                .iter()
                .any(|r| r.contains("12") && r.contains("50")),
            "a reason must name the attempts shortfall with numbers: {:?}",
            score.reasons
        );
        Ok(())
    }

    /// 2. At least 50 graded attempts across covered topics -> score, with a
    ///    valid non-degenerate band `0 <= low <= point <= high <= 100`, a
    ///    correct coverage figure, and a fresh timestamp.
    #[test]
    fn scores_with_valid_band_and_coverage() -> Result<()> {
        let mut col = Collection::new();
        // cardio(1) + renal(1) covered; neuro(2) is in the taxonomy but has no
        // attempts, so covered weight is 2 of a total 4 -> 50% coverage.
        seed_topics(&mut col, &[("cardio", 1.0), ("renal", 1.0), ("neuro", 2.0)]);
        seed_attempts(&mut col, "cardio", 30, 24);
        seed_attempts(&mut col, "renal", 30, 21);

        let score = col.performance_score()?;
        assert!(!score.abstained, "60 attempts >= 50 must score");
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
            "band must be non-degenerate: {} .. {}",
            score.low,
            score.high
        );
        assert!(
            (score.coverage_pct - 50.0).abs() < 1e-6,
            "covered weight 2 of total 4 -> 50%, got {}",
            score.coverage_pct
        );
        assert!(score.updated_at > 0, "updated_at must be set");
        assert!(!score.reasons.is_empty());
        Ok(())
    }

    /// 3. Per-topic Beta-Binomial: a topic with `k`/`n` reports the Laplace
    ///    posterior mean `(k+1)/(n+2)` bracketed by its Wilson band (`low <
    ///    accuracy < high`); and a higher-accuracy topic yields a higher
    ///    overall point.
    #[test]
    fn per_topic_beta_binomial_and_higher_accuracy_scores_higher() -> Result<()> {
        // Part A: per-topic Laplace mean + Wilson band on one covered topic.
        let mut col = Collection::new();
        seed_topics(&mut col, &[("cardio", 1.0)]);
        seed_attempts(&mut col, "cardio", 50, 40);

        let score = col.performance_score()?;
        let cardio = find_topic(&score, "cardio");
        assert_eq!(cardio.attempts, 50);
        assert_eq!(cardio.correct, 40);
        let expected = 41.0 / 52.0; // (k + 1) / (n + 2)
        assert!(
            (cardio.accuracy - expected).abs() < 1e-9,
            "accuracy should be the Laplace posterior mean {expected}, got {}",
            cardio.accuracy
        );
        assert!(
            cardio.low < cardio.accuracy && cardio.accuracy < cardio.high,
            "Wilson band must bracket the accuracy: {} < {} < {}",
            cardio.low,
            cardio.accuracy,
            cardio.high
        );
        assert!(
            cardio.low >= 0.0 && cardio.high <= 1.0,
            "band must be clamped to [0,1]: {} .. {}",
            cardio.low,
            cardio.high
        );

        // Part B: a higher-accuracy topic yields a higher overall point.
        let point_high = {
            let mut col = Collection::new();
            seed_topics(&mut col, &[("cardio", 1.0)]);
            seed_attempts(&mut col, "cardio", 60, 54); // ~0.9
            col.performance_score()?.point
        };
        let point_low = {
            let mut col = Collection::new();
            seed_topics(&mut col, &[("cardio", 1.0)]);
            seed_attempts(&mut col, "cardio", 60, 30); // 0.5
            col.performance_score()?.point
        };
        assert!(
            point_high > point_low,
            "higher accuracy {point_high} should exceed lower accuracy {point_low}"
        );
        Ok(())
    }

    /// 4. Read-only + deterministic: two calls on fixed data agree (including
    ///    the per-topic order), no undo step is created, and the database stays
    ///    uncorrupted.
    #[test]
    fn read_only_and_deterministic() -> Result<()> {
        let mut col = Collection::new();
        seed_topics(&mut col, &[("cardio", 1.0), ("renal", 1.0)]);
        seed_attempts(&mut col, "cardio", 30, 20);
        seed_attempts(&mut col, "renal", 30, 15);

        let undo_before = col.undo_status().last_step;
        let first = col.performance_score()?;
        let second = col.performance_score()?;
        let undo_after = col.undo_status().last_step;

        assert!(!first.abstained, "60 attempts should score");
        assert_eq!(first.point, second.point);
        assert_eq!(first.low, second.low);
        assert_eq!(first.high, second.high);
        assert_eq!(first.coverage_pct, second.coverage_pct);
        assert_eq!(first.abstained, second.abstained);
        assert_eq!(first.topics.len(), second.topics.len());
        for (a, b) in first.topics.iter().zip(second.topics.iter()) {
            assert_eq!(a.topic_id, b.topic_id, "per-topic order must be stable");
            assert_eq!(a.accuracy, b.accuracy);
            assert_eq!(a.low, b.low);
            assert_eq!(a.high, b.high);
        }
        assert_eq!(
            undo_before, undo_after,
            "performance_score must not create an undo step"
        );

        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }

    /// 5. Honesty guard: attempts whose topic is NOT a blueprint topic cannot
    ///    back a blueprint-weighted score, so they are excluded from coverage
    ///    and the abstain count (here: abstain at 0 covered attempts, 0%
    ///    coverage) — but the unmapped topic still surfaces in the per-topic
    ///    breakdown.
    #[test]
    fn attempts_outside_taxonomy_excluded_from_score() -> Result<()> {
        let mut col = Collection::new();
        // The taxonomy has cardio (no attempts); all 60 attempts are in
        // "mystery", which is not a blueprint topic.
        seed_topics(&mut col, &[("cardio", 1.0)]);
        seed_attempts(&mut col, "mystery", 60, 40);

        let score = col.performance_score()?;
        assert!(
            score.abstained,
            "unmapped attempts cannot back a blueprint score"
        );
        assert_eq!(score.point, 0.0);
        assert!(
            (score.coverage_pct - 0.0).abs() < 1e-6,
            "no covered blueprint weight -> 0% coverage, got {}",
            score.coverage_pct
        );
        let mystery = find_topic(&score, "mystery");
        assert_eq!(mystery.attempts, 60);
        assert_eq!(mystery.correct, 40);
        assert!(!score.reasons.is_empty());
        Ok(())
    }

    /// The dependency-free Wilson interval brackets the raw rate and the
    /// Laplace mean, stays within [0,1], and has a strictly positive width.
    #[test]
    fn wilson_interval_is_well_formed() {
        // A small tolerance absorbs floating-point rounding at the p=0 / p=1
        // boundaries, where the Wilson bound equals the raw rate exactly in real
        // arithmetic but lands an ULP away in f64.
        let eps = 1e-9;
        for &(k, n) in &[(0u32, 1u32), (1, 1), (0, 50), (25, 50), (50, 50), (40, 60)] {
            let (low, high) = wilson_interval(k, n, WILSON_Z_90);
            let rate = k as f64 / n as f64;
            let laplace = (k as f64 + 1.0) / (n as f64 + 2.0);
            assert!(low >= 0.0 && high <= 1.0, "clamped for k={k} n={n}");
            assert!(high > low, "strictly positive width for k={k} n={n}");
            assert!(
                low - eps <= rate && rate <= high + eps,
                "brackets raw rate {rate} for k={k} n={n}: {low}..{high}"
            );
            assert!(
                low - eps <= laplace && laplace <= high + eps,
                "brackets Laplace mean {laplace} for k={k} n={n}: {low}..{high}"
            );
        }
    }
}
