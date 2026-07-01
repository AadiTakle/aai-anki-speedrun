# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

# Speedrun — seed the LIVE collection so the Speedrun features light up in-app.
#
# The MVP has no GUI yet for seeding the topic taxonomy / weights / weakness that
# points-at-stake (F5) and the memory score (F6) consume, so this script seeds a
# sample mapping onto your current collection for a hands-on in-app test.
#
# HOW TO USE (dev build — `just run` sets ANKIDEV):
#   1. Open Anki on a profile that has some cards (import/add a few if empty).
#   2. Press  Ctrl+:  to open the Debug Console.
#   3. Run:
#        exec(open("/Users/atakle/aai-anki-speedrun/proof/wednesday/seed_in_app.py").read())
#   4. Study the current deck  ->  cards come out topic-INTERLEAVED, highest
#      points-at-stake (blueprint_weight x weakness) leading and recurring.
#   5. Memory score page:  http://localhost:40000/_anki/pages/memory-score
#
# The Debug Console injects `mw`. Re-runnable (fully replaces prior seed data).

import time

from anki import speedrun_pb2 as sp
from anki.cards import FSRSMemoryState

col = mw.col  # noqa: F821  (mw is provided by the Debug Console)

cids = list(col.find_cards(""))[:60]
if not cids:
    print("No cards found — add or import a deck first, then re-run.")
else:
    # (topic id, display name, blueprint weight, weakness). points_at_stake =
    # weight * weakness, so Cardiology (0.27) >> Neurology (0.075): cardio leads.
    topics = [
        ("cardio", "Cardiology", 0.30, 0.90),
        ("renal", "Nephrology", 0.25, 0.60),
        ("neuro", "Neurology", 0.25, 0.30),
        ("gi", "Gastroenterology", 0.20, 0.50),
    ]
    # A recent, high-stability memory state so recall is *backed* (not the
    # unbacked-0 sentinel) and the score computes instead of abstaining.
    mem = FSRSMemoryState(stability=120.0, difficulty=5.0)

    crosswalk, updated = [], []
    for i, cid in enumerate(cids):
        c = col.get_card(cid)
        c.type = 2  # CARD_TYPE_REV
        c.queue = 2  # QUEUE_TYPE_REV
        c.ivl = 30
        c.due = 0
        c.memory_state = mem
        try:
            c.last_review_time = int(time.time())  # keep retrievability high
        except Exception:
            pass
        updated.append(c)
        crosswalk.append((cid, topics[i % len(topics)][0]))
    col.update_cards(updated)

    col._backend.set_topic_weights(
        topics=[sp.Topic(id=t[0], name=t[1], blueprint_weight=t[2]) for t in topics],
        card_topics=[
            sp.CardTopic(card_id=cid, topic_id=tid) for cid, tid in crosswalk
        ],
        weaknesses=[sp.TopicWeakness(topic_id=t[0], weakness=t[3]) for t in topics],
    )

    # >= 200 graded reviews so the memory score clears its give-up gate.
    base = int(time.time() * 1000)
    col.db.executemany(
        "insert into revlog (id,cid,usn,ease,ivl,lastIvl,factor,time,type) "
        "values (?,?,0,3,30,30,2500,0,1)",
        [(base + i, cids[0]) for i in range(260)],
    )

    did = col.decks.selected()
    conf = col.decks.config_dict_for_deck_id(did)
    conf["reviewOrder"] = 13  # REVIEW_CARD_ORDER_POINTS_AT_STAKE
    col.decks.save(conf)
    col.reset()
    print(
        f"Seeded {len(crosswalk)} cards across {len(topics)} topics + 260 reviews; "
        "points-at-stake enabled on the current deck.\n"
        "-> Study the deck to see interleaving; open "
        "http://localhost:40000/_anki/pages/memory-score for the score."
    )
