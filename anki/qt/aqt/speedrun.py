# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Speedrun desktop entry points (Lane B/C).

Adds a ``Tools → Speedrun`` submenu and a first-run onboarding wizard:

* **Set up STAT** (:func:`run_setup_wizard`) — the frictionless transition:
  import an Anki deck, one-click auto-crosswalk its AnKing tags onto the
  22-topic blueprint (:func:`auto_crosswalk_from_tags`), paste QBank
  performance, and record a practice-test score, then open Today. Offered
  automatically on first run (:func:`maybe_offer_onboarding`).
* **STAT console pages** — the five daily-loop destinations, opened in
  API-enabled webviews so their backend RPC calls are authorized.
* **Memory Score** — opens the ``memory-score`` SvelteKit page.
* **Seed sample data (demo)** — seeds the blueprint with illustrative data so
  the console can be demoed without real imports.
"""

from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, cast

from anki import deck_config_pb2, scheduler_pb2, speedrun_pb2
from anki.cards import Card
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_REV
from anki.scheduler.v3 import Scheduler as V3Scheduler
from aqt.operations import QueryOp
from aqt.qt import *
from aqt.utils import askUser, disable_help_button, restoreGeom, saveGeom, tooltip
from aqt.webview import AnkiWebView, AnkiWebViewKind

if TYPE_CHECKING:
    from anki.collection import Collection
    from aqt.main import AnkiQt

_MEMORY_SCORE_GEOM_KEY = "speedrunMemoryScore"

# Hold references to open dialogs so they aren't garbage-collected while shown.
_open_dialogs: list[QDialog] = []


# SvelteKit page dialogs (API-enabled webviews)
##########################################################################


def _open_page_dialog(
    mw: AnkiQt,
    *,
    page: str,
    title: str,
    kind: AnkiWebViewKind,
    geom_key: str,
    default_size: tuple[int, int],
) -> QDialog:
    """Open a SvelteKit page in a dialog whose webview is API-enabled, so its
    backend RPC calls are authorized (the AuthInterceptor injects the key)."""
    dialog = QDialog(mw, Qt.WindowType.Window)
    dialog.setWindowTitle(title)
    disable_help_button(dialog)
    mw.garbage_collect_on_dialog_finish(dialog)
    restoreGeom(dialog, geom_key, default_size=default_size)

    layout = QVBoxLayout(dialog)
    layout.setContentsMargins(0, 0, 0, 0)
    web = AnkiWebView(kind=kind)
    # Make the webview its own pycmd bridge context, so the js-message hook can
    # eval back to this exact view (the custom reviewer feeds cards this way).
    web.set_bridge_command(lambda _cmd: None, web)
    layout.addWidget(web)
    web.load_sveltekit_page(page)

    _open_dialogs.append(dialog)

    def on_finished(_result: int) -> None:
        saveGeom(dialog, geom_key)
        web.cleanup()
        if dialog in _open_dialogs:
            _open_dialogs.remove(dialog)

    qconnect(dialog.finished, on_finished)
    dialog.show()
    return dialog


def show_memory_score(mw: AnkiQt) -> QDialog:
    """Open the memory-score page in an API-enabled webview dialog."""
    return _open_page_dialog(
        mw,
        page="memory-score",
        title="Memory Score",
        kind=AnkiWebViewKind.MEMORY_SCORE,
        geom_key=_MEMORY_SCORE_GEOM_KEY,
        default_size=(520, 720),
    )


# The five STAT console destinations (menu label, sveltekit page/route).
_STAT_PAGES: list[tuple[str, str]] = [
    ("Today", "today"),
    ("Reviewer", "reviewer"),
    ("Import \u2192 Auto-link", "import"),
]


def show_stat_page(mw: AnkiQt, page: str, title: str) -> QDialog:
    """Open one STAT console page in an API-enabled dialog. In-app nav then moves
    between the five destinations within the same webview."""
    return _open_page_dialog(
        mw,
        page=page,
        title=f"STAT \u00b7 {title}",
        kind=AnkiWebViewKind.SPEEDRUN,
        geom_key=f"speedrun_{page}",
        default_size=(1180, 820),
    )


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
            "Memory Score stays blank until you review cards so FSRS can record recall.",
            parent=mw,
            period=6000,
        )

    QueryOp(parent=mw, op=_seed, success=on_success).with_progress(
        "Seeding Speedrun sample data…"
    ).run_in_background()


# Every col.conf store the Speedrun engine owns (see rslib speedrun/*). Clearing
# these returns STAT to a first-run state without touching Anki decks/cards.
_SPEEDRUN_CONF_KEYS = [
    "speedrun:topics",  # F1 taxonomy / blueprint weights
    "speedrun:card_topics",  # card -> topic crosswalk
    "speedrun:weakness",  # per-topic weakness
    "speedrun:qbank_aggregates",  # imported UWorld / QBank results (F2)
    "speedrun:attempts",  # per-question attempts
    "speedrun:practice_tests",  # imported practice-test scores
    "speedrun:error_log",  # F3 miss log
    "speedrun:onboarded",  # "don't offer Set up STAT again" flag
]


def reset_speedrun_data(mw: AnkiQt) -> None:
    """Clear all Speedrun/STAT state — taxonomy, card→topic mapping, weakness,
    imported UWorld/QBank + practice-test results, attempts, and the error log —
    returning the app to its first-run state (Set up STAT will offer again).

    Anki decks and cards are NOT touched; to also start with no decks, run a
    fresh profile (see the RUNBOOK / Cloud notes) instead."""
    col = mw.col
    if col is None:
        return
    if not askUser(
        "Reset all Speedrun data?\n\nThis clears your STAT setup — topic mapping, "
        "weakness, and imported UWorld / QBank + practice-test results — returning "
        "the app to its first-run state.\n\nYour Anki decks and cards are NOT "
        "affected.",
        parent=mw,
        title="Reset Speedrun data",
    ):
        return

    def op(col: Collection) -> int:
        cleared = 0
        for key in _SPEEDRUN_CONF_KEYS:
            if col.get_config(key, None) is not None:
                col.remove_config(key)
                cleared += 1
        return cleared

    def on_success(cleared: int) -> None:
        mw.reset()
        tooltip(
            f"Speedrun data reset — cleared {cleared} store(s). "
            "Open Tools → Speedrun → Set up STAT to start fresh.",
            parent=mw,
            period=5000,
        )

    QueryOp(parent=mw, op=op, success=on_success).with_progress(
        "Resetting Speedrun data…"
    ).run_in_background()


# Onboarding: transition QBank + practice test + Anki -> STAT
##########################################################################

# AnKing #Subjects / !Shelf label (normalized) -> blueprint topic id. The
# AnKing Step 2 deck is tag-organized under #AK_Step2_v*::#Subjects::<Subject>;
# this is the curated crosswalk that makes the Anki->STAT step one click.
_ANKING_SUBJECT_MAP: dict[str, str] = {
    "cardiology": "cardio",
    "cardiovascular": "cardio",
    "pulmonology": "pulm",
    "pulmonary": "pulm",
    "respiratory": "pulm",
    "gastroenterology": "gi",
    "gastrointestinal": "gi",
    "nephrology": "renal",
    "renal": "renal",
    "urology": "renal",
    "genitourinary": "renal",
    "endocrinology": "endo",
    "endocrine": "endo",
    "neurology": "neuro",
    "hematology": "heme_onc",
    "oncology": "heme_onc",
    "hematology oncology": "heme_onc",
    "heme onc": "heme_onc",
    "infectious disease": "id",
    "infectious diseases": "id",
    "psychiatry": "psych",
    "behavioral science": "psych",
    "pediatrics": "peds",
    "obstetrics": "obgyn",
    "gynecology": "obgyn",
    "obstetrics gynecology": "obgyn",
    "ob gyn": "obgyn",
    "obgyn": "obgyn",
    "reproductive": "obgyn",
    "womens health": "obgyn",
    "surgery": "surg",
    "general surgery": "surg",
    "anesthesiology": "surg",
    "perioperative": "surg",
    "rheumatology": "msk",
    "orthopedics": "msk",
    "musculoskeletal": "msk",
    "orthopedic surgery": "msk",
    "dermatology": "derm",
    "ophthalmology": "ophtho",
    "otolaryngology": "ent",
    "ent": "ent",
    "ear nose throat": "ent",
    "biostatistics": "biostat",
    "epidemiology": "biostat",
    "biostatistics epidemiology": "biostat",
    "biostats": "biostat",
    "ethics": "ethics",
    "medical ethics": "ethics",
    "professionalism": "ethics",
    "social sciences": "ethics",
    "genetics": "genetics",
    "medical genetics": "genetics",
    "immunology": "immuno",
    "allergy immunology": "immuno",
    "allergy": "immuno",
    "nutrition": "nutrition",
    "emergency medicine": "emerg",
    "emergency": "emerg",
    "critical care": "emerg",
}

# !Shelf rotations that map cleanly to one topic. Broad rotations (Internal /
# Family Medicine) intentionally omitted — they span the whole blueprint.
_SHELF_MAP: dict[str, str] = {
    "pediatrics": "peds",
    "obstetrics gynecology": "obgyn",
    "ob gyn": "obgyn",
    "surgery": "surg",
    "psychiatry": "psych",
    "neurology": "neuro",
    "emergency medicine": "emerg",
}

_SUBJECT_RE = re.compile(r"#Subjects::([^:]+)", re.IGNORECASE)
_SHELF_RE = re.compile(r"!Shelf::([^:]+)", re.IGNORECASE)

# col.conf keys owned by the Rust speedrun store (see rslib speedrun/store.rs).
_TOPICS_KEY = "speedrun:topics"
_CARD_TOPICS_KEY = "speedrun:card_topics"

# Blueprint topic id -> display name (reviewer picker options + tooltips).
_TOPIC_NAMES: dict[str, str] = {tid: name for tid, name, _w in _FULL_BLUEPRINT}

# Card ids the user chose to skip categorizing this session (don't re-prompt).
_session_skipped: set[str] = set()
# Cached set of already-categorized card ids, so the reviewer prompt is O(1) per
# answered card. Cleared on collection load / crosswalk change.
_categorized_cache: set[str] | None = None


def _clear_categorized_cache() -> None:
    global _categorized_cache
    _categorized_cache = None


def _categorized_card_ids(col: Collection) -> set[str]:
    global _categorized_cache
    if _categorized_cache is None:
        stored = col.get_config(_CARD_TOPICS_KEY, None)
        _categorized_cache = set(stored.keys()) if isinstance(stored, dict) else set()
    return _categorized_cache


def assign_card_topic(col: Collection, card_id: int, topic_id: str) -> None:
    """File one card under a blueprint topic. Updates the Rust store's crosswalk
    map (`speedrun:card_topics`: stringified card id -> topic id) in place — a
    light, undo-neutral config write, so it never rewrites the whole taxonomy or
    clutters the reviewer's undo history."""
    stored = col.get_config(_CARD_TOPICS_KEY, None)
    crosswalk = dict(stored) if isinstance(stored, dict) else {}
    crosswalk[str(card_id)] = topic_id
    col.set_config(_CARD_TOPICS_KEY, crosswalk)
    if _categorized_cache is not None:
        _categorized_cache.add(str(card_id))


