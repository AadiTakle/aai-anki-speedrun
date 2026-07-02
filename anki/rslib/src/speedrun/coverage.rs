// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! F7 — blueprint **coverage map**.
//!
//! Reports, per canonical exam topic, whether it has any study material mapped
//! to it, and rolls that up into a single blueprint-weighted "how much of the
//! exam is covered" fraction. It is a pure, read-only projection of the F1
//! topic store (see `crate::speedrun::store`): it never mutates the collection.
//!
//! Definitions (frozen with the wave-2 contract):
//! - **mapped_cards**: the number of cards mapped to the topic via the
//!   card->topic crosswalk that *still exist* in the collection. Crosswalk
//!   entries whose card has since been deleted are skipped, so a stale mapping
//!   never inflates coverage.
//! - **covered**: `mapped_cards > 0` — the section has at least one real card
//!   to study.
//! - **covered_pct**: the blueprint-weighted fraction of the exam that has
//!   study material — `sum(blueprint_weight over covered topics) /
//!   sum(blueprint_weight over all topics)`, in `0.0..=1.0`. When the total
//!   blueprint weight is `0` (an empty store, or every topic weighted `0`), it
//!   is `0.0` — never a divide-by-zero.
//! - **order**: sections are returned by `blueprint_weight` descending (the
//!   biggest exam sections lead), with a deterministic tie-break by `topic_id`
//!   ascending.

use std::collections::HashMap;

use anki_proto::speedrun::CoverageMapResponse;
use anki_proto::speedrun::CoverageSection;

use crate::prelude::*;

impl Collection {
    /// Blueprint coverage map over the F1 topic store (read-only). See the
    /// module docs for the exact `mapped_cards`/`covered`/`covered_pct` rules.
    pub(crate) fn coverage_map(&self) -> Result<CoverageMapResponse> {
        let topics = self.speedrun_topics()?;
        let card_topics = self.speedrun_card_topics()?;

        // Count, per topic, the crosswalk cards that still exist. Entries whose
        // card was deleted are skipped so a stale mapping can't inflate a
        // section's coverage.
        let mut mapped_by_topic: HashMap<String, u32> = HashMap::new();
        for (card_id, topic_id) in &card_topics {
            if self.storage.get_card(*card_id)?.is_some() {
                *mapped_by_topic.entry(topic_id.clone()).or_insert(0) += 1;
            }
        }

        let mut total_weight = 0.0;
        let mut covered_weight = 0.0;
        let mut sections: Vec<CoverageSection> = Vec::with_capacity(topics.len());
        for (topic_id, info) in topics {
            let mapped_cards = mapped_by_topic.get(&topic_id).copied().unwrap_or(0);
            let covered = mapped_cards > 0;
            total_weight += info.blueprint_weight;
            if covered {
                covered_weight += info.blueprint_weight;
            }
            sections.push(CoverageSection {
                topic_id,
                name: info.name,
                blueprint_weight: info.blueprint_weight,
                covered,
                mapped_cards,
            });
        }

        // Blueprint-weighted coverage. An empty store (or every topic weighted
        // 0) has no exam to cover -> honest 0.0, never a divide-by-zero.
        // `covered_weight <= total_weight`, so this stays within `0.0..=1.0`.
        let covered_pct = if total_weight > 0.0 {
            covered_weight / total_weight
        } else {
            0.0
        };

        // Biggest exam sections lead; deterministic tie-break by topic_id
        // ascending. Weights are store-clamped finite, so `total_cmp` gives a
        // total order without special-casing NaN.
        sections.sort_by(|a, b| {
            b.blueprint_weight
                .total_cmp(&a.blueprint_weight)
                .then_with(|| a.topic_id.cmp(&b.topic_id))
        });

        Ok(CoverageMapResponse {
            sections,
            covered_pct,
        })
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::CoverageSection;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;

    use super::*;
    use crate::services::SpeedrunService;

    /// Add a minimal card and return its id. Coverage only checks a mapped
    /// card's *existence*, so a default card (no note needed) is enough.
    fn add_card(col: &mut Collection) -> CardId {
        let mut card = Card::default();
        col.add_card(&mut card).unwrap();
        card.id
    }

    /// Seed the F1 taxonomy (id, name, blueprint_weight) + card->topic
    /// crosswalk in one undo-safe write, mirroring how the store is
    /// populated in prod.
    fn seed(col: &mut Collection, topics: &[(&str, &str, f64)], crosswalk: &[(i64, &str)]) {
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
                .map(|(card_id, topic_id)| CardTopic {
                    card_id: *card_id,
                    topic_id: (*topic_id).into(),
                })
                .collect(),
            weaknesses: vec![],
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    fn find<'a>(sections: &'a [CoverageSection], topic_id: &str) -> &'a CoverageSection {
        sections
            .iter()
            .find(|s| s.topic_id == topic_id)
            .unwrap_or_else(|| panic!("missing topic {topic_id} in coverage map"))
    }

