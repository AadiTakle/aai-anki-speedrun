// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// STAT design tokens (TypeScript mirror of ./_tokens.scss).
//
// The clinical system from the STAT design canvases: ink-on-paper + one teal,
// with signal colors used ONLY to encode a real clinical/product state
// (acuity, pacing, abstain) — never as decoration. Keep the two in sync; the
// SCSS partial is the source of truth for component styling, this module is for
// values that must be chosen in JS (e.g. SVG geometry, charts in screen lanes).

import type { Acuity } from "./types";

/** Neutral field: ink-on-paper + the one confident teal accent. */
export const STAT_COLORS = {
    /** Clinical teal — the one confident accent. */
    primary: "#0E6E62",
    /** ~5% teal wash for the primary-action panel background. */
    primaryWash: "#0E6E620D",
    /** Charting ink (blue-green near-black) — default text. */
    ink: "#16241F",
    /** Muted ink — secondary text / labels. */
    inkSoft: "#5A6B64",
    /** Cool clinical off-white (not cream) — window surface. */
    paper: "#F4F6F4",
    /** Container surface. */
    surface: "#FFFFFF",
    /** Ruled-paper hairline. */
    line: "#D7DED8",
} as const;

/** Signal palette — encodes clinical/product STATE only, never decoration. */
export const STAT_SIGNAL: Record<Acuity, string> = {
    /** Urgent / high points-at-stake / miss. */
    critical: "#B23A25",
    /** Pacing: accurate-but-slow. */
    watch: "#B26B12",
    /** In-range / strong / correct. */
    stable: "#2F7D53",
    /** Abstain / paused / insufficient. */
    muted: "#8A938F",
} as const;

/** ~5% washes for signal-tinted panels (critical) and the teal order card. */
export const STAT_WASH = {
    critical: "#B23A250D",
    primary: "#0E6E620D",
} as const;

/**
 * Font families, referenced by family name with graceful system fallback (no
 * @font-face, no external fetch). DISPLAY = wordmark + directives, BODY = UI +
 * reading copy, MONO = the "readout" voice for every score/range/timer/label.
 */
export const STAT_FONTS = {
    display: "'Archivo Expanded', 'Archivo Expanded Variable', 'Archivo', ui-sans-serif, system-ui, sans-serif",
    body: "'IBM Plex Sans', ui-sans-serif, system-ui, sans-serif",
    mono: "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
} as const;

/** Small radii — a lab-requisition feel (6px cards, 12px panels, pills). */
export const STAT_RADIUS = {
    sm: "6px",
    md: "12px",
    pill: "999px",
} as const;

/** The USMLE scaled-score axis the Readiness gauge is drawn on. */
export const STAT_SCALE = {
    /** Axis minimum. */
    min: 194,
    /** Axis maximum. */
    max: 300,
    /** Passing scaled score (dashed marker). */
    pass: 214,
} as const;

/** Resolve a signal hex from an acuity. */
export function acuityColor(acuity: Acuity): string {
    return STAT_SIGNAL[acuity];
}
