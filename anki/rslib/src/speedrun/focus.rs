// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! Points-at-stake **display** view — the ranked "Today's focus" list.
//!
//! Presents every canonical topic ordered by how much exam score is at stake
//! if it stays weak: `points = blueprint_weight * weakness`. It is a pure,
//! read-only projection of the F1 topic store (see `crate::speedrun::store`) —
//! it never mutates the collection and never recomputes weakness from cards.
//!
//! This is the *display* surface for the "Today's focus" list; it is distinct
//! from F5's in-queue review ordering in `scheduler/queue/builder`, which
//! sorts the actual due queue. Both read the same F1 signals, but this view
//! ranks the taxonomy for presentation rather than reordering cards.
//!
//! Definitions (frozen with the next-slice contract):
//! - **points**: `blueprint_weight * weakness`, where `weakness` defaults to
//!   `0.0` when a topic has no stored weakness entry (an unweak/uncovered topic
//!   has nothing at stake). The store already clamps `blueprint_weight >= 0`
//!   and `weakness` to `[0, 1]`, so `points` is finite and non-negative.
//! - **order**: `points` descending (study the highest-stakes topic first),
//!   with a deterministic tie-break by `topic_id` ascending so equal-points
//!   topics — and a topic with no weakness data (points `0.0`) — always sort
//!   the same way. An empty taxonomy yields an empty list.

use anki_proto::speedrun::PointsAtStakeResponse;
use anki_proto::speedrun::PointsAtStakeTopic;

use crate::prelude::*;

impl Collection {
    /// Ranked points-at-stake topics for the "Today's focus" display list.
    ///
    /// Read-only: reads the stored taxonomy + per-topic weakness (F1) and
    /// returns one [`PointsAtStakeTopic`] per topic, highest `points` first
    /// (see the module docs for the exact rule). It never mutates the
    /// collection.
    pub(crate) fn points_at_stake(&self) -> Result<PointsAtStakeResponse> {
        let topics = self.speedrun_topics()?;
        let weakness = self.speedrun_topic_weakness()?;

        let mut ranked: Vec<PointsAtStakeTopic> = topics
            .into_iter()
            .map(|(topic_id, info)| {
                // A topic with no stored weakness has nothing at stake yet
                // (0.0), so it ranks last rather than being dropped.
                let weakness = weakness.get(&topic_id).copied().unwrap_or(0.0);
                PointsAtStakeTopic {
                    topic_id,
                    name: info.name,
                    blueprint_weight: info.blueprint_weight,
                    weakness,
                    points: info.blueprint_weight * weakness,
                }
            })
            .collect();

        // Highest points first; deterministic tie-break by topic_id ascending
        // so equal-points topics (and zero-points topics) always sort the same
        // way. `total_cmp` gives a total order over the (store-clamped, finite)
        // points without needing to special-case NaN.
        ranked.sort_by(|a, b| {
            b.points
                .total_cmp(&a.points)
                .then_with(|| a.topic_id.cmp(&b.topic_id))
        });

        Ok(PointsAtStakeResponse { topics: ranked })
    }
}

#[cfg(test)]
mod test {
    use anki_proto::speedrun::SetTopicWeightsRequest;
    use anki_proto::speedrun::Topic;
    use anki_proto::speedrun::TopicWeakness;

    use super::*;
    use crate::services::SpeedrunService;

    /// Seed the F1 taxonomy (id, name, blueprint_weight) and per-topic weakness
    /// in one undo-safe write, mirroring how the store is populated in prod.
    fn seed(col: &mut Collection, topics: &[(&str, &str, f64)], weakness: &[(&str, f64)]) {
        let req = SetTopicWeightsRequest {
            topics: topics
                .iter()
                .map(|(id, name, w)| Topic {
                    id: (*id).into(),
                    name: (*name).into(),
                    blueprint_weight: *w,
                })
                .collect(),
            card_topics: vec![],
            weaknesses: weakness
                .iter()
                .map(|(id, w)| TopicWeakness {
                    topic_id: (*id).into(),
                    weakness: *w,
                })
                .collect(),
        };
        let _ = col.set_topic_weights(req).unwrap();
    }

