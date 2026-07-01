// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use std::collections::HashMap;

use crate::collection::Collection;
use crate::error;
use crate::prelude::CardId;
use crate::speedrun::store::TopicInfo;

// Stub implementations frozen with the contract. The Wednesday-slice features
// replace these with real logic + tests:
//   - set_topic_weights  -> F1 (persist taxonomy/crosswalk/weakness to
//     col.conf, via transact)
//   - get_topic_mastery  -> F4 (per-topic mastered count + avg FSRS recall)
//   - get_memory_score   -> F6 (memory score + range + give-up rule)
impl crate::services::SpeedrunService for Collection {
    fn set_topic_weights(
        &mut self,
        input: anki_proto::speedrun::SetTopicWeightsRequest,
    ) -> error::Result<anki_proto::collection::OpChanges> {
        let topics: HashMap<String, TopicInfo> = input
            .topics
            .into_iter()
            .map(|t| {
                (
                    t.id,
                    TopicInfo {
                        name: t.name,
                        blueprint_weight: t.blueprint_weight,
                    },
                )
            })
            .collect();
        let card_topics: HashMap<CardId, String> = input
            .card_topics
            .into_iter()
            .map(|ct| (CardId(ct.card_id), ct.topic_id))
            .collect();
        let weakness: HashMap<String, f64> = input
            .weaknesses
            .into_iter()
            .map(|w| (w.topic_id, w.weakness))
            .collect();
        self.set_speedrun_topic_weights(topics, card_topics, weakness)
            .map(Into::into)
    }

    fn get_topic_mastery(
        &mut self,
        input: anki_proto::speedrun::GetTopicMasteryRequest,
    ) -> error::Result<anki_proto::speedrun::TopicMasteryResponse> {
        self.topic_mastery(input.topic_ids)
    }

    fn get_memory_score(&mut self) -> error::Result<anki_proto::speedrun::MemoryScore> {
        self.memory_score()
    }

    // --- Next slice (build-readiness plan): frozen stubs. Lane workers replace
    // each with real logic + tests (F2 ingest; performance/readiness scores;
    // points-at-stake display view). Scores default to an honest abstain.
    fn import_qbank_data(
        &mut self,
        input: anki_proto::speedrun::ImportQbankDataRequest,
    ) -> error::Result<anki_proto::collection::OpChanges> {
        // Map the proto request into the col.conf-backed store, which merges +
        // dedups undo-safely (see speedrun::attempts). The 2-arg inherent
        // Collection::import_qbank_data is selected over this trait method.
        self.import_qbank_data(
            input.attempts.into_iter().map(Into::into).collect(),
            input.tests.into_iter().map(Into::into).collect(),
        )
        .map(Into::into)
    }

    fn get_performance_score(&mut self) -> error::Result<anki_proto::speedrun::PerformanceScore> {
        self.performance_score()
    }

    fn get_readiness_score(&mut self) -> error::Result<anki_proto::speedrun::ReadinessScore> {
        Ok(anki_proto::speedrun::ReadinessScore {
            abstained: true,
            reasons: vec!["readiness not calibrated to NBME/UWSA yet".to_string()],
            ..Default::default()
        })
    }

    fn get_points_at_stake(
        &mut self,
    ) -> error::Result<anki_proto::speedrun::PointsAtStakeResponse> {
        // Read-only ranked "Today's focus" view over the F1 topic store.
        self.points_at_stake()
    }

    // --- Wave 2 (build-readiness plan): implemented. Each trait method delegates
    // to its inherent Collection method: F3 relink misses + error log
    // (speedrun::relink), next-action (speedrun::next_action), coverage map
    // (speedrun::coverage).
    fn relink_misses(&mut self) -> error::Result<anki_proto::collection::OpChanges> {
        // The inherent Collection::relink_misses is selected over this trait
        // method; it recomputes weakness, unsuspends missed topics' cards, and
        // appends the error log undo-safely (F3, see speedrun::relink).
        Collection::relink_misses(self).map(Into::into)
    }

    fn get_error_log(&mut self) -> error::Result<anki_proto::speedrun::ErrorLogResponse> {
        Collection::get_error_log(self)
    }

    fn get_next_action(&mut self) -> error::Result<anki_proto::speedrun::NextAction> {
        self.next_action()
    }

    fn get_coverage_map(&mut self) -> error::Result<anki_proto::speedrun::CoverageMapResponse> {
        // Read-only per-section blueprint coverage view over the F1 topic store.
        self.coverage_map()
    }

    // --- Wave 3 (real data): implemented. The inherent
    // Collection::import_qbank_aggregate is selected over this trait method; it
    // replaces the source's rows, ensures canonical blueprint topics, and
    // returns the import counts undo-safely (F2, see speedrun::qbank). The
    // aggregates feed performance scoring and weakness recompute.
    fn import_qbank_aggregate(
        &mut self,
        input: anki_proto::speedrun::ImportQbankAggregateRequest,
    ) -> error::Result<anki_proto::speedrun::ImportQbankAggregateResponse> {
        Collection::import_qbank_aggregate(self, input.source, input.rows)
    }
}
