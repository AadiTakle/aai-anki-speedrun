// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! Persistence for imported QBank / practice-test performance data (PRD F2).
//!
//! Mirrors [`super::store`]: everything is stored as JSON in the existing
//! `col.conf` (config table) under dedicated `speedrun:*` keys — there is no
//! new SQLite schema or migration. Writes go through the standard
//! config/op/undo machinery so they are sync-safe and undo-safe.
//!
//! Only the student's *own* performance metadata is stored (which question was
//! answered, when, and whether it was correct) — never third-party question
//! content. This module owns the stable internal accessor API the
//! Performance-score lane consumes.

use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use crate::prelude::*;

/// `col.conf` key holding the imported QBank question attempts (JSON array).
pub(crate) const ATTEMPTS_KEY: &str = "speedrun:attempts";
/// `col.conf` key holding the imported practice-test results (JSON array).
pub(crate) const PRACTICE_TESTS_KEY: &str = "speedrun:practice_tests";

/// A single graded QBank question attempt, as stored in `col.conf`.
///
/// Mirrors the `speedrun.QuestionAttempt` proto message but is kept as an
/// internal serde struct so the on-disk representation is independent of the
/// protobuf codegen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredQuestionAttempt {
    /// QBank name, e.g. "uworld" / "amboss".
    pub source: String,
    /// The QBank's own question id (used for dedup + linking).
    pub external_id: String,
    /// Unix seconds the question was answered.
    pub answered_at: i64,
    /// Canonical topic id (crosswalk); may be empty if unmapped.
    pub topic_id: String,
    pub correct: bool,
    /// Time spent on the question, in seconds (0 if unknown).
    pub seconds: u32,
}

/// A calibrated practice-test result (NBME/UWSA/Free120), as stored in
/// `col.conf`. Mirrors the `speedrun.PracticeTestResult` proto message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredPracticeTest {
    pub source: String,
    /// Form identifier, e.g. "NBME 14" / "UWSA2".
    pub form: String,
    /// Unix seconds the test was taken.
    pub taken_at: i64,
    /// Reported scaled score (Step 2 scale), if any.
    pub scaled_score: f64,
    pub percent_correct: f64,
}

impl From<anki_proto::speedrun::QuestionAttempt> for StoredQuestionAttempt {
    fn from(a: anki_proto::speedrun::QuestionAttempt) -> Self {
        Self {
            source: a.source,
            external_id: a.external_id,
            answered_at: a.answered_at,
            topic_id: a.topic_id,
            correct: a.correct,
            seconds: a.seconds,
        }
    }
}

impl From<anki_proto::speedrun::PracticeTestResult> for StoredPracticeTest {
    fn from(t: anki_proto::speedrun::PracticeTestResult) -> Self {
        Self {
            source: t.source,
            form: t.form,
            taken_at: t.taken_at,
            // Non-finite doubles cannot be JSON-serialized into col.conf without
            // corrupting the store, so coerce NaN/Infinity to 0.0 at the write
            // boundary (honesty bar: an unbacked score is never fabricated).
            scaled_score: sanitize_finite(t.scaled_score),
            percent_correct: sanitize_finite(t.percent_correct),
        }
    }
}

/// Dedup key for a question attempt: `(source, external_id, answered_at)`.
type AttemptKey = (String, String, i64);
/// Dedup key for a practice-test result: `(source, form, taken_at)`.
type TestKey = (String, String, i64);

fn attempt_key(a: &StoredQuestionAttempt) -> AttemptKey {
    (a.source.clone(), a.external_id.clone(), a.answered_at)
}

fn test_key(t: &StoredPracticeTest) -> TestKey {
    (t.source.clone(), t.form.clone(), t.taken_at)
}

