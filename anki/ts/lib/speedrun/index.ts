// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Barrel for the shared STAT front-end foundation. Screen lanes import from
// here, e.g.:
//
//     import { AppShell, ReadinessGauge, getReadinessScore, readinessToGauge } from "$lib/speedrun";
//
// Component styling reads tokens from "$lib/speedrun/tokens" (the SCSS partial).

// Design tokens (TS mirror of _tokens.scss).
export * from "./tokens";

// Shared types (mirror the planned anki.speedrun.* proto messages).
export * from "./types";

// Pure display / formatting helpers (honesty-bar range + abstain logic).
export * from "./display";

// The five destinations that make up the daily loop.
export * from "./nav";

// Typed data-access adapter (mock-backed, single swap seam per function).
export {
    getCoverageMap,
    getMemoryScore,
    getNextAction,
    getPerformanceScore,
    getPointsAtStake,
    getReadinessScore,
} from "./speedrunClient";

// Seeded persona scenarios (US-1..US-4) behind the mock adapter.
export { activePersona, activePersonaId, DEFAULT_PERSONA, PERSONAS, setMockPersona } from "./personas";
export type { Persona, PersonaId } from "./personas";

// Shared Svelte components.
export { default as AppShell } from "./AppShell.svelte";
export { default as Chip } from "./Chip.svelte";
export { default as ConfidenceChip } from "./ConfidenceChip.svelte";
export { default as LoopPathway } from "./LoopPathway.svelte";
export { default as PointsAtStakeRow } from "./PointsAtStakeRow.svelte";
export { default as ReadinessGauge } from "./ReadinessGauge.svelte";
export { default as StatusDot } from "./StatusDot.svelte";
export { default as VitalCard } from "./VitalCard.svelte";
export { default as VitalReadout } from "./VitalReadout.svelte";
