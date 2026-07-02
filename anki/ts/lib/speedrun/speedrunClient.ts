// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Typed data-access adapter for the STAT UI — the single seam between the
// screens and the backend. Every score/signal is a typed async function that
// calls its real @generated/backend RPC and maps the proto message to a *View.
//
// HONESTY: on an RPC error (engine unreachable) each function returns an honest
// "unavailable" view — abstained / empty — NEVER fabricated numbers. In the
// running app the RPC always answers (with real data or an honest abstain), so
// these fallbacks only surface if the backend genuinely can't be reached.

import {
    getCoverageMap as backendGetCoverageMap,
    getDailyPlan as backendGetDailyPlan,
    getMemoryScore as backendGetMemoryScore,
    getNextAction as backendGetNextAction,
    getPerformanceScore as backendGetPerformanceScore,
    getPointsAtStake as backendGetPointsAtStake,
    getReadinessScore as backendGetReadinessScore,
} from "@generated/backend";

import type {
    CoverageMapView,
    DailyPlanView,
    DailyTaskState,
    MemoryScoreView,
    NextActionView,
    PerformanceScoreView,
    PointsAtStakeView,
    ReadinessScoreView,
    ScoreView,
} from "./types";

/** Minimal structural shape shared by the generated score messages we read. */
interface ScoreMessageLike {
    abstained: boolean;
    point: number;
    low: number;
    high: number;
    coveragePct: number;
    reasons: string[];
    updatedAt: bigint | number;
}

/** Normalize a generated score message into the plain-number view shape. */
function toScoreView(msg: ScoreMessageLike): ScoreView {
    return {
        abstained: msg.abstained,
        point: Number(msg.point),
        low: Number(msg.low),
        high: Number(msg.high),
        coveragePct: Number(msg.coveragePct),
        reasons: [...msg.reasons],
        updatedAt: Number(msg.updatedAt),
    };
}

/** Honest "couldn't reach the engine" score — abstained, never a fake number. */
function unavailableScore(): ScoreView {
    return {
        abstained: true,
        point: 0,
        low: 0,
        high: 0,
        coveragePct: 0,
        reasons: ["Score unavailable — couldn't reach the engine."],
        updatedAt: 0,
    };
}

/** Memory (FSRS recall) score. */
export async function getMemoryScore(): Promise<MemoryScoreView> {
    try {
        return toScoreView(await backendGetMemoryScore({}));
    } catch {
        return unavailableScore();
    }
}

/** Performance (QBank application) score with a per-topic breakdown. */
export async function getPerformanceScore(): Promise<PerformanceScoreView> {
    try {
        const msg = await backendGetPerformanceScore({});
        return {
            ...toScoreView(msg),
            topics: msg.topics.map((t) => ({
                topicId: t.topicId,
                attempts: t.attempts,
                correct: t.correct,
                accuracy: Number(t.accuracy),
                low: Number(t.low),
                high: Number(t.high),
            })),
        };
    } catch {
        return { ...unavailableScore(), topics: [] };
    }
}

/**
 * Readiness score on the ~194..300 scaled-score axis. The backend honestly
 * ABSTAINS until calibrated to NBME/UWSA, so a real response keeps readiness in
 * its "not enough info" state (honesty bar).
 */
export async function getReadinessScore(): Promise<ReadinessScoreView> {
    try {
        return toScoreView(await backendGetReadinessScore({}));
    } catch {
        return unavailableScore();
    }
}

/** Blueprint coverage map (F7). */
export async function getCoverageMap(): Promise<CoverageMapView> {
    try {
        const msg = await backendGetCoverageMap({});
        return {
            // Backend covered_pct is a 0..1 blueprint-weighted fraction.
            coveragePct: Number(msg.coveredPct) * 100,
            // Readiness abstains below 50% coverage (design constant, not on the RPC).
            abstainThresholdPct: 50,
            sections: msg.sections.map((s) => ({
                id: s.topicId,
                name: s.name,
                // F7 knows covered / not (has mapped cards), not a gradient — so a
                // section reads 100% or 0%.
                coveragePct: s.covered ? 100 : 0,
                // 0..1 fraction on the RPC -> 0..100 design scale (matches
                // points-at-stake so the same yield thresholds apply).
                blueprintWeight: Number(s.blueprintWeight) * 100,
            })),
        };
    } catch {
        return { coveragePct: 0, abstainThresholdPct: 50, sections: [] };
    }
}

/** Ranked points-at-stake topics (blueprint_weight * weakness) for "Today's focus". */
export async function getPointsAtStake(): Promise<PointsAtStakeView> {
    try {
        const msg = await backendGetPointsAtStake({});
        return {
            topics: msg.topics.map((t) => ({
                topicId: t.topicId,
                name: t.name,
                // Backend blueprint_weight + points are 0..1 fractions (a topic's
                // share of the exam); the STAT UI (yield thresholds, "N pts") is
                // built on the design's 0..100 scale, so scale at this one seam —
                // mirroring how coveredPct is scaled in getCoverageMap.
                blueprintWeight: Number(t.blueprintWeight) * 100,
                weakness: Number(t.weakness),
                points: Number(t.points) * 100,
            })),
        };
    } catch {
        return { topics: [] };
    }
}

// Backend DailyTask.State enum (0/1/2) -> the view's string state.
const DAILY_TASK_STATE: Record<number, DailyTaskState> = {
    0: "upcoming",
    1: "current",
    2: "done",
};

/**
 * The daily loop as a progressing to-do list (Plan -> Q-block -> Review ->
 * Close). Each task's state is derived read-only by the engine from real
 * signals; the frontend just renders it.
 */
export async function getDailyPlan(): Promise<DailyPlanView> {
    try {
        const msg = await backendGetDailyPlan({});
        return {
            tasks: msg.tasks.map((t) => ({
                id: t.id,
                label: t.label,
                detail: t.detail,
                state: DAILY_TASK_STATE[t.state] ?? "upcoming",
                doneCount: t.doneCount,
                totalCount: t.totalCount,
            })),
        };
    } catch {
        return { tasks: [] };
    }
}

/** The single recommended next block (the "do this now" order). */
export async function getNextAction(): Promise<NextActionView> {
    try {
        const msg = await backendGetNextAction({});
        return {
            available: !msg.abstained,
            headline: msg.headline,
            meta: msg.reason,
            topicIds: [...msg.topicIds],
            blockSize: msg.blockSize,
            // The backend recommends a review block of due cards; it does not yet
            // estimate a duration.
            estimatedMinutes: 0,
            mode: msg.abstained ? "none" : "review",
        };
    } catch {
        return {
            available: false,
            headline: "",
            meta: "",
            topicIds: [],
            blockSize: 0,
            estimatedMinutes: 0,
            mode: "none",
        };
    }
}
