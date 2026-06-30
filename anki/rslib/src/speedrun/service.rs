// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use crate::collection::Collection;
use crate::error;

// Stub implementations frozen with the contract. The Wednesday-slice features
// replace these with real logic + tests:
//   - set_topic_weights  -> F1 (persist taxonomy/crosswalk/weakness to
//     col.conf, via transact)
//   - get_topic_mastery  -> F4 (per-topic mastered count + avg FSRS recall)
//   - get_memory_score   -> F6 (memory score + range + give-up rule)
impl crate::services::SpeedrunService for Collection {
    fn set_topic_weights(
        &mut self,
        _input: anki_proto::speedrun::SetTopicWeightsRequest,
    ) -> error::Result<anki_proto::collection::OpChanges> {
        Ok(anki_proto::collection::OpChanges::default())
    }

    fn get_topic_mastery(
        &mut self,
        _input: anki_proto::speedrun::GetTopicMasteryRequest,
    ) -> error::Result<anki_proto::speedrun::TopicMasteryResponse> {
        Ok(anki_proto::speedrun::TopicMasteryResponse::default())
    }

    fn get_memory_score(&mut self) -> error::Result<anki_proto::speedrun::MemoryScore> {
        Ok(anki_proto::speedrun::MemoryScore::default())
    }
}
