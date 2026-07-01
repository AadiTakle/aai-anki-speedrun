// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Pure display logic for the F6 memory score. Maps the frozen
// `anki.speedrun.MemoryScore` contract onto honest, render-ready fields.
//
// Honesty bar (docs/project_guidelines.md §2): when the engine abstains there is
// NO real number — point/low/high are unset — so the UI must show a plain "not
// enough data yet" state plus the reasons, and MUST NOT fabricate a score. When
// scored, the memory score is always shown as a RANGE (low–high), never a single
// blended number.

// U+2013 EN DASH, used to join the low and high bounds of the range.
const EN_DASH = "\u2013";

/**
 * The subset of `anki.speedrun.MemoryScore` this module needs. The generated
 * proto message is structurally assignable to this, so callers can pass the RPC
 * result directly (see the contract-fidelity test).
 */
export interface MemoryScoreLike {
    abstained: boolean;
    point: number;
    low: number;
    high: number;
    coveragePct: number;
    reasons: string[];
}

export type MemoryScoreMode = "abstained" | "scored";

export interface MemoryScoreDisplay {
    mode: MemoryScoreMode;
    /** Big headline: the range when scored, an honest note when abstaining. */
    headline: string;
    /** "low–high" when scored; null when abstaining (no fabricated number). */
    rangeLabel: string | null;
    /** Rounded point estimate when scored; null when abstaining. */
    point: number | null;
    /** Rounded blueprint coverage, always shown. */
    coveragePct: number;
    /** Coverage formatted for display, e.g. "64%". */
    coverageLabel: string;
    /** Human-readable reasons behind the score / abstention. */
    reasons: string[];
}

const ABSTAIN_HEADLINE = "Not enough data yet";

export function memoryScoreDisplay(score: MemoryScoreLike): MemoryScoreDisplay {
    const coveragePct = Math.round(score.coveragePct);
    const coverageLabel = `${coveragePct}%`;
    // Copy so callers can't observe shared mutable state.
    const reasons = [...score.reasons];

    if (score.abstained) {
        return {
            mode: "abstained",
            headline: ABSTAIN_HEADLINE,
            rangeLabel: null,
            point: null,
            coveragePct,
            coverageLabel,
            reasons,
        };
    }

    const low = Math.round(score.low);
    const high = Math.round(score.high);
    const rangeLabel = `${low}${EN_DASH}${high}`;

    return {
        mode: "scored",
        headline: rangeLabel,
        rangeLabel,
        point: Math.round(score.point),
        coveragePct,
        coverageLabel,
        reasons,
    };
}
