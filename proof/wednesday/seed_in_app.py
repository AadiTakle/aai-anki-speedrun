# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

# Speedrun — REALISTIC seed for an honest in-app demo.
#
# Unlike a naive seed (which defined the "blueprint" as only the few topics you
# had cards in -> a bogus 100% coverage), this seeds the FULL Step 2 CK blueprint
# (~22 weighted topics) and maps your actual cards to only a HANDFUL of them.
# Result: coverage is honestly LOW, so the memory score does the right thing and
# ABSTAINS ("not enough coverage yet") instead of inventing a confident number.
#
# It also simulates heavy study of those few topics (260 graded reviews) *on
# purpose*, to show the key point: even with lots of reviews, the score still
# refuses to grade you until you've covered enough of the exam. That is the
# honesty bar working.
#
# HOW TO USE (dev build — `just run` sets ANKIDEV):
#   1. Open Anki on a profile with some cards (import/add a few if empty).
#   2. Press  Ctrl+:  (Debug Console) and run:
#        exec(open("/Users/atakle/aai-anki-speedrun/proof/wednesday/seed_in_app.py").read())
#   3. Open the memory-score page in an API-enabled webview (Debug Console):
#        from aqt.qt import QDialog, QVBoxLayout
#        from aqt.webview import AnkiWebView, AnkiWebViewKind
#        d = QDialog(mw); d.resize(520, 640); lay = QVBoxLayout(d)
#        w = AnkiWebView(kind=AnkiWebViewKind.DECK_STATS); lay.addWidget(w)
#        w.load_sveltekit_page("memory-score"); d.show(); mw._speedrun_dlg = d
#      -> expect an honest ABSTAIN with "coverage ~NN% (< 50% required)".
#   4. Study the deck to see points-at-stake INTERLEAVING across the covered
#      topics (Cardiology leads/recurs; others interleave; unmapped cards last).

import time

from anki import speedrun_pb2 as sp

col = mw.col  # noqa: F821  (mw is provided by the Debug Console)

# The FULL exam blueprint (id, display name, relative weight). This is the whole
# Step 2 CK surface — coverage is measured against ALL of it.
FULL_BLUEPRINT = [
    ("cardio", "Cardiovascular", 0.11),
    ("pulm", "Pulmonary", 0.09),
    ("gi", "Gastrointestinal", 0.09),
    ("obgyn", "Reproductive / OB-GYN", 0.09),
    ("peds", "Pediatrics", 0.08),
    ("psych", "Psychiatry", 0.07),
    ("renal", "Renal / GU", 0.06),
    ("endo", "Endocrine", 0.06),
    ("heme_onc", "Heme / Onc", 0.06),
    ("id", "Infectious Disease", 0.06),
    ("neuro", "Neurology", 0.06),
    ("msk", "Musculoskeletal / Rheum", 0.05),
    ("surg", "Surgery / Perioperative", 0.05),
    ("emerg", "Emergency / Critical Care", 0.04),
    ("derm", "Dermatology", 0.03),
    ("ophtho", "Ophthalmology", 0.02),
    ("ent", "ENT", 0.02),
    ("biostat", "Biostatistics / Epi", 0.02),
    ("ethics", "Ethics / Professionalism", 0.02),
    ("genetics", "Genetics", 0.02),
    ("immuno", "Immunology", 0.02),
    ("nutrition", "Nutrition", 0.01),
]

# You've only meaningfully studied a few topics -> map your cards to just these.
# Distinct points-at-stake (weight x weakness) so interleaving is visible.
COVERED = {
    "cardio": 0.90,  # points ~0.099  (leads)
    "renal": 0.50,  # points ~0.030
    "endo": 0.30,  # points ~0.018
}
# A modest baseline weakness for the (unstudied) rest of the blueprint.
DEFAULT_WEAKNESS = 0.5

cids = list(col.find_cards(""))[:60]
if not cids:
    print("No cards found — add or import a deck first, then re-run.")
else:
    covered_ids = list(COVERED)
    crosswalk, updated = [], []
    for i, cid in enumerate(cids):
        c = col.get_card(cid)
        c.type = 2  # CARD_TYPE_REV
        c.queue = 2  # QUEUE_TYPE_REV
        c.ivl = 30
        c.due = 0
        updated.append(c)
        crosswalk.append((cid, covered_ids[i % len(covered_ids)]))
    col.update_cards(updated)

    weaknesses = [
        sp.TopicWeakness(topic_id=tid, weakness=COVERED.get(tid, DEFAULT_WEAKNESS))
        for tid, _name, _w in FULL_BLUEPRINT
    ]
    col._backend.set_topic_weights(
        topics=[
            sp.Topic(id=tid, name=name, blueprint_weight=w)
            for tid, name, w in FULL_BLUEPRINT
        ],
        card_topics=[
            sp.CardTopic(card_id=cid, topic_id=tid) for cid, tid in crosswalk
        ],
        weaknesses=weaknesses,
    )

    # Simulate heavy study of the covered topics (>= 200 graded reviews) so the
    # review-count gate passes and the ONLY reason to abstain is low coverage.
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

    total_w = sum(w for _, _, w in FULL_BLUEPRINT)
    covered_w = sum(w for tid, _, w in FULL_BLUEPRINT if tid in COVERED)
    print(
        f"Seeded the full {len(FULL_BLUEPRINT)}-topic blueprint; mapped your "
        f"{len(crosswalk)} cards to {len(COVERED)} topics.\n"
        f"Coverage = {100 * covered_w / total_w:.1f}% of the blueprint (< 50%), "
        "so the memory score should ABSTAIN even with 260 reviews.\n"
        "-> Open the memory-score page: expect 'not enough data' + a low coverage %.\n"
        "-> Study the deck: covered topics interleave by points-at-stake."
    )