/// Coerce a non-finite double to `0.0` so it can be JSON-serialized into
/// `col.conf` (serde_json cannot round-trip NaN/Infinity).
fn sanitize_finite(v: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

impl Collection {
    /// All imported QBank question attempts, in insertion order.
    ///
    /// Returns an empty vec when nothing has been imported yet. Stable internal
    /// accessor consumed by the Performance-score lane.
    pub(crate) fn speedrun_question_attempts(&self) -> Result<Vec<StoredQuestionAttempt>> {
        Ok(self
            .get_config_optional::<Vec<StoredQuestionAttempt>, _>(ATTEMPTS_KEY)
            .unwrap_or_default())
    }

    /// All imported practice-test results, in insertion order.
    ///
    /// Returns an empty vec when nothing has been imported yet. Stable internal
    /// accessor consumed by the Performance-score lane.
    pub(crate) fn speedrun_practice_tests(&self) -> Result<Vec<StoredPracticeTest>> {
        Ok(self
            .get_config_optional::<Vec<StoredPracticeTest>, _>(PRACTICE_TESTS_KEY)
            .unwrap_or_default())
    }

    /// Merge imported QBank attempts + practice tests into the sets already
    /// stored in `col.conf`, in a single undo-safe transaction.
    ///
    /// Dedup is idempotent and cross-import: attempts are keyed on
    /// `(source, external_id, answered_at)` and tests on
    /// `(source, form, taken_at)`. An incoming record whose key already exists
    /// (either previously stored, or earlier in the same batch) is skipped, so
    /// re-importing the same export is a no-op and never double-counts.
    pub(crate) fn import_qbank_data(
        &mut self,
        attempts: Vec<StoredQuestionAttempt>,
        tests: Vec<StoredPracticeTest>,
    ) -> Result<OpChanges> {
        // Merge into the existing sets, keeping insertion order and skipping any
        // record whose dedup key is already present (previously stored, or seen
        // earlier in this batch). `HashSet::insert` returns false for a repeat,
        // which makes both the cross-import and within-batch dedup a single
        // idempotent pass.
        let mut stored_attempts = self.speedrun_question_attempts()?;
        let mut seen_attempts: HashSet<AttemptKey> =
            stored_attempts.iter().map(attempt_key).collect();
        for a in attempts {
            if seen_attempts.insert(attempt_key(&a)) {
                stored_attempts.push(a);
            }
        }

        let mut stored_tests = self.speedrun_practice_tests()?;
        let mut seen_tests: HashSet<TestKey> = stored_tests.iter().map(test_key).collect();
        for t in tests {
            if seen_tests.insert(test_key(&t)) {
                stored_tests.push(t);
            }
        }

        // Both keys are written in one transaction via the standard
        // config/op/undo machinery, so the import is atomic and undo-safe.
        self.transact(Op::UpdateConfig, |col| {
            col.set_config(ATTEMPTS_KEY, &stored_attempts)?;
            col.set_config(PRACTICE_TESTS_KEY, &stored_tests)?;
            Ok(())
        })
        .map(|out| out.changes)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn attempt(
        source: &str,
        external_id: &str,
        answered_at: i64,
        correct: bool,
    ) -> StoredQuestionAttempt {
        StoredQuestionAttempt {
            source: source.into(),
            external_id: external_id.into(),
            answered_at,
            topic_id: "cardio".into(),
            correct,
            seconds: 60,
        }
    }

    fn practice_test(source: &str, form: &str, taken_at: i64) -> StoredPracticeTest {
        StoredPracticeTest {
            source: source.into(),
            form: form.into(),
            taken_at,
            scaled_score: 245.0,
            percent_correct: 0.8,
        }
    }

    /// R1: import then read back returns exactly the imported attempts + tests.
    #[test]
    fn imports_and_reads_back() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_data(
            vec![
                attempt("uworld", "q1", 1000, true),
                attempt("uworld", "q2", 1001, false),
            ],
            vec![practice_test("nbme", "NBME 14", 5000)],
        )?;

        let attempts = col.speedrun_question_attempts()?;
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0], attempt("uworld", "q1", 1000, true));
        assert_eq!(attempts[1], attempt("uworld", "q2", 1001, false));

        let tests = col.speedrun_practice_tests()?;
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0], practice_test("nbme", "NBME 14", 5000));
        Ok(())
    }

    /// R2: dedup — importing the same `(source, external_id, answered_at)`
    /// twice stores it once, both within a single batch and across imports;
    /// tests dedup on `(source, form, taken_at)` the same way.
    #[test]
    fn dedups_on_key() -> Result<()> {
        let mut col = Collection::new();
        // Within a single batch: the same attempt/test key appears twice.
        let _ = col.import_qbank_data(
            vec![
                attempt("uworld", "q1", 1000, true),
                attempt("uworld", "q1", 1000, true),
            ],
            vec![
                practice_test("nbme", "NBME 14", 5000),
                practice_test("nbme", "NBME 14", 5000),
            ],
        )?;
        assert_eq!(col.speedrun_question_attempts()?.len(), 1);
        assert_eq!(col.speedrun_practice_tests()?.len(), 1);

        // Across imports: re-importing the same keys is a no-op.
        let _ = col.import_qbank_data(
            vec![attempt("uworld", "q1", 1000, true)],
            vec![practice_test("nbme", "NBME 14", 5000)],
        )?;
        assert_eq!(col.speedrun_question_attempts()?.len(), 1);
        assert_eq!(col.speedrun_practice_tests()?.len(), 1);
        Ok(())
    }

    /// R3: undo-safe — after an import, `undo()` restores the prior state and
    /// the database still passes `pragma integrity_check`.
    #[test]
    fn undo_restores_previous_state() -> Result<()> {
        let mut col = Collection::new();
        // First import establishes a known prior state (one attempt).
        let _ = col.import_qbank_data(vec![attempt("uworld", "q1", 1000, true)], vec![])?;
        // Second import changes it.
        let _ = col.import_qbank_data(vec![attempt("uworld", "q2", 1001, false)], vec![])?;
        assert_eq!(col.speedrun_question_attempts()?.len(), 2);

        // Undo the second import -> back to the single earlier attempt.
        col.undo()?;
        let attempts = col.speedrun_question_attempts()?;
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0], attempt("uworld", "q1", 1000, true));

        // Undo the first import -> back to empty.
        col.undo()?;
        assert!(col.speedrun_question_attempts()?.is_empty());

        // No corruption introduced.
        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }

    /// R4: merge — a second import adds new records without dropping earlier
    /// ones, preserving insertion order.
    #[test]
    fn merges_new_without_dropping_existing() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_data(
            vec![attempt("uworld", "q1", 1000, true)],
            vec![practice_test("nbme", "NBME 14", 5000)],
        )?;
        let _ = col.import_qbank_data(
            vec![attempt("uworld", "q2", 1001, false)],
            vec![practice_test("uwsa", "UWSA2", 6000)],
        )?;

        let attempts = col.speedrun_question_attempts()?;
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].external_id, "q1");
        assert_eq!(attempts[1].external_id, "q2");

        let tests = col.speedrun_practice_tests()?;
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].form, "NBME 14");
        assert_eq!(tests[1].form, "UWSA2");
        Ok(())
    }

    /// R5: accessors return empty vecs on a fresh collection (no panics).
    #[test]
    fn empty_on_fresh_collection() -> Result<()> {
        let col = Collection::new();
        assert!(col.speedrun_question_attempts()?.is_empty());
        assert!(col.speedrun_practice_tests()?.is_empty());
        Ok(())
    }
}
