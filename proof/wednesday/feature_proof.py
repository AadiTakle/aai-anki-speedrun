#!/usr/bin/env python3
# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Feature interactivity proof — shows each Speedrun engine feature COMPUTING
from real inputs (output changes when inputs change), not emitting static text.

Run from the repo's `anki/` dir:
    PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 ./out/pyenv/bin/python \
        ../proof/wednesday/feature_proof.py

Every check prints INPUT -> ACTUAL vs EXPECTED and PASS/FAIL, and exits non-zero
if any check fails.
"""

from __future__ import annotations

import os
import tempfile
import time

from anki.collection import Collection  # import first to avoid a circular import
from anki import cards_pb2, deck_config_pb2, speedrun_pb2
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV

FSRSMemoryState = cards_pb2.FsrsMemoryState

DAY = deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_DAY
POINTS_AT_STAKE = deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE

_failures = 0


def check(label: str, actual, expected) -> None:
    global _failures
    ok = actual == expected
    _failures += 0 if ok else 1
    print(f"    {'PASS' if ok else 'FAIL'}  {label}: actual={actual!r} expected={expected!r}")


def check_true(label: str, cond: bool, detail: str = "") -> None:
    global _failures
    _failures += 0 if cond else 1
    print(f"    {'PASS' if cond else 'FAIL'}  {label} {detail}")


def fresh() -> Collection:
    fd, path = tempfile.mkstemp(suffix=".anki2")
    os.close(fd)
    os.unlink(path)
    return Collection(path)


def add_card(col, front, ivl=30, memory=None, queue=QUEUE_TYPE_REV):
    note = col.newNote()
    note["Front"] = front
    col.addNote(note)
    c = note.cards()[0]
    c.ivl = ivl
    c.due = 0
    c.type = CARD_TYPE_REV
    c.queue = queue
    if memory is not None:
        c.memory_state = memory
        c.last_review_time = int(time.time())
    col.update_cards([c], skip_undo_entry=True)
    return c.id


def seed(col, topics, card_topics, weaknesses):
    col._backend.set_topic_weights(
        topics=[speedrun_pb2.Topic(id=t, name=t, blueprint_weight=w) for t, w in topics],
        card_topics=[speedrun_pb2.CardTopic(card_id=c, topic_id=t) for c, t in card_topics],
        weaknesses=[speedrun_pb2.TopicWeakness(topic_id=t, weakness=w) for t, w in weaknesses],
    )


def seed_reviews(col, cid, n):
    base = int(time.time() * 1000)
    col.db.executemany(
        "insert into revlog (id,cid,usn,ease,ivl,lastIvl,factor,time,type) "
        "values (?,?,0,3,30,30,2500,0,1)",
        [(base + i, cid) for i in range(n)],
    )


def order_under(col, review_order):
    conf = col.decks.config_dict_for_deck_id(1)
    conf["reviewOrder"] = review_order
    col.decks.save(conf)
    col.sched.reset()
    return [qc.card.id for qc in col.sched.get_queued_cards(fetch_limit=50).cards]


# ── F1: config-backed topic store is real + undo-safe ──────────────────────
def prove_f1():
    print("\n== F1  topic store (persist + undo) ==")
    col = fresh()
    c = add_card(col, "x")
    seed(col, [("cardio", 1.0)], [(c, "cardio")], [("cardio", 0.5)])
    m = list(col._backend.get_topic_mastery(topic_ids=[]))
    check("after set_topic_weights, taxonomy has 1 topic", len(m), 1)
    check("stored topic id round-trips", m[0].topic_id if m else None, "cardio")
    col.undo()  # undo the set_topic_weights op
    m2 = list(col._backend.get_topic_mastery(topic_ids=[]))
    check("after undo(), taxonomy reverts to empty", len(m2), 0)
    col.close()


# ── F4: per-topic mastery is COMPUTED from card state ──────────────────────
def prove_f4():
    print("\n== F4  mastery query (computes mastered/total/recall from cards) ==")
    col = fresh()
    mem = FSRSMemoryState(stability=200.0, difficulty=5.0)
    mature_a = add_card(col, "a", ivl=30, memory=mem)  # mature + recall-backed
    mature_b = add_card(col, "b", ivl=25, memory=mem)  # mature + recall-backed
    young = add_card(col, "c", ivl=5)  # not mature
    suspended = add_card(col, "d", ivl=30, queue=-1)  # mature but SUSPENDED
    seed(
        col,
        [("cardio", 1.0)],
        [(mature_a, "cardio"), (mature_b, "cardio"), (young, "cardio"), (suspended, "cardio")],
        [("cardio", 0.5)],
    )
    t = col._backend.get_topic_mastery(topic_ids=[])[0]
    print(f"    input: 4 cards mapped to cardio (2 mature+memory, 1 young, 1 suspended-mature)")
    check("total (suspended excluded)", t.total, 3)
    check("mastered (mature, non-suspended)", t.mastered, 2)
    check("recall_card_count (memory-backed)", t.recall_card_count, 2)
    check_true("avg_recall in (0,1]", 0.0 < t.avg_recall <= 1.0, f"(avg_recall={t.avg_recall:.3f})")
    col.close()


# ── F5: points-at-stake ORDER changes when weights change ──────────────────
def prove_f5():
    print("\n== F5  points-at-stake queue (order is computed from weight x weakness) ==")
    col = fresh()
    x = add_card(col, "x", ivl=10)
    y = add_card(col, "y", ivl=10)
    z = add_card(col, "z", ivl=10)
    topo = {x: "X", y: "Y", z: "Z"}

    day = order_under(col, DAY)  # feature OFF (gather/hash order)

    # weights make X the clear top; Z the bottom
    seed(col, [("X", 0.9), ("Y", 0.5), ("Z", 0.1)], [(x, "X"), (y, "Y"), (z, "Z")],
         [("X", 1.0), ("Y", 1.0), ("Z", 1.0)])
    pas1 = order_under(col, POINTS_AT_STAKE)
    print(f"    weights X=0.9 Y=0.5 Z=0.1 -> order topics: {[topo[c] for c in pas1]}")
    check("highest-weight topic (X) is first", topo[pas1[0]], "X")
    check("lowest-weight topic (Z) is last", topo[pas1[-1]], "Z")

    # FLIP the weights: now Z is top, X is bottom
    seed(col, [("X", 0.1), ("Y", 0.5), ("Z", 0.9)], [(x, "X"), (y, "Y"), (z, "Z")],
         [("X", 1.0), ("Y", 1.0), ("Z", 1.0)])
    pas2 = order_under(col, POINTS_AT_STAKE)
    print(f"    weights X=0.1 Y=0.5 Z=0.9 -> order topics: {[topo[c] for c in pas2]}")
    check("after flip, Z is first", topo[pas2[0]], "Z")
    check("after flip, X is last", topo[pas2[-1]], "X")
    check_true("order actually CHANGED with the inputs", pas1 != pas2,
               "(not static: same fn, different weights -> different order)")
    print(f"    (DAY/feature-off order for contrast: {[topo[c] for c in day]})")
    col.close()


# ── F6: memory score REACTS to data (abstain -> score) ─────────────────────
def show_score(label, s):
    if s.abstained:
        print(f"    [{label}] abstained=TRUE   coverage={s.coverage_pct:5.1f}%  reasons={list(s.reasons)}")
    else:
        print(f"    [{label}] abstained=FALSE  score={s.point:.0f} range=[{s.low:.0f},{s.high:.0f}] "
              f"coverage={s.coverage_pct:5.1f}%  reasons={list(s.reasons)}")


def prove_f6():
    print("\n== F6  memory score (honest abstain -> range; reacts to data) ==")
    col = fresh()

    # State A: brand-new collection, nothing studied.
    sA = col._backend.get_memory_score()
    show_score("A fresh", sA)
    check("A: abstains on no data", sA.abstained, True)
    check("A: coverage is 0", round(sA.coverage_pct, 1), 0.0)

    # 5-topic blueprint (equal weight). Make 5 review cards, memory-backed.
    mem = FSRSMemoryState(stability=200.0, difficulty=5.0)
    cards = [add_card(col, f"c{i}", ivl=30, memory=mem) for i in range(5)]
    blueprint = [(f"t{i}", 0.2) for i in range(5)]  # total weight 1.0
    weak = [(f"t{i}", 0.5) for i in range(5)]
    seed_reviews(col, cards[0], 260)  # clear the >=200 graded-reviews gate

    # State B: map ALL cards to ONE topic -> 20% coverage (< 50%).
    seed(col, blueprint, [(c, "t0") for c in cards], weak)
    sB = col._backend.get_memory_score()
    show_score("B 1/5 topics", sB)
    check("B: abstains on low coverage", sB.abstained, True)
    check("B: coverage ~20%", round(sB.coverage_pct), 20)
    check_true("B: reason cites coverage (not reviews)",
               any("coverage" in r for r in sB.reasons), f"reasons={list(sB.reasons)}")

    # State C: spread cards across 3 topics -> 60% coverage (>= 50%).
    spread = [(cards[i], f"t{i % 3}") for i in range(5)]  # covers t0,t1,t2
    seed(col, blueprint, spread, weak)
    sC = col._backend.get_memory_score()
    show_score("C 3/5 topics", sC)
    check("C: now SCORES (not abstain)", sC.abstained, False)
    check("C: coverage 60%", round(sC.coverage_pct), 60)
    check_true("C: non-degenerate range low<high", sC.low < sC.high, f"[{sC.low:.1f},{sC.high:.1f}]")
    check_true("C: point within range", sC.low <= sC.point <= sC.high, f"point={sC.point:.1f}")
    check_true("score CHANGED with coverage (B abstain -> C scored)",
               sB.abstained and not sC.abstained, "(proves it's computed, not static)")
    col.close()


if __name__ == "__main__":
    print("=" * 70)
    print("Speedrun feature interactivity proof — real backend, real computation")
    print("=" * 70)
    prove_f1()
    prove_f4()
    prove_f5()
    prove_f6()
    print("\n" + "=" * 70)
    if _failures:
        print(f"RESULT: {_failures} CHECK(S) FAILED")
        raise SystemExit(1)
    print("RESULT: ALL CHECKS PASSED — every feature computed from its inputs.")
    print("=" * 70)
