// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Route-local seeded data for the STAT targeted-review reviewer SCREEN. This is
// an illustrative mock — NOT the real scheduler and NOT live data. Each card
// stands in for a card the (planned) Rust queue would surface: seeded by a real
// QBank miss, front-loaded by points-at-stake, interleaved across systems.
//
// Nothing here fabricates a readiness/score number (honesty bar): `points` and
// `targetSeconds` are session-shaping metadata for this demo, clearly a mock.

import type { Acuity } from "$lib/speedrun";

/** Body organ system — the axis the session interleaves across. */
export type System =
    | "Renal"
    | "Cardiology"
    | "Endocrine"
    | "Neurology"
    | "Pulmonology";

/** The four Anki grades — KEPT verbatim (FSRS next-states). */
export type Grade = "again" | "hard" | "good" | "easy";

/** FSRS interval labels shown under each grade — Anki's, unchanged. */
export interface GradeIntervals {
    again: string;
    hard: string;
    good: string;
    easy: string;
}

/** Why this card is in front of you (STAT-only provenance line). */
export interface Provenance {
    /** QBank + question id, e.g. "UWorld Q#14". */
    source: string;
    /** Canonical topic, e.g. "Acid–base". */
    topic: string;
    /** Interleave system this card belongs to. */
    system: System;
    /** Relative age of the miss, e.g. "2h ago". */
    ageLabel: string;
}

/** The miss reframe / differential surfaced on the answer side. */
export interface Reframe {
    /** The option the learner actually chose (the miss). */
    wrongAnswer: string;
    /** How the vignette would need to change for the wrong answer to be right. */
    differential: string;
    /** The learner's one-line takeaway (the reasoning artifact). */
    takeaway: string;
}

/** One seeded review card in the targeted session. */
export interface ReviewCard {
    id: string;
    provenance: Provenance;
    /** Points-at-stake acuity — drives the order bar + seed framing. */
    acuity: Acuity;
    /** Blueprint-weighted points at stake (higher = reviewed earlier). */
    points: number;
    /** Question stem; `**bold**` marks the emphasised clause. */
    question: string;
    /** Answer/explanation; `**bold**` marks the punchline. */
    answer: string;
    reframe: Reframe;
    /** Target seconds/question; over this reads as accurate-but-slow (watch). */
    targetSeconds: number;
    intervals: GradeIntervals;
}

const FSRS: GradeIntervals = { again: "<10m", hard: "2d", good: "4d", easy: "9d" };

