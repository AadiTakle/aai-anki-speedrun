# Copyright: Ankitects Pty Ltd and contributors
# License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

"""Tests for the STAT onboarding auto-crosswalk (AnKing tags -> blueprint).

These cover the pure mapping logic that makes the Anki -> STAT transition one
click; the Qt wizard + the col-backed scan are exercised in-app."""

from __future__ import annotations

import tempfile

from anki import speedrun_pb2
from anki.collection import Collection
from aqt.speedrun import (
    _extract_topic_from_tags,
    _normalize_label,
    _topic_picker_js,
    apply_crosswalk,
    assign_card_topic,
    auto_crosswalk_from_tags,
    store_practice_test,
)


class TestNormalizeLabel:
    def test_underscores_and_slashes_become_spaces(self) -> None:
        assert _normalize_label("Infectious_Disease") == "infectious disease"
        assert _normalize_label("OB/GYN") == "ob gyn"
        assert _normalize_label("Hematology_Oncology") == "hematology oncology"

    def test_case_and_trim(self) -> None:
        assert _normalize_label("  Cardiology  ") == "cardiology"


class TestExtractTopicFromTags:
    def test_maps_anking_subject_to_blueprint_topic(self) -> None:
        assert _extract_topic_from_tags(["#AK_Step2_v12::#Subjects::Cardiology"]) == (
            "cardio",
            "Cardiology",
        )

    def test_uses_only_the_leading_subject_segment(self) -> None:
        # Deeper sub-tags (::CKD) must not defeat the map.
        assert _extract_topic_from_tags(
            ["#AK_Step2_v12::#Subjects::Nephrology::CKD"]
        ) == ("renal", "Nephrology")

    def test_version_prefix_agnostic(self) -> None:
        assert _extract_topic_from_tags(["#AK_Step2_v11::#Subjects::Dermatology"]) == (
            "derm",
            "Dermatology",
        )

    def test_prefers_subjects_over_shelf(self) -> None:
        tags = [
            "#AK_Step2_v12::!Shelf::Pediatrics",
            "#AK_Step2_v12::#Subjects::Cardiology",
        ]
        assert _extract_topic_from_tags(tags)[0] == "cardio"

    def test_shelf_is_a_fallback(self) -> None:
        assert _extract_topic_from_tags(["#AK_Step2_v12::!Shelf::Pediatrics"]) == (
            "peds",
            "Pediatrics",
        )

    def test_unmatched_subject_returns_label_for_review(self) -> None:
        topic, label = _extract_topic_from_tags(
            ["#AK_Step2_v12::#Subjects::Toxicology"]
        )
        assert topic is None
        assert label == "Toxicology"

    def test_no_anking_tags_returns_none(self) -> None:
        assert _extract_topic_from_tags(["marked", "leech", "AK_Update"]) == (
            None,
            None,
        )


def _col_with_anking_cards() -> Collection:
    """A scratch collection whose cards carry AnKing #Subjects tags."""
    col = Collection(tempfile.mktemp(suffix=".anki2"))
    # A raw collection has no default notetypes, so build a minimal one.
    model = col.models.new("Basic-STAT")
    col.models.add_field(model, col.models.new_field("Front"))
    col.models.add_field(model, col.models.new_field("Back"))
    template = col.models.new_template("Card 1")
    template["qfmt"] = "{{Front}}"
    template["afmt"] = "{{Back}}"
    col.models.add_template(model, template)
    col.models.add(model)
    model = col.models.by_name("Basic-STAT")
    assert model is not None
    did = col.decks.id("Default")
    assert did is not None
    for subject, count in [("Cardiology", 3), ("Nephrology", 2), ("Toxicology", 1)]:
        for i in range(count):
            note = col.new_note(model)
            note["Front"] = f"{subject} {i}"
            note["Back"] = "answer"
            note.tags = [f"#AK_Step2_v12::#Subjects::{subject}"]
            col.add_note(note, did)
    return col


class TestOnboardingEndToEnd:
    """Drives the exact transition the wizard runs: AnKing tags -> crosswalk ->
    QBank aggregate import -> performance/weakness, with readiness abstaining."""

    def test_full_pipeline(self) -> None:
        col = _col_with_anking_cards()
        try:
            # 1. Auto-crosswalk from tags: 3 cardio + 2 renal mapped, Toxicology not.
            result = auto_crosswalk_from_tags(col)
            assert result.total_mapped == 5
            assert set(result.card_topics.values()) == {"cardio", "renal"}
            assert "Toxicology" in result.unmatched

            # 2. Persist blueprint + crosswalk.
            apply_crosswalk(col, result.card_topics)
            assert col.get_config("speedrun:topics")

            # 3. Import a weak cardio QBank aggregate (20/100).
            col._backend.import_qbank_aggregate(
                source="UWorld",
                rows=[
                    speedrun_pb2.QbankTopicResult(
                        topic_id="cardio", correct=20, total=100
                    )
                ],
            )
            perf = col._backend.get_performance_score()
            assert any(
                t.topic_id == "cardio" and t.attempts == 100 for t in perf.topics
            )

            # 4. Relink turns aggregate accuracy into weakness -> points-at-stake.
            #    (get_points_at_stake unwraps to the topics list — single repeated
            #    field, per Anki's Python backend convention.)
            col._backend.relink_misses()
            pas = col._backend.get_points_at_stake()
            assert any(t.topic_id == "cardio" and t.weakness > 0.5 for t in pas)

            # 5. Readiness stays honestly abstained (no calibration yet).
            assert col._backend.get_readiness_score().abstained

            # 6. A practice-test score stores without error.
            store_practice_test(col, "UWSA2", "UWSA2", 245.0)
        finally:
            col.close()


class TestReviewerCategorization:
    """The in-reviewer 'file this uncategorized card' flow."""

    def test_assign_card_topic_persists_to_crosswalk(self) -> None:
        col = Collection(tempfile.mktemp(suffix=".anki2"))
        try:
            assign_card_topic(col, 42, "cardio")
            stored = col.get_config("speedrun:card_topics")
            assert stored["42"] == "cardio"
            # A second card is added, not replaced.
            assign_card_topic(col, 99, "renal")
            stored = col.get_config("speedrun:card_topics")
            assert stored["42"] == "cardio"
            assert stored["99"] == "renal"
        finally:
            col.close()

    def test_topic_picker_js_embeds_card_and_topics(self) -> None:
        js = _topic_picker_js(12345)
        assert "12345" in js
        assert "speedrunSetTopic" in js
        assert "speedrunSkipTopic" in js
        # A real blueprint topic option is present.
        assert "cardio" in js
