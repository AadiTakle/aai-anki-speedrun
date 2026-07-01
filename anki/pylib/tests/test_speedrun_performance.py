# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for the Speedrun **performance** (application) score.

Mirrors the Rust acceptance tests through the ``col`` backend:

* A fresh collection has no imported QBank attempts, so the backend must abstain
  (never fabricate an accuracy number) while still reporting non-empty reasons
  (the honesty bar).
* After importing enough graded attempts across blueprint topics, the backend
  scores with a valid, non-degenerate range ``0 <= low <= point <= high <= 100``
  and a per-topic breakdown — verifying the new ``PerformanceScore`` proto fields
  round-trip through the RPC.
"""

from anki import speedrun_pb2
from tests.shared import getEmptyCol


def test_performance_score_abstains_on_fresh_collection():
    col = getEmptyCol()

    score = col._backend.get_performance_score()

    assert score.abstained is True
    assert score.point == 0.0
    assert score.coverage_pct == 0.0
    assert len(score.reasons) > 0


def test_performance_score_scores_after_import():
    col = getEmptyCol()

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id="cardio", name="Cardiology", blueprint_weight=1.0),
            speedrun_pb2.Topic(id="renal", name="Nephrology", blueprint_weight=1.0),
        ],
        card_topics=[],
        weaknesses=[],
    )

    attempts = [
        speedrun_pb2.QuestionAttempt(
            source="uworld",
            external_id=f"cardio-{i}",
            answered_at=1000 + i,
            topic_id="cardio",
            correct=i < 24,
            seconds=60,
        )
        for i in range(30)
    ] + [
        speedrun_pb2.QuestionAttempt(
            source="uworld",
            external_id=f"renal-{i}",
            answered_at=2000 + i,
            topic_id="renal",
            correct=i < 18,
            seconds=60,
        )
        for i in range(30)
    ]
    col._backend.import_qbank_data(attempts=attempts, tests=[])

    score = col._backend.get_performance_score()

    assert score.abstained is False
    assert 0.0 <= score.low <= score.point <= score.high <= 100.0
    assert score.high > score.low
    assert abs(score.coverage_pct - 100.0) < 1e-6
    assert len(score.topics) == 2
    assert len(score.reasons) > 0
