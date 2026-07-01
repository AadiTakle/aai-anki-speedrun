// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! F3 — turn imported QBank misses into action (the "miss -> focus" loop).
//!
//! `relink_misses` recomputes per-topic weakness from the student's own QBank
//! accuracy, auto-unsuspends the flashcards behind missed questions, and
//! appends one error-log entry per miss — all in a single undo-safe
//! transaction. `error_log` reads those entries back with a clinical-reasoning
//! reframe prompt for the console's "errors" review page. See docs/PRD.md (F3).

use std::collections::HashMap;
use std::collections::HashSet;

use anki_proto::speedrun::ErrorLogEntry;
use anki_proto::speedrun::ErrorLogResponse;
use serde::Deserialize;
use serde::Serialize;

use crate::card::CardQueue;
use crate::prelude::*;
use crate::speedrun::attempts::StoredQuestionAttempt;
use crate::speedrun::store::WEAKNESS_KEY;

/// `col.conf` key holding the miss error-log (JSON array). Namespaced the same
/// way as the other Speedrun stores (`speedrun:attempts`, `speedrun:weakness`).
pub(crate) const ERROR_LOG_KEY: &str = "speedrun:error_log";

/// Honesty guard for thin data: a topic must have at least this many graded
/// attempts before its weakness is recomputed from QBank accuracy. Below it we
/// keep the prior weakness rather than swinging a topic to 0/1 on one or two
/// questions (noise). 5 is a small, documented floor — enough that a single
/// lucky/unlucky answer can't dominate, while still reacting to a short block.
const MIN_ATTEMPTS: u32 = 5;

/// One missed question, as stored in `col.conf`. Mirrors the fields the
/// `speedrun.ErrorLogEntry` proto needs, minus the derived `topic_name` /
/// `reframe_prompt`, which are synthesized on read in
/// [`Collection::get_error_log`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StoredErrorLogEntry {
    pub source: String,
    pub external_id: String,
    pub answered_at: i64,
    pub topic_id: String,
    /// How many suspended cards this specific miss unsuspended when relinked.
    pub unsuspended_cards: u32,
}

/// Dedup key for a stored error-log entry, matching the attempt dedup key.
type ErrorKey = (String, String, i64);

fn error_key(e: &StoredErrorLogEntry) -> ErrorKey {
    (e.source.clone(), e.external_id.clone(), e.answered_at)
}

impl Collection {
    /// All stored error-log entries, in insertion order.
    pub(crate) fn speedrun_error_log(&self) -> Result<Vec<StoredErrorLogEntry>> {
        Ok(self
            .get_config_optional::<Vec<StoredErrorLogEntry>, _>(ERROR_LOG_KEY)
            .unwrap_or_default())
    }

