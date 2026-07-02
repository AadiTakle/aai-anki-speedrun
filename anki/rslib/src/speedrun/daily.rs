// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! The daily loop rendered as a **progressing to-do list** for the "Today"
//! console — the merged "what to do next" surface.
//!
//! The PRD daily loop (Plan → Q-block → auto-link → Review → Close → Stop) is
//! projected onto four ordered tasks, each with a `done`/`current`/`upcoming`
//! state derived **read-only from real signals** — never a manual checkbox that
//! could show fabricated progress. It creates no state and writes nothing, so
//! it is trivially undo-safe, and it resets every day because every "today"
//! test is taken against the scheduler's day rollover.
//!
//! Definitions:
//! - **today_start**: the last day-rollover, `timing_today().next_day_at - 1
//!   day` (unix seconds). Everything "today" is measured against it, so the
//!   plan resets automatically at the rollover.
//! - **plan**: always `done` — you are looking at the plan right now.
//! - **qblock**: `done` when any QBank aggregate was imported today
//!   (`speedrun_qbank_last_import_secs >= today_start`).
//! - **review**: `done` when no mapped card is studyable right now (`remaining
//!   == 0`). `done_count` = distinct cards with a `revlog` row today;
//!   `total_count = done_count + remaining` — a live "N of M" for the console's
//!   progress bar.
//! - **close**: `done` when both qblock and review are done (the loop is
//!   closed); it is passive (never the CURRENT step).
//! - **current**: the first *not-done* task in loop order — the one "do this
//!   now" step that carries the console's CTA. When every task is done there is
//!   no current step and the console shows the honest "loop closed — rest"
//!   state.

use anki_proto::speedrun::daily_task::State as TaskState;
use anki_proto::speedrun::DailyPlanResponse;
use anki_proto::speedrun::DailyTask;

use crate::card::CardQueue;
use crate::prelude::*;

impl Collection {
    /// The daily loop as a progressing to-do list (see the module docs for each
    /// task's exact derivation). Read-only: it never mutates the collection.
    pub(crate) fn daily_plan(&mut self) -> Result<DailyPlanResponse> {
        // Start of the collection's current day (unix seconds), from the
        // scheduler rollover — so every "today" test resets at the rollover.
        let today_start_secs = self.timing_today()?.next_day_at.0 - 86_400;
        let today_start_ms = today_start_secs * 1000;

        // qblock: were today's QBank results imported yet?
        let qblock_done = self
            .speedrun_qbank_last_import_secs()
            .is_some_and(|secs| secs >= today_start_secs);

        // review: how many mapped cards are still studyable, and how many
        // distinct cards were reviewed today (the live progress numerator).
        let remaining = self.studyable_review_remaining()?;
        let reviewed_today = self.cards_reviewed_today(today_start_ms)?;
        let review_done = remaining == 0;
        let review_total = reviewed_today + remaining;

        let close_done = qblock_done && review_done;

        // CURRENT is the first not-done task in loop order. plan is always done;
        // close is passive, so the CTA lands on qblock or review.
        let current_id: &str = if !qblock_done {
            "qblock"
        } else if !review_done {
            "review"
        } else {
            ""
        };

        let review_detail = if review_done {
            if reviewed_today > 0 {
                format!("cleared \u{b7} {reviewed_today} today")
            } else {
                "nothing due".to_string()
            }
        } else {
            format!("{reviewed_today} / {review_total} done")
        };

        let qblock_detail = if qblock_done {
            "imported today"
        } else {
            "no results yet"
        };

        let tasks = vec![
            task(
                "plan",
                "Review Today's focus",
                "today's plan",
                true,
                0,
                0,
                current_id,
            ),
            task(
                "qblock",
                "Import your QBank results",
                qblock_detail,
                qblock_done,
                0,
                0,
                current_id,
            ),
            task(
                "review",
                "Clear your review queue",
                &review_detail,
                review_done,
                reviewed_today,
                review_total,
                current_id,
            ),
            task(
                "close",
                "Close the loop",
                "scores refresh \u{b7} rest",
                close_done,
                0,
                0,
                current_id,
            ),
        ];

        Ok(DailyPlanResponse { tasks })
    }

