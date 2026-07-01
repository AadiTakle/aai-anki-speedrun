# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for the points-at-stake display RPC ("Today's focus").

Mirrors the Rust acceptance tests through the ``col`` backend: with topics
weighted and per-topic weakness seeded, ``get_points_at_stake`` returns the
taxonomy ranked by ``blueprint_weight * weakness`` descending (deterministic
tie-break by ``topic_id`` ascending). A topic with no weakness entry gets
weakness 0.0 / points 0.0 and sorts last, even when its blueprint weight is the
largest — proving the ranking key is points, not raw weight.
"""

from anki import speedrun_pb2
from tests.shared import getEmptyCol


def test_points_at_stake_orders_highest_first():
    col = getEmptyCol()

    # points: cardio 0.5*0.9=0.45, renal 0.4*0.5=0.20, gi 0.2*0.3=0.06.
    # immuno has the LARGEST blueprint weight but NO weakness entry, so its
    # points are 0.0 and it must sort last.
    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id="gi", name="Gastroenterology", blueprint_weight=0.2),
            speedrun_pb2.Topic(id="immuno", name="Immunology", blueprint_weight=1.0),
            speedrun_pb2.Topic(id="cardio", name="Cardiology", blueprint_weight=0.5),
            speedrun_pb2.Topic(id="renal", name="Nephrology", blueprint_weight=0.4),
        ],
        card_topics=[],
        weaknesses=[
            speedrun_pb2.TopicWeakness(topic_id="cardio", weakness=0.9),
            speedrun_pb2.TopicWeakness(topic_id="renal", weakness=0.5),
            speedrun_pb2.TopicWeakness(topic_id="gi", weakness=0.3),
        ],
    )

    # The response message has a single field, so the generated backend unwraps
    # it: get_points_at_stake() returns the repeated PointsAtStakeTopic directly.
    topics = col._backend.get_points_at_stake()

    ids = [t.topic_id for t in topics]
    assert ids == ["cardio", "renal", "gi", "immuno"]

    points = {t.topic_id: t.points for t in topics}
    assert abs(points["cardio"] - 0.45) < 1e-9
    assert abs(points["renal"] - 0.20) < 1e-9
    assert abs(points["gi"] - 0.06) < 1e-9

    # immuno had no weakness entry -> weakness/points default to 0.0, sorts last
    # despite the largest blueprint weight; other fields carry through.
    immuno = topics[-1]
    assert immuno.topic_id == "immuno"
    assert immuno.name == "Immunology"
    assert immuno.blueprint_weight == 1.0
    assert immuno.weakness == 0.0
    assert immuno.points == 0.0


def test_points_at_stake_empty_taxonomy():
    col = getEmptyCol()
    topics = col._backend.get_points_at_stake()
    assert list(topics) == []
