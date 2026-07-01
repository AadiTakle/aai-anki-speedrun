// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { MemoryScore } from "@generated/anki/speedrun_pb";
import { expect, test } from "vitest";

import { memoryScoreDisplay, type MemoryScoreLike } from "./lib";

// U+2013 EN DASH, used to join the low/high bounds of the range.
const EN_DASH = "\u2013";

function abstainedScore(overrides: Partial<MemoryScoreLike> = {}): MemoryScoreLike {
    return {
        abstained: true,
        // Per the frozen F6 contract, point/low/high are unset (0) when abstaining.
        point: 0,
        low: 0,
        high: 0,
        coveragePct: 12.5,
        reasons: [
            "Only 3% of the exam blueprint reviewed",
            "No practice test recorded yet",
        ],
        ...overrides,
    };
}

function scoredScore(overrides: Partial<MemoryScoreLike> = {}): MemoryScoreLike {
    return {
        abstained: false,
        point: 76,
        low: 72,
        high: 81,
        coveragePct: 64,
        reasons: ["1,240 cards reviewed", "2 practice tests taken"],
        ...overrides,
    };
}

test("abstained score renders an honest 'no number yet' state", () => {
    const display = memoryScoreDisplay(abstainedScore());

    expect(display.mode).toBe("abstained");
    expect(display.headline).toBe("Not enough data yet");

    // Honesty bar: NEVER surface a fabricated number when abstaining.
    expect(display.rangeLabel).toBeNull();
    expect(display.point).toBeNull();
    expect(/\d/.test(display.headline)).toBe(false);

    // Reasons must be present and preserved verbatim.
    expect(display.reasons.length).toBeGreaterThan(0);
    expect(display.reasons).toEqual([
        "Only 3% of the exam blueprint reviewed",
        "No practice test recorded yet",
    ]);

    // Coverage is still reported even while abstaining.
    expect(display.coveragePct).toBe(13);
    expect(display.coverageLabel).toBe("13%");
});

test("scored score renders a low-high range, not a single blended number", () => {
    const display = memoryScoreDisplay(scoredScore());

    expect(display.mode).toBe("scored");
    expect(display.rangeLabel).toBe(`72${EN_DASH}81`);
    expect(display.headline).toBe(`72${EN_DASH}81`);

    // The point estimate must land inside the reported range.
    expect(display.point).toBe(76);
    expect(display.point!).toBeGreaterThanOrEqual(72);
    expect(display.point!).toBeLessThanOrEqual(81);

    expect(display.coveragePct).toBe(64);
    expect(display.coverageLabel).toBe("64%");
    expect(display.reasons).toEqual(["1,240 cards reviewed", "2 practice tests taken"]);
});

test("scored values are rounded for display", () => {
    const display = memoryScoreDisplay(
        scoredScore({ point: 76.4, low: 71.5, high: 80.5, coveragePct: 42.4 }),
    );

    expect(display.rangeLabel).toBe(`72${EN_DASH}81`);
    expect(display.point).toBe(76);
    expect(display.coveragePct).toBe(42);
    expect(display.coverageLabel).toBe("42%");
});

test("a degenerate zero-width range is still shown honestly", () => {
    const display = memoryScoreDisplay(scoredScore({ point: 50, low: 50, high: 50 }));

    expect(display.mode).toBe("scored");
    expect(display.rangeLabel).toBe(`50${EN_DASH}50`);
    expect(display.point).toBe(50);
    expect(display.point!).toBeGreaterThanOrEqual(50);
    expect(display.point!).toBeLessThanOrEqual(50);
});

test("accepts a real generated MemoryScore message (contract fidelity)", () => {
    // Building from the generated class proves memoryScoreDisplay consumes the
    // exact frozen F6 contract shape, not just an ad-hoc object.
    const scored = new MemoryScore({
        abstained: false,
        point: 76,
        low: 72,
        high: 81,
        coveragePct: 64,
        reasons: ["from the generated proto"],
        updatedAt: 0n,
    });
    const scoredDisplay = memoryScoreDisplay(scored);
    expect(scoredDisplay.mode).toBe("scored");
    expect(scoredDisplay.rangeLabel).toBe(`72${EN_DASH}81`);

    const abstained = new MemoryScore({
        abstained: true,
        coveragePct: 5,
        reasons: ["insufficient data"],
        updatedAt: 0n,
    });
    const abstainedDisplay = memoryScoreDisplay(abstained);
    expect(abstainedDisplay.mode).toBe("abstained");
    expect(abstainedDisplay.rangeLabel).toBeNull();
    expect(abstainedDisplay.point).toBeNull();
    expect(abstainedDisplay.reasons).toEqual(["insufficient data"]);
});
