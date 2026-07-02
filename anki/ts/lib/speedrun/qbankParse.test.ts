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

test("keeps the WHOLE multi-word label on single-space rows (not just the first word)", () => {
    // A plain-text / PDF paste where columns are single spaces and there is no
    // header — the label must not be truncated to its first token.
    const { rows, warnings } = parseQbankReport(
        ["Female Reproductive System & Breast 20 40", "Nervous System 30 50"].join("\n"),
    );

    expect(warnings).toEqual([]);
    expect(rows).toEqual([
        { label: "Female Reproductive System & Breast", correct: 20, total: 40 },
        { label: "Nervous System", correct: 30, total: 50 },
    ]);
});

test("treats a comma inside a label as a word break, never a column break", () => {
    // Single-space data rows whose labels contain commas ("Renal, Urinary …")
    // must still be recognized as data and keep the full label.
    const { rows, warnings } = parseQbankReport(
        [
            "Renal, Urinary & Electrolyte 30 60",
            "Pregnancy, Childbirth & Puerperium 15 30",
        ].join("\n"),
    );

    expect(warnings).toEqual([]);
    expect(rows).toEqual([
        { label: "Renal Urinary & Electrolyte", correct: 30, total: 60 },
        { label: "Pregnancy Childbirth & Puerperium", correct: 15, total: 30 },
    ]);
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

test("parses a UWorld 'Performance by System' PDF export (n (x%) cells + wrapped names)", () => {
    const text = [
        "Cardiovascular System \t5/205 \t5 (100%) \t0 (0%) \t0 (0%) \t-",
        "Endocrine, Diabetes & Metabolism \t3/113 \t2 (67%) \t1 (33%) \t0 (0%) \t-",
        "Social Sciences",
        "(Ethics/Legal/Professional) 1/43 \t1 (100%) \t0 (0%) \t0 (0%) \t-",
        "Renal, Urinary Systems & Electrolytes 3/112 \t1 (33%) \t2 (67%) \t0 (0%) \t-",
    ].join("\n");
    const { rows } = parseQbankReport(text);

    // total = correct + incorrect + omitted; used/available and % are ignored.
    expect(rows).toContainEqual({ label: "Cardiovascular System", correct: 5, total: 5 });
    expect(rows).toContainEqual({
        label: "Endocrine, Diabetes & Metabolism",
        correct: 2,
        total: 3,
    });
    // The wrapped system name is rejoined across the two lines.
    expect(rows).toContainEqual({
        label: "Social Sciences (Ethics/Legal/Professional)",
        correct: 1,
        total: 1,
    });
    expect(rows).toContainEqual({
        label: "Renal, Urinary Systems & Electrolytes",
        correct: 1,
        total: 3,
    });
});