def _normalize_label(label: str) -> str:
    """Lowercase + collapse punctuation to spaces so 'Infectious_Disease' and
    'Infectious Disease' both key the map."""
    return re.sub(r"[^a-z0-9]+", " ", label.lower()).strip()


def _extract_topic_from_tags(tags: list[str]) -> tuple[str | None, str | None]:
    """Map a card's tags to one blueprint topic (primary = first mapped
    #Subjects, then !Shelf). Returns (topic_id, raw_label): topic_id is None when
    a subject/shelf label was found but not mapped (raw_label carries it for the
    review screen); both None means no AnKing subject/shelf tag at all."""
    subjects = [m.group(1) for tag in tags if (m := _SUBJECT_RE.search(tag))]
    shelves = [m.group(1) for tag in tags if (m := _SHELF_RE.search(tag))]
    for label in subjects:
        topic = _ANKING_SUBJECT_MAP.get(_normalize_label(label))
        if topic:
            return topic, label
    for label in shelves:
        topic = _SHELF_MAP.get(_normalize_label(label))
        if topic:
            return topic, label
    if subjects:
        return None, subjects[0]
    if shelves:
        return None, shelves[0]
    return None, None


@dataclass
class _CrosswalkResult:
    card_topics: dict[int, str]
    per_topic: dict[str, int]
    # Unmatched subject/shelf label -> the cards carrying it (for the review UI).
    unmatched: dict[str, list[int]]
    total_scanned: int

    @property
    def total_mapped(self) -> int:
        return len(self.card_topics)


