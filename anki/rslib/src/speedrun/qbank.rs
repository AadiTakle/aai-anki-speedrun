// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! Aggregate QBank ingestion (PRD F2, wave 3) — the counts a QBank exposes
//! per subject/system (e.g. UWorld "Performance by System") when there is no
//! per-question export.
//!
//! Mirrors [`super::store`] / [`super::attempts`]: everything is stored as JSON
//! in the existing `col.conf` (config table) under a dedicated `speedrun:*` key
//! — there is no new SQLite schema or migration. Writes go through the standard
//! config/op/undo machinery so they are sync-safe and undo-safe.
//!
//! The store is keyed by `source` then `topic_id`, each holding
//! `{correct, total, updated_at}`. Importing a source **replaces all prior rows
//! for that source** (idempotent updates: re-pasting corrected numbers never
//! double-counts, and no fabricated per-question records are ever created).
//!
//! Only the student's own aggregate performance metadata is stored (how many
//! answered / how many correct per topic) — never third-party question content.

use std::collections::HashMap;

use anki_proto::speedrun::ImportQbankAggregateResponse;
use anki_proto::speedrun::QbankTopicResult;
use serde::Deserialize;
use serde::Serialize;

use crate::prelude::*;
use crate::speedrun::store::TopicInfo;
use crate::speedrun::store::TOPICS_KEY;

/// `col.conf` key holding the aggregate QBank store, keyed `source -> (topic_id
/// -> {correct, total, updated_at})`. Namespaced like the other Speedrun stores
/// (`speedrun:attempts`, `speedrun:weakness`, ...).
pub(crate) const QBANK_AGGREGATES_KEY: &str = "speedrun:qbank_aggregates";

/// The canonical 22-topic USMLE Step 2 CK blueprint: `(topic id, blueprint
/// weight)`. Weights are relative (they need not sum to 1 — the performance
/// score normalises by the total covered weight). Used so imported topics can
/// be scored on a blueprint-weighted basis even before the user has seeded a
/// taxonomy: `import_qbank_aggregate` ensures every imported canonical topic
/// exists in the F1 store with this weight, and `performance.rs` falls back to
/// equal weighting for any covered topic that still lacks a positive weight.
pub(crate) const STEP2_BLUEPRINT: &[(&str, f64)] = &[
    ("cardio", 0.11),
    ("pulm", 0.09),
    ("gi", 0.09),
    ("obgyn", 0.09),
    ("peds", 0.08),
    ("psych", 0.07),
    ("renal", 0.06),
    ("endo", 0.06),
    ("heme_onc", 0.06),
    ("id", 0.06),
    ("neuro", 0.06),
    ("msk", 0.05),
    ("surg", 0.05),
    ("emerg", 0.04),
    ("derm", 0.03),
    ("ophtho", 0.02),
    ("ent", 0.02),
    ("biostat", 0.02),
    ("ethics", 0.02),
    ("genetics", 0.02),
    ("immuno", 0.02),
    ("nutrition", 0.01),
];

/// One topic's aggregate result for a single source, as stored in `col.conf`.
/// Mirrors `speedrun.QbankTopicResult` plus the import timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredQbankAggregate {
    pub correct: u32,
    pub total: u32,
    /// Unix seconds this row was last imported.
    pub updated_at: i64,
}

/// The on-disk shape: `source -> (topic_id -> aggregate)`. Nesting by source
/// makes "replace all rows for this source" a single map overwrite.
type QbankAggregateStore = HashMap<String, HashMap<String, StoredQbankAggregate>>;

/// The canonical blueprint weight for a topic id, if it is one of the 22
/// Step 2 CK blueprint topics.
fn canonical_blueprint_weight(id: &str) -> Option<f64> {
    STEP2_BLUEPRINT
        .iter()
        .find(|(tid, _)| *tid == id)
        .map(|(_, w)| *w)
}

/// A human-readable display name for a canonical blueprint topic id (falls back
/// to the id for anything not in the canonical set).
fn canonical_topic_name(id: &str) -> String {
    let name = match id {
        "cardio" => "Cardiovascular",
        "pulm" => "Pulmonary & Critical Care",
        "gi" => "Gastroenterology",
        "obgyn" => "Obstetrics & Gynecology",
        "peds" => "Pediatrics",
        "psych" => "Psychiatry & Behavioral",
        "renal" => "Renal & Urinary",
        "endo" => "Endocrine",
        "heme_onc" => "Hematology & Oncology",
        "id" => "Infectious Disease",
        "neuro" => "Neurology",
        "msk" => "Musculoskeletal & Rheumatology",
        "surg" => "Surgery",
        "emerg" => "Emergency Medicine",
        "derm" => "Dermatology",
        "ophtho" => "Ophthalmology",
        "ent" => "Otolaryngology (ENT)",
        "biostat" => "Biostatistics & Epidemiology",
        "ethics" => "Ethics & Professionalism",
        "genetics" => "Genetics",
        "immuno" => "Immunology",
        "nutrition" => "Nutrition",
        other => other,
    };
    name.to_string()
}