// Authored already front-loaded by points-at-stake (critical → watch → stable)
// and interleaved so no two adjacent cards share a system, mirroring the timed,
// randomized exam. See STAT-system-design.canvas.tsx §1.
export const SESSION_CARDS: ReviewCard[] = [
    {
        id: "acid-base-winters",
        provenance: { source: "UWorld Q#14", topic: "Acid\u2013base", system: "Renal", ageLabel: "2h ago" },
        acuity: "critical",
        points: 9.1,
        question: "28-year-old in DKA. ABG shows a metabolic acidosis: HCO\u2083\u207b 14 mEq/L, PaCO\u2082 30 mmHg. "
            + "Which PaCO\u2082 confirms **appropriate respiratory compensation**?",
        answer:
            "Winter's formula: expected PaCO\u2082 = 1.5 \u00d7 [HCO\u2083\u207b] + 8 \u00b1 2 = 1.5(14) + 8 = **29 \u00b1 2 mmHg**. "
            + "Measured 30 falls in range \u2192 appropriate respiratory compensation for a pure high-anion-gap "
            + "metabolic acidosis (no second primary process).",
        reframe: {
            wrongAnswer: "primary respiratory alkalosis",
            differential:
                "PaCO\u2082 would have to fall well below Winter's 29 \u00b1 2 (e.g. ~20) \u2014 an independent "
                + "respiratory drive (sepsis, salicylates, anxiety), not compensation.",
            takeaway:
                "If PaCO\u2082 is below Winter's predicted value, suspect a second primary disorder \u2014 run the "
                + "formula before calling it compensation.",
        },
        targetSeconds: 90,
        intervals: FSRS,
    },
    {
        id: "cards-vt-vs-svt",
        provenance: {
            source: "UWorld Q#31",
            topic: "Wide-complex tachycardia",
            system: "Cardiology",
            ageLabel: "2h ago",
        },
        acuity: "critical",
        points: 8.4,
        question: "68-year-old with **prior myocardial infarction**, regular wide-complex tachycardia at 160/min, "
            + "hemodynamically stable. Most likely rhythm?",
        answer: "**Ventricular tachycardia.** Structural heart disease (prior MI) makes VT far more likely than SVT "
            + "with aberrancy \u2014 a regular wide-complex tachycardia is VT until proven otherwise.",
        reframe: {
            wrongAnswer: "SVT with aberrancy",
            differential: "You'd need no structural heart disease plus a rate-dependent bundle-branch morphology (or a "
                + "documented prior identical narrow-complex SVT) \u2014 then aberrancy climbs the differential.",
            takeaway: "Wide-complex tachycardia + structural heart disease = VT until proven otherwise.",
        },
        targetSeconds: 75,
        intervals: FSRS,
    },
    {
        id: "neuro-tpa-window",
        provenance: { source: "UWorld Q#22", topic: "Ischemic stroke", system: "Neurology", ageLabel: "5h ago" },
        acuity: "critical",
        points: 7.8,
        question: "70-year-old with sudden right hemiparesis and aphasia, last known well **3 hours ago**. "
            + "Non-contrast CT shows no hemorrhage. Best next step?",
        answer: "**IV alteplase (tPA).** Within 4.5 h of last-known-well, with hemorrhage excluded and no "
            + "contraindications, thrombolysis is indicated \u2014 then assess for thrombectomy if a large-vessel "
            + "occlusion is present.",
        reframe: {
            wrongAnswer: "aspirin now, admit for observation",
            differential:
                "Push last-known-well beyond 4.5 h (or add a thrombolysis contraindication) \u2014 then aspirin "
                + "\u00b1 thrombectomy selection replaces tPA.",
            takeaway:
                "Inside 4.5 h with no bleed \u2192 tPA; aspirin is for when the window or a contraindication rules "
                + "tPA out.",
        },
        targetSeconds: 80,
        intervals: FSRS,
    },
    {
        id: "renal-thiazide-hypona",
        provenance: { source: "UWorld Q#12", topic: "Hyponatremia", system: "Renal", ageLabel: "2h ago" },
        acuity: "watch",
        points: 5.2,
        question: "72-year-old on **hydrochlorothiazide**. Na\u207a 124, clinically euvolemic, urine osm 420 mOsm/kg, "
            + "urine Na\u207a 45 mEq/L. Best explanation for the hyponatremia?",
        answer: "**Thiazide-induced hyponatremia.** Thiazides impair urinary dilution and can mimic SIADH's labs; "
            + "the drug history is the pivot clue over a primary SIADH cause.",
        reframe: {
            wrongAnswer: "SIADH",
            differential:
                "Remove the thiazide and keep euvolemia with an identifiable ADH stimulus (CNS/pulmonary lesion, "
                + "pain, nausea) and no diuretic \u2014 then SIADH becomes the diagnosis of exclusion.",
            takeaway: "Check the med list first \u2014 a diuretic on board outranks SIADH.",
        },
        targetSeconds: 60,
        intervals: FSRS,
    },
    {
        id: "pulm-ctpa-vs-ddimer",
        provenance: { source: "UWorld Q#08", topic: "Pulmonary embolism", system: "Pulmonology", ageLabel: "5h ago" },
        acuity: "watch",
        points: 4.6,
        question: "Post-op patient with sudden dyspnea and pleuritic chest pain, HR 110, **SpO\u2082 89%**, "
            + "hemodynamically stable. High pretest probability. Best initial diagnostic test?",
        answer: "**CT pulmonary angiography.** With a high pretest probability and a stable patient, CTPA is the "
            + "initial test \u2014 a D-dimer can't rule out PE when pretest probability is high.",
        reframe: {
            wrongAnswer: "D-dimer",
            differential:
                "Lower the pretest probability (Wells low / PERC-negative) \u2014 then a D-dimer is the correct "
                + "rule-out before any imaging.",
            takeaway: "High pretest probability skips the D-dimer and goes straight to CTPA.",
        },
        targetSeconds: 65,
        intervals: FSRS,
    },
    {
        id: "endo-graves",
        provenance: { source: "UWorld Q#15", topic: "Hyperthyroidism", system: "Endocrine", ageLabel: "5h ago" },
        acuity: "stable",
        points: 2.4,
        question: "Woman with palpitations, weight loss, a **diffuse goiter**, and proptosis. TSH low, free T4 high. "
            + "Most likely diagnosis?",
        answer: "**Graves disease.** Diffuse goiter + ophthalmopathy + low TSH / high T4 points to autoimmune (TSI) "
            + "hyperthyroidism; confirm with elevated TSI or diffuse uptake on RAIU.",
        reframe: {
            wrongAnswer: "toxic multinodular goiter",
            differential:
                "Swap the diffuse goiter for a nodular gland, drop the ophthalmopathy (Graves-specific), and show "
                + "patchy uptake on RAIU \u2014 then toxic MNG fits.",
            takeaway: "Ophthalmopathy + diffuse goiter is Graves; nodular gland with patchy uptake is toxic MNG.",
        },
        targetSeconds: 70,
        intervals: FSRS,
    },
];

// -------------------------------------------------------------------- helpers
// Pure, framework-free — safe to unit-test and reuse in the HUD/card face.

/** One inline text run; `strong` renders bold. */
export interface TextRun {
    text: string;
    strong: boolean;
}

/**
 * Split a `**bold**`-marked string into inline runs (odd segments are bold).
 * Lets the seeded copy stay readable without `{@html}` on trusted-but-static
 * strings.
 */
export function toRuns(source: string): TextRun[] {
    return source.split("**").map((text, i) => ({ text, strong: i % 2 === 1 }));
}

/** Distinct interleave systems in queue order (first appearance wins). */
export function sessionSystems(cards: readonly ReviewCard[]): System[] {
    const seen: System[] = [];
    for (const card of cards) {
        if (!seen.includes(card.provenance.system)) {
            seen.push(card.provenance.system);
        }
    }
    return seen;
}

/** The reframe prompt, generated from the miss so copy stays DRY. */
export function reframePrompt(card: ReviewCard): string {
    return `How would the vignette need to change for "${card.reframe.wrongAnswer}" to be right?`;
}

/**
 * Pacing acuity for the live per-card timer: stable while under the target,
 * watch once it crosses (accurate-but-slow). Never "critical" — being slow is a
 * pacing signal, not a miss.
 */
export function pacingAcuity(elapsedSeconds: number, targetSeconds: number): Acuity {
    return elapsedSeconds > targetSeconds ? "watch" : "stable";
}