def auto_crosswalk_from_tags(col: Collection) -> _CrosswalkResult:
    """Scan AnKing #Subjects / !Shelf tags and map each card to one blueprint
    topic. One batched SQL read (no N+1), so it scales to a 35k-card deck."""
    rows = col.db.all(
        "select c.id, n.tags from cards c join notes n on c.nid = n.id "
        "where n.tags like '%#Subjects::%' or n.tags like '%!Shelf::%'"
    )
    card_topics: dict[int, str] = {}
    per_topic: dict[str, int] = {}
    unmatched: dict[str, list[int]] = {}
    for cid, tags_str in rows:
        topic, label = _extract_topic_from_tags((tags_str or "").split())
        if topic is not None:
            card_topics[int(cid)] = topic
            per_topic[topic] = per_topic.get(topic, 0) + 1
        elif label is not None:
            unmatched.setdefault(label, []).append(int(cid))
    return _CrosswalkResult(card_topics, per_topic, unmatched, len(rows))


def apply_crosswalk(
    col: Collection,
    card_topics: dict[int, str],
    *,
    enable_points_at_stake: bool = True,
) -> None:
    """Persist the full blueprint + card->topic crosswalk (undo-safe via the
    backend op), preserving any QBank-derived weakness, then switch the current
    deck to the points-at-stake review order."""
    existing = col.get_config("speedrun:weakness", None)
    weakness_by_topic = existing if isinstance(existing, dict) else {}
    topic_ids = {tid for tid, _n, _w in _FULL_BLUEPRINT} | set(card_topics.values())

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id=tid, name=name, blueprint_weight=weight)
            for tid, name, weight in _FULL_BLUEPRINT
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cid, topic_id=tid)
            for cid, tid in card_topics.items()
        ],
        weaknesses=[
            speedrun_pb2.TopicWeakness(
                topic_id=tid,
                weakness=float(weakness_by_topic.get(tid, _DEFAULT_WEAKNESS)),
            )
            for tid in topic_ids
        ],
    )
    if enable_points_at_stake:
        did = col.decks.selected()
        conf = col.decks.config_dict_for_deck_id(did)
        conf["reviewOrder"] = (
            deck_config_pb2.DeckConfig.Config.REVIEW_CARD_ORDER_POINTS_AT_STAKE
        )
        col.decks.save(conf)
    # The crosswalk changed, so the reviewer's "already categorized" set is stale.
    _clear_categorized_cache()