impl Collection {
    /// Load the raw aggregate store (empty when nothing has been imported).
    fn load_qbank_aggregates(&self) -> QbankAggregateStore {
        self.get_config_optional::<QbankAggregateStore, _>(QBANK_AGGREGATES_KEY)
            .unwrap_or_default()
    }

    /// The most recent QBank aggregate import time (unix seconds), or `None`
    /// when nothing has been imported. The daily plan uses this to tell whether
    /// today's Q-block results are already in.
    pub(crate) fn speedrun_qbank_last_import_secs(&self) -> Option<i64> {
        self.load_qbank_aggregates()
            .values()
            .flat_map(|rows| rows.values())
            .map(|agg| agg.updated_at)
            .max()
    }

    /// All imported aggregate rows as `(source, topic_id, correct, total)`,
    /// ordered by `(source, topic_id)` for determinism. Stable internal
    /// accessor.
    pub(crate) fn speedrun_qbank_aggregates(&self) -> Result<Vec<(String, String, u32, u32)>> {
        let store = self.load_qbank_aggregates();
        let mut out: Vec<(String, String, u32, u32)> = Vec::new();
        for (source, rows) in &store {
            for (topic_id, agg) in rows {
                out.push((source.clone(), topic_id.clone(), agg.correct, agg.total));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        Ok(out)
    }

    /// Aggregate `(correct, total)` per topic, **summed across all sources**.
    /// This is the combined view scoring (`performance.rs`) and weakness
    /// (`relink.rs`) fold together with the per-question attempts.
    pub(crate) fn speedrun_qbank_topic_totals(&self) -> Result<HashMap<String, (u32, u32)>> {
        let mut out: HashMap<String, (u32, u32)> = HashMap::new();
        for (_source, topic_id, correct, total) in self.speedrun_qbank_aggregates()? {
            let entry = out.entry(topic_id).or_insert((0, 0));
            entry.0 = entry.0.saturating_add(correct);
            entry.1 = entry.1.saturating_add(total);
        }
        Ok(out)
    }

    /// Import aggregate QBank results for `source`, **replacing all prior rows
    /// for that source** (idempotent updates), undo-safely.
    ///
    /// Normalisation rules (frozen with the contract):
    /// - rows with `total == 0` are dropped (nothing to score);
    /// - rows with an empty `topic_id` are dropped (cannot attribute a topic);
    /// - `correct` is clamped to `<= total`;
    /// - duplicate topic ids within a single import are summed (order-
    ///   independent), so the stored value is one aggregate per topic;
    /// - all prior rows for `source` are replaced (delete-then-insert) so
    ///   re-pasting corrected numbers never double-counts.
    ///
    /// It also **ensures** each imported *canonical* blueprint topic exists in
    /// the F1 store with its canonical weight IF missing — without clobbering
    /// an existing topic's weight/name, or the card->topic crosswalk /
    /// weakness (only [`TOPICS_KEY`] is touched). All writes share the one
    /// transaction, so a single `undo()` reverts both the aggregates and
    /// any topics created.
    ///
    /// Returns `topics_imported` (distinct topics kept — equal to the number of
    /// rows kept when an import has no duplicate topic ids) and
    /// `total_questions` (the sum of their `total`s).
    pub(crate) fn import_qbank_aggregate(
        &mut self,
        source: String,
        rows: Vec<QbankTopicResult>,
    ) -> Result<ImportQbankAggregateResponse> {
        let now = TimestampSecs::now().0;
        // Build the replacement block for this source, applying the
        // normalisation rules above.
        let mut new_rows: HashMap<String, StoredQbankAggregate> = HashMap::new();
        for r in rows {
            if r.total == 0 || r.topic_id.is_empty() {
                continue;
            }
            let correct = r.correct.min(r.total);
            let entry = new_rows.entry(r.topic_id).or_insert(StoredQbankAggregate {
                correct: 0,
                total: 0,
                updated_at: now,
            });
            entry.correct = entry.correct.saturating_add(correct);
            entry.total = entry.total.saturating_add(r.total);
            entry.updated_at = now;
        }
        let topics_imported = new_rows.len() as u32;
        let total_questions = new_rows
            .values()
            .fold(0u32, |acc, a| acc.saturating_add(a.total));

        let out = self.transact(Op::UpdateConfig, |col| {
            let mut store = col.load_qbank_aggregates();
            // Replace ALL prior rows for this source (delete-then-insert). An
            // import that keeps no rows clears the source entirely.
            if new_rows.is_empty() {
                store.remove(&source);
            } else {
                store.insert(source.clone(), new_rows.clone());
            }
            col.set_config(QBANK_AGGREGATES_KEY, &store)?;
            // Ensure imported canonical topics exist so they can be scored, but
            // never clobber existing taxonomy/crosswalk/weakness.
            col.ensure_blueprint_topics(new_rows.keys())?;
            Ok(())
        })?;

        Ok(ImportQbankAggregateResponse {
            changes: Some(out.changes.into()),
            topics_imported,
            total_questions,
        })
    }

    /// Ensure each given topic id that is a canonical blueprint topic exists in
    /// the F1 taxonomy with its canonical weight + name. Existing topics are
    /// left untouched (no clobbering), and only [`TOPICS_KEY`] is written, so
    /// the card->topic crosswalk and per-topic weakness are preserved.
    fn ensure_blueprint_topics<'a>(
        &mut self,
        topic_ids: impl IntoIterator<Item = &'a String>,
    ) -> Result<()> {
        let mut topics = self.speedrun_topics()?;
        let mut changed = false;
        for id in topic_ids {
            if topics.contains_key(id) {
                continue;
            }
            if let Some(weight) = canonical_blueprint_weight(id) {
                topics.insert(
                    id.clone(),
                    TopicInfo {
                        name: canonical_topic_name(id),
                        blueprint_weight: weight,
                    },
                );
                changed = true;
            }
        }
        if changed {
            self.set_config(TOPICS_KEY, &topics)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::QbankTopicResult;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;

    use super::*;
    use crate::services::SpeedrunService;

    fn row(topic_id: &str, correct: u32, total: u32) -> QbankTopicResult {
        QbankTopicResult {
            topic_id: topic_id.into(),
            correct,
            total,
        }
    }

    /// Sum of `(correct, total)` for a topic across all stored sources.
    fn topic_total(col: &Collection, topic: &str) -> (u32, u32) {
        col.speedrun_qbank_topic_totals()
            .unwrap()
            .get(topic)
            .copied()
            .unwrap_or((0, 0))
    }

    /// 1. Import stores the rows and returns the right counts.
    #[test]
    fn import_stores_rows_and_returns_counts() -> Result<()> {
        let mut col = Collection::new();
        let resp = col.import_qbank_aggregate(
            "uworld".into(),
            vec![
                row("cardio", 60, 100),
                row("renal", 30, 50),
                row("gi", 8, 10),
            ],
        )?;

        assert_eq!(resp.topics_imported, 3, "three rows kept");
        assert_eq!(resp.total_questions, 160, "100 + 50 + 10");

        assert_eq!(topic_total(&col, "cardio"), (60, 100));
        assert_eq!(topic_total(&col, "renal"), (30, 50));
        assert_eq!(topic_total(&col, "gi"), (8, 10));

        let rows = col.speedrun_qbank_aggregates()?;
        assert_eq!(rows.len(), 3);
        Ok(())
    }

    /// 2. Re-importing the same source REPLACES its rows (never double-counts).
    #[test]
    fn reimport_same_source_replaces() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_aggregate("uworld".into(), vec![row("cardio", 60, 100)])?;
        // A corrected paste for the same source: must replace, not accumulate.
        let resp = col.import_qbank_aggregate("uworld".into(), vec![row("cardio", 70, 120)])?;

        assert_eq!(
            topic_total(&col, "cardio"),
            (70, 120),
            "replace: aggregate is 70/120, not 130/220"
        );
        assert_eq!(resp.topics_imported, 1);
        assert_eq!(resp.total_questions, 120);
        assert_eq!(col.speedrun_qbank_aggregates()?.len(), 1, "still one row");
        Ok(())
    }

    /// 3. Two different sources coexist and COMBINE per topic.
    #[test]
    fn two_sources_coexist_and_combine() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_aggregate("uworld".into(), vec![row("cardio", 60, 100)])?;
        let _ = col.import_qbank_aggregate("amboss".into(), vec![row("cardio", 30, 50)])?;

        // Both sources are kept, and the per-topic total sums across them.
        assert_eq!(col.speedrun_qbank_aggregates()?.len(), 2);
        assert_eq!(
            topic_total(&col, "cardio"),
            (90, 150),
            "combined 60/100 + 30/50 = 90/150"
        );

        // Re-importing one source only replaces that source's block.
        let _ = col.import_qbank_aggregate("uworld".into(), vec![row("cardio", 10, 20)])?;
        assert_eq!(
            topic_total(&col, "cardio"),
            (40, 70),
            "uworld replaced to 10/20; amboss 30/50 untouched"
        );
        Ok(())
    }

    /// 4. Clamp `correct <= total`, and drop rows with `total == 0`.
    #[test]
    fn clamps_correct_and_drops_zero_total() -> Result<()> {
        let mut col = Collection::new();
        let resp = col.import_qbank_aggregate(
            "uworld".into(),
            vec![
                row("cardio", 200, 100), // correct clamped to 100
                row("renal", 5, 0),      // dropped (total == 0)
                row("gi", 8, 10),
            ],
        )?;

        assert_eq!(resp.topics_imported, 2, "renal (total 0) dropped");
        assert_eq!(resp.total_questions, 110, "100 + 10 (renal excluded)");
        assert_eq!(topic_total(&col, "cardio"), (100, 100), "correct clamped");
        assert_eq!(topic_total(&col, "renal"), (0, 0), "zero-total not stored");
        assert_eq!(topic_total(&col, "gi"), (8, 10));
        Ok(())
    }

    /// 5. Import is undo-safe: `undo()` removes the imported aggregates and the
    ///    database stays uncorrupted.
    #[test]
    fn undo_removes_imported_aggregates() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_aggregate("uworld".into(), vec![row("cardio", 60, 100)])?;
        assert!(!col.speedrun_qbank_aggregates()?.is_empty());

        col.undo()?;
        assert!(
            col.speedrun_qbank_aggregates()?.is_empty(),
            "undo removed the imported aggregates"
        );

        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }

    /// 6. Import ensures each canonical topic exists in the F1 store with its
    ///    canonical weight IF missing — without clobbering an existing topic.
    #[test]
    fn import_ensures_canonical_blueprint_topic_without_clobbering() -> Result<()> {
        let mut col = Collection::new();
        // Pre-seed cardio with a custom weight + name that must NOT be clobbered.
        let _ = col.set_topic_weights(SetTopicWeightsRequest {
            topics: vec![Topic {
                id: "cardio".into(),
                name: "My Cardio".into(),
                blueprint_weight: 0.99,
            }],
            card_topics: vec![CardTopic {
                card_id: 7,
                topic_id: "cardio".into(),
            }],
            weaknesses: vec![],
        })?;

        // Import touches cardio (already present) and renal (missing).
        let _ = col.import_qbank_aggregate(
            "uworld".into(),
            vec![row("cardio", 10, 20), row("renal", 5, 10)],
        )?;

        let topics = col.speedrun_topics()?;
        // renal was created with its canonical weight + name.
        let renal = topics.get("renal").expect("renal created");
        assert!(
            (renal.blueprint_weight - 0.06).abs() < 1e-9,
            "canonical weight"
        );
        assert_eq!(renal.name, "Renal & Urinary", "canonical name");
        // cardio kept its pre-existing custom weight + name (not clobbered).
        let cardio = topics.get("cardio").expect("cardio kept");
        assert!(
            (cardio.blueprint_weight - 0.99).abs() < 1e-9,
            "not clobbered"
        );
        assert_eq!(cardio.name, "My Cardio");
        // The existing crosswalk is preserved.
        assert_eq!(
            col.speedrun_card_topics()?.get(&CardId(7)),
            Some(&"cardio".to_string()),
            "crosswalk preserved"
        );
        Ok(())
    }

