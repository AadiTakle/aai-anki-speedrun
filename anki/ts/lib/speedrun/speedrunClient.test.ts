// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { expect, test, vi } from "vitest";

// The adapter is the single seam that maps the backend's 0..1 blueprint
// fractions onto the 0..100 scale the STAT UI is built on (yield thresholds,
// "N pts"). Mock the generated RPC layer so we can assert that seam in
// isolation — a regression guard for the "everything reads 0.0 pts / all
// lower-yield" bug, where the raw fractions were passed through unscaled.
vi.mock("@generated/backend", () => ({
    getPointsAtStake: async () => ({
        topics: [
            { topicId: "cardio", name: "Cardiovascular", blueprintWeight: 0.11, weakness: 0.9, points: 0.099 },
            { topicId: "nutrition", name: "Nutrition", blueprintWeight: 0.01, weakness: 0.5, points: 0.005 },
        ],
    }),
    getCoverageMap: async () => ({
        coveredPct: 0.61,
        sections: [{ topicId: "cardio", name: "Cardiovascular", covered: true, blueprintWeight: 0.11 }],
    }),
    getDailyPlan: async () => ({
        tasks: [
            {
                id: "plan",
                label: "Review Today's focus",
                detail: "today's plan",
                state: 2,
                doneCount: 0,
                totalCount: 0,
            },
            {
                id: "qblock",
                label: "Import your QBank results",
                detail: "no results yet",
                state: 1,
                doneCount: 0,
                totalCount: 0,
            },
            {
                id: "review",
                label: "Clear your review queue",
                detail: "3 / 5 done",
                state: 0,
                doneCount: 3,
                totalCount: 5,
            },
        ],
    }),
    getMemoryScore: async () => ({}),
    getPerformanceScore: async () => ({}),
    getReadinessScore: async () => ({}),
    getNextAction: async () => ({}),
}));

import { getCoverageMap, getDailyPlan, getPointsAtStake } from "./speedrunClient";

test("points-at-stake scales backend 0..1 fractions onto the 0..100 UI scale", async () => {
    const { topics } = await getPointsAtStake();

    const cardio = topics.find((t) => t.topicId === "cardio")!;
    // 0.11 -> 11 (clears the >= 10 "high-yield" threshold), and
    // 0.11 * 0.9 = 0.099 -> 9.9, which renders "9.9 pts" (not "0.1 pts").
    expect(cardio.blueprintWeight).toBeCloseTo(11, 6);
    expect(cardio.points).toBeCloseTo(9.9, 6);
    // Weakness stays a 0..1 fraction (it drives acuity), so it is NOT scaled.
    expect(cardio.weakness).toBeCloseTo(0.9, 6);

    const nutrition = topics.find((t) => t.topicId === "nutrition")!;
    // The old bug: 0.005 rendered as "0.0 pts". Scaled it is 0.5 — small but
    // honestly nonzero, and correctly the lowest-yield topic.
    expect(nutrition.points).toBeCloseTo(0.5, 6);
});

test("coverage sections carry the same 0..100 blueprint-weight scale", async () => {
    const { sections } = await getCoverageMap();
    expect(sections[0].blueprintWeight).toBeCloseTo(11, 6);
});

test("daily plan maps backend enum states to the view's string states", async () => {
    const { tasks } = await getDailyPlan();
    // Backend DailyTask.State {DONE=2, CURRENT=1, UPCOMING=0} -> view strings.
    expect(tasks.map((t) => t.state)).toEqual(["done", "current", "upcoming"]);
    const review = tasks.find((t) => t.id === "review")!;
    expect(review.doneCount).toBe(3);
    expect(review.totalCount).toBe(5);
    expect(review.detail).toBe("3 / 5 done");
});
