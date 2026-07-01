// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { expect, test } from "vitest";

import {
    ABSTAIN_HEADLINE,
    acuityFromWeakness,
    EN_DASH,
    formatCoverage,
    formatScorePoint,
    formatScoreRange,
    formatStakePoints,
    formatUpdatedAt,
    readinessToGauge,
} from "./display";
import type { ReadinessScoreView, ScoreView } from "./types";

function scored(overrides: Partial<ScoreView> = {}): ScoreView {
    return {
        abstained: false,
        point: 244.4,
        low: 238.5,
        high: 250.4,
        coveragePct: 61,
        reasons: ["Only 61% of the blueprint is covered"],
        updatedAt: 0,
        ...overrides,
    };
}

function abstained(overrides: Partial<ScoreView> = {}): ScoreView {
    return { ...scored(), abstained: true, ...overrides };
}

test("a scored score renders a rounded low–high range, not one number", () => {
    expect(formatScoreRange(scored())).toBe(`239${EN_DASH}250`);
    expect(formatScorePoint(scored())).toBe(244);
});

test("honesty bar: an abstaining score fabricates no number", () => {
    expect(formatScoreRange(abstained())).toBeNull();
    expect(formatScorePoint(abstained())).toBeNull();
    // The headline never contains a digit.
    expect(/\d/.test(ABSTAIN_HEADLINE)).toBe(false);
});

test("readinessToGauge maps abstain to the honest flatline, never a band", () => {
    const gauge = readinessToGauge(abstained() as ReadinessScoreView, {
        target: 240,
        unlock: "Needs ≥50% coverage · ≥200 reviews",
    });
    expect(gauge.mode).toBe("abstain");
    expect(gauge.low).toBeUndefined();
    expect(gauge.high).toBeUndefined();
    expect(gauge.point).toBeUndefined();
    expect(gauge.unlock).toContain("50%");
});

test("readinessToGauge produces a rounded confident band", () => {
    const gauge = readinessToGauge(scored() as ReadinessScoreView, { target: 245 });
    expect(gauge.mode).toBe("confident");
    expect(gauge).toMatchObject({ low: 239, high: 250, point: 244, target: 245 });
});

test("coverage + stake weight formatting", () => {
    expect(formatCoverage(61.4)).toBe("61%");
    expect(formatStakePoints(9.1)).toBe("9.1 pts");
    expect(formatStakePoints(0)).toBe("\u2014");
});

test("weakness maps to the four clinical acuities", () => {
    expect(acuityFromWeakness(0.82)).toBe("critical");
    expect(acuityFromWeakness(0.5)).toBe("watch");
    expect(acuityFromWeakness(0.3)).toBe("stable");
    expect(acuityFromWeakness(0.1)).toBe("muted");
});

test("formatUpdatedAt is relative, and omits an unknown time", () => {
    expect(formatUpdatedAt(0)).toBeNull();
    const now = 10_000;
    expect(formatUpdatedAt(now - 2 * 60 * 60, now)).toBe("updated 2h ago");
    expect(formatUpdatedAt(now - 30, now)).toBe("updated just now");
});