    /// 7. Non-canonical imported topics are stored but not auto-added to the
    ///    taxonomy (they carry no canonical blueprint weight).
    #[test]
    fn non_canonical_topic_stored_but_not_added_to_taxonomy() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.import_qbank_aggregate("uworld".into(), vec![row("mystery", 5, 10)])?;
        assert_eq!(topic_total(&col, "mystery"), (5, 10), "row is stored");
        assert!(
            !col.speedrun_topics()?.contains_key("mystery"),
            "no canonical weight -> not added to taxonomy"
        );
        Ok(())
    }

    /// 8. Accessors return empty on a fresh collection (no panics).
    #[test]
    fn empty_on_fresh_collection() -> Result<()> {
        let col = Collection::new();
        assert!(col.speedrun_qbank_aggregates()?.is_empty());
        assert!(col.speedrun_qbank_topic_totals()?.is_empty());
        Ok(())
    }

    /// 9. The canonical blueprint constant is well-formed: 22 unique topic ids,
    ///    each with a positive weight and a resolvable canonical weight.
    #[test]
    fn blueprint_constant_is_well_formed() {
        assert_eq!(STEP2_BLUEPRINT.len(), 22, "22-topic blueprint");
        let mut ids = std::collections::HashSet::new();
        for (id, weight) in STEP2_BLUEPRINT {
            assert!(ids.insert(*id), "duplicate blueprint id: {id}");
            assert!(*weight > 0.0, "blueprint weight for {id} must be > 0");
            assert_eq!(canonical_blueprint_weight(id), Some(*weight));
        }
        assert_eq!(canonical_blueprint_weight("not_a_topic"), None);
    }
}
