// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Route-local mock for the STAT Import screen (C3). ILLUSTRATIVE SCENARIO DATA
// ONLY — no live data, no fetch. This stands in for the QBank/practice-test
// ingestion + auto-link RPC being built concurrently on the engine lane; the
// screen consumes it exactly as it will consume the real importer response.
//
// Scenario: US-1 Aisha pastes a 40-question UWorld timed block. Idempotent
// dedup on (source, external_id, timestamp) drops re-imported rows, then the
// auto-link moment unsuspends the AnKing cards mapped to each miss and opens an
// error-log reframe per miss. Numbers are the design canvas's flagship event:
// 12 misses -> 31 cards unsuspended -> 12 reframes opened.

import type { Acuity } from "$lib/speedrun";

/** The three accepted ingestion inputs (one importer, three source adapters). */
export type ImportFormat = "paste" | "csv" | "json";

/** One normalized attempt from the parsed block (a representative preview row). */
export interface ParsedRow {
    externalId: string;
    topic: string;
    correct: boolean;
    seconds: number;
    /** True when idempotent dedup skips this row (already ingested). */
    duplicate?: boolean;
}

/** The parse + dedup summary shown before the organizing action. */
export interface ImportPreview {
    /** Total rows parsed from the block. */
    parsed: number;
    /** Fresh rows kept after dedup. */
    fresh: number;
    /** Rows skipped as idempotent duplicates. */
    duplicates: number;
    /** Representative subset for the preview table (not the whole block). */
    rows: ParsedRow[];
}

/** Per-topic slice of the auto-link event (misses -> reframes + mapped cards). */
export interface AutoLinkTopic {
    topicId: string;
    name: string;
    /** Misses in this topic — one error-log reframe opens per miss. */
    misses: number;
    /** AnKing cards unsuspended for this topic's mapped set. */
    cards: number;
    acuity: Acuity;
}

/** The settled result of the one calm auto-link pulse. */
export interface AutoLinkResult {
    misses: number;
    cardsUnsuspended: number;
    reframesOpened: number;
    topics: AutoLinkTopic[];
}

/** Everything the Import screen needs, behind one stable seam. */
export interface ImportScenario {
    /** Human label for the block, e.g. shown as a source chip. */
    source: string;
    /** A ready-to-paste sample per input format (the "sample paste"). */
    samples: Record<ImportFormat, string>;
    preview: ImportPreview;
    autoLink: AutoLinkResult;
    /** One-line explanation of the idempotent-dedup guarantee. */
    dedupNote: string;
}

// A duplicate row shares (source, external_id, timestamp) with an earlier row,
// so dedup drops it — the last Q14 line below re-appears verbatim on purpose.
const CSV_SAMPLE = `source,external_id,topic,correct,seconds,timestamp
UWorld,UW-Q12,Renal,0,88,2026-06-30T18:04:00Z
UWorld,UW-Q13,Cardiology,1,61,2026-06-30T18:05:20Z
UWorld,UW-Q14,Acid–base,0,95,2026-06-30T18:07:10Z
UWorld,UW-Q15,Endocrine,1,52,2026-06-30T18:08:33Z
UWorld,UW-Q16,Renal,0,74,2026-06-30T18:10:01Z
UWorld,UW-Q17,Cardiology,1,66,2026-06-30T18:11:44Z
UWorld,UW-Q14,Acid–base,0,95,2026-06-30T18:07:10Z
# … 33 more rows in this 40-question block`;

const JSON_SAMPLE = `[
  { "source": "UWorld", "external_id": "UW-Q12", "topic": "Renal",      "correct": false, "seconds": 88, "timestamp": "2026-06-30T18:04:00Z" },
  { "source": "UWorld", "external_id": "UW-Q13", "topic": "Cardiology", "correct": true,  "seconds": 61, "timestamp": "2026-06-30T18:05:20Z" },
  { "source": "UWorld", "external_id": "UW-Q14", "topic": "Acid–base",  "correct": false, "seconds": 95, "timestamp": "2026-06-30T18:07:10Z" },
  { "source": "UWorld", "external_id": "UW-Q15", "topic": "Endocrine",  "correct": true,  "seconds": 52, "timestamp": "2026-06-30T18:08:33Z" },
  { "source": "UWorld", "external_id": "UW-Q16", "topic": "Renal",      "correct": false, "seconds": 74, "timestamp": "2026-06-30T18:10:01Z" },
  { "source": "UWorld", "external_id": "UW-Q17", "topic": "Cardiology", "correct": true,  "seconds": 66, "timestamp": "2026-06-30T18:11:44Z" },
  { "source": "UWorld", "external_id": "UW-Q14", "topic": "Acid–base",  "correct": false, "seconds": 95, "timestamp": "2026-06-30T18:07:10Z" }
]`;

const PASTE_SAMPLE = `UWorld · 40-question timed block · 2026-06-30
UW-Q12  Renal        MISS     88s
UW-Q13  Cardiology   correct  61s
UW-Q14  Acid–base    MISS     95s
UW-Q15  Endocrine    correct  52s
UW-Q16  Renal        MISS     74s
UW-Q17  Cardiology   correct  66s
UW-Q14  Acid–base    MISS     95s   (re-paste — deduped)
… 33 more`;

/** The single seeded scenario served to the Import screen. */
export const IMPORT_SCENARIO: ImportScenario = {
    source: "UWorld · 40-question timed block",
    samples: {
        paste: PASTE_SAMPLE,
        csv: CSV_SAMPLE,
        json: JSON_SAMPLE,
    },
    preview: {
        parsed: 40,
        fresh: 37,
        duplicates: 3,
        rows: [
            { externalId: "UW-Q12", topic: "Renal", correct: false, seconds: 88 },
            { externalId: "UW-Q13", topic: "Cardiology", correct: true, seconds: 61 },
            { externalId: "UW-Q14", topic: "Acid–base", correct: false, seconds: 95 },
            { externalId: "UW-Q15", topic: "Endocrine", correct: true, seconds: 52 },
            { externalId: "UW-Q16", topic: "Renal", correct: false, seconds: 74 },
            { externalId: "UW-Q17", topic: "Cardiology", correct: true, seconds: 66 },
            { externalId: "UW-Q14", topic: "Acid–base", correct: false, seconds: 95, duplicate: true },
        ],
    },
    autoLink: {
        misses: 12,
        cardsUnsuspended: 31,
        reframesOpened: 12,
        topics: [
            { topicId: "renal", name: "Renal", misses: 7, cards: 17, acuity: "critical" },
            { topicId: "cardiology", name: "Cardiology", misses: 4, cards: 11, acuity: "critical" },
            { topicId: "acid-base", name: "Acid–base", misses: 1, cards: 3, acuity: "watch" },
        ],
    },
    dedupNote:
        "Re-importing the same block never double-counts a review — dedup is idempotent on (source, external_id, timestamp).",
};
