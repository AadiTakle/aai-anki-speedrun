# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Speedrun desktop entry points (Lane B/C).

Adds a ``Tools → Speedrun`` submenu so the two Speedrun demo flows are
click-to-open instead of requiring the Debug Console:

* **Memory Score** — opens the ``memory-score`` SvelteKit page inside a dialog
  whose webview runs in an API-enabled kind (:data:`AnkiWebViewKind.MEMORY_SCORE`),
  so its ``getMemoryScore`` backend call is authorized.
* **Seed sample data (demo)** — seeds the FULL Step 2 CK blueprint and maps the
  current cards onto only a handful of topics. Coverage is honestly LOW, so the
  memory score does the right thing and ABSTAINS instead of inventing a number.
  This is a straight port of ``proof/wednesday/seed_in_app.py``.
"""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import TYPE_CHECKING

from anki import deck_config_pb2, speedrun_pb2
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV
from aqt.operations import QueryOp
from aqt.qt import *
from aqt.utils import disable_help_button, restoreGeom, saveGeom, tooltip
from aqt.webview import AnkiWebView, AnkiWebViewKind

if TYPE_CHECKING:
    from anki.collection import Collection
    from aqt.main import AnkiQt

_MEMORY_SCORE_GEOM_KEY = "speedrunMemoryScore"

# Hold references to open dialogs so they aren't garbage-collected while shown.
_open_dialogs: list[QDialog] = []


# Memory Score dialog
##########################################################################


def show_memory_score(mw: AnkiQt) -> QDialog:
    """Open the memory-score page in an API-enabled webview dialog."""
    dialog = QDialog(mw, Qt.WindowType.Window)
    dialog.setWindowTitle("Memory Score")
    disable_help_button(dialog)
    mw.garbage_collect_on_dialog_finish(dialog)
    restoreGeom(dialog, _MEMORY_SCORE_GEOM_KEY, default_size=(520, 720))

    layout = QVBoxLayout(dialog)
    layout.setContentsMargins(0, 0, 0, 0)
    web = AnkiWebView(kind=AnkiWebViewKind.MEMORY_SCORE)
    layout.addWidget(web)
    web.load_sveltekit_page("memory-score")

    _open_dialogs.append(dialog)

    def on_finished(_result: int) -> None:
        saveGeom(dialog, _MEMORY_SCORE_GEOM_KEY)
        web.cleanup()
        if dialog in _open_dialogs:
            _open_dialogs.remove(dialog)

    qconnect(dialog.finished, on_finished)
    dialog.show()
    return dialog


# Sample-data seeding (demo)
##########################################################################

# The FULL exam blueprint (id, display name, relative weight). Coverage is
# measured against ALL of it, so mapping cards to only a few topics reads as an
# honestly low coverage.
_FULL_BLUEPRINT: list[tuple[str, str, float]] = [
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

# Only a few topics are meaningfully studied -> map the current cards to these.
# Distinct points-at-stake (weight x weakness) so interleaving is visible.
_COVERED: dict[str, float] = {
    "cardio": 0.90,  # points ~0.099  (leads)
    "renal": 0.50,  # points ~0.030
    "endo": 0.30,  # points ~0.018
}
# A modest baseline weakness for the (unstudied) rest of the blueprint.
_DEFAULT_WEAKNESS = 0.5
# Simulated graded reviews so the review-count gate passes and the only reason
# to abstain is low coverage.
_SIMULATED_REVIEWS = 260


@dataclass
class _SeedSummary:
    cards_mapped: int
    topics_covered: int
    total_topics: int
    coverage_pct: float
    reviews: int


def _seed(col: Collection) -> _SeedSummary | None:
    cids = list(col.find_cards(""))[:60]
    if not cids:
        return None

    covered_ids = list(_COVERED)
    updated = []
    crosswalk: list[tuple[int, str]] = []
    for i, cid in enumerate(cids):
        card = col.get_card(cid)
        card.type = CARD_TYPE_REV
        card.queue = QUEUE_TYPE_REV
        card.ivl = 30
        card.due = 0
        updated.append(card)
        crosswalk.append((cid, covered_ids[i % len(covered_ids)]))
    col.update_cards(updated)

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id=tid, name=name, blueprint_weight=weight)
            for tid, name, weight in _FULL_BLUEPRINT
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cid, topic_id=tid) for cid, tid in crosswalk
        ],
        weaknesses=[
            speedrun_pb2.TopicWeakness(
                topic_id=tid, weakness=_COVERED.get(tid, _DEFAULT_WEAKNESS)
            )
            for tid, _name, _weight in _FULL_BLUEPRINT
        ],
    )

    # Simulate heavy study of the covered topics so the review-count gate passes.
    base = int(time.time() * 1000)
    col.db.executemany(
        "insert into revlog (id,cid,usn,ease,ivl,lastIvl,factor,time,type) "
        "values (?,?,0,3,30,30,2500,0,1)",
        [(base + i, cids[0]) for i in range(_SIMULATED_REVIEWS)],
    )

    # Turn on points-at-stake ordering on the current deck's config.
    did = col.decks.selected()
    conf = col.decks.config_dict_for_deck_id(did)
    conf["reviewOrder"] = (
        deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE
    )
    col.decks.save(conf)

    total_w = sum(weight for _tid, _name, weight in _FULL_BLUEPRINT)
    covered_w = sum(weight for tid, _name, weight in _FULL_BLUEPRINT if tid in _COVERED)
    return _SeedSummary(
        cards_mapped=len(crosswalk),
        topics_covered=len(_COVERED),
        total_topics=len(_FULL_BLUEPRINT),
        coverage_pct=100 * covered_w / total_w if total_w else 0.0,
        reviews=_SIMULATED_REVIEWS,
    )


def seed_sample_data(mw: AnkiQt) -> None:
    """Seed the realistic Step 2 demo, then refresh the UI with a summary."""

    def on_success(summary: _SeedSummary | None) -> None:
        if summary is None:
            tooltip(
                "No cards found — add or import a deck first, then seed again.",
                parent=mw,
            )
            return
        mw.reset()
        tooltip(
            f"Seeded the full {summary.total_topics}-topic Step 2 blueprint; "
            f"mapped {summary.cards_mapped} cards to {summary.topics_covered} "
            f"topics (~{summary.coverage_pct:.0f}% coverage) with "
            f"{summary.reviews} reviews.<br>"
            "Memory Score should ABSTAIN (coverage &lt; 50%).",
            parent=mw,
            period=6000,
        )

    QueryOp(parent=mw, op=_seed, success=on_success).with_progress(
        "Seeding Speedrun sample data…"
    ).run_in_background()


# Menu wiring
##########################################################################


def setup_speedrun_menu(mw: AnkiQt) -> None:
    """Add a ``Speedrun`` submenu to Tools with the demo actions."""
    tools_menu = mw.form.menuTools
    menu = QMenu("Speedrun", tools_menu)
    tools_menu.addSeparator()
    tools_menu.addMenu(menu)

    score_action = menu.addAction("Memory Score")
    assert score_action is not None
    qconnect(score_action.triggered, lambda: show_memory_score(mw))

    seed_action = menu.addAction("Seed sample data (demo)")
    assert seed_action is not None
    qconnect(seed_action.triggered, lambda: seed_sample_data(mw))
