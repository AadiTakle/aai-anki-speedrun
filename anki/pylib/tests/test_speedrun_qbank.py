# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for F2 (wave 3) — aggregate QBank ingestion.

Drives the Rust aggregate-import engine through the ``col`` backend (the same
path the desktop UI uses): the import persists + returns counts, re-importing a
source REPLACES its rows (idempotent updates, no double-counting), the import is
undo-safe, and the imported aggregate feeds the Performance score end-to-end —
verifying the ``ImportQbankAggregateResponse`` proto fields round-trip through the
RPC. Mirrors ``test_speedrun_ingest.py`` / ``test_speedrun_performance.py``.
"""

from anki import speedrun_pb2
from tests.shared import getEmptyCol

# col.conf key owned by the Rust aggregate store (speedrun/qbank.rs):
# {source: {topic_id: {correct, total, updated_at}}}.
QBANK_AGGREGATES_KEY = "speedrun:qbank_aggregates"


def _row(topic_id: str, correct: int, total: int):
    return speedrun_pb2.QbankTopicResult(
        topic_id=topic_id, correct=correct, total=total
    )


def _store(col):
    return col.get_config(QBANK_AGGREGATES_KEY, {})


def test_import_persists_and_returns_counts():
    col = getEmptyCol()

    # A fresh collection has imported no aggregates yet.
    assert _store(col) == {}

    resp = col._backend.import_qbank_aggregate(
        source="uworld",
        rows=[_row("cardio", 60, 100), _row("renal", 30, 50)],
    )
    # The response reports the rows kept and the summed question count.
    assert resp.topics_imported == 2
    assert resp.total_questions == 150

    # Round-trip: the aggregate is persisted under the source in col.conf.
    store = _store(col)
    assert store["uworld"]["cardio"]["correct"] == 60
    assert store["uworld"]["cardio"]["total"] == 100
    assert store["uworld"]["renal"]["total"] == 50


def test_reimport_same_source_replaces():
    col = getEmptyCol()

    col._backend.import_qbank_aggregate(source="uworld", rows=[_row("cardio", 60, 100)])
    # A corrected paste for the same source REPLACES its rows (never accumulates,
    # so re-importing updated numbers can't double-count).
    col._backend.import_qbank_aggregate(source="uworld", rows=[_row("cardio", 70, 120)])

    store = _store(col)
    assert store["uworld"]["cardio"]["correct"] == 70
    assert store["uworld"]["cardio"]["total"] == 120


def test_two_sources_coexist():
    col = getEmptyCol()

    col._backend.import_qbank_aggregate(source="uworld", rows=[_row("cardio", 60, 100)])
    col._backend.import_qbank_aggregate(source="amboss", rows=[_row("cardio", 30, 50)])

    store = _store(col)
    # Both sources are kept independently (combined per topic downstream).
    assert store["uworld"]["cardio"]["total"] == 100
    assert store["amboss"]["cardio"]["total"] == 50


def test_import_is_undo_safe():
    col = getEmptyCol()

    col._backend.import_qbank_aggregate(source="uworld", rows=[_row("cardio", 60, 100)])
    assert _store(col) != {}

    # The import is a single undoable op; undoing it restores the prior state.
    col.undo()
    assert _store(col) == {}


def test_aggregate_feeds_performance_score():
    col = getEmptyCol()

    # A canonical blueprint topic backed ONLY by aggregate data (auto-added to
    # the taxonomy with its canonical weight) still produces a Performance score
    # once there are enough combined questions (>= 50).
    col._backend.import_qbank_aggregate(source="uworld", rows=[_row("cardio", 40, 60)])

    score = col._backend.get_performance_score()
    assert score.abstained is False
    cardio = next(t for t in score.topics if t.topic_id == "cardio")
    assert cardio.attempts == 60
    assert cardio.correct == 40
