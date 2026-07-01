// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! Persistence for the Speedrun topic taxonomy, the card->topic crosswalk, and
//! per-topic weakness.
//!
//! Everything is stored as JSON in the existing `col.conf` (config table) under
//! dedicated `speedrun:*` keys — there is no new SQLite schema or migration
//! (see docs/wednesday_plan.md §2). Writes go through the standard
//! config/op/undo machinery so they are sync-safe and undo-safe.
//!
//! This module owns the **internal Rust accessor API** that downstream features
//! depend on (F4 mastery, F5 points-at-stake queue, F6 memory score). The
//! accessor names/signatures are intended to be stable.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::prelude::*;

/// `col.conf` key holding the canonical topic taxonomy
/// (`topic_id -> {name, blueprint_weight}`).
pub(crate) const TOPICS_KEY: &str = "speedrun:topics";
/// `col.conf` key holding the card->topic crosswalk
/// (stringified `card_id -> topic_id`).
pub(crate) const CARD_TOPICS_KEY: &str = "speedrun:card_topics";
/// `col.conf` key holding per-topic weakness (`topic_id -> weakness` in 0..1).
pub(crate) const WEAKNESS_KEY: &str = "speedrun:weakness";

/// A canonical exam topic with its blueprint weight, as stored in `col.conf`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopicInfo {
    pub name: String,
    /// Relative weight of this topic on the exam blueprint (>= 0).
    pub blueprint_weight: f64,
}

impl Collection {
    /// Canonical topic taxonomy: `topic_id -> {name, blueprint_weight}`.
    ///
    /// Returns an empty map when nothing has been stored yet.
    pub fn speedrun_topics(&self) -> Result<HashMap<String, TopicInfo>> {
        Ok(self
            .get_config_optional::<HashMap<String, TopicInfo>, _>(TOPICS_KEY)
            .unwrap_or_default())
    }

    /// Card->topic crosswalk: `card_id -> topic_id`.
    ///
    /// Returns an empty map when nothing has been stored yet.
    pub fn speedrun_card_topics(&self) -> Result<HashMap<CardId, String>> {
        let stored: HashMap<String, String> = self
            .get_config_optional(CARD_TOPICS_KEY)
            .unwrap_or_default();
        let mut out = HashMap::with_capacity(stored.len());
        for (card_id, topic_id) in stored {
            let id = match card_id.parse::<i64>() {
                Ok(n) => CardId(n),
                Err(_) => invalid_input!("invalid card id in crosswalk: {card_id}"),
            };
            out.insert(id, topic_id);
        }
        Ok(out)
    }

    /// Per-topic weakness: `topic_id -> weakness` (0.0 = strong, 1.0 = weak).
    ///
    /// Returns an empty map when nothing has been stored yet.
    pub fn speedrun_topic_weakness(&self) -> Result<HashMap<String, f64>> {
        Ok(self
            .get_config_optional::<HashMap<String, f64>, _>(WEAKNESS_KEY)
            .unwrap_or_default())
    }

    /// Persist the topic taxonomy, the card->topic crosswalk, and per-topic
    /// weakness in `col.conf`, fully replacing any previously stored values (no
    /// stale merge). Undo-safe: all three keys are written in a single
    /// transaction via the standard config/op/undo machinery.
    pub(crate) fn set_speedrun_topic_weights(
        &mut self,
        topics: HashMap<String, TopicInfo>,
        card_topics: HashMap<CardId, String>,
        weakness: HashMap<String, f64>,
    ) -> Result<OpChanges> {
        // JSON object keys must be strings, so the crosswalk is stored with the
        // card id stringified; the accessor parses it back to a CardId.
        let stored_card_topics: HashMap<String, String> = card_topics
            .into_iter()
            .map(|(card_id, topic_id)| (card_id.0.to_string(), topic_id))
            .collect();
        self.transact(Op::UpdateConfig, |col| {
            col.set_config(TOPICS_KEY, &topics)?;
            col.set_config(CARD_TOPICS_KEY, &stored_card_topics)?;
            col.set_config(WEAKNESS_KEY, &weakness)?;
            Ok(())
        })
        .map(|out| out.changes)
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use anki_proto::speedrun::TopicWeakness;

    use super::*;
    use crate::services::SpeedrunService;

    fn sample_request() -> SetTopicWeightsRequest {
        SetTopicWeightsRequest {
            topics: vec![
                Topic {
                    id: "cardio".into(),
                    name: "Cardiology".into(),
                    blueprint_weight: 0.25,
                },
                Topic {
                    id: "renal".into(),
                    name: "Nephrology".into(),
                    blueprint_weight: 0.1,
                },
            ],
            card_topics: vec![CardTopic {
                card_id: 42,
                topic_id: "cardio".into(),
            }],
            weaknesses: vec![TopicWeakness {
                topic_id: "cardio".into(),
                weakness: 0.8,
            }],
        }
    }

    /// R1: the three accessors read back exactly what was stored.
    #[test]
    fn stores_and_reads_back() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.set_topic_weights(sample_request())?;

        let topics = col.speedrun_topics()?;
        assert_eq!(topics.len(), 2);
        assert_eq!(
            topics.get("cardio"),
            Some(&TopicInfo {
                name: "Cardiology".into(),
                blueprint_weight: 0.25,
            })
        );
        assert_eq!(
            topics.get("renal"),
            Some(&TopicInfo {
                name: "Nephrology".into(),
                blueprint_weight: 0.1,
            })
        );

        let card_topics = col.speedrun_card_topics()?;
        assert_eq!(card_topics.len(), 1);
        assert_eq!(card_topics.get(&CardId(42)), Some(&"cardio".to_string()));

        let weakness = col.speedrun_topic_weakness()?;
        assert_eq!(weakness.len(), 1);
        assert_eq!(weakness.get("cardio"), Some(&0.8));

        Ok(())
    }

