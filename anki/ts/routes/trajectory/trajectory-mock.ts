// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Route-local, ILLUSTRATIVE time-series for the Exam-Trajectory view.
//
// This is NOT live data and does NOT come from the `$lib/speedrun` adapter — the
// shared foundation exposes point-in-time Views (a single readiness snapshot),
// not history. The trajectory needs weeks of observations, so the longitudinal
// series lives here, local to the route, exactly as sanctioned by the lane spec.
// It reproduces the US-1 "Aisha" scenario from the STAT system-design spec §4.
//
// Honesty bar (docs/project_guidelines.md §2): readiness ABSTAINS in the early
// weeks (below 50% blueprint coverage there is no score — nulls, not a guess),
// then reports a narrowing RANGE. The final exam-day point is a PROJECTION, not
// a promise, and is always drawn dashed + labeled as such.

export const TRAJECTORY_SCENARIO = "US-1 Aisha";

/** The weeks-to-exam axis; the final bucket is exam day (the projection). */
export const WEEKS = [
    "W-10",
    "W-9",
    "W-8",
    "W-7",
    "W-6",
    "W-5",
    "W-4",
    "W-3",
    "W-2",
    "Exam",
] as const;

/** Index of "now" — the latest OBSERVED week (matches the Today console). */
export const NOW_INDEX = 8;

/** USMLE scaled-score reference lines. */
export const TARGET = 245;
export const PASS = 214;

/** Readiness un-abstains once blueprint coverage crosses this line (F7 gate). */
export const COVERAGE_UNLOCK_PCT = 50;

export interface ReadinessPoint {
    /** Range bounds + best-estimate point; all null while abstaining. */
    lo: number | null;
    hi: number | null;
    pt: number | null;
    /** True only for the dashed exam-day projection bucket. */
    projection?: boolean;
}

/**
 * Readiness range per week. The first three weeks are null = an honest abstain
 * (coverage < 50%); then a band that narrows toward the target. The last bucket
 * is the exam-day projection.
 */
export const READINESS: ReadinessPoint[] = [
    { lo: null, hi: null, pt: null },
    { lo: null, hi: null, pt: null },
    { lo: null, hi: null, pt: null },
    { lo: 226, hi: 248, pt: 236 },
    { lo: 230, hi: 250, pt: 240 },
    { lo: 233, hi: 250, pt: 241 },
    { lo: 236, hi: 251, pt: 243 },
    { lo: 238, hi: 250, pt: 244 },
    { lo: 239, hi: 250, pt: 244 }, // "now" — matches the Today console (US-1)
    { lo: 242, hi: 252, pt: 247, projection: true },
];

/** Blueprint coverage (%) per week — grows as more of the outline is exposed. */
export const COVERAGE = [12, 20, 31, 39, 46, 52, 55, 58, 61, 66];

/** Mean FSRS memory stability (days) per week — how long recall survives. */
export const STABILITY = [6, 7, 9, 11, 13, 16, 18, 20, 22, 25];

export interface GapTopic {
    topic: string;
    /** FSRS recall probability (%). */
    memory: number;
    /** QBank application accuracy (%). */
    performance: number;
}

/**
 * Memory vs. performance per topic. The gap (recall − application) is the
 * "knows it, can't apply it (yet)" signal that drives the fix-next queue.
 */
export const GAP: GapTopic[] = [
    { topic: "Acid\u2013base", memory: 83, performance: 60 },
    { topic: "Renal", memory: 64, performance: 57 },
    { topic: "Cardiology", memory: 70, performance: 62 },
    { topic: "Endocrine", memory: 88, performance: 86 },
];

export interface RankedGapTopic extends GapTopic {
    /** memory − performance (positive = recall outruns application). */
    gap: number;
}

/** Topics ranked by the biggest memory→performance gap first (fix-next order). */
export function rankGap(topics: readonly GapTopic[] = GAP): RankedGapTopic[] {
    return topics
        .map((t) => ({ ...t, gap: t.memory - t.performance }))
        .sort((a, b) => b.gap - a.gap);
}