def store_practice_test(
    col: Collection, source: str, form: str, scaled_score: float
) -> None:
    """Record a practice-test scaled score (NBME/UWSA/Free120). Readiness stays
    honestly abstained until calibration exists, but the score is stored now."""
    col._backend.import_qbank_data(
        attempts=[],
        tests=[
            speedrun_pb2.PracticeTestResult(
                source=source,
                form=form,
                taken_at=int(time.time()),
                scaled_score=float(scaled_score),
                percent_correct=0.0,
            )
        ],
    )


def should_offer_onboarding(col: Collection) -> bool:
    """First-run gate: no taxonomy set yet and the user hasn't dismissed setup."""
    return not col.get_config(_TOPICS_KEY, None) and not col.get_config(
        "speedrun:onboarded", None
    )


class _ReviewMappingDialog(QDialog):
    """Assign blueprint topics to AnKing labels the auto-mapper couldn't place."""

    def __init__(self, parent: QWidget, unmatched: dict[str, list[int]]) -> None:
        super().__init__(parent)
        self.setWindowTitle("Review unmapped tags")
        self._combos: dict[str, QComboBox] = {}
        layout = QVBoxLayout(self)
        note = QLabel(
            "These deck tags didn't match a Step 2 topic. Assign the big ones, "
            "or leave them as Skip."
        )
        note.setWordWrap(True)
        layout.addWidget(note)
        layout.addWidget(self._build_scroll(unmatched))
        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        qconnect(buttons.accepted, self.accept)
        qconnect(buttons.rejected, self.reject)
        layout.addWidget(buttons)

    def _build_scroll(self, unmatched: dict[str, list[int]]) -> QScrollArea:
        grid = QGridLayout()
        grid.addWidget(QLabel("<b>Tag</b>"), 0, 0)
        grid.addWidget(QLabel("<b>Cards</b>"), 0, 1)
        grid.addWidget(QLabel("<b>Topic</b>"), 0, 2)
        ordered = sorted(unmatched.items(), key=lambda kv: len(kv[1]), reverse=True)
        for row, (label, cids) in enumerate(ordered, start=1):
            grid.addWidget(QLabel(label), row, 0)
            grid.addWidget(QLabel(str(len(cids))), row, 1)
            combo = QComboBox()
            combo.addItem("— Skip —", "")
            for tid, name, _w in _FULL_BLUEPRINT:
                combo.addItem(name, tid)
            grid.addWidget(combo, row, 2)
            self._combos[label] = combo
        holder = QWidget()
        holder.setLayout(grid)
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setWidget(holder)
        return scroll

    def assignments(self) -> dict[str, str]:
        out: dict[str, str] = {}
        for label, combo in self._combos.items():
            tid = combo.currentData()
            if tid:
                out[label] = str(tid)
        return out


