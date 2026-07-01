// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Sample paste for the Import screen's EMPTY STATE only. This is illustrative
// INPUT the user could paste — it is NOT a result. Every number the screen shows
// as an outcome comes from the real ImportQbankAggregate / RelinkMisses RPCs
// (see +page.svelte); nothing here is displayed as if it were live data.
//
// Shape: a UWorld "Performance by System" block (tab-separated, with a header
// row and the System/Correct/Incorrect/Omitted/% columns). The last row
// ("Miscellaneous") intentionally does NOT map to a canonical topic, so the
// preview demonstrates the "assign a topic" flow for unmapped rows.

/** Default provenance label shown in the source field (free-text; any QBank). */
export const DEFAULT_SOURCE = "UWorld";

/** A ready-to-paste UWorld-style aggregate block for the empty state. */
export const SAMPLE_PASTE = [
    "System\tCorrect\tIncorrect\tOmitted\t%",
    "Cardiovascular\t64\t22\t4\t71%",
    "Pulmonary & Critical Care\t38\t18\t2\t66%",
    "Gastrointestinal & Nutrition\t41\t25\t3\t59%",
    "Renal, Urinary & Electrolyte\t29\t23\t5\t53%",
    "Endocrine, Diabetes & Metabolism\t33\t14\t1\t69%",
    "Nervous System\t45\t20\t3\t65%",
    "Infectious Diseases\t37\t19\t2\t66%",
    "Female Reproductive System & Breast\t28\t16\t1\t62%",
    "Hematology & Oncology\t22\t18\t4\t52%",
    "Biostatistics & Epidemiology\t19\t6\t0\t76%",
    "Miscellaneous (multisystem)\t12\t9\t2\t54%",
].join("\n");
