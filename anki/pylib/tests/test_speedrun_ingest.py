# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for F2 — QBank / practice-test ingestion.

Drives the Rust ingest engine through the ``col`` backend (the same path the
desktop UI uses) and reads the result back out of ``col.conf`` to confirm the
round-trip: import persists, re-importing the same export dedups, and a second
import merges without dropping earlier data. Mirrors the seeding style of
``test_speedrun.py``.
"""

from anki import speedrun_pb2
from tests.shared import getEmptyCol

# col.conf keys owned by the Rust ingest store (speedrun/attempts.rs).
ATTEMPTS_KEY = "speedrun:attempts"
PRACTICE_TESTS_KEY = "speedrun:practice_tests"


def _attempt(external_id: str, answered_at: int, correct: bool = True):
    return speedrun_pb2.QuestionAttempt(
        source="uworld",
        external_id=external_id,
        answered_at=answered_at,
        topic_id="cardio",
        correct=correct,
        seconds=60,
    )


def _test(form: str, taken_at: int):
    return speedrun_pb2.PracticeTestResult(
        source="nbme",
        form=form,
        taken_at=taken_at,
        scaled_score=245.0,
        percent_correct=0.8,
    )


def _stored_attempts(col):
    return col.get_config(ATTEMPTS_KEY, [])


def _stored_tests(col):
    return col.get_config(PRACTICE_TESTS_KEY, [])


def test_import_persists_dedups_and_merges():
    col = getEmptyCol()

    # A fresh collection has imported nothing yet.
    assert _stored_attempts(col) == []
    assert _stored_tests(col) == []

    # First import persists the attempts + tests through the Rust backend.
    col._backend.import_qbank_data(
        attempts=[_attempt("q1", 1000), _attempt("q2", 1001, correct=False)],
        tests=[_test("NBME 14", 5000)],
    )
    attempts = _stored_attempts(col)
    tests = _stored_tests(col)
    assert len(attempts) == 2
    assert len(tests) == 1
    # Round-trip: the student's own metadata is preserved verbatim.
    assert attempts[0]["source"] == "uworld"
    assert attempts[0]["external_id"] == "q1"
    assert attempts[0]["correct"] is True
    assert attempts[1]["correct"] is False
    assert tests[0]["form"] == "NBME 14"

    # Re-importing the same export dedups on (source, external_id, answered_at)
    # and (source, form, taken_at) -> counts are unchanged (idempotent).
    col._backend.import_qbank_data(
        attempts=[_attempt("q1", 1000), _attempt("q2", 1001, correct=False)],
        tests=[_test("NBME 14", 5000)],
    )
    assert len(_stored_attempts(col)) == 2
    assert len(_stored_tests(col)) == 1

    # A second import with new records merges without dropping the earlier ones.
    col._backend.import_qbank_data(
        attempts=[_attempt("q3", 1002)],
        tests=[_test("UWSA2", 6000)],
    )
    merged_attempts = _stored_attempts(col)
    merged_tests = _stored_tests(col)
    assert len(merged_attempts) == 3
    assert [a["external_id"] for a in merged_attempts] == ["q1", "q2", "q3"]
    assert [t["form"] for t in merged_tests] == ["NBME 14", "UWSA2"]


def test_import_is_undo_safe():
    col = getEmptyCol()

    col._backend.import_qbank_data(attempts=[_attempt("q1", 1000)], tests=[])
    assert len(_stored_attempts(col)) == 1

    # The ingest is a single undoable op; undoing it restores the prior state.
    col.undo()
    assert _stored_attempts(col) == []
