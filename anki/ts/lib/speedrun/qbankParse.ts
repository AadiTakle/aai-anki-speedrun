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
 * Parse a data row with no known header layout. The row is "<label words…>
 * <counts…>": the label is the WHOLE leading run of word tokens (so multi-word
 * system names like "Female Reproductive System & Breast" survive intact), and
 * the counts are the trailing integers. Two counts → `correct, total`; three or
 * more → UWorld-style `correct, incorrect, omitted` (total = their sum).
 *
 * Tokenizes on whitespace AND commas together, so a comma inside a label
 * ("Renal, Urinary & Electrolyte") is treated as a word break, never a column
 * break. Percentages and "used/available" (`5/205`) cells are ignored; tokens
 * after the first count (e.g. a trailing "-") are dropped.
 */
function parseLooseDataLine(rawLine: string): ParsedQbankRow | null {
    const tokens = rawLine
        .trim()
        .split(/[\s,]+/)
        .map((token) => token.trim())
        .filter((token) => token.length > 0);
    const labelParts: string[] = [];
    const counts: number[] = [];
    let seenCount = false;
    for (const token of tokens) {
        const parsed = parseCountCell(token);
        if (parsed !== null) {
            // A percentage is never a count; both a real count and a percentage
            // mark the end of the leading label region.
            if (!parsed.percent) {
                counts.push(Math.round(parsed.value));
            }
            seenCount = true;
            continue;
        }
        if (/^\d+\/\d+$/.test(token)) {
            // "used/available" (e.g. 5/205) — not a label word, not a count.
            seenCount = true;
            continue;
        }
        if (!seenCount) {
            labelParts.push(token);
        }
    }
    const label = labelParts.join(" ").trim();
    if (label.length === 0 || counts.length < 2) {
        return null;
    }
    if (counts.length === 2) {
        return { label, correct: counts[0], total: counts[1] };
    }
    const [correct, incorrect, omitted] = counts;
    return { label, correct, total: correct + incorrect + omitted };
}

// A UWorld "Performance by System" data line:
//   "<name>  <used>/<available>  c (c%)  i (i%)  o (o%)  [p-rank]"
// The three "(n%)" cells are correct / incorrect / omitted; total is their sum
// (which equals the used numerator). Tabs or spaces separate the columns.
const UWORLD_LINE_RE = /^(.*?)\s+\d+\/\d+\s+(\d+)\s*\(\d+%\)\s+(\d+)\s*\(\d+%\)\s+(\d+)\s*\(\d+%\)/;

/** Parse one UWorld report line, or null if it isn't one. */
function parseUworldLine(rawLine: string): ParsedQbankRow | null {
    const match = UWORLD_LINE_RE.exec(rawLine.trim());
    if (match === null) {
        return null;
    }
    const correct = Number(match[2]);
    const total = correct + Number(match[3]) + Number(match[4]);
    return { label: match[1].trim(), correct, total };
}

/** Validate a parsed row and push it, or record a warning against `line`. */
function pushRow(
    rows: ParsedQbankRow[],
    warnings: string[],
    parsed: ParsedQbankRow | null,
    line: string,
): void {
    if (parsed === null || parsed.label.length === 0) {
        warnings.push(`Couldn't parse: "${line}"`);
        return;
    }
    if (parsed.total <= 0 || parsed.correct < 0 || parsed.correct > parsed.total) {
        warnings.push(
            `Skipped (correct ${parsed.correct} of total ${parsed.total} is invalid): "${line}"`,
        );
        return;
    }
    rows.push(parsed);
}

/**
 * Parse pasted QBank tabular text into aggregate subject rows + warnings.
 * Handles simple CSV/TSV tables AND the UWorld "Performance by System" report
 * (grouped by discipline, "n (x%)" cells, and system names that wrap onto the
 * previous line). Repeated systems are returned once per occurrence; callers
 * aggregate per topic. Never throws — unparseable lines become warnings.
 */
export function parseQbankReport(text: string): QbankParseResult {
    const rows: ParsedQbankRow[] = [];
    const warnings: string[] = [];
    let header: HeaderMap | null = null;
    let seenContent = false;
    // A digit-free line may be the first half of a wrapped UWorld system name
    // (e.g. "Social Sciences" then "(Ethics/Legal/Professional) 1/43 …"). Hold it
    // until the next line: a following UWorld row consumes it, else it's warned.
    let carry = "";
    const flushCarry = (): void => {
        if (carry.length > 0) {
            warnings.push(`Skipped (no counts found): "${carry}"`);
            carry = "";
        }
    };

    for (const rawLine of text.split(/\r?\n/)) {
        const line = rawLine.trim();
        if (line.length === 0) {
            continue;
        }

        const uworld = parseUworldLine(rawLine);
        if (uworld !== null) {
            // Once real data is seen, later all-text lines are name fragments or
            // junk — never a header.
            seenContent = true;
            const label = carry.length > 0 ? `${carry} ${uworld.label}`.trim() : uworld.label;
            carry = "";
            pushRow(rows, warnings, { ...uworld, label }, line);
            continue;
        }

        const cells = splitCells(rawLine);
        // Detect counts from EITHER a structured split (tab/comma/2-space cells)
        // or a plain whitespace/comma tokenization — so a data row whose label
        // contains a comma (e.g. "Renal, Urinary & Electrolyte 30 60") is still
        // recognized as data, not mistaken for a header/name fragment.
        const hasNumeric =
            cells.some((cell) => parseCountCell(cell) !== null) ||
            line.split(/[\s,]+/).some((token) => parseCountCell(token) !== null);

        if (!hasNumeric) {
            // A leading all-text row is treated as a header (skipped, not warned).
            if (!seenContent) {
                seenContent = true;
                header = parseHeader(cells);
                continue;
            }
            flushCarry();
            if (/\d/.test(line)) {
                warnings.push(`Skipped (no counts found): "${line}"`);
            } else {
                carry = line;
            }
            continue;
        }

        seenContent = true;
        flushCarry();
        // With a header, trust the mapped columns; fall back to the loose parser
        // when a row doesn't fit the header shape (e.g. a comma inside a label).
        const parsed =
            header === null
                ? parseLooseDataLine(rawLine)
                : (parseWithHeader(cells, header) ?? parseLooseDataLine(rawLine));
        pushRow(rows, warnings, parsed, line);
    }

    flushCarry();
    return { rows, warnings };
}
