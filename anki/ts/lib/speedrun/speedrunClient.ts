// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Typed data-access adapter for the STAT UI — the single seam between the
// screens and the backend. Every score/signal the console needs is exposed as a
// typed async function returning a *View from ./types.
//
// WIRED (integration): every function calls its real @generated/backend RPC and
// maps the proto message into the *View shape, with the seeded persona data kept
// as a FALLBACK (try/catch) so the screens still render standalone — in vitest,
// or in the SvelteKit dev server before the Anki backend is reachable. In the
// running app the real RPC answers, so the fallback never fires and scores are
// live. Readiness honestly ABSTAINS (backend stub) until calibration exists.

import {
    getCoverageMap as backendGetCoverageMap,
    getMemoryScore as backendGetMemoryScore,
    getNextAction as backendGetNextAction,
    getPerformanceScore as backendGetPerformanceScore,
    getPointsAtStake as backendGetPointsAtStake,
    getReadinessScore as backendGetReadinessScore,
} from "@generated/backend";

import { activePersona } from "./personas";
import type {
    CoverageMapView,
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

/** How long ago (seconds) the seeded scores were "computed" — for "updated 2h ago". */
const MOCK_UPDATED_SECONDS_AGO = 2 * 60 * 60;

function nowSeconds(): number {
    return Math.floor(Date.now() / 1000);
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

/** Stamp a seeded score with a fresh relative "updated" time. */
function stamp<T extends ScoreView>(score: T): T {
    return { ...score, updatedAt: nowSeconds() - MOCK_UPDATED_SECONDS_AGO };
}

/**
 * Memory (FSRS recall) score. Wired to the real frozen F6 RPC, with a seeded
 * fallback so the UI renders even when the backend is unavailable.
 */
export async function getMemoryScore(): Promise<MemoryScoreView> {
    try {
        // The frozen F6 RPC takes no arguments (generic.Empty).
        const msg = await backendGetMemoryScore({});
        return toScoreView(msg);
    } catch {
        return stamp(activePersona().memory);
    }
}

/**
 * Performance (QBank application) score with a per-topic breakdown.
 * Seeded until the backend RPC lands.
 */
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
        const persona = activePersona();
        return { ...stamp(persona.performance), topics: persona.performance.topics };
    }
}

/**
 * Readiness score on the ~194..300 scaled-score axis. Honestly ABSTAINS until
 * calibrated (US-2's scenario). Seeded until the backend RPC lands.
 */
export async function getReadinessScore(): Promise<ReadinessScoreView> {
    try {
        // The backend honestly ABSTAINS until calibrated to NBME/UWSA, so a real
        // response keeps readiness in its "not enough info" state (honesty bar).
        return toScoreView(await backendGetReadinessScore({}));
    } catch {
        return stamp(activePersona().readiness);
    }
}

/**
 * Blueprint coverage map + the readiness abstain gate (planned F7 RPC).
 * Seeded until the backend RPC lands.
 */
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
                blueprintWeight: Number(s.blueprintWeight),
            })),
        };
    } catch {
        return activePersona().coverage;
    }
}

/**
 * Ranked points-at-stake topics (blueprint_weight * weakness) for "Today's
 * focus". A read-only view over the F5 signals. Seeded until the RPC is exposed.
 */
export async function getPointsAtStake(): Promise<PointsAtStakeView> {
    try {
        const msg = await backendGetPointsAtStake({});
        return {
            topics: msg.topics.map((t) => ({
                topicId: t.topicId,
                name: t.name,
                blueprintWeight: Number(t.blueprintWeight),
                weakness: Number(t.weakness),
                points: Number(t.points),
            })),
        };
    } catch {
        return { topics: activePersona().pointsAtStake };
    }
}

/**
 * The single imperative "do this now" order (planned next-action recommender).
 * Seeded until the backend RPC lands.
 */
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
        return activePersona().nextAction;
    }
}
