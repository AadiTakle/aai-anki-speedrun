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
        _input: anki_proto::speedrun::ImportQbankDataRequest,
    ) -> error::Result<anki_proto::collection::OpChanges> {
        // Stub: no-op until F2 lands (real impl wraps a transact with dedup).
        Ok(anki_proto::collection::OpChanges::default())
    }

    fn get_performance_score(&mut self) -> error::Result<anki_proto::speedrun::PerformanceScore> {
        Ok(anki_proto::speedrun::PerformanceScore {
            abstained: true,
            reasons: vec!["no QBank attempts imported yet".to_string()],
            ..Default::default()
        })
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
        // Stub: empty until the display RPC worker computes ranked topics.
        Ok(anki_proto::speedrun::PointsAtStakeResponse::default())
    }
}