    /// F3 — turn imported QBank misses into action, undo-safely.
    ///
    /// In a single transaction this:
    /// 1. recomputes per-topic weakness = `1 - accuracy` from the stored QBank
    ///    attempts, **replacing** any seeded weakness for topics with at least
    ///    [`MIN_ATTEMPTS`] graded attempts (thin-data topics keep their prior
    ///    weakness — the honesty guard);
    /// 2. auto-unsuspends the cards mapped (via the F1 crosswalk) to a topic
    ///    the student missed, but only those currently `Suspended` — cards of
    ///    topics with no miss are left untouched (the SPOV "selective
    ///    unsuspension");
    /// 3. appends one error-log entry per miss (deduped on `(source,
    ///    external_id, answered_at)` so re-running is idempotent).
    ///
    /// All three writes go through the standard config/card undo machinery, so
    /// a single `undo()` restores the prior weakness *and* the prior
    /// suspension.
    pub(crate) fn relink_misses(&mut self) -> Result<OpChanges> {
        let attempts = self.speedrun_question_attempts()?;
        let card_topics = self.speedrun_card_topics()?;
        // Start from the prior weakness; we overwrite only well-sampled topics.
        let mut new_weakness = self.speedrun_topic_weakness()?;
        let mut error_log = self.speedrun_error_log()?;

        // Per-topic (correct, total) over attempts that carry a topic id.
        let mut tally: HashMap<String, (u32, u32)> = HashMap::new();
        for a in &attempts {
            if a.topic_id.is_empty() {
                continue;
            }
            let entry = tally.entry(a.topic_id.clone()).or_insert((0, 0));
            entry.1 += 1;
            if a.correct {
                entry.0 += 1;
            }
        }

        // Overwrite only the topics with enough data; thin-data / no-data
        // topics keep their prior weakness (the honesty guard).
        for (topic_id, (correct, total)) in &tally {
            if *total >= MIN_ATTEMPTS {
                let accuracy = f64::from(*correct) / f64::from(*total);
                new_weakness.insert(topic_id.clone(), 1.0 - accuracy);
            }
        }

        // Reverse crosswalk: topic -> its mapped card ids.
        let mut cards_by_topic: HashMap<String, Vec<CardId>> = HashMap::new();
        for (card_id, topic_id) in &card_topics {
            cards_by_topic
                .entry(topic_id.clone())
                .or_default()
                .push(*card_id);
        }

        // Misses (wrong + mapped to a topic) in a deterministic order, so the
        // per-miss unsuspend attribution is stable across runs.
        let mut misses: Vec<&StoredQuestionAttempt> = attempts
            .iter()
            .filter(|a| !a.correct && !a.topic_id.is_empty())
            .collect();
        misses.sort_by(|a, b| {
            a.answered_at
                .cmp(&b.answered_at)
                .then(a.source.cmp(&b.source))
                .then(a.external_id.cmp(&b.external_id))
        });

        self.transact(Op::UpdateConfig, |col| {
            let usn = col.usn()?;
            // Entries already stored, so re-running relink doesn't duplicate.
            let mut logged: HashSet<ErrorKey> = error_log.iter().map(error_key).collect();
            // A card is only counted once even if several misses share its topic.
            let mut already_unsuspended: HashSet<CardId> = HashSet::new();
            let mut new_entries: Vec<StoredErrorLogEntry> = Vec::new();

            for &miss in &misses {
                let mut unsuspended = 0u32;
                if let Some(cids) = cards_by_topic.get(&miss.topic_id) {
                    for &cid in cids {
                        if already_unsuspended.contains(&cid) {
                            continue;
                        }
                        let Some(card) = col.storage.get_card(cid)? else {
                            continue;
                        };
                        // Only currently-suspended cards; buried/active untouched.
                        if card.queue == CardQueue::Suspended {
                            let original = card.clone();
                            let mut card = card;
                            card.restore_queue_from_type();
                            col.update_card_inner(&mut card, original, usn)?;
                            already_unsuspended.insert(cid);
                            unsuspended += 1;
                        }
                    }
                }

                if logged.insert(error_key_from(miss)) {
                    new_entries.push(StoredErrorLogEntry {
                        source: miss.source.clone(),
                        external_id: miss.external_id.clone(),
                        answered_at: miss.answered_at,
                        topic_id: miss.topic_id.clone(),
                        unsuspended_cards: unsuspended,
                    });
                }
            }

            error_log.extend(new_entries);
            col.set_config(ERROR_LOG_KEY, &error_log)?;
            col.set_config(WEAKNESS_KEY, &new_weakness)?;
            Ok(())
        })
        .map(|out| out.changes)
    }

