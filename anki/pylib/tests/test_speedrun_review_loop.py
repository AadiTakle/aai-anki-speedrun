# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""F10 - Exam-deck review loop: end-to-end proof of the F5 engine change.

This is the automated, re-runnable proof that the mandatory Rust engine change
(F5 points-at-stake review order) works on a real, multi-topic Step 2-style
deck driven through the *actual* v3 scheduler - not just a unit assertion on a
freshly-gathered queue.

On a synthetic 9-card / 3-topic deck (all cards tied at gather time so the
points-at-stake post-sort is what decides the order) it proves:

* Ordering: with ``REVIEW_CARD_ORDER_POINTS_AT_STAKE`` selected the queue comes
  out grouped by ``blueprint_weight(topic) * weakness(topic)`` descending. Cards
  are inserted round-robin across topics, so the plain gather order (due, then a
  hash tiebreak) is interleaved; only the F5 post-sort produces topic blocks.
* Full loop: every due review card can be answered through the scheduler until
  none remain - the session completes, nothing is skipped or stuck.
* No corruption: ``pragma integrity_check`` is ``ok`` afterwards.
* Undo-safe: undoing the last answer restores that card to the due queue.

Mirrors the seeding pattern of ``test_speedrun.py`` and the answer pattern of
``test_schedv3.py``.
"""

from anki import deck_config_pb2, speedrun_pb2
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV
from tests.shared import getEmptyCol

POINTS_AT_STAKE = (
    deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE
)

# (topic id, display name, blueprint_weight, weakness). Points-at-stake is
# weight * weakness, so cardio 0.45 > renal 0.20 > gi 0.06 - unambiguous blocks.
TOPICS = [
    ("cardio", "Cardiology", 0.5, 0.9),
    ("renal", "Nephrology", 0.4, 0.5),
    ("gi", "Gastroenterology", 0.2, 0.3),
]
CARDS_PER_TOPIC = 3

# Good/pass answer button in the v3 scheduler (Again=1, Hard=2, Good=3, Easy=4).
GOOD = 3


def _add_due_review_card(col, front: str) -> int:
    """Add a note and turn its card into a due Review-state card.

    All cards share the same due/interval so the gather-time SQL ordering is a
    tie and the points-at-stake post-sort is what determines the order (mirrors
    ``_add_due_review_card`` in ``test_speedrun.py``).
    """
    note = col.newNote()
    note["Front"] = front
    col.addNote(note)
    c = note.cards()[0]
    c.ivl = 10
    c.due = 0
    c.type = CARD_TYPE_REV
    c.queue = QUEUE_TYPE_REV
    c.flush()
    return c.id


def _build_multi_topic_deck(col) -> dict[int, str]:
    """Create a 9-card / 3-topic deck and seed topics/weights/weakness.

    Cards are added round-robin across topics so their natural gather order is
    interleaved; returns a mapping of card id -> topic id.
    """
    card_topic: dict[int, str] = {}
    # Round-robin insertion: cardio-0, renal-0, gi-0, cardio-1, ...
    for i in range(CARDS_PER_TOPIC):
        for topic_id, _name, _weight, _weakness in TOPICS:
            cid = _add_due_review_card(col, f"{topic_id}-{i}")
            card_topic[cid] = topic_id

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id=tid, name=name, blueprint_weight=weight)
            for tid, name, weight, _weakness in TOPICS
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cid, topic_id=tid)
            for cid, tid in card_topic.items()
        ],
        weaknesses=[
            speedrun_pb2.TopicWeakness(topic_id=tid, weakness=weakness)
            for tid, _name, _weight, weakness in TOPICS
        ],
    )
    return card_topic


def _select_review_order(col, order) -> None:
    conf = col.decks.config_dict_for_deck_id(1)
    conf["reviewOrder"] = order
    col.decks.save(conf)


def test_exam_deck_review_loop_points_at_stake():
    col = getEmptyCol()
    card_topic = _build_multi_topic_deck(col)
    points = {tid: weight * weakness for tid, _n, weight, weakness in TOPICS}

    _select_review_order(col, POINTS_AT_STAKE)

    # --- Ordering: the queue is grouped by weight*weakness descending. --------
    col.sched.reset()
    queued = col.sched.get_queued_cards(fetch_limit=50)
    ordered = [qc.card.id for qc in queued.cards]

    # every due review card is present in the queue
    assert len(ordered) == len(card_topic)

    ordered_topics = [card_topic[cid] for cid in ordered]
    expected_topics = [
        tid for tid, _n, _w, _wk in TOPICS for _ in range(CARDS_PER_TOPIC)
    ]
    assert ordered_topics == expected_topics

    # equivalently, the points-at-stake values are non-increasing
    ordered_points = [points[card_topic[cid]] for cid in ordered]
    assert ordered_points == sorted(ordered_points, reverse=True)

    # --- Full loop: answer every due review card through the scheduler. -------
    col.sched.reset()
    num_due = len(ordered)
    answered = 0
    last_cid = None
    guard = num_due + 5
    while guard > 0:
        c = col.sched.getCard()
        if c is None:
            break
        col.sched.answerCard(c, GOOD)
        last_cid = c.id
        answered += 1
        guard -= 1

    # session completes with no card skipped or stuck
    assert col.sched.getCard() is None
    assert answered == num_due
    assert col.sched.counts() == (0, 0, 0)

    # --- Undo-safe: undoing the last answer restores that card to the queue. --
    # Checked before the raw col.db query below, which discards the undo queue.
    assert col.undo_status().undo, "the last answer should be undoable"
    col.undo()
    col.sched.reset()
    restored = col.sched.getCard()
    assert restored is not None
    assert restored.id == last_cid
    assert col.sched.counts() == (0, 0, 1)

    # --- No corruption after the full answer + undo cycle. --------------------
    assert col.db.scalar("pragma integrity_check") == "ok"