    /// Count mapped cards that are studyable right now (the STAT review queue).
    /// Mirrors the "studyable" queue-membership check used by the next-action
    /// recommender; dangling crosswalk ids (card deleted) are skipped.
    fn studyable_review_remaining(&self) -> Result<u32> {
        let mut count = 0u32;
        for (card_id, _topic_id) in self.speedrun_card_topics()? {
            if let Some(card) = self.storage.get_card(card_id)? {
                if is_studyable(card.queue) {
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Distinct cards that have a `revlog` row on or after `today_start_ms`
    /// (revlog ids are epoch milliseconds), i.e. cards actually reviewed today.
    fn cards_reviewed_today(&self, today_start_ms: i64) -> Result<u32> {
        let n: u32 = self.storage.db.query_row(
            "select count(distinct cid) from revlog where id >= ?",
            [today_start_ms],
            |row| row.get(0),
        )?;
        Ok(n)
    }
}

/// Build one task, assigning `done -> DONE`, else `CURRENT` when it is the
/// chosen current step, else `UPCOMING`.
fn task(
    id: &str,
    label: &str,
    detail: &str,
    done: bool,
    done_count: u32,
    total_count: u32,
    current_id: &str,
) -> DailyTask {
    let state = if done {
        TaskState::Done
    } else if id == current_id {
        TaskState::Current
    } else {
        TaskState::Upcoming
    };
    DailyTask {
        id: id.to_string(),
        label: label.to_string(),
        detail: detail.to_string(),
        state: state as i32,
        done_count,
        total_count,
    }
}

/// Whether a card in this queue is studyable right now (active study queues:
/// New / intraday Learn / interday DayLearn / Review). Suspended, buried, and
/// preview cards are not.
fn is_studyable(queue: CardQueue) -> bool {
    matches!(
        queue,
        CardQueue::New | CardQueue::Learn | CardQueue::DayLearn | CardQueue::Review
    )
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::CardTopic;
    use anki_proto::speedrun::QbankTopicResult;
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;

    use super::*;
    use crate::card::CardType;
    use crate::services::SpeedrunService;

    fn add_card_in_queue(col: &mut Collection, queue: CardQueue) -> CardId {
        let ctype = match queue {
            CardQueue::New => CardType::New,
            CardQueue::Learn | CardQueue::DayLearn => CardType::Learn,
            _ => CardType::Review,
        };
        let mut card = Card {
            ctype,
            queue,
            ..Default::default()
        };
        col.add_card(&mut card).unwrap();
        card.id
    }

    /// Map cards to a canonical topic (only the crosswalk matters to the review
    /// count) in one undo-safe write.
    fn seed_crosswalk(col: &mut Collection, crosswalk: &[(CardId, &str)]) {
        let req = SetTopicWeightsRequest {
            topics: vec![Topic {
                id: "cardio".into(),
                name: "Cardiology".into(),
                blueprint_weight: 1.0,
            }],
            card_topics: crosswalk
                .iter()
                .map(|(cid, tid)| CardTopic {
                    card_id: cid.0,
                    topic_id: (*tid).into(),
                })
                .collect(),
            weaknesses: vec![],
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    /// Insert `n` revlog rows dated "today" for `cid` (ids are epoch ms; now is
    /// within the current collection day).
    fn seed_revlog_today(col: &mut Collection, cid: CardId, n: usize) {
        let base = TimestampSecs::now().0 * 1000;
        for i in 0..n {
            col.storage
                .db
                .execute(
                    "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type)\
                     values (?, ?, 0, 3, 10, 10, 2500, 0, 1)",
                    [base + i as i64, cid.0],
                )
                .unwrap();
        }
    }

    fn find<'a>(resp: &'a DailyPlanResponse, id: &str) -> &'a DailyTask {
        resp.tasks.iter().find(|t| t.id == id).unwrap()
    }

    fn current(resp: &DailyPlanResponse) -> Option<&DailyTask> {
        resp.tasks.iter().find(|t| t.state() == TaskState::Current)
    }

    /// Fresh collection: the loop is returned in order; Plan is already done,
    /// Q-block is the current step (nothing imported), Review is done (nothing
    /// due), Close is upcoming. Exactly one task is CURRENT.
    #[test]
    fn plan_present_and_qblock_current_on_empty() -> Result<()> {
        let mut col = Collection::new();
        let plan = col.daily_plan()?;

        let ids: Vec<&str> = plan.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["plan", "qblock", "review", "close"], "loop order");

        assert_eq!(find(&plan, "plan").state(), TaskState::Done);
        assert_eq!(find(&plan, "qblock").state(), TaskState::Current);
        assert_eq!(
            find(&plan, "review").state(),
            TaskState::Done,
            "nothing due -> review done"
        );
        assert_eq!(find(&plan, "close").state(), TaskState::Upcoming);

        assert_eq!(
            plan.tasks
                .iter()
                .filter(|t| t.state() == TaskState::Current)
                .count(),
            1,
            "exactly one current step"
        );
        Ok(())
    }

    /// With due cards, the current step is Q-block until QBank results are
    /// imported today; after an import it advances to Review.
    #[test]
    fn current_advances_from_qblock_to_review_after_import() -> Result<()> {
        let mut col = Collection::new();
        let c = add_card_in_queue(&mut col, CardQueue::Review);
        seed_crosswalk(&mut col, &[(c, "cardio")]);

        // Before importing: Q-block is the current step; Review waits.
        let before = col.daily_plan()?;
        assert_eq!(current(&before).map(|t| t.id.as_str()), Some("qblock"));
        assert_eq!(find(&before, "review").state(), TaskState::Upcoming);

        // Import today's QBank results -> Q-block done, Review becomes current.
        let _ = col.import_qbank_aggregate(
            "uworld".into(),
            vec![QbankTopicResult {
                topic_id: "cardio".into(),
                correct: 8,
                total: 10,
            }],
        )?;
        let after = col.daily_plan()?;
        assert_eq!(find(&after, "qblock").state(), TaskState::Done);
        assert_eq!(current(&after).map(|t| t.id.as_str()), Some("review"));
        Ok(())
    }

    /// Review carries a live "N of M" derived from cards reviewed today vs
    /// cards still studyable.
    #[test]
    fn review_reports_live_progress_counts() -> Result<()> {
        let mut col = Collection::new();
        // Three studyable mapped cards; one already reviewed today.
        let a = add_card_in_queue(&mut col, CardQueue::Review);
        let b = add_card_in_queue(&mut col, CardQueue::Review);
        let c = add_card_in_queue(&mut col, CardQueue::New);
        seed_crosswalk(&mut col, &[(a, "cardio"), (b, "cardio"), (c, "cardio")]);
        seed_revlog_today(&mut col, a, 2); // 2 rows, 1 distinct card

        let review = find(&col.daily_plan()?, "review").clone();
        assert_eq!(review.state(), TaskState::Upcoming); // qblock leads as current
        assert_eq!(review.done_count, 1, "one distinct card reviewed today");
        assert_eq!(review.total_count, 4, "1 reviewed + 3 studyable remaining");
        assert!(
            review.detail.contains("1 / 4"),
            "detail shows live progress: {}",
            review.detail
        );
        Ok(())
    }

    /// When QBank is imported today and nothing is studyable, the whole loop is
    /// done: Close is DONE and there is no current step (console shows rest).
    #[test]
    fn all_done_closes_the_loop() -> Result<()> {
        let mut col = Collection::new();
        // A suspended mapped card is not studyable -> review has nothing due.
        let s = add_card_in_queue(&mut col, CardQueue::Suspended);
        seed_crosswalk(&mut col, &[(s, "cardio")]);
        let _ = col.import_qbank_aggregate(
            "uworld".into(),
            vec![QbankTopicResult {
                topic_id: "cardio".into(),
                correct: 5,
                total: 5,
            }],
        )?;

        let plan = col.daily_plan()?;
        assert_eq!(find(&plan, "qblock").state(), TaskState::Done);
        assert_eq!(find(&plan, "review").state(), TaskState::Done);
        assert_eq!(find(&plan, "close").state(), TaskState::Done);
        assert!(current(&plan).is_none(), "no current step when loop closed");
        Ok(())
    }

    /// Read-only: computing the plan creates no undo step and leaves the
    /// database uncorrupted.
    #[test]
    fn read_only_and_uncorrupted() -> Result<()> {
        let mut col = Collection::new();
        let c = add_card_in_queue(&mut col, CardQueue::Review);
        seed_crosswalk(&mut col, &[(c, "cardio")]);

        let undo_before = col.undo_status().last_step;
        let _ = col.daily_plan()?;
        let undo_after = col.undo_status().last_step;
        assert_eq!(
            undo_before, undo_after,
            "daily_plan must not create an undo step"
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
