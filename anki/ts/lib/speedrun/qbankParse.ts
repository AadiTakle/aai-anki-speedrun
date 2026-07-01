// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Pure, framework-free parser for pasted QBank "performance by subject/system"
// reports. Most QBanks expose only per-subject AGGREGATE counts (correct out of
// total) — no per-question export — so we ingest tabular text and return one row
// per subject. It is forgiving by design: it tolerates a header row, tab / comma
// / multi-space separators, and two common column shapes, and it collects a
// warning for every line it can't understand instead of throwing (so one bad
// line never loses the whole paste).

/** One aggregate subject row: correct out of total for a QBank label. */
export interface ParsedQbankRow {
    /** The QBank's own subject/system label (unmapped, verbatim). */
    label: string;
    correct: number;
    total: number;
}

export interface QbankParseResult {
    rows: ParsedQbankRow[];
    /** One human-readable note per line that couldn't be parsed. */
    warnings: string[];
}

interface Count {
    value: number;
    /** True when the cell was a percentage (e.g. "64%") — ignored for counts. */
    percent: boolean;
}

interface HeaderMap {
    labelCol: number;
    correctCol: number;
    incorrectCol: number;
    omittedCol: number;
    totalCol: number;
}

/** Parse a single cell as an integer/percentage count, or null if it isn't one. */
function parseCountCell(cell: string): Count | null {
    const trimmed = cell.trim();
    const percent = trimmed.endsWith("%");
    const body = (percent ? trimmed.slice(0, -1) : trimmed).replace(/,/g, "").trim();
    if (!/^\d+(\.\d+)?$/.test(body)) {
        return null;
    }
    return { value: Number(body), percent };
}

/** Split a line into trimmed, non-empty cells, detecting the separator. */
function splitCells(rawLine: string): string[] {
    let parts: string[];
    if (rawLine.includes("\t")) {
        parts = rawLine.split("\t");
    } else if (rawLine.includes(",")) {
        parts = rawLine.split(",");
    } else {
        const trimmed = rawLine.trim();
        parts = trimmed.split(/\s{2,}/);
        if (parts.length === 1) {
            parts = trimmed.split(/\s+/);
        }
    }
    return parts.map((part) => part.trim()).filter((part) => part.length > 0);
}

/** Map header cells to column roles by name (e.g. "Correct", "Omitted", "%"). */
function parseHeader(cells: string[]): HeaderMap {
    const header: HeaderMap = {
        labelCol: -1,
        correctCol: -1,
        incorrectCol: -1,
        omittedCol: -1,
        totalCol: -1,
    };
    cells.forEach((cell, index) => {
        const name = cell.toLowerCase();
        if (/%|percent|score|accuracy/.test(name)) {
            // Percentage / score column — never a raw count.
            return;
        }
        if (/incorrect|wrong/.test(name)) {
            header.incorrectCol = index;
        } else if (/correct/.test(name)) {
            header.correctCol = index;
        } else if (/omit/.test(name)) {
            header.omittedCol = index;
        } else if (/\btotal\b|questions|attempted|attempts|used|count/.test(name)) {
            header.totalCol = index;
        } else if (
            header.labelCol === -1
            && /system|subject|topic|section|category|discipline|organ|name/.test(name)
        ) {
            header.labelCol = index;
        }
    });
    if (header.labelCol === -1) {
        header.labelCol = 0;
    }
    return header;
}

/** Read the rounded integer count at a column, skipping percentages / gaps. */
function countAt(cells: string[], index: number): number | null {
    if (index < 0 || index >= cells.length) {
        return null;
    }
    const parsed = parseCountCell(cells[index]);
    if (parsed === null || parsed.percent) {
        return null;
    }
    return Math.round(parsed.value);
}

/** Parse a data row using a known header layout. */
function parseWithHeader(cells: string[], header: HeaderMap): ParsedQbankRow | null {
    const label = header.labelCol < cells.length ? cells[header.labelCol] : "";
    const correct = countAt(cells, header.correctCol);
    if (label.length === 0 || correct === null) {
        return null;
    }
    let total = header.totalCol === -1 ? null : countAt(cells, header.totalCol);
    if (total === null) {
        const incorrect = countAt(cells, header.incorrectCol) ?? 0;
        const omitted = countAt(cells, header.omittedCol) ?? 0;
        total = correct + incorrect + omitted;
    }
    return { label, correct, total };
}

/**
 * Parse a data row with no header. Two counts → `correct, total`; three or more
 * → UWorld-style `correct, incorrect, omitted` (total = their sum). The first
 * non-numeric cell is the label; percentages are ignored.
 */
function parseHeuristic(cells: string[]): ParsedQbankRow | null {
    let label = "";
    const counts: number[] = [];
    for (const cell of cells) {
        const parsed = parseCountCell(cell);
        if (parsed === null) {
            if (label.length === 0) {
                label = cell;
            }
        } else if (!parsed.percent) {
            counts.push(Math.round(parsed.value));
        }
    }
    if (label.length === 0 || counts.length < 2) {
        return null;
    }
    if (counts.length === 2) {
        return { label, correct: counts[0], total: counts[1] };
    }
    const [correct, incorrect, omitted] = counts;
    return { label, correct, total: correct + incorrect + omitted };
}

/**
 * Parse pasted QBank tabular text into aggregate subject rows + warnings.
 * Never throws: unparseable lines become warnings so a single bad row can't
 * discard the rest of the paste.
 */
export function parseQbankReport(text: string): QbankParseResult {
    const rows: ParsedQbankRow[] = [];
    const warnings: string[] = [];
    let header: HeaderMap | null = null;
    let seenContent = false;

    for (const rawLine of text.split(/\r?\n/)) {
        const line = rawLine.trim();
        if (line.length === 0) {
            continue;
        }
        const cells = splitCells(rawLine);
        const hasNumeric = cells.some((cell) => parseCountCell(cell) !== null);

        // A leading all-text row is treated as a header (skipped, not a warning).
        if (!seenContent) {
            seenContent = true;
            if (!hasNumeric) {
                header = parseHeader(cells);
                continue;
            }
        }

        if (!hasNumeric) {
            warnings.push(`Skipped (no counts found): "${line}"`);
            continue;
        }

        const parsed = header === null ? parseHeuristic(cells) : parseWithHeader(cells, header);
        if (parsed === null) {
            warnings.push(`Couldn't parse: "${line}"`);
            continue;
        }
        if (parsed.total <= 0 || parsed.correct < 0 || parsed.correct > parsed.total) {
            warnings.push(
                `Skipped (correct ${parsed.correct} of total ${parsed.total} is invalid): "${line}"`,
            );
            continue;
        }
        rows.push(parsed);
    }

    return { rows, warnings };
}