    /// F3 — read the miss error log for the console, most recent first.
    ///
    /// Joins `topic_name` from the F1 taxonomy and synthesizes a short
    /// clinical-reasoning `reframe_prompt` per entry. Read-only.
    pub(crate) fn get_error_log(&self) -> Result<ErrorLogResponse> {
        let topics = self.speedrun_topics()?;
        let stored = self.speedrun_error_log()?;

        let mut entries: Vec<ErrorLogEntry> = stored
            .into_iter()
            .map(|e| {
                let topic_name = topics
                    .get(&e.topic_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                // Prefer the human topic name; fall back to the id so the
                // prompt is never blank (misses always carry a topic id).
                let label = if topic_name.is_empty() {
                    e.topic_id.clone()
                } else {
                    topic_name.clone()
                };
                let reframe_prompt = format!(
                    "You missed a {label} question — what single finding would have changed your answer?"
                );
                ErrorLogEntry {
                    source: e.source,
                    external_id: e.external_id,
                    answered_at: e.answered_at,
                    topic_id: e.topic_id,
                    topic_name,
                    reframe_prompt,
                    unsuspended_cards: e.unsuspended_cards,
                }
            })
            .collect();

        // Most recent first, with a deterministic tie-break for equal times.
        entries.sort_by(|a, b| {
            b.answered_at
                .cmp(&a.answered_at)
                .then(a.source.cmp(&b.source))
                .then(a.external_id.cmp(&b.external_id))
        });

        Ok(ErrorLogResponse { entries })
    }
}

/// Dedup key for an incoming miss (borrowed attempt), matching [`error_key`].
fn error_key_from(a: &StoredQuestionAttempt) -> ErrorKey {
    (a.source.clone(), a.external_id.clone(), a.answered_at)
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use anki_proto::speedrun::TopicWeakness;

    use super::*;
    use crate::card::CardQueue;
    use crate::card::CardType;
    use crate::services::SpeedrunService;
    use crate::speedrun::attempts::StoredQuestionAttempt;

    /// A stored QBank attempt for a given topic (source fixed; ids/times vary).
    fn attempt(
        external_id: &str,
        answered_at: i64,
        topic: &str,
        correct: bool,
    ) -> StoredQuestionAttempt {
        StoredQuestionAttempt {
            source: "uworld".into(),
            external_id: external_id.into(),
            answered_at,
            topic_id: topic.into(),
            correct,
            seconds: 60,
        }
    }

    /// Add a Review-state card already Suspended, returning its id.
    fn add_suspended_review_card(col: &mut Collection) -> CardId {
        let mut card = Card {
            ctype: CardType::Review,
            queue: CardQueue::Suspended,
            ..Default::default()
        };
        col.add_card(&mut card).unwrap();
        card.id
    }

    /// Seed the F1 taxonomy + per-topic (prior) weakness in one undo-safe
    /// write.
    fn seed_topics(col: &mut Collection, topics: &[(&str, &str, f64)], weakness: &[(&str, f64)]) {
        seed_topics_with_crosswalk(col, topics, &[], weakness);
    }

    /// Seed taxonomy + card->topic crosswalk + per-topic weakness in one write.
    fn seed_topics_with_crosswalk(
        col: &mut Collection,
        topics: &[(&str, &str, f64)],
        crosswalk: &[(i64, &str)],
        weakness: &[(&str, f64)],
    ) {
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
            weaknesses: weakness
                .iter()
                .map(|(topic_id, w)| TopicWeakness {
                    topic_id: (*topic_id).into(),
                    weakness: *w,
                })
                .collect(),
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    /// Weakness is recomputed from QBank accuracy (weakness = 1 - accuracy),
    /// replacing any seeded weakness: a mostly-wrong topic becomes weak, a
    /// mostly-right topic becomes strong.
    #[test]
    fn weakness_recomputed_from_attempts() -> Result<()> {
        let mut col = Collection::new();
        seed_topics(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            // seeded priors that must be REPLACED by the recomputed values.
            &[("cardio", 0.5), ("renal", 0.5)],
        );

        let mut attempts = Vec::new();
        // cardio: 8 attempts, 2 correct -> accuracy 0.25 -> weakness 0.75.
        for i in 0..8i64 {
            attempts.push(attempt(&format!("c{i}"), 1000 + i, "cardio", i < 2));
        }
        // renal: 8 attempts, 7 correct -> accuracy 0.875 -> weakness 0.125.
        for i in 0..8i64 {
            attempts.push(attempt(&format!("r{i}"), 2000 + i, "renal", i < 7));
        }
        let _ = col.import_qbank_data(attempts, vec![])?;

        let _ = col.relink_misses()?;

        let w = col.speedrun_topic_weakness()?;
        let cardio = *w.get("cardio").expect("cardio weakness");
        let renal = *w.get("renal").expect("renal weakness");
        assert!((cardio - 0.75).abs() < 1e-9, "cardio weakness {cardio}");
        assert!((renal - 0.125).abs() < 1e-9, "renal weakness {renal}");
        assert!(
            cardio > renal,
            "mostly-wrong topic is weaker than mostly-right"
        );
        Ok(())
    }

    /// Thin-data honesty guard: a topic with fewer than MIN_ATTEMPTS attempts
    /// keeps its prior weakness rather than swinging on noise; a topic with
    /// enough attempts is recomputed.
    #[test]
    fn thin_data_guard_keeps_prior_weakness() -> Result<()> {
        let mut col = Collection::new();
        seed_topics(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[("cardio", 0.3), ("renal", 0.6)],
        );

        let mut attempts = Vec::new();
        // cardio: 6 attempts (>= MIN_ATTEMPTS), all wrong -> recomputed to 1.0.
        for i in 0..6i64 {
            attempts.push(attempt(&format!("c{i}"), 1000 + i, "cardio", false));
        }
        // renal: 2 attempts (< MIN_ATTEMPTS), both wrong -> prior 0.6 KEPT.
        for i in 0..2i64 {
            attempts.push(attempt(&format!("r{i}"), 2000 + i, "renal", false));
        }
        let _ = col.import_qbank_data(attempts, vec![])?;

        let _ = col.relink_misses()?;

        let w = col.speedrun_topic_weakness()?;
        assert!(
            (*w.get("cardio").unwrap() - 1.0).abs() < 1e-9,
            "cardio recomputed to 1.0"
        );
        assert!(
            (*w.get("renal").unwrap() - 0.6).abs() < 1e-9,
            "renal kept prior weakness 0.6 (thin data)"
        );
        Ok(())
    }

    /// Selective unsuspension: a miss in topic T unsuspends T's suspended
    /// cards, while a topic with no miss keeps its cards suspended.
    #[test]
    fn miss_unsuspends_topic_cards_selectively() -> Result<()> {
        let mut col = Collection::new();
        let cardio_card = add_suspended_review_card(&mut col);
        let renal_card = add_suspended_review_card(&mut col);
        seed_topics_with_crosswalk(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[(cardio_card.0, "cardio"), (renal_card.0, "renal")],
            &[],
        );

        let _ = col.import_qbank_data(
            vec![
                attempt("c1", 1000, "cardio", false), // miss -> unsuspend cardio's card
                attempt("r1", 2000, "renal", true),   // correct -> renal untouched
            ],
            vec![],
        )?;

        let _ = col.relink_misses()?;

        let cardio = col.storage.get_card(cardio_card)?.unwrap();
        let renal = col.storage.get_card(renal_card)?.unwrap();
        assert_ne!(
            cardio.queue,
            CardQueue::Suspended,
            "missed topic's card was unsuspended"
        );
        assert_eq!(
            cardio.queue,
            CardQueue::Review,
            "restored to its review queue"
        );
        assert_eq!(
            renal.queue,
            CardQueue::Suspended,
            "topic with no miss keeps its card suspended"
        );
        Ok(())
    }

    /// Undo-safety: a single `undo()` after `relink_misses` restores BOTH the
    /// prior weakness AND the prior suspension state, with no corruption.
    #[test]
    fn undo_restores_weakness_and_suspension() -> Result<()> {
        let mut col = Collection::new();
        let card = add_suspended_review_card(&mut col);
        seed_topics_with_crosswalk(
            &mut col,
            &[("cardio", "Cardiology", 0.5)],
            &[(card.0, "cardio")],
            &[("cardio", 0.2)], // prior weakness
        );

        let mut attempts = Vec::new();
        for i in 0..5i64 {
            attempts.push(attempt(&format!("c{i}"), 1000 + i, "cardio", false));
        }
        let _ = col.import_qbank_data(attempts, vec![])?;

        let _ = col.relink_misses()?;
        // Post-conditions: weakness recomputed to 1.0 and the card unsuspended.
        assert!(
            (*col.speedrun_topic_weakness()?.get("cardio").unwrap() - 1.0).abs() < 1e-9,
            "weakness recomputed before undo"
        );
        assert_ne!(
            col.storage.get_card(card)?.unwrap().queue,
            CardQueue::Suspended,
            "card unsuspended before undo"
        );

        col.undo()?;

        assert!(
            (*col.speedrun_topic_weakness()?.get("cardio").unwrap() - 0.2).abs() < 1e-9,
            "undo restored prior weakness"
        );
        assert_eq!(
            col.storage.get_card(card)?.unwrap().queue,
            CardQueue::Suspended,
            "undo restored the prior suspension"
        );

        let integrity: String = col
            .storage
            .db
            .pragma_query_value(None, "integrity_check", |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
        Ok(())
    }

    /// The error log has one entry per miss (correct attempts excluded), most
    /// recent first, each with a joined topic name, a non-empty reframe prompt,
    /// and the count of cards that miss unsuspended.
    #[test]
    fn error_log_one_entry_per_miss_recent_first() -> Result<()> {
        let mut col = Collection::new();
        let c1 = add_suspended_review_card(&mut col);
        let c2 = add_suspended_review_card(&mut col);
        let r1 = add_suspended_review_card(&mut col);
        seed_topics_with_crosswalk(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[(c1.0, "cardio"), (c2.0, "cardio"), (r1.0, "renal")],
            &[],
        );

        let _ = col.import_qbank_data(
            vec![
                attempt("cardio-q", 1000, "cardio", false), // miss, older
                attempt("renal-q", 5000, "renal", false),   // miss, newer
                attempt("ok-q", 9000, "cardio", true),      // correct -> not logged
            ],
            vec![],
        )?;

        let _ = col.relink_misses()?;
        let resp = col.get_error_log()?;

        assert_eq!(
            resp.entries.len(),
            2,
            "one entry per miss (correct excluded)"
        );
        // Most-recent-first: renal (answered_at 5000) before cardio (1000).
        assert_eq!(resp.entries[0].external_id, "renal-q");
        assert_eq!(resp.entries[1].external_id, "cardio-q");

        // Topic name is joined and the reframe prompt references it.
        assert_eq!(resp.entries[0].topic_name, "Nephrology");
        assert!(
            resp.entries[0].reframe_prompt.contains("Nephrology"),
            "reframe: {}",
            resp.entries[0].reframe_prompt
        );
        assert!(!resp.entries[0].reframe_prompt.is_empty());
        assert!(!resp.entries[1].reframe_prompt.is_empty());

        // renal's miss unsuspended its 1 card; cardio's miss unsuspended its 2.
        assert_eq!(resp.entries[0].unsuspended_cards, 1);
        assert_eq!(resp.entries[1].unsuspended_cards, 2);
        Ok(())
    }
}
