// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { expect, test } from "vitest";

import { parseQbankReport } from "./qbankParse";

test("parses the `label, correct, total` CSV shape (with a header row)", () => {
    const text = ["label,correct,total", "Cardiovascular,45,70", "Renal,30,60"].join("\n");
    const { rows, warnings } = parseQbankReport(text);

    expect(warnings).toEqual([]);
    expect(rows).toEqual([
        { label: "Cardiovascular", correct: 45, total: 70 },
        { label: "Renal", correct: 30, total: 60 },
    ]);
});

test("parses the UWorld System/Correct/Incorrect/Omitted/% shape (total = correct+incorrect+omitted)", () => {
    const text = [
        "System\tCorrect\tIncorrect\tOmitted\t%",
        "Cardiovascular\t45\t20\t5\t64%",
        "Pulmonary & Critical Care\t18\t10\t2\t60%",
    ].join("\n");
    const { rows, warnings } = parseQbankReport(text);

    expect(warnings).toEqual([]);
    // 45 + 20 + 5 = 70 ; 18 + 10 + 2 = 30 — the % column is ignored for counts.
    expect(rows).toEqual([
        { label: "Cardiovascular", correct: 45, total: 70 },
        { label: "Pulmonary & Critical Care", correct: 18, total: 30 },
    ]);
});

test("prefers an explicit Total column over summing when the header provides one", () => {
    const text = [
        "System, Correct, Incorrect, Omitted, Total, %",
        "Nervous System, 30, 8, 2, 50, 60%",
    ].join("\n");
    const { rows, warnings } = parseQbankReport(text);

    expect(warnings).toEqual([]);
    // Total column (50) wins over correct+incorrect+omitted (40).
    expect(rows).toEqual([{ label: "Nervous System", correct: 30, total: 50 }]);
});

test("collects warnings for junk lines instead of throwing", () => {
    const text = [
        "label,correct,total",
        "Cardiovascular,45,70",
        "# … 33 more rows in this block",
        "just some prose without any numbers",
        "Renal,30,60",
    ].join("\n");
    const { rows, warnings } = parseQbankReport(text);

    expect(rows).toEqual([
        { label: "Cardiovascular", correct: 45, total: 70 },
        { label: "Renal", correct: 30, total: 60 },
    ]);
    expect(warnings).toHaveLength(2);
    expect(warnings.some((w) => w.includes("33 more rows"))).toBe(true);
    expect(warnings.some((w) => w.includes("prose"))).toBe(true);
});

test("without a header, infers label/correct/total from two counts", () => {
    // Multi-space aligned columns (a common copy/paste shape), no header.
    const { rows, warnings } = parseQbankReport("Cardiology    45    70");

    expect(warnings).toEqual([]);
    expect(rows).toEqual([{ label: "Cardiology", correct: 45, total: 70 }]);
});

test("flags impossible rows (correct exceeds total, or zero total) as warnings", () => {
    const text = ["label,correct,total", "Renal,80,60", "Cardio,0,0"].join("\n");
    const { rows, warnings } = parseQbankReport(text);

    expect(rows).toEqual([]);
    expect(warnings).toHaveLength(2);
});

test("returns empty results (no throw) for empty input", () => {
    expect(parseQbankReport("")).toEqual({ rows: [], warnings: [] });
    expect(parseQbankReport("   \n  \n")).toEqual({ rows: [], warnings: [] });
});
