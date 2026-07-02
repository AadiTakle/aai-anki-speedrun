// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Route-local mock for the STAT Error Log / Differential screen (C3).
// ILLUSTRATIVE SCENARIO DATA ONLY — no live data, no fetch. Stands in for the
// error-log RPC being built concurrently; the screen consumes it exactly as it
// will consume the real response.
//
// Each miss is a reasoning artifact, not a re-read: it is built around the
// reframe prompt "How would the vignette need to change for [your wrong answer]
// to be the right answer?", carries a one-line takeaway, an error-type tag
// (knowledge gap vs reasoning error vs misread), and stays linked to the cards
// it unsuspended. Entries sharing a confusion pair group together.

import type { Acuity } from "$lib/speedrun";

/** The error-type taxonomy that builds metacognitive calibration over time. */
export type ErrorKind = "reasoning" | "knowledge" | "misread";

export interface ErrorEntry {
    id: string;
    /** Clinical topic, e.g. "Acid–base · DKA". */
    topic: string;
    /** Confusion pair used to group look-alikes, e.g. "VT vs SVT-aberrancy". */
    confusion: string;
    /** The wrong answer the user chose (drives the reframe prompt). */
    chosen: string;
    /** The correct answer. */
    correct: string;
    kind: ErrorKind;
    /** Short human detail on the error, e.g. "skipped Winter's formula". */
    kindDetail: string;
    acuity: Acuity;
    /** AnKing cards this miss unsuspended / stays linked to. */
    cards: number;
    /** Provenance of the miss, e.g. "UWorld Q#14 · 2h ago". */
    source: string;
    /** A seeded one-line takeaway (may be empty — the user writes their own). */
    takeaway: string;
}

/** Human label + accent for an error kind (color is never the only signal). */
export const KIND_META: Record<ErrorKind, { label: string; acuity: Acuity }> = {
    reasoning: { label: "Reasoning error", acuity: "critical" },
    knowledge: { label: "Knowledge gap", acuity: "watch" },
    misread: { label: "Misread", acuity: "muted" },
};

// Two Cardiology entries share the "VT vs SVT-aberrancy" confusion pair on
// purpose, so the screen demonstrates confusion-pair grouping.
export const ERROR_ENTRIES: ErrorEntry[] = [
    {
        id: "e-acidbase-dka",
        topic: "Acid–base · DKA",
        confusion: "Compensation vs primary process",
        chosen: "Primary respiratory alkalosis",
        correct: "Appropriate respiratory compensation",
        kind: "reasoning",
        kindDetail: "skipped Winter's formula",
        acuity: "critical",
        cards: 4,
        source: "UWorld Q#14 · 2h ago",
        takeaway:
            "If PaCO₂ < Winter's predicted, it's a second primary disorder — check the formula before calling compensation.",
    },
    {
        id: "e-renal-hypona",
        topic: "Renal · Hyponatremia",
        confusion: "SIADH vs thiazide-induced",
        chosen: "SIADH",
        correct: "Thiazide-induced hyponatremia",
        kind: "misread",
        kindDetail: "missed HCTZ on the medication list",
        acuity: "watch",
        cards: 5,
        source: "UWorld Q#22 · 2h ago",
        takeaway: "",
    },
    {
        id: "e-cards-wct",
        topic: "Cardiology · Wide-complex tachycardia",
        confusion: "VT vs SVT-aberrancy",
        chosen: "SVT with aberrancy",
        correct: "Ventricular tachycardia",
        kind: "reasoning",
        kindDetail: "didn't risk-stratify on structural heart disease",
        acuity: "critical",
        cards: 6,
        source: "UWorld Q#31 · 2h ago",
        takeaway: "",
    },
    {
        id: "e-cards-wct2",
        topic: "Cardiology · Regular wide-complex",
        confusion: "VT vs SVT-aberrancy",
        chosen: "SVT with aberrancy",
        correct: "Ventricular tachycardia",
        kind: "knowledge",
        kindDetail: "Brugada morphology criteria not recalled",
        acuity: "watch",
        cards: 3,
        source: "NBME 14 · 1d ago",
        takeaway: "",
    },
    {
        id: "e-endo-thyroid",
        topic: "Endocrine · Hyperthyroidism",
        confusion: "Graves vs toxic multinodular goiter",
        chosen: "Graves disease",
        correct: "Toxic multinodular goiter",
        kind: "knowledge",
        kindDetail: "age + heterogeneous uptake pattern",
        acuity: "watch",
        cards: 3,
        source: "UWorld Q#40 · 2h ago",
        takeaway: "",
    },
    {
        id: "e-pulm-pe",
        topic: "Pulmonary · Dyspnea",
        confusion: "PE vs anxiety hyperventilation",
        chosen: "Anxiety / hyperventilation",
        correct: "Pulmonary embolism",
        kind: "misread",
        kindDetail: "overlooked unilateral leg swelling",
        acuity: "critical",
        cards: 4,
        source: "UWorld Q#7 · 3d ago",
        takeaway: "",
    },
];
