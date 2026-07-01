// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Seeded scenario data for the four PRD personas (US-1 Aisha, US-2 Marcus,
// US-3 Priya, US-4 Ben), reused verbatim from the STAT design canvases. This is
// the data the mock adapter (./speedrunClient) serves while the real backend
// RPCs are built concurrently — it is NOT live data. US-2 is the honest-abstain
// scenario (readiness abstains below 50% coverage / 200 reviews).
//
// Every `points` value equals `blueprintWeight * weakness` (rounded), so the
// ranking is internally consistent and reproduces the design's acuity split via
// `acuityFromWeakness()` in ./display.

import type {
    CoverageMapView,
    MemoryScoreView,
    NextActionView,
    PerformanceScoreView,
    PointsAtStakeTopicView,
    ReadinessScoreView,
} from "./types";

export type PersonaId = "US-1" | "US-2" | "US-3" | "US-4";

export interface Persona {
    id: PersonaId;
    name: string;
    who: string;
    memory: MemoryScoreView;
    performance: PerformanceScoreView;
    readiness: ReadinessScoreView;
    coverage: CoverageMapView;
    pointsAtStake: PointsAtStakeTopicView[];
    nextAction: NextActionView;
    /** Target scaled score + the readiness abstain unlock rule (design copy). */
    readinessTarget: number;
    readinessUnlock?: string;
}

const round1 = (n: number): number => Math.round(n * 10) / 10;

/** Build a proto-shaped points-at-stake topic with a consistent `points`. */
function stake(
    topicId: string,
    name: string,
    blueprintWeight: number,
    weakness: number,
): PointsAtStakeTopicView {
    return { topicId, name, blueprintWeight, weakness, points: round1(blueprintWeight * weakness) };
}

const ABSTAIN_SCORE = { point: 0, low: 0, high: 0 } as const;

