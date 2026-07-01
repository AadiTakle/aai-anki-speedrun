// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Typed data-access adapter for the STAT UI — the single seam between the
// screens and the backend. Every score/signal the console needs is exposed as a
// typed async function returning a *View from ./types.
//
// DECOUPLING (deliberate): the backend RPCs for performance / readiness /
// coverage / points-at-stake / next-action are being implemented concurrently
// by another agent. This module MUST NOT import proto or generated symbols that
// don't exist yet. So those functions are backed by SEEDED persona data behind
// the stable interface, each with a single, clearly-marked swap seam. Wiring to
// the real client later is a one-line change:
//
//     // TODO(swap): replace mock with `getPerformanceScore` from "@generated/backend"
//     const msg = await getPerformanceScore({});
//     return toPerformanceView(msg);
//
// `getMemoryScore` ALREADY exists in @generated/backend (frozen F6 RPC), so it
// is wired for real here, with a mock fallback so the foundation still compiles
// and renders standalone (e.g. in tests, or before the backend is running).

import { getMemoryScore as backendGetMemoryScore } from "@generated/backend";

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
    // TODO(swap): replace mock with `getPerformanceScore` from "@generated/backend":
    //   const msg = await getPerformanceScore({});
    //   return { ...toScoreView(msg), topics: msg.topics };
    const persona = activePersona();
    return { ...stamp(persona.performance), topics: persona.performance.topics };
}

/**
 * Readiness score on the ~194..300 scaled-score axis. Honestly ABSTAINS until
 * calibrated (US-2's scenario). Seeded until the backend RPC lands.
 */
export async function getReadinessScore(): Promise<ReadinessScoreView> {
    // TODO(swap): replace mock with `getReadinessScore` from "@generated/backend":
    //   const msg = await getReadinessScore({});
    //   return toScoreView(msg);
    return stamp(activePersona().readiness);
}

/**
 * Blueprint coverage map + the readiness abstain gate (planned F7 RPC).
 * Seeded until the backend RPC lands.
 */
export async function getCoverageMap(): Promise<CoverageMapView> {
    // TODO(swap): replace mock with the F7 coverage-map RPC from "@generated/backend".
    return activePersona().coverage;
}

/**
 * Ranked points-at-stake topics (blueprint_weight * weakness) for "Today's
 * focus". A read-only view over the F5 signals. Seeded until the RPC is exposed.
 */
export async function getPointsAtStake(): Promise<PointsAtStakeView> {
    // TODO(swap): replace mock with `getPointsAtStake` from "@generated/backend":
    //   return await getPointsAtStake({});
    return { topics: activePersona().pointsAtStake };
}

/**
 * The single imperative "do this now" order (planned next-action recommender).
 * Seeded until the backend RPC lands.
 */
export async function getNextAction(): Promise<NextActionView> {
    // TODO(swap): replace mock with the next-action recommender RPC from "@generated/backend".
    return activePersona().nextAction;
}
