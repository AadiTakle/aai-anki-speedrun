// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Route-local, pure presentational helpers for the "Today" console. These
// derive display-only LABELS from the typed foundation Views — they never
// fabricate a score. Each derivation is an honest, human-readable descriptor
// over the real range / coverage / weight fields the adapter already returns.

import { stakeAcuity } from "$lib/speedrun";
import type { CoverageMapView, PointsAtStakeTopicView, ReadinessScoreView } from "$lib/speedrun";

/**
 * A confidence label derived from the WIDTH of the readiness band: a wider band
 * means less certainty. This is a qualitative descriptor over the real low/high
 * range — never a fabricated point. Abstaining scores read "Not enough info".
 */
export function confidenceLabel(readiness: ReadinessScoreView): string {
    if (readiness.abstained) {
        return "Not enough info";
    }
    const width = readiness.high - readiness.low;
    if (width >= 11) {
        return "Low confidence";
    }
    if (width >= 7) {
        return "Medium confidence";
    }
    return "High confidence";
}

/** Yield descriptor bucketed from a topic's blueprint weight. */
function yieldLabel(blueprintWeight: number): string {
    if (blueprintWeight >= 10) {
        return "high-yield";
    }
    if (blueprintWeight >= 6) {
        return "mid-yield";
    }
    return "lower-yield";
}

/** Weakness descriptor, paired with (and consistent with) the acuity hue. */
function weaknessLabel(weakness: number): string {
    if (weakness >= 0.7) {
        return "weak accuracy";
    }
    if (weakness >= 0.45) {
        return "building";
    }
    if (weakness >= 0.25) {
        return "moderate";
    }
    return "strong on both";
}

/**
 * A short reason string for a points-at-stake row, e.g. "high-yield · weak
 * accuracy". Composed from the topic's real blueprint weight + weakness so the
 * word always agrees with the acuity color (never color alone).
 */
export function stakeReason(topic: PointsAtStakeTopicView): string {
    return `${yieldLabel(topic.blueprintWeight)} \u00b7 ${weaknessLabel(topic.weakness)}`;
}

/**
 * A trailing badge for a stake row, derived from real coverage + acuity: a
 * section with 0% coverage reads "Untouched" (it blocks readiness); a strong,
 * low-weakness topic reads "Paused" (a guardrail / diminishing-returns
 * candidate). Otherwise no badge.
 */
export function stakeNote(topic: PointsAtStakeTopicView, coverage: CoverageMapView): string | null {
    const section = coverage.sections.find((s) => s.id === topic.topicId);
    if (section && section.coveragePct === 0) {
        return "Untouched";
    }
    if (stakeAcuity(topic) === "muted") {
        return "Paused";
    }
    return null;
}