class SetupStatDialog(QDialog):
    """The 'Set up STAT' onboarding: connect the Anki deck, QBank performance,
    and a practice-test score so the console runs on real data in a few clicks."""

    def __init__(self, mw: AnkiQt) -> None:
        super().__init__(mw, Qt.WindowType.Window)
        self.mw = mw
        self.setWindowTitle("Set up STAT")
        disable_help_button(self)
        self._build_ui()
        self._refresh_deck_label()

    def _build_ui(self) -> None:
        layout = QVBoxLayout(self)
        intro = QLabel(
            "<h2>Set up STAT</h2>Connect your tools once — your cards, your QBank "
            "performance, and a practice-test score — and the console runs on your "
            "real data. Skip any step you like."
        )
        intro.setWordWrap(True)
        layout.addWidget(intro)
        layout.addWidget(self._deck_group())
        layout.addWidget(self._map_group())
        layout.addWidget(self._qbank_group())
        layout.addWidget(self._practice_group())
        layout.addWidget(self._footer())

    def _deck_group(self) -> QGroupBox:
        box = QGroupBox("1. Your Anki deck")
        v = QVBoxLayout(box)
        self._deck_label = QLabel()
        self._deck_label.setWordWrap(True)
        v.addWidget(self._deck_label)
        btn = QPushButton("Import a deck (.apkg / .colpkg)…")
        qconnect(btn.clicked, self._import_deck)
        v.addWidget(btn)
        return box

    def _map_group(self) -> QGroupBox:
        box = QGroupBox("2. Map your deck to the Step 2 blueprint")
        v = QVBoxLayout(box)
        self._map_label = QLabel(
            "One click auto-maps AnKing tags onto the 22-topic blueprint."
        )
        self._map_label.setWordWrap(True)
        v.addWidget(self._map_label)
        btn = QPushButton("Scan & auto-map my deck")
        qconnect(btn.clicked, self._scan_and_map)
        v.addWidget(btn)
        return box

    def _qbank_group(self) -> QGroupBox:
        box = QGroupBox("3. QBank performance")
        v = QVBoxLayout(box)
        v.addWidget(QLabel("Paste your UWorld/AMBOSS 'Performance by System' numbers."))
        btn = QPushButton("Open QBank import…")
        qconnect(btn.clicked, lambda: show_stat_page(self.mw, "import", "Import"))
        v.addWidget(btn)
        return box

    def _practice_group(self) -> QGroupBox:
        box = QGroupBox("4. Latest practice test (optional)")
        form = QFormLayout(box)
        self._pt_source = QComboBox()
        self._pt_source.addItems(["UWSA2", "UWSA1", "NBME", "Free 120", "Other"])
        self._pt_form = QLineEdit()
        self._pt_form.setPlaceholderText("e.g. NBME 14")
        self._pt_score = QSpinBox()
        self._pt_score.setRange(150, 300)
        self._pt_score.setValue(245)
        save = QPushButton("Save score")
        qconnect(save.clicked, self._save_practice_test)
        form.addRow("Source", self._pt_source)
        form.addRow("Form", self._pt_form)
        form.addRow("Scaled score", self._pt_score)
        form.addRow("", save)
        return box

    def _footer(self) -> QWidget:
        row = QWidget()
        h = QHBoxLayout(row)
        h.addStretch()
        close = QPushButton("Close")
        qconnect(close.clicked, self._close_and_remember)
        today = QPushButton("Open Today")
        today.setDefault(True)
        qconnect(today.clicked, self._open_today)
        h.addWidget(close)
        h.addWidget(today)
        return row

    def _refresh_deck_label(self) -> None:
        count = len(self.mw.col.find_cards("")) if self.mw.col else 0
        self._deck_label.setText(
            f"Your collection has <b>{count:,}</b> cards. Import a deck below, or "
            "use what you already have."
        )

    def _import_deck(self) -> None:
        from aqt.import_export.importing import prompt_for_file_then_import

        prompt_for_file_then_import(self.mw)
        self._refresh_deck_label()

    def _scan_and_map(self) -> None:
        col = self.mw.col
        if col is None:
            return
        result = auto_crosswalk_from_tags(col)
        if result.total_mapped == 0 and not result.unmatched:
            tooltip(
                "No AnKing subject tags found. Import an AnKing deck, or connect a "
                "QBank instead.",
                parent=self,
            )
            return
        if result.unmatched:
            dialog = _ReviewMappingDialog(self, result.unmatched)
            if dialog.exec():
                for label, tid in dialog.assignments().items():
                    for cid in result.unmatched.get(label, []):
                        result.card_topics[cid] = tid
        apply_crosswalk(col, result.card_topics)
        topics = len(set(result.card_topics.values()))
        self._map_label.setText(
            f"Mapped <b>{result.total_mapped:,}</b> cards across <b>{topics}</b> "
            "topics. Points-at-stake review order is on."
        )
        self.mw.reset()

    def _save_practice_test(self) -> None:
        if self.mw.col is None:
            return
        store_practice_test(
            self.mw.col,
            self._pt_source.currentText(),
            self._pt_form.text().strip() or self._pt_source.currentText(),
            float(self._pt_score.value()),
        )
        tooltip("Practice-test score saved.", parent=self)

    def _mark_onboarded(self) -> None:
        if self.mw.col is not None:
            self.mw.col.set_config("speedrun:onboarded", True)

    def _close_and_remember(self) -> None:
        self._mark_onboarded()
        self.reject()

    def _open_today(self) -> None:
        self._mark_onboarded()
        self.accept()
        show_stat_page(self.mw, "today", "Today")