    /// Every topic has at least one existing mapped card -> covered_pct == 1.0
    /// and every section covered. Dangling crosswalk ids (card deleted) are
    /// skipped, so they never inflate a section's mapped_cards.
    #[test]
    fn all_topics_covered_pct_is_one() -> Result<()> {
        let mut col = Collection::new();
        let c1 = add_card(&mut col);
        let c2 = add_card(&mut col);
        let c3 = add_card(&mut col);
        let c4 = add_card(&mut col);
        seed(
            &mut col,
            &[
                ("cardio", "Cardiology", 0.5),
                ("renal", "Nephrology", 0.3),
                ("gi", "Gastroenterology", 0.2),
            ],
            &[
                (c1.0, "cardio"),
                (c2.0, "cardio"),
                (999_999, "cardio"), // dangling id: card never existed -> skipped
                (c3.0, "renal"),
                (c4.0, "gi"),
            ],
        );

        let resp = col.coverage_map()?;
        assert!(
            (resp.covered_pct - 1.0).abs() < 1e-9,
            "all topics covered -> covered_pct 1.0, got {}",
            resp.covered_pct
        );
        assert_eq!(resp.sections.len(), 3);
        for s in &resp.sections {
            assert!(s.covered, "{} should be covered", s.topic_id);
        }
        let cardio = find(&resp.sections, "cardio");
        assert_eq!(
            cardio.mapped_cards, 2,
            "two existing cards; the dangling crosswalk id is skipped"
        );
        Ok(())
    }

    /// Partial coverage -> covered_pct is the *blueprint-weighted* fraction,
    /// not the covered/total topic count. Weights are uneven so the two
    /// differ, and the assertion pins the weighted math. Also checks
    /// weight-descending order and that a topic with only a dangling
    /// crosswalk is uncovered.
    #[test]
    fn partial_coverage_is_blueprint_weighted() -> Result<()> {
        let mut col = Collection::new();
        let cardio_card = add_card(&mut col);
        let renal_card = add_card(&mut col);
        seed(
            &mut col,
            &[
                ("cardio", "Cardiology", 5.0),
                ("renal", "Nephrology", 1.0),
                ("gi", "Gastroenterology", 4.0),
            ],
            &[
                (cardio_card.0, "cardio"),
                (renal_card.0, "renal"),
                (999_999, "gi"), // only a dangling id -> gi stays uncovered
            ],
        );

        let resp = col.coverage_map()?;

        // covered weight = 5 + 1 = 6; total = 5 + 1 + 4 = 10 -> 0.6.
        assert!(
            (resp.covered_pct - 0.6).abs() < 1e-9,
            "blueprint-weighted coverage should be 0.6, got {}",
            resp.covered_pct
        );
        // 2 of 3 topics are covered; the count fraction (0.667) must NOT be what
        // we report, proving the weighting is real.
        let count_fraction = 2.0 / 3.0;
        assert!(
            (resp.covered_pct - count_fraction).abs() > 1e-3,
            "covered_pct must be blueprint-weighted, not the topic-count fraction"
        );

        let ids: Vec<&str> = resp.sections.iter().map(|s| s.topic_id.as_str()).collect();
        assert_eq!(
            ids,
            ["cardio", "gi", "renal"],
            "sections ordered by blueprint_weight descending"
        );

        let gi = find(&resp.sections, "gi");
        assert!(
            !gi.covered,
            "gi's only crosswalk id is dangling -> uncovered"
        );
        assert_eq!(gi.mapped_cards, 0);
        Ok(())
    }

    /// Empty store -> covered_pct 0.0 with no divide-by-zero, and no sections.
    #[test]
    fn empty_store_covered_pct_zero() -> Result<()> {
        let col = Collection::new();
        let resp = col.coverage_map()?;
        assert_eq!(resp.covered_pct, 0.0, "empty store -> 0.0, not NaN");
        assert!(resp.sections.is_empty());
        Ok(())
    }

    /// A topic with zero mapped cards -> covered == false, mapped_cards == 0,
    /// and (total weight positive, covered weight zero) covered_pct == 0.0
    /// without a divide-by-zero.
    #[test]
    fn uncovered_topic_reports_zero() -> Result<()> {
        let mut col = Collection::new();
        seed(&mut col, &[("cardio", "Cardiology", 1.0)], &[]);

        let resp = col.coverage_map()?;
        assert_eq!(resp.sections.len(), 1);
        let cardio = find(&resp.sections, "cardio");
        assert!(!cardio.covered, "no mapped cards -> not covered");
        assert_eq!(cardio.mapped_cards, 0);
        assert_eq!(resp.covered_pct, 0.0, "0 covered / positive total -> 0.0");
        Ok(())
    }

    /// Equal-weight sections tie-break by topic_id ascending, and the read-only
    /// query creates no undo step and leaves the database uncorrupted.
    #[test]
    fn ties_break_by_topic_id_and_read_only() -> Result<()> {
        let mut col = Collection::new();
        // c weight 3.0 leads; a and b tie at 2.0 -> a before b.
        seed(
            &mut col,
            &[("c", "C", 3.0), ("b", "B", 2.0), ("a", "A", 2.0)],
            &[],
        );

        let undo_before = col.undo_status().last_step;
        let resp = col.coverage_map()?;
        let undo_after = col.undo_status().last_step;

        let ids: Vec<&str> = resp.sections.iter().map(|s| s.topic_id.as_str()).collect();
        assert_eq!(
            ids,
            ["c", "a", "b"],
            "weight descending, then topic_id ascending on ties"
        );
        assert_eq!(
            undo_before, undo_after,
            "coverage_map must not create an undo step"
        );

        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }
}
