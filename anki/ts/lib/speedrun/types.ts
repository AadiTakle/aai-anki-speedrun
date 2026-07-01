// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Shared TypeScript types for the STAT (Step 2 CK "Speedrun") front-end
// foundation. The *View interfaces mirror the planned `anki.speedrun.*` proto
// messages (see anki/proto/anki/speedrun.proto + the build-readiness canvas) so
// that the seam between the mock adapter and the real generated client is a
// one-line change. Numbers are carried in plain JS types (int64 -> number
// seconds) so screen lanes never have to reason about protobuf `bigint`.
//
// Honesty bar (docs/project_guidelines.md §2): every score is shown as a RANGE
// or an explicit abstain — never a single blended number. `abstained === true`
// means point/low/high are NOT real; the UI must not fabricate a value.

/** Clinical signal state. Encodes acuity/pacing/abstain — never decoration. */
export type Acuity = "critical" | "watch" | "stable" | "muted";

/**
 * The honesty-bar score shape shared by Memory / Performance / Readiness
 * (mirrors `MemoryScore` / `PerformanceScore` / `ReadinessScore`). When
 * `abstained` is true, `point`/`low`/`high` are meaningless and must not be
 * rendered as a number.
 */
export interface ScoreView {
    abstained: boolean;
    point: number;
    low: number;
    high: number;
    /** Percent of the exam blueprint covered so far (0..100). */
    coveragePct: number;
    /** Human-readable reasons behind the number / abstention. */
    reasons: string[];
    /** Unix seconds when the score was last computed; 0 = unknown. */
    updatedAt: number;
}

/** Memory (FSRS recall) score — `anki.speedrun.MemoryScore`. */
export type MemoryScoreView = ScoreView;

/** Readiness score on the ~194..300 scaled-score axis — `ReadinessScore`. */
export type ReadinessScoreView = ScoreView;

/** Per-topic application accuracy — `anki.speedrun.TopicPerformance`. */
export interface TopicPerformanceView {
    topicId: string;
    attempts: number;
    correct: number;
    /** Posterior mean accuracy (0..1) and its credible band. */
    accuracy: number;
    low: number;
    high: number;
}

/** Performance (QBank application) score — `anki.speedrun.PerformanceScore`. */
export interface PerformanceScoreView extends ScoreView {
    topics: TopicPerformanceView[];
}

/** One topic in the "Today's focus" ranked list — `PointsAtStakeTopic`. */
export interface PointsAtStakeTopicView {
    topicId: string;
    name: string;
    /** Relative weight of this topic on the exam blueprint (>= 0). */
    blueprintWeight: number;
    /** 0.0 = strong, 1.0 = weak. */
    weakness: number;
    /** blueprintWeight * weakness — the ranking key (higher = study first). */
    points: number;
}

/** Ranked points-at-stake list — `anki.speedrun.PointsAtStakeResponse`. */
export interface PointsAtStakeView {
    /** Highest points-at-stake first. */
    topics: PointsAtStakeTopicView[];
}

/**
 * One blueprint section's coverage. Mirrors the planned F7 coverage-map RPC
 * (not yet in the proto — see the build-readiness canvas).
 */
export interface CoverageSectionView {
    id: string;
    name: string;
    coveragePct: number;
    blueprintWeight: number;
}

/** Blueprint coverage map + the abstain gate for readiness (F7). */
export interface CoverageMapView {
    /** Overall percent of the official Step 2 outline covered (0..100). */
    coveragePct: number;
    /** Readiness abstains below this coverage (design: 50%). */
    abstainThresholdPct: number;
    sections: CoverageSectionView[];
}

/** The single imperative "do this now" order (the planned next-action RPC). */
export interface NextActionView {
    /** False when there is no confident next move (renders a muted rest state). */
    available: boolean;
    /** Imperative headline, e.g. "Start a 40-question UWorld block". */
    headline: string;
    /** Supporting meta line, e.g. "Weighted to Renal + Cardiology · ~60 min". */
    meta: string;
    /** Canonical topic ids this action targets. */
    topicIds: string[];
    /** Number of questions/cards in the block (0 = not applicable). */
    blockSize: number;
    /** Rough duration estimate in minutes (0 = unknown). */
    estimatedMinutes: number;
    mode: "qblock" | "review" | "none";
}

// --------------------------------------------------------------------------
// Presentational view models (consumed directly by the shared components).
// --------------------------------------------------------------------------

export type GaugeMode = "confident" | "abstain";

/**
 * Everything `ReadinessGauge` needs to draw itself. Derive it from a
 * `ReadinessScoreView` with `readinessToGauge()` in ./display.
 */
export interface GaugeView {
    mode: GaugeMode;
    /** Confident band bounds + best-estimate point (scaled score). */
    low?: number;
    high?: number;
    point?: number;
    /** Target scaled score marker (e.g. 245). */
    target?: number;
    /** Abstain-state unlock rule, e.g. "Needs ≥50% coverage · ≥200 reviews". */
    unlock?: string;
}

/** Props for a single `PointsAtStakeRow` (richer than the proto row). */
export interface StakeView {
    topic: string;
    reason: string;
    /** Point weight; a number renders as "N pts", null/"—" renders muted. */
    weight?: number | string | null;
    acuity: Acuity;
    /** Optional trailing badge, e.g. "Paused" / "Untouched". */
    note?: string;
}

/** One step in the daily-loop pathway strip. */
export interface LoopStep {
    label: string;
    caption?: string;
}

/** The five STAT destinations that make up the daily loop. */
export type DestinationId = "today" | "reviewer" | "import" | "errors" | "trajectory";

export interface Destination {
    id: DestinationId;
    /** Nav label, e.g. "Today". */
    label: string;
    /** SPA route, e.g. "/today". */
    href: string;
    /** The loop step it owns, e.g. "plan". */
    caption: string;
}