def run_setup_wizard(mw: AnkiQt) -> QDialog:
    """Open the Set up STAT dialog (from the menu or first-run)."""
    dialog = SetupStatDialog(mw)
    mw.garbage_collect_on_dialog_finish(dialog)
    _open_dialogs.append(dialog)

    def on_finished(_result: int) -> None:
        if dialog in _open_dialogs:
            _open_dialogs.remove(dialog)

    qconnect(dialog.finished, on_finished)
    dialog.show()
    return dialog


def maybe_offer_onboarding(mw: AnkiQt) -> None:
    """profile_did_open hook: show the wizard once when STAT has no taxonomy."""
    if mw.col is not None and should_offer_onboarding(mw.col):
        QTimer.singleShot(0, lambda: run_setup_wizard(mw))


def ensure_fsrs_enabled(mw: AnkiQt) -> None:
    """profile_did_open hook: STAT's Memory score is defined as FSRS recall, so
    the engine must run FSRS for reviews to record the per-card memory state the
    score reads. A fresh profile defaults to SM-2 (no memory state), which would
    make Memory abstain forever no matter how much you review. Turn FSRS on once
    (idempotent); reviews from then on record the recall the score needs."""
    col = mw.col
    if col is None or col.get_config("fsrs", False):
        return
    col.set_config("fsrs", True)


# Reviewer: categorize uncategorized cards on the fly
##########################################################################


