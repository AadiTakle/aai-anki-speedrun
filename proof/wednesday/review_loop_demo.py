#!/usr/bin/env python3
# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""F10 - Exam-deck review loop demo (proves the F5 engine change end-to-end).

Human-runnable, re-runnable proof for the Wednesday recording. It builds a
synthetic multi-topic Step 2 CK deck, then drives it through the *real* v3
scheduler to show the mandatory Rust engine change (F5 points-at-stake review
order) working end-to-end:

  1. the same deck under the stock DAY order vs. the new
     REVIEW_CARD_ORDER_POINTS_AT_STAKE order (recency-decayed weighted
     interleaving: the dominant topic leads and recurs early/often, yet no
     topic is ever shown twice in a row);
  2. a full review session - every due card answered until none remain;
  3. no database corruption afterwards (pragma integrity_check);
  4. undo of the last answer restoring that card to the due queue.

Run it (from the repo's `anki/` dir, adjusting the path to this file):

    PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 ./out/pyenv/bin/python \
        ../proof/wednesday/review_loop_demo.py
"""

from __future__ import annotations

import os
import tempfile

from anki import deck_config_pb2, speedrun_pb2
from anki.collection import Collection
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV

DAY = deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_DAY
POINTS_AT_STAKE = deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE
GOOD = 3  # v3 answer button: Again=1, Hard=2, Good=3, Easy=4

# (topic id, display name, blueprint_weight, weakness). points = weight*weakness
# -> cardio 0.45 > renal 0.20 > gi 0.06, so the target order is unambiguous.
TOPICS = [
    ("cardio", "Cardiology", 0.5, 0.9),
    ("renal", "Nephrology", 0.4, 0.5),
    ("gi", "Gastroenterology", 0.2, 0.3),
]
CARDS_PER_TOPIC = 3

POINTS = {tid: w * wk for tid, _n, w, wk in TOPICS}
NAMES = {tid: n for tid, n, _w, _wk in TOPICS}


def open_fresh_collection() -> Collection:
    fd, path = tempfile.mkstemp(suffix=".anki2")
    os.close(fd)
    os.unlink(path)
    return Collection(path)


def add_due_review_card(col: Collection, front: str) -> int:
    """Add a note and turn its one card into a due Review-state card.

    Every card shares due=0 / ivl=10 so the gather-time ordering is a tie and
    the points-at-stake post-sort is what decides the order.
    """
    note = col.newNote()
    note["Front"] = front
    col.addNote(note)
    c = note.cards()[0]
    c.ivl = 10
    c.due = 0
    c.type = CARD_TYPE_REV
    c.queue = QUEUE_TYPE_REV
    # equivalent to the (deprecated) Card.flush(); avoids console noise.
    col.update_cards([c], skip_undo_entry=True)
    return c.id


def build_deck(col: Collection) -> dict[int, str]:
    """Create a 9-card / 3-topic deck (round-robin) and seed topic data."""
    card_topic: dict[int, str] = {}
    for i in range(CARDS_PER_TOPIC):
        for tid, _n, _w, _wk in TOPICS:
            cid = add_due_review_card(col, f"{tid}-{i}")
            card_topic[cid] = tid

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id=tid, name=n, blueprint_weight=w)
            for tid, n, w, _wk in TOPICS
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cid, topic_id=tid)
            for cid, tid in card_topic.items()
        ],
        weaknesses=[
            speedrun_pb2.TopicWeakness(topic_id=tid, weakness=wk)
            for tid, _n, _w, wk in TOPICS
        ],
    )
    return card_topic


def select_review_order(col: Collection, order) -> None:
    conf = col.decks.config_dict_for_deck_id(1)
    conf["reviewOrder"] = order
    col.decks.save(conf)


def queue_order(col: Collection) -> list[int]:
    return [qc.card.id for qc in col.sched.get_queued_cards(fetch_limit=50).cards]


def print_order(col: Collection, card_topic: dict[int, str], label: str) -> list[int]:
    ordered = queue_order(col)
    print(f"\n{label}")
    print(f"    {'#':>2}  {'card_id':>15}  {'topic':<17} {'points_at_stake':>15}")
    for pos, cid in enumerate(ordered, start=1):
        tid = card_topic[cid]
        print(f"    {pos:>2}  {cid:>15}  {NAMES[tid]:<17} {POINTS[tid]:>15.3f}")
    return ordered


def main() -> None:
    col = open_fresh_collection()
    try:
        card_topic = build_deck(col)

        print("=" * 68)
        print("Speedrun F10 - Exam-deck review loop (F5 points-at-stake order)")
        print("=" * 68)
        print(
            f"\nDeck: {len(card_topic)} due review cards across {len(TOPICS)} "
            "Step 2 CK topics."
        )
        print("\nTopic blueprint weight x current weakness = points at stake:")
        print(f"    {'topic':<17} {'weight':>7} {'weakness':>9} {'points':>8}")
        for tid, name, w, wk in TOPICS:
            print(f"    {name:<17} {w:>7.2f} {wk:>9.2f} {POINTS[tid]:>8.3f}")
        print(
            "\nCards were added round-robin (cardio, renal, gi, ...), so their "
            "natural\ngather order is interleaved; only the F5 engine post-sort "
            "reorders them."
        )

        # Baseline: stock DAY order - not aware of topics/points.
        select_review_order(col, DAY)
        print_order(col, card_topic, "--- Stock DAY order (feature OFF) ---")

        # The mandatory engine change: points-at-stake order.
        select_review_order(col, POINTS_AT_STAKE)
        ordered = print_order(
            col, card_topic, "--- POINTS_AT_STAKE order (F5 engine change) ---"
        )
        # Verify the recency-decayed weighted interleaving: the dominant
        # (highest points-at-stake) topic leads, and no topic is ever shown
        # twice in a row (longest single-topic run == 1).
        topics_seq = [card_topic[cid] for cid in ordered]
        dominant = max(POINTS, key=lambda tid: POINTS[tid])
        max_run = run = 1
        for prev, cur in zip(topics_seq, topics_seq[1:]):
            run = run + 1 if cur == prev else 1
            max_run = max(max_run, run)
        dominant_first = topics_seq[0] == dominant
        print("\n    topic sequence: " + " -> ".join(NAMES[t] for t in topics_seq))
        print(
            f"    -> interleaved (dominant '{NAMES[dominant]}' first = "
            f"{dominant_first}; longest single-topic run = {max_run}): "
            f"{'OK' if dominant_first and max_run == 1 else 'FAIL'}"
        )
        assert dominant_first, "dominant topic must lead the interleaved queue"
        assert max_run == 1, f"no topic may repeat back-to-back: {topics_seq}"

        # Full review session through the real scheduler.
        print("\n--- Full review session (answer every due card 'Good') ---")
        total = len(ordered)
        answered = 0
        last_cid = None
        while True:
            c = col.sched.getCard()
            if c is None:
                break
            tid = card_topic[c.id]
            answered += 1
            print(
                f"    review {answered:>2}/{total}: {NAMES[tid]:<17} "
                f"(points {POINTS[tid]:.3f})  ->  Good"
            )
            col.sched.answerCard(c, GOOD)
            last_cid = c.id
        new_c, lrn_c, rev_c = col.sched.counts()
        print(
            f"\n    session complete: answered {answered}/{total}; "
            f"remaining queue counts (new, lrn, rev) = ({new_c}, {lrn_c}, {rev_c})"
        )
        assert answered == total, "every due card should be answered exactly once"
        assert (new_c, lrn_c, rev_c) == (0, 0, 0), "the queue should drain to empty"

        # Undo-safety (checked before the raw col.db query below, which clears
        # the backend undo queue).
        print("\n--- Undo-safety ---")
        undo_name = col.undo_status().undo
        col.undo()
        restored = col.get_card(last_cid)
        back_in_queue = last_cid in queue_order(col)
        print(
            f"    undo '{undo_name}' -> card {last_cid} "
            f"({NAMES[card_topic[last_cid]]}) restored: "
            f"queue={'review' if restored.queue == QUEUE_TYPE_REV else restored.queue}, "
            f"due={restored.due}, back in due queue={back_in_queue}"
        )
        assert back_in_queue, "undo should restore the last card to the due queue"

        # No corruption after the full answer + undo cycle.
        integrity = col.db.scalar("pragma integrity_check")
        print("\n--- Database integrity ---")
        print(f"    pragma integrity_check -> {integrity}")
        assert integrity == "ok", "database must not be corrupted"

        print("\n" + "=" * 68)
        print(
            "PASS: points-at-stake interleaved the deck (dominant topic first, no\n"
            "topic twice in a row), the session drained fully, the DB is intact,\n"
            "and the last answer was undoable."
        )
        print("=" * 68)
    finally:
        col.close()


if __name__ == "__main__":
    main()