    /// R2: a second call fully replaces prior values (no stale merge); an empty
    /// request clears everything back to empty maps.
    #[test]
    fn replaces_and_clears() -> Result<()> {
        let mut col = Collection::new();
        let _ = col.set_topic_weights(sample_request())?;

        // A second call with different data fully replaces the previous data.
        let _ = col.set_topic_weights(SetTopicWeightsRequest {
            topics: vec![Topic {
                id: "neuro".into(),
                name: "Neurology".into(),
                blueprint_weight: 0.4,
            }],
            card_topics: vec![CardTopic {
                card_id: 7,
                topic_id: "neuro".into(),
            }],
            weaknesses: vec![TopicWeakness {
                topic_id: "neuro".into(),
                weakness: 0.3,
            }],
        })?;

        let topics = col.speedrun_topics()?;
        assert_eq!(topics.len(), 1);
        assert!(topics.contains_key("neuro"));
        assert!(!topics.contains_key("cardio"));

        let card_topics = col.speedrun_card_topics()?;
        assert_eq!(card_topics.len(), 1);
        assert_eq!(card_topics.get(&CardId(7)), Some(&"neuro".to_string()));
        assert!(!card_topics.contains_key(&CardId(42)));

        let weakness = col.speedrun_topic_weakness()?;
        assert_eq!(weakness.len(), 1);
        assert_eq!(weakness.get("neuro"), Some(&0.3));

        // An empty request clears all three stores back to empty maps.
        let _ = col.set_topic_weights(SetTopicWeightsRequest::default())?;
        assert!(col.speedrun_topics()?.is_empty());
        assert!(col.speedrun_card_topics()?.is_empty());
        assert!(col.speedrun_topic_weakness()?.is_empty());

        Ok(())
    }

    /// R3: undo after `set_topic_weights` restores the previous config state
    /// and leaves the database uncorrupted.
    #[test]
    fn undo_restores_previous_state() -> Result<()> {
        let mut col = Collection::new();

        // First write establishes a known prior state.
        let _ = col.set_topic_weights(sample_request())?;
        // Second write changes it.
        let _ = col.set_topic_weights(SetTopicWeightsRequest {
            topics: vec![Topic {
                id: "neuro".into(),
                name: "Neurology".into(),
                blueprint_weight: 0.4,
            }],
            card_topics: vec![],
            weaknesses: vec![],
        })?;
        assert!(col.speedrun_topics()?.contains_key("neuro"));

        // Undo the second write -> back to the first write's state.
        col.undo()?;
        let topics = col.speedrun_topics()?;
        assert_eq!(topics.len(), 2);
        assert!(topics.contains_key("cardio"));
        assert_eq!(
            col.speedrun_card_topics()?.get(&CardId(42)),
            Some(&"cardio".to_string())
        );
        assert_eq!(col.speedrun_topic_weakness()?.get("cardio"), Some(&0.8));

        // Undo the first write -> back to empty.
        col.undo()?;
        assert!(col.speedrun_topics()?.is_empty());
        assert!(col.speedrun_card_topics()?.is_empty());
        assert!(col.speedrun_topic_weakness()?.is_empty());

        // No corruption introduced.
        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");

        Ok(())
    }

    /// R4: accessors return empty maps on a fresh collection (no panics).
    #[test]
    fn empty_on_fresh_collection() -> Result<()> {
        let col = Collection::new();
        assert!(col.speedrun_topics()?.is_empty());
        assert!(col.speedrun_card_topics()?.is_empty());
        assert!(col.speedrun_topic_weakness()?.is_empty());
        Ok(())
    }
}