def _topic_picker_js(card_id: int) -> str:
    """JS that drops a small 'file this card' bar at the bottom of the reviewer,
    with the 22 blueprint topics + a Skip button, wired back via pycmd."""
    options = "".join(
        f'<option value="{tid}">{name}</option>' for tid, name, _w in _FULL_BLUEPRINT
    )
    inner = (
        "<span>STAT \u00b7 uncategorized card \u2014 file under:</span>"
        '<select id="speedrun-cat-sel" style="font-size:13px;padding:2px 4px;">'
        '<option value="" selected disabled>Choose a topic\u2026</option>'
        f"{options}</select>"
        '<button id="speedrun-cat-skip" '
        'style="font-size:13px;padding:2px 8px;cursor:pointer;">Skip</button>'
    )
    return (
        "(function(){"
        "var id='speedrun-cat';"
        "var old=document.getElementById(id); if(old){old.remove();}"
        "var bar=document.createElement('div'); bar.id=id;"
        "bar.style.cssText='position:fixed;left:0;right:0;bottom:0;"
        "z-index:2147483000;display:flex;gap:8px;align-items:center;"
        "justify-content:center;padding:6px 10px;font-size:13px;"
        "background:var(--canvas-elevated,#e9e9e9);color:var(--fg,#111);"
        "border-top:1px solid var(--border,#ccc);';"
        f"bar.innerHTML={json.dumps(inner)};"
        "document.body.appendChild(bar);"
        f"var cid={card_id};"
        "document.getElementById('speedrun-cat-sel').addEventListener('change',"
        "function(){if(this.value){pycmd('speedrunSetTopic:'+cid+':'+this.value);}});"
        "document.getElementById('speedrun-cat-skip').addEventListener('click',"
        "function(){pycmd('speedrunSkipTopic:'+cid);});"
        "})();"
    )


def _close_picker(mw: AnkiQt) -> None:
    mw.reviewer.web.eval(
        "(function(){var b=document.getElementById('speedrun-cat');"
        "if(b){b.remove();}})();"
    )


def _maybe_prompt_uncategorized(mw: AnkiQt, card: Card) -> None:
    """On show-answer: if STAT is set up and this card isn't mapped to a topic,
    offer the one-click picker (or skip, leaving it uncategorized)."""
    col = mw.col
    if col is None or not col.get_config(_TOPICS_KEY, None):
        return
    cid = str(card.id)
    if cid in _session_skipped or cid in _categorized_card_ids(col):
        return
    mw.reviewer.web.eval(_topic_picker_js(card.id))


# --- Custom reviewer bridge: real cards via the v3 scheduler ----------------
# Anki's scheduler stays AUTHORITATIVE — each grade goes through the real
# build_answer/answer_card, so intervals + undo + integrity stay correct. The
# STAT reviewer webview only displays the card and relays the chosen rating.
_review_card: Card | None = None
_review_states: Any = None

_RATINGS = {
    1: scheduler_pb2.CardAnswer.AGAIN,
    2: scheduler_pb2.CardAnswer.HARD,
    3: scheduler_pb2.CardAnswer.GOOD,
    4: scheduler_pb2.CardAnswer.EASY,
}


def _review_card_payload(col: Collection, card: Card, states: Any) -> dict[str, Any]:
    crosswalk = col.get_config(_CARD_TOPICS_KEY, None)
    topic_id = crosswalk.get(str(card.id)) if isinstance(crosswalk, dict) else None
    weakness_map = col.get_config("speedrun:weakness", None)
    weakness = (
        float(weakness_map.get(topic_id, 0.0))
        if topic_id and isinstance(weakness_map, dict)
        else 0.0
    )
    return {
        "question": card.question(),
        "answer": card.answer(),
        "topicName": _TOPIC_NAMES.get(topic_id, "") if topic_id else "",
        "weakness": weakness,
        "buttons": list(cast(V3Scheduler, col.sched).describe_next_states(states)),
    }


def _review_next(col: Collection, web: AnkiWebView) -> None:
    global _review_card, _review_states
    queued = cast(V3Scheduler, col.sched).get_queued_cards(fetch_limit=1)
    if not queued.cards:
        _review_card = None
        web.eval("window.speedrunSessionDone && window.speedrunSessionDone();")
        return
    qc = queued.cards[0]
    card = Card(col, backend_card=qc.card)
    card.start_timer()
    _review_card = card
    _review_states = qc.states
    payload = _review_card_payload(col, card, qc.states)
    payload["counts"] = {
        "new": queued.new_count,
        "learning": queued.learning_count,
        "review": queued.review_count,
    }
    web.eval(
        f"window.speedrunSetCard && window.speedrunSetCard({json.dumps(payload)});"
    )