    /// (a) ranking: three topics with distinct `blueprint_weight * weakness`
    /// come out highest-first, each carrying the right fields + `points`.
    #[test]
    fn ranks_by_points_descending() -> Result<()> {
        let mut col = Collection::new();
        // cardio 0.5*0.9=0.45, renal 0.4*0.5=0.20, gi 0.2*0.3=0.06
        seed(
            &mut col,
            &[
                ("gi", "Gastroenterology", 0.2),
                ("cardio", "Cardiology", 0.5),
                ("renal", "Nephrology", 0.4),
            ],
            &[("cardio", 0.9), ("renal", 0.5), ("gi", 0.3)],
        );

        let resp = col.points_at_stake()?.topics;
        let ids: Vec<&str> = resp.iter().map(|t| t.topic_id.as_str()).collect();
        assert_eq!(
            ids,
            ["cardio", "renal", "gi"],
            "highest points-at-stake first"
        );

        // points are blueprint_weight * weakness.
        assert!(
            (resp[0].points - 0.45).abs() < 1e-9,
            "cardio points {}",
            resp[0].points
        );
        assert!(
            (resp[1].points - 0.20).abs() < 1e-9,
            "renal points {}",
            resp[1].points
        );
        assert!(
            (resp[2].points - 0.06).abs() < 1e-9,
            "gi points {}",
            resp[2].points
        );

        // the other display fields are carried straight through from the store.
        assert_eq!(resp[0].topic_id, "cardio");
        assert_eq!(resp[0].name, "Cardiology");
        assert_eq!(resp[0].blueprint_weight, 0.5);
        assert_eq!(resp[0].weakness, 0.9);
        Ok(())
    }

    /// (b) empty taxonomy -> empty list (no panic).
    #[test]
    fn empty_taxonomy_yields_empty_list() -> Result<()> {
        let col = Collection::new();
        assert!(col.points_at_stake()?.topics.is_empty());
        Ok(())
    }

    /// (c) tie-break: equal-points topics sort by `topic_id` ascending, and a
    /// topic with no weakness entry gets weakness 0.0 / points 0.0 and sorts
    /// last.
    #[test]
    fn ties_break_by_topic_id_and_missing_weakness_sorts_last() -> Result<()> {
        let mut col = Collection::new();
        // alpha, beta, delta all tie at 1.0*0.5 = 0.5; gamma has NO weakness
        // entry -> weakness 0.0, points 0.0 -> sorts last. Topics are seeded in
        // a scrambled order to prove the sort (not insertion) decides output.
        seed(
            &mut col,
            &[
                ("delta", "Delta", 1.0),
                ("gamma", "Gamma", 1.0),
                ("beta", "Beta", 1.0),
                ("alpha", "Alpha", 1.0),
            ],
            &[("beta", 0.5), ("delta", 0.5), ("alpha", 0.5)],
        );

        let resp = col.points_at_stake()?.topics;
        let ids: Vec<&str> = resp.iter().map(|t| t.topic_id.as_str()).collect();
        assert_eq!(
            ids,
            ["alpha", "beta", "delta", "gamma"],
            "equal points tie-break by topic_id asc; missing-weakness topic last"
        );

        // The three tied topics share points 0.5.
        for t in resp.iter().take(3) {
            assert!(
                (t.points - 0.5).abs() < 1e-9,
                "{} points {}",
                t.topic_id,
                t.points
            );
        }

        // gamma: no weakness entry -> weakness 0.0, points 0.0, sorted last.
        let gamma = resp.last().unwrap();
        assert_eq!(gamma.topic_id, "gamma");
        assert_eq!(gamma.weakness, 0.0, "absent weakness defaults to 0.0");
        assert_eq!(gamma.points, 0.0);
        assert_eq!(gamma.blueprint_weight, 1.0);
        Ok(())
    }

    /// Read-only + deterministic: repeated calls agree, no undo step is
    /// created, and the database stays uncorrupted.
    #[test]
    fn read_only_and_deterministic() -> Result<()> {
        let mut col = Collection::new();
        seed(
            &mut col,
            &[("cardio", "Cardiology", 0.5), ("renal", "Nephrology", 0.4)],
            &[("cardio", 0.9), ("renal", 0.5)],
        );

        let undo_before = col.undo_status().last_step;
        let first: Vec<String> = col
            .points_at_stake()?
            .topics
            .into_iter()
            .map(|t| t.topic_id)
            .collect();
        let second: Vec<String> = col
            .points_at_stake()?
            .topics
            .into_iter()
            .map(|t| t.topic_id)
            .collect();
        let undo_after = col.undo_status().last_step;

        assert_eq!(first, second, "identical results across calls");
        assert_eq!(
            undo_before, undo_after,
            "points_at_stake must not create an undo step"
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
