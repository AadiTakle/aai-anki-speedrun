# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for the Speedrun memory score (F6).

Mirrors the Rust acceptance tests: a fresh collection has no graded reviews and
no blueprint coverage, so the backend must abstain (never fabricate a readiness
number) while still reporting a real coverage of 0% and non-empty reasons.
"""

from anki import speedrun_pb2  # noqa: F401  (registers anki.speedrun_pb2)
from tests.shared import getEmptyCol


def test_memory_score_abstains_on_fresh_collection():
    col = getEmptyCol()

    score = col._backend.get_memory_score()

    assert score.abstained is True
    assert score.coverage_pct == 0.0
    assert len(score.reasons) > 0
