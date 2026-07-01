// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Pure, framework-free display logic shared by every STAT screen. Keeping the
// honesty-bar formatting here (never fabricate a number while abstaining) means
// all three vitals — and both apps — render uncertainty the same way.

import type { Acuity, GaugeView, PointsAtStakeTopicView, ReadinessScoreView, ScoreView } from "./types";

// U+2013 EN DASH — joins the low/high bounds of a range.
export const EN_DASH = "\u2013";

/** Honest abstain headline for the dominant readiness vital. */
export const ABSTAIN_HEADLINE = "NOT ENOUGH INFO";

/**
 * "low–high" (rounded) when scored; `null` when abstaining. Never returns a
 * fabricated number — an abstaining score has no range to show.
 */
export function formatScoreRange(score: ScoreView): string | null {
    if (score.abstained) {
        return null;
    }
    return `${Math.round(score.low)}${EN_DASH}${Math.round(score.high)}`;
}

/** Rounded best-estimate point when scored; `null` when abstaining. */
export function formatScorePoint(score: ScoreView): number | null {
    return score.abstained ? null : Math.round(score.point);
}

/** Blueprint coverage formatted for display, e.g. "61%". */
export function formatCoverage(coveragePct: number): string {
    return `${Math.round(coveragePct)}%`;
}

/** A percentage value (0..100) formatted, e.g. "68%". */
export function formatPercent(pct: number): string {
    return `${Math.round(pct)}%`;
}

/**
 * A percentage-style range, e.g. "68–74%", for the small Memory / Performance
 * "input" readouts. `null` when abstaining.
 */
export function formatPercentRange(score: ScoreView): string | null {
    const range = formatScoreRange(score);
    return range === null ? null : `${range}%`;
}

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/**
 * Relative "updated" label, e.g. "updated 2h ago". Returns `null` when the
 * timestamp is unknown (0) so callers can omit it rather than invent a time.
 */
export function formatUpdatedAt(updatedAt: number, nowSeconds: number = Date.now() / 1000): string | null {
    if (!updatedAt) {
        return null;
    }
    const delta = Math.max(0, Math.floor(nowSeconds - updatedAt));
    if (delta < MINUTE) {
        return "updated just now";
    }
    if (delta < HOUR) {
        return `updated ${Math.floor(delta / MINUTE)}m ago`;
    }
    if (delta < DAY) {
        return `updated ${Math.floor(delta / HOUR)}h ago`;
    }
    return `updated ${Math.floor(delta / DAY)}d ago`;
}

/**
 * Derive everything `ReadinessGauge` needs from a readiness score. Abstaining
 * scores map to the honest flatline (`mode: "abstain"`) with the unlock rule,
 * never a band.
 */
export function readinessToGauge(
    score: ReadinessScoreView,
    opts: { target?: number; unlock?: string } = {},
): GaugeView {
    if (score.abstained) {
        return { mode: "abstain", target: opts.target, unlock: opts.unlock };
    }
    return {
        mode: "confident",
        low: Math.round(score.low),
        high: Math.round(score.high),
        point: Math.round(score.point),
        target: opts.target,
    };
}

/**
 * Map a per-topic `weakness` (0 strong .. 1 weak) to a clinical acuity for the
 * points-at-stake list. Thresholds chosen so the seeded personas reproduce the
 * design's critical/watch/stable/muted split; screen lanes may override.
 */
export function acuityFromWeakness(weakness: number): Acuity {
    if (weakness >= 0.7) {
        return "critical";
    }
    if (weakness >= 0.45) {
        return "watch";
    }
    if (weakness >= 0.25) {
        return "stable";
    }
    return "muted";
}

/** Points-at-stake weight formatted for a row, e.g. "9.1 pts" or "—". */
export function formatStakePoints(points: number): string {
    return points > 0 ? `${points.toFixed(1)} pts` : "\u2014";
}

/** Convenience: acuity for a proto-shaped points-at-stake topic. */
export function stakeAcuity(topic: PointsAtStakeTopicView): Acuity {
    return acuityFromWeakness(topic.weakness);
}

/** Human label for an acuity (pairs a word with the hue — color is never alone). */
export function acuityLabel(acuity: Acuity): string {
    switch (acuity) {
        case "critical":
            return "Critical";
        case "watch":
            return "Watch";
        case "stable":
            return "Stable";
        case "muted":
            return "Paused";
    }
}
