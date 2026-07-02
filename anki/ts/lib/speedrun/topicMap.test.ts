// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { expect, test } from "vitest";

import { BLUEPRINT_TOPICS, mapLabelToTopic, TOPIC_IDS } from "./topicMap";

test("maps real UWorld Step 2 system names to canonical topic ids", () => {
    expect(mapLabelToTopic("Cardiovascular")).toBe("cardio");
    expect(mapLabelToTopic("Pulmonary & Critical Care")).toBe("pulm");
    expect(mapLabelToTopic("Gastrointestinal & Nutrition")).toBe("gi");
    expect(mapLabelToTopic("Renal, Urinary & Electrolyte")).toBe("renal");
    expect(mapLabelToTopic("Endocrine, Diabetes & Metabolism")).toBe("endo");
    expect(mapLabelToTopic("Nervous System")).toBe("neuro");
    expect(mapLabelToTopic("Hematology & Oncology")).toBe("heme_onc");
    expect(mapLabelToTopic("Infectious Diseases")).toBe("id");
    expect(mapLabelToTopic("Female Reproductive System & Breast")).toBe("obgyn");
    expect(mapLabelToTopic("Dermatology")).toBe("derm");
    expect(mapLabelToTopic("Ophthalmology")).toBe("ophtho");
    expect(mapLabelToTopic("Ear, Nose & Throat")).toBe("ent");
    expect(mapLabelToTopic("Biostatistics & Epidemiology")).toBe("biostat");
    expect(mapLabelToTopic("Allergy & Immunology")).toBe("immuno");
});

test("is case-, whitespace- and punctuation-insensitive", () => {
    expect(mapLabelToTopic("  cardiovascular  ")).toBe("cardio");
    // A different separator (slash) must resolve the same as the ampersand form.
    expect(mapLabelToTopic("PULMONARY / CRITICAL CARE")).toBe("pulm");
    expect(mapLabelToTopic("ear nose and throat")).toBe("ent");
    expect(mapLabelToTopic("Hematology/Oncology")).toBe("heme_onc");
});

test("accepts common short synonyms and alternate discipline names", () => {
    expect(mapLabelToTopic("Cardiology")).toBe("cardio");
    expect(mapLabelToTopic("Nephrology")).toBe("renal");
    expect(mapLabelToTopic("OB/GYN")).toBe("obgyn");
    expect(mapLabelToTopic("Peds")).toBe("peds");
    expect(mapLabelToTopic("Psychiatry")).toBe("psych");
});

test("maps the exact system names from a UWorld 'Performance by System' PDF", () => {
    // Previously missed by a single word — now covered:
    expect(mapLabelToTopic("Renal, Urinary Systems & Electrolytes")).toBe("renal");
    expect(mapLabelToTopic("Psychiatric/Behavioral & Substance Abuse")).toBe("psych");
    // Spot-check other exact PDF names:
    expect(mapLabelToTopic("Cardiovascular System")).toBe("cardio");
    expect(mapLabelToTopic("Poisoning & Environmental Exposure")).toBe("emerg");
    expect(mapLabelToTopic("Pregnancy, Childbirth & Puerperium")).toBe("obgyn");
    expect(mapLabelToTopic("Social Sciences (Ethics/Legal/Professional)")).toBe("ethics");
    expect(mapLabelToTopic("Rheumatology/Orthopedics & Sports")).toBe("msk");
});

test("maps names with trailing parenthetical abbreviations and filler words", () => {
    // Trailing "(ENT)" must not defeat the match.
    expect(mapLabelToTopic("Ear, Nose & Throat (ENT)")).toBe("ent");
    // Filler "the" is ignored.
    expect(mapLabelToTopic("The Immune System")).toBe("immuno");
    expect(mapLabelToTopic("Pregnancy, Childbirth & the Puerperium")).toBe("obgyn");
});

test("maps the remaining real UWorld Step 2 system names", () => {
    expect(mapLabelToTopic("Blood & Lymphoreticular System")).toBe("heme_onc");
    expect(mapLabelToTopic("Male Reproductive System")).toBe("renal");
    expect(mapLabelToTopic("Respiratory System")).toBe("pulm");
    expect(mapLabelToTopic("Biostatistics & Epidemiology/Population Health")).toBe("biostat");
});

test("ignores odd internal spacing (strip-spaces fallback)", () => {
    expect(mapLabelToTopic("Cardio vascular")).toBe("cardio");
    expect(mapLabelToTopic("Gastro intestinal")).toBe("gi");
});

test("returns null for an unknown label (never silently guesses)", () => {
    expect(mapLabelToTopic("Wizardry & Potions")).toBeNull();
    expect(mapLabelToTopic("Miscellaneous")).toBeNull();
    expect(mapLabelToTopic("")).toBeNull();
    expect(mapLabelToTopic("   ")).toBeNull();
});

test("exposes exactly the 22 canonical blueprint topics", () => {
    expect(TOPIC_IDS).toHaveLength(22);
    expect(BLUEPRINT_TOPICS).toHaveLength(22);
    // Every listed topic has a non-empty human name and a stable id.
    for (const topic of BLUEPRINT_TOPICS) {
        expect(topic.id.length).toBeGreaterThan(0);
        expect(topic.name.length).toBeGreaterThan(0);
        expect(TOPIC_IDS).toContain(topic.id);
    }
    // No duplicate ids.
    expect(new Set(TOPIC_IDS).size).toBe(TOPIC_IDS.length);
    // Every canonical id maps to itself.
    for (const id of TOPIC_IDS) {
        expect(mapLabelToTopic(id)).toBe(id);
    }
});