export const PERSONAS: Record<PersonaId, Persona> = {
    "US-1": {
        id: "US-1",
        name: "Aisha Rahman",
        who: "IMG · dedicated period · Desktop",
        memory: {
            abstained: false,
            point: 71,
            low: 68,
            high: 74,
            coveragePct: 61,
            reasons: ["1,240 cards reviewed across the blueprint", "FSRS recall steady on covered topics"],
            updatedAt: 0,
        },
        performance: {
            abstained: false,
            point: 63,
            low: 60,
            high: 66,
            coveragePct: 61,
            reasons: ["612 QBank questions graded", "Application still trails recall on high-yield topics"],
            updatedAt: 0,
            topics: [
                { topicId: "renal", attempts: 84, correct: 48, accuracy: 0.57, low: 0.5, high: 0.64 },
                { topicId: "cardiology", attempts: 92, correct: 57, accuracy: 0.62, low: 0.55, high: 0.69 },
                { topicId: "acid-base", attempts: 40, correct: 24, accuracy: 0.6, low: 0.5, high: 0.7 },
                { topicId: "endocrine", attempts: 70, correct: 60, accuracy: 0.86, low: 0.8, high: 0.91 },
            ],
        },
        readiness: {
            abstained: false,
            point: 244,
            low: 239,
            high: 250,
            coveragePct: 61,
            reasons: ["Only 61% of the blueprint is covered", "Band stays wide until more QBank data lands"],
            updatedAt: 0,
        },
        readinessTarget: 245,
        coverage: {
            coveragePct: 61,
            abstainThresholdPct: 50,
            sections: [
                { id: "renal", name: "Renal", coveragePct: 55, blueprintWeight: 11.7 },
                { id: "cardiology", name: "Cardiology", coveragePct: 60, blueprintWeight: 11.1 },
                { id: "acid-base", name: "Acid\u2013base", coveragePct: 70, blueprintWeight: 4.0 },
                { id: "endocrine", name: "Endocrine", coveragePct: 80, blueprintWeight: 6.0 },
            ],
        },
        pointsAtStake: [
            stake("renal", "Renal", 11.7, 0.78),
            stake("cardiology", "Cardiology", 11.1, 0.76),
            stake("endocrine", "Endocrine", 6.0, 0.12),
        ],
        nextAction: {
            available: true,
            headline: "Start a 40-question UWorld block \u2014 timed, randomized.",
            meta: "Weighted to Renal + Cardiology \u00b7 ~60 min",
            topicIds: ["renal", "cardiology"],
            blockSize: 40,
            estimatedMinutes: 60,
            mode: "qblock",
        },
    },

    "US-2": {
        id: "US-2",
        name: "Marcus Bell",
        who: "DO · double-testing · on rotations · Desktop \u2192 Phone",
        memory: {
            abstained: false,
            point: 59,
            low: 55,
            high: 63,
            coveragePct: 38,
            reasons: ["Partial AnKing imported", "Recall thin outside Internal Medicine"],
            updatedAt: 0,
        },
        performance: {
            abstained: false,
            point: 54,
            low: 50,
            high: 58,
            coveragePct: 38,
            reasons: ["UWorld history imported", "Most topics still below 30 attempts"],
            updatedAt: 0,
            topics: [
                { topicId: "internal-medicine", attempts: 60, correct: 33, accuracy: 0.54, low: 0.45, high: 0.63 },
                { topicId: "gi", attempts: 28, correct: 15, accuracy: 0.52, low: 0.4, high: 0.64 },
                { topicId: "cardiology", attempts: 18, correct: 11, accuracy: 0.58, low: 0.42, high: 0.73 },
            ],
        },
        // The honest abstain: below 50% coverage / 200 reviews there is no score.
        readiness: {
            abstained: true,
            ...ABSTAIN_SCORE,
            coveragePct: 38,
            reasons: ["Needs \u226550% coverage \u2014 at 38%", "Needs \u2265200 graded reviews \u2014 at 120"],
            updatedAt: 0,
        },
        readinessTarget: 240,
        readinessUnlock: "Needs \u226550% coverage \u00b7 \u2265200 reviews (at 38% \u00b7 120)",
        coverage: {
            coveragePct: 38,
            abstainThresholdPct: 50,
            sections: [
                { id: "internal-medicine", name: "Internal Medicine", coveragePct: 52, blueprintWeight: 15.0 },
                { id: "gi", name: "GI", coveragePct: 40, blueprintWeight: 8.0 },
                { id: "cardiology", name: "Cardiology", coveragePct: 30, blueprintWeight: 12.0 },
                { id: "pediatrics", name: "Pediatrics", coveragePct: 0, blueprintWeight: 9.0 },
            ],
        },
        pointsAtStake: [
            stake("internal-medicine", "Internal Medicine", 15.0, 0.8),
            stake("pediatrics", "Pediatrics", 9.0, 0.8),
            stake("cardiology", "Cardiology", 12.0, 0.5),
        ],
        nextAction: {
            available: true,
            headline: "Do a 15-question shelf-mode block \u2014 Internal Medicine.",
            meta: "IM is 55\u201365% of the exam \u00b7 ~20 min",
            topicIds: ["internal-medicine"],
            blockSize: 15,
            estimatedMinutes: 20,
            mode: "qblock",
        },
    },

    "US-3": {
        id: "US-3",
        name: "Priya Nair",
        who: "US-MD · ADHD / learning difference · Desktop / Phone",
        memory: {
            abstained: false,
            point: 77,
            low: 74,
            high: 80,
            coveragePct: 72,
            reasons: ["2,050 cards reviewed", "Strong, durable recall on most systems"],
            updatedAt: 0,
        },
        performance: {
            abstained: false,
            point: 73,
            low: 70,
            high: 76,
            coveragePct: 72,
            reasons: ["1,180 QBank questions graded", "Pacing drags Biostatistics despite accuracy"],
            updatedAt: 0,
            topics: [
                { topicId: "neurology", attempts: 96, correct: 63, accuracy: 0.66, low: 0.59, high: 0.72 },
                { topicId: "renal", attempts: 88, correct: 62, accuracy: 0.7, low: 0.63, high: 0.77 },
                { topicId: "biostatistics", attempts: 40, correct: 32, accuracy: 0.8, low: 0.68, high: 0.89 },
            ],
        },
        readiness: {
            abstained: false,
            point: 251,
            low: 246,
            high: 256,
            coveragePct: 72,
            reasons: ["72% coverage", "Held back by accurate-but-slow topics (Biostatistics)"],
            updatedAt: 0,
        },
        readinessTarget: 250,
        coverage: {
            coveragePct: 72,
            abstainThresholdPct: 50,
            sections: [
                { id: "neurology", name: "Neurology", coveragePct: 68, blueprintWeight: 10.0 },
                { id: "renal", name: "Renal", coveragePct: 74, blueprintWeight: 8.0 },
                { id: "biostatistics", name: "Biostatistics", coveragePct: 80, blueprintWeight: 4.0 },
                { id: "cardiology", name: "Cardiology", coveragePct: 78, blueprintWeight: 9.0 },
            ],
        },
        pointsAtStake: [
            stake("neurology", "Neurology", 10.0, 0.78),
            stake("biostatistics", "Biostatistics", 8.2, 0.5),
            stake("cardiology", "Cardiology", 9.0, 0.12),
        ],
        nextAction: {
            available: true,
            headline: "One block now: 20 interleaved questions, seeded by yesterday's misses.",
            meta: "One decision \u00b7 ~25 min",
            topicIds: ["neurology", "renal"],
            blockSize: 20,
            estimatedMinutes: 25,
            mode: "qblock",
        },
    },

    "US-4": {
        id: "US-4",
        name: "Ben Carter",
        who: "US-MD · secondary · mid-clerkship · Phone-first",
        memory: {
            abstained: false,
            point: 70,
            low: 66,
            high: 73,
            coveragePct: 49,
            reasons: ["Building coverage during clerkships", "Recall solid on the current rotation"],
            updatedAt: 0,
        },
        performance: {
            abstained: false,
            point: 66,
            low: 62,
            high: 70,
            coveragePct: 49,
            reasons: ["430 QBank questions graded", "Longitudinal prep \u2014 not the dedicated period yet"],
            updatedAt: 0,
            topics: [
                { topicId: "internal-medicine", attempts: 74, correct: 47, accuracy: 0.63, low: 0.56, high: 0.7 },
                { topicId: "surgery", attempts: 52, correct: 30, accuracy: 0.58, low: 0.49, high: 0.67 },
                { topicId: "neurology", attempts: 44, correct: 29, accuracy: 0.66, low: 0.56, high: 0.75 },
            ],
        },
        readiness: {
            abstained: false,
            point: 240,
            low: 233,
            high: 248,
            coveragePct: 49,
            reasons: ["49% coverage and growing", "Longitudinal prep \u2014 not the dedicated period yet"],
            updatedAt: 0,
        },
        readinessTarget: 245,
        coverage: {
            coveragePct: 49,
            abstainThresholdPct: 50,
            sections: [
                { id: "internal-medicine", name: "Internal Medicine", coveragePct: 58, blueprintWeight: 14.0 },
                { id: "surgery", name: "Surgery", coveragePct: 44, blueprintWeight: 11.0 },
                { id: "neurology", name: "Neurology", coveragePct: 46, blueprintWeight: 14.9 },
            ],
        },
        pointsAtStake: [
            stake("internal-medicine", "Internal Medicine", 14.0, 0.75),
            stake("surgery", "Surgery", 11.0, 0.6),
            stake("neurology", "Neurology", 14.9, 0.35),
        ],
        nextAction: {
            available: true,
            headline: "Shelf-mode micro-block: 15 questions \u2014 current rotation.",
            meta: "On your phone \u00b7 hospital downtime \u00b7 ~18 min",
            topicIds: ["surgery"],
            blockSize: 15,
            estimatedMinutes: 18,
            mode: "qblock",
        },
    },
};

export const DEFAULT_PERSONA: PersonaId = "US-1";

function isPersonaId(value: string | null): value is PersonaId {
    return value === "US-1" || value === "US-2" || value === "US-3" || value === "US-4";
}

let override: PersonaId | null = null;

/**
 * Force the mock persona (for demos / tests). Screen lanes can also select a
 * persona at runtime via a `?persona=US-2` query param.
 */
export function setMockPersona(id: PersonaId): void {
    override = id;
}

/** The currently selected mock persona id (override > ?persona= > default). */
export function activePersonaId(): PersonaId {
    if (override) {
        return override;
    }
    if (typeof window !== "undefined") {
        const fromQuery = new URLSearchParams(window.location.search).get("persona");
        if (isPersonaId(fromQuery)) {
            return fromQuery;
        }
    }
    return DEFAULT_PERSONA;
}

/** The currently selected mock persona's seeded data. */
export function activePersona(): Persona {
    return PERSONAS[activePersonaId()];
}