def _review_answer(col: Collection, web: AnkiWebView, ease: int) -> None:
    global _review_card
    if _review_card is None:
        return
    rating = _RATINGS.get(ease, scheduler_pb2.CardAnswer.GOOD)
    sched = cast(V3Scheduler, col.sched)
    answer = sched.build_answer(card=_review_card, states=_review_states, rating=rating)
    sched.answer_card(answer)  # authoritative scheduling + undo entry
    _review_card = None
    _review_next(col, web)


def _speedrun_js_handler(
    mw: AnkiQt, handled: tuple[bool, Any], message: str, context: Any
) -> tuple[bool, Any]:
    if handled[0]:
        return handled
    if message == "speedrunReviewNext":
        if mw.col is not None and hasattr(context, "eval"):
            _review_next(mw.col, context)
        return (True, None)
    if message.startswith("speedrunReviewAnswer:"):
        if mw.col is not None and hasattr(context, "eval"):
            _review_answer(mw.col, context, int(message.split(":", 1)[1]))
        return (True, None)
    if message == "speedrunStudy":
        # Native-reviewer fallback (Today's "Start block", the menu item): close
        # the console dialog(s) and study the current deck in Anki's reviewer.
        for dialog in list(_open_dialogs):
            dialog.close()
        mw.moveToState("review")
        return (True, None)
    if message.startswith("speedrunSetTopic:"):
        _, cid, tid = message.split(":", 2)
        if mw.col is not None:
            assign_card_topic(mw.col, int(cid), tid)
        _close_picker(mw)
        tooltip(f"Filed under {_TOPIC_NAMES.get(tid, tid)}.", parent=mw)
        return (True, None)
    if message.startswith("speedrunSkipTopic:"):
        _, cid = message.split(":", 1)
        _session_skipped.add(cid)
        _close_picker(mw)
        return (True, None)
    return handled


def setup_speedrun_reviewer(mw: AnkiQt) -> None:
    """Register the reviewer hooks that offer topic categorization for
    uncategorized cards during study (once per session)."""
    from aqt import gui_hooks

    gui_hooks.reviewer_did_show_answer.append(
        lambda card: _maybe_prompt_uncategorized(mw, card)
    )
    gui_hooks.webview_did_receive_js_message.append(
        lambda handled, message, context: _speedrun_js_handler(
            mw, handled, message, context
        )
    )
    gui_hooks.collection_did_load.append(lambda _col: _clear_categorized_cache())


# Menu wiring
##########################################################################


def setup_speedrun_menu(mw: AnkiQt) -> None:
    """Add a ``Speedrun`` submenu to Tools with the STAT console + demo actions."""
    tools_menu = mw.form.menuTools
    menu = QMenu("Speedrun", tools_menu)
    tools_menu.addSeparator()
    tools_menu.addMenu(menu)

    setup_action = menu.addAction("Set up STAT…")
    assert setup_action is not None
    qconnect(setup_action.triggered, lambda: run_setup_wizard(mw))
    menu.addSeparator()

    # The five STAT console destinations (the daily loop).
    for label, page in _STAT_PAGES:
        action = menu.addAction(label)
        assert action is not None
        # Default args bind the loop variables per iteration; the leading
        # parameter absorbs QAction.triggered's `checked` bool.
        qconnect(
            action.triggered,
            lambda _checked=False, p=page, t=label: show_stat_page(mw, p, t),
        )

    menu.addSeparator()

    # Native-reviewer fallback (guaranteed-correct study loop).
    study_action = menu.addAction("Study (native reviewer)")
    assert study_action is not None
    qconnect(study_action.triggered, lambda: mw.moveToState("review"))

    score_action = menu.addAction("Memory Score")
    assert score_action is not None
    qconnect(score_action.triggered, lambda: show_memory_score(mw))

    seed_action = menu.addAction("Seed sample data (demo)")
    assert seed_action is not None
    qconnect(seed_action.triggered, lambda: seed_sample_data(mw))

    reset_action = menu.addAction("Reset Speedrun data (demo)")
    assert reset_action is not None
    qconnect(reset_action.triggered, lambda: reset_speedrun_data(mw))
