# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Python-side coverage for F3 — turn imported misses into action (relink).

Drives the Rust relink engine through the ``col`` backend (the same path the
desktop UI uses) to prove the whole "miss -> focus" loop end to end:

* weakness is recomputed from QBank accuracy (weakness = 1 - accuracy),
  replacing the seeded value, with a thin-data topic keeping its prior weakness;
* the flashcards behind a missed topic are auto-unsuspended, while a topic with
  no miss keeps its cards suspended (selective unsuspension);
* one error-log entry is produced per miss, each with a joined topic name and a
  non-empty clinical-reasoning reframe prompt;
* the whole thing is a single undoable op — one ``undo()`` restores both the
  prior weakness and the prior suspension.

Mirrors the seeding style of ``test_speedrun_focus.py`` /
``test_speedrun_review_loop.py``.
"""

from anki import speedrun_pb2
from anki.consts import CARD_TYPE_REV, QUEUE_TYPE_SUSPENDED
from tests.shared import getEmptyCol

# col.conf keys owned by the Rust stores (speedrun/store.rs, speedrun/relink.rs).
WEAKNESS_KEY = "speedrun:weakness"


def _attempt(external_id: str, answered_at: int, topic: str, correct: bool):
    return speedrun_pb2.QuestionAttempt(
        source="uworld",
        external_id=external_id,
        answered_at=answered_at,
        topic_id=topic,
        correct=correct,
        seconds=60,
    )


def _add_suspended_review_card(col, front: str) -> int:
    """Add a note and leave its card a Suspended Review-state card."""
    note = col.newNote()
    note["Front"] = front
    col.addNote(note)
    c = note.cards()[0]
    c.type = CARD_TYPE_REV
    c.queue = QUEUE_TYPE_SUSPENDED
    c.flush()
    return c.id


def test_relink_recomputes_weakness_unsuspends_and_logs():
    col = getEmptyCol()

    # A suspended card behind cardio (which will be missed) and one behind renal
    # (which will not) — proves selective unsuspension end to end.
    cardio_cid = _add_suspended_review_card(col, "cardio card")
    renal_cid = _add_suspended_review_card(col, "renal card")

    col._backend.set_topic_weights(
        topics=[
            speedrun_pb2.Topic(id="cardio", name="Cardiology", blueprint_weight=0.5),
            speedrun_pb2.Topic(id="renal", name="Nephrology", blueprint_weight=0.4),
        ],
        card_topics=[
            speedrun_pb2.CardTopic(card_id=cardio_cid, topic_id="cardio"),
            speedrun_pb2.CardTopic(card_id=renal_cid, topic_id="renal"),
        ],
        # Seeded priors that relink must replace (cardio) or keep (renal).
        weaknesses=[
            speedrun_pb2.TopicWeakness(topic_id="cardio", weakness=0.2),
            speedrun_pb2.TopicWeakness(topic_id="renal", weakness=0.7),
        ],
    )

    # cardio: 5 attempts (>= MIN_ATTEMPTS), all wrong -> weakness recomputed 1.0.
    # renal: 1 correct attempt (< MIN_ATTEMPTS, no miss) -> keeps prior 0.7.
    attempts = [_attempt(f"c{i}", 1000 + i, "cardio", False) for i in range(5)]
    attempts.append(_attempt("r0", 2000, "renal", True))
    col._backend.import_qbank_data(attempts=attempts, tests=[])

    col._backend.relink_misses()

    # Weakness recomputed from accuracy; renal kept (thin data + no miss).
    weakness = col.get_config(WEAKNESS_KEY, {})
    assert abs(weakness["cardio"] - 1.0) < 1e-9
    assert abs(weakness["renal"] - 0.7) < 1e-9

    # Selective unsuspension: cardio's card is live again, renal's stays suspended.
    assert col.get_card(cardio_cid).queue != QUEUE_TYPE_SUSPENDED
    assert col.get_card(renal_cid).queue == QUEUE_TYPE_SUSPENDED

    # Error log: one entry per miss (5), joined topic name + non-empty reframe.
    # The response's single repeated field is unwrapped by the generated backend.
    entries = col._backend.get_error_log()
    assert len(entries) == 5
    assert all(e.topic_id == "cardio" for e in entries)
    assert all(e.topic_name == "Cardiology" for e in entries)
    assert all("Cardiology" in e.reframe_prompt for e in entries)
    # Exactly one miss recorded the single card it unsuspended (the rest 0).
    assert sum(e.unsuspended_cards for e in entries) == 1

    # Undo-safe: one undo restores BOTH the prior weakness and the suspension.
    col.undo()
    weakness = col.get_config(WEAKNESS_KEY, {})
    assert abs(weakness["cardio"] - 0.2) < 1e-9
    assert col.get_card(cardio_cid).queue == QUEUE_TYPE_SUSPENDED
    assert col.db.scalar("pragma integrity_check") == "ok"
