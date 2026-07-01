# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for the Speedrun engine additions.

Mirrors the Rust acceptance tests through the ``col`` backend:

* F5 — with topics weighted and per-topic weakness seeded, selecting the
  points-at-stake review order makes the queue come out sorted by
  ``blueprint_weight * weakness`` descending.
* F6 — a fresh collection has no graded reviews and no blueprint coverage, so
  the backend must abstain (never fabricate a readiness number) while still
  reporting a real coverage of 0% and non-empty reasons.
"""

from anki import deck_config_pb2, speedrun_pb2
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV
from tests.shared import getEmptyCol

POINTS_AT_STAKE = deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE


def _add_due_review_card(col, front: str) -> int:
    note = col.newNote()
    note["Front"] = front
    col.addNote(note)
    c = note.cards()[0]
    # All cards share the same due/interval so the gather-time SQL ordering is a
    # tie and the points-at-stake post-sort is what determines the order.
    c.ivl = 10
    c.due = 0
    c.type = CARD_TYPE_REV
    c.queue = QUEUE_TYPE_REV
    c.flush()
    return c.id


def test_points_at_stake_queue_orders_highest_first():
    col = getEmptyCol()

    # points-at-stake: cardio 0.5*0.9=0.45, renal 0.4*0.5=0.20, gi 0.2*0.3=0.06
    cardio = _add_due_review_card(col, "cardio")
    renal = _add_due_review_card(col, "renal")
    gi = _add_due_review_card(col, "gi")

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id="cardio", name="Cardiology", blueprint_weight=0.5),
            speedrun_pb2.Topic(id="renal", name="Nephrology", blueprint_weight=0.4),
            speedrun_pb2.Topic(id="gi", name="Gastroenterology", blueprint_weight=0.2),
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cardio, topic_id="cardio"),
            speedrun_pb2.CardTopic(card_id=renal, topic_id="renal"),
            speedrun_pb2.CardTopic(card_id=gi, topic_id="gi"),
        ],
        weaknesses=[
            speedrun_pb2.TopicWeakness(topic_id="cardio", weakness=0.9),
            speedrun_pb2.TopicWeakness(topic_id="renal", weakness=0.5),
            speedrun_pb2.TopicWeakness(topic_id="gi", weakness=0.3),
        ],
    )

    # select the points-at-stake review order on the default deck's config
    conf = col.decks.config_dict_for_deck_id(1)
    conf["reviewOrder"] = POINTS_AT_STAKE
    col.decks.save(conf)

    col.sched.reset()
    queued = col.sched.get_queued_cards(fetch_limit=10)
    ordered = [qc.card.id for qc in queued.cards]

    assert ordered == [cardio, renal, gi]


def test_memory_score_abstains_on_fresh_collection():
    col = getEmptyCol()

    score = col._backend.get_memory_score()

    assert score.abstained is True
    assert score.coverage_pct == 0.0
    assert len(score.reasons) > 0
