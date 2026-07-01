// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// Pure, framework-free mapping from a QBank's subject/system label to one of the
// 22 canonical Step 2 CK blueprint topics. Most QBanks (UWorld, Amboss, …) only
// expose per-subject/system aggregate counts, and each names its systems
// slightly differently, so we normalize the label and look it up in a synonym
// table. An unknown label returns `null` — it is surfaced for the user to assign
// by hand, never silently dropped or guessed (honesty bar).

/** The 22 canonical blueprint topic ids (stable identifiers used engine-side). */
export const TOPIC_IDS = [
    "cardio",
    "pulm",
    "gi",
    "obgyn",
    "peds",
    "psych",
    "renal",
    "endo",
    "heme_onc",
    "id",
    "neuro",
    "msk",
    "surg",
    "emerg",
    "derm",
    "ophtho",
    "ent",
    "biostat",
    "ethics",
    "genetics",
    "immuno",
    "nutrition",
] as const;

/** One of the 22 canonical blueprint topic ids. */
export type TopicId = (typeof TOPIC_IDS)[number];

export interface BlueprintTopic {
    id: TopicId;
    /** Human-readable name for pickers / preview tables. */
    name: string;
}

/** The 22 topics with display names, in blueprint order (for the topic picker). */
export const BLUEPRINT_TOPICS: readonly BlueprintTopic[] = [
    { id: "cardio", name: "Cardiovascular" },
    { id: "pulm", name: "Pulmonary & Critical Care" },
    { id: "gi", name: "Gastrointestinal" },
    { id: "obgyn", name: "Obstetrics & Gynecology" },
    { id: "peds", name: "Pediatrics" },
    { id: "psych", name: "Psychiatry" },
    { id: "renal", name: "Renal & Urinary" },
    { id: "endo", name: "Endocrine & Metabolism" },
    { id: "heme_onc", name: "Hematology & Oncology" },
    { id: "id", name: "Infectious Diseases" },
    { id: "neuro", name: "Neurology" },
    { id: "msk", name: "Musculoskeletal & Rheumatology" },
    { id: "surg", name: "Surgery" },
    { id: "emerg", name: "Emergency Medicine" },
    { id: "derm", name: "Dermatology" },
    { id: "ophtho", name: "Ophthalmology" },
    { id: "ent", name: "Ear, Nose & Throat" },
    { id: "biostat", name: "Biostatistics & Epidemiology" },
    { id: "ethics", name: "Ethics & Professionalism" },
    { id: "genetics", name: "Genetics" },
    { id: "immuno", name: "Allergy & Immunology" },
    { id: "nutrition", name: "Nutrition" },
];

/** Human name for a canonical topic id (falls back to the id if unknown). */
export function topicDisplayName(id: string): string {
    const found = BLUEPRINT_TOPICS.find((topic) => topic.id === id);
    return found ? found.name : id;
}

// Extra synonyms beyond each topic's canonical id + display name. Covers the
// UWorld Step 2 CK "Performance by System" AND "by Subject" names, plus common
// abbreviations and discipline aliases. Written in human form; they are
// normalized (lower-cased, punctuation → spaces, "and" dropped) before lookup.
const SYNONYMS: Record<TopicId, readonly string[]> = {
    cardio: ["Cardiology", "Cardiac", "Heart", "Cardiovascular System", "CV"],
    pulm: [
        "Pulmonary",
        "Pulmonology",
        "Respiratory",
        "Respiratory System",
        "Pulmonary & Critical Care Medicine",
        "Critical Care",
        "Pulmonary / Critical Care",
    ],
    gi: [
        "Gastrointestinal",
        "Gastroenterology",
        "GI",
        "Gastrointestinal System",
        "Gastrointestinal & Nutrition",
        "Gastrointestinal & Hepatology",
        "Digestive System",
    ],
    obgyn: [
        "Obstetrics & Gynecology",
        "OB/GYN",
        "OB GYN",
        "Gynecology",
        "Obstetrics",
        "Female Reproductive System",
        "Female Reproductive System & Breast",
        "Pregnancy, Childbirth & Puerperium",
        "Women's Health",
        "Female Genital System",
    ],
    peds: ["Pediatrics", "Peds", "Pediatric", "Child Health"],
    psych: [
        "Psychiatry",
        "Psych",
        "Psychiatric",
        "Behavioral Science",
        "Psychiatric/Behavioral & Substance Use Disorder",
        "Behavioral Health",
        "Psychiatry/Behavioral Science",
    ],
    renal: [
        "Renal",
        "Nephrology",
        "Renal & Urinary",
        "Renal, Urinary & Electrolyte",
        "Kidney",
        "Urinary",
        "Genitourinary",
        "Male Reproductive System",
        "Urology",
    ],
    endo: [
        "Endocrine",
        "Endocrinology",
        "Endocrine & Metabolism",
        "Endocrine, Diabetes & Metabolism",
        "Diabetes",
        "Metabolism",
    ],
    heme_onc: [
        "Hematology & Oncology",
        "Heme/Onc",
        "Hematology",
        "Oncology",
        "Heme",
        "Cancer",
        "Hematologic & Lymphatic System",
    ],
    id: ["Infectious Diseases", "Infectious Disease", "ID", "Infection", "Microbiology", "Infectious"],
    neuro: [
        "Nervous System",
        "Neurology",
        "Neuro",
        "Nervous",
        "Neurologic",
        "Central Nervous System",
        "Brain",
    ],
    msk: [
        "Musculoskeletal System",
        "Musculoskeletal",
        "MSK",
        "Rheumatology",
        "Orthopedics",
        "Ortho",
        "Rheumatology/Orthopedics & Sports",
        "Musculoskeletal, Skin & Connective Tissue",
    ],
    surg: ["Surgery", "Surg", "General Surgery", "Surgical", "Perioperative Care"],
    emerg: [
        "Emergency Medicine",
        "Emergency",
        "Emerg",
        "EM",
        "Emergency Department",
        "Poisoning & Environmental Exposure",
        "Toxicology",
    ],
    derm: ["Dermatology", "Derm", "Skin", "Skin & Subcutaneous Tissue", "Integumentary System"],
    ophtho: ["Ophthalmology", "Ophtho", "Eye", "Eyes", "Ocular"],
    ent: [
        "Ear, Nose & Throat",
        "ENT",
        "Otolaryngology",
        "Otorhinolaryngology",
        "Head & Neck",
    ],
    biostat: [
        "Biostatistics & Epidemiology",
        "Biostatistics",
        "Biostat",
        "Biostats",
        "Epidemiology",
        "Epi",
        "Biostatistics, Epidemiology & Population Health",
        "Population Health",
    ],
    ethics: [
        "Ethics",
        "Medical Ethics",
        "Ethics & Professionalism",
        "Professionalism",
        "Social Sciences",
        "Social Sciences (Ethics/Legal/Professional)",
        "Jurisprudence",
    ],
    genetics: ["Genetics", "Medical Genetics", "Genetics & Genomics", "Genomics"],
    immuno: ["Allergy & Immunology", "Immunology", "Immuno", "Allergy", "Immune System"],
    nutrition: ["Nutrition", "Nutritional", "Clinical Nutrition"],
};

// Connector words dropped during normalization so "A & B", "A / B" and "A and B"
// all collapse to the same key.
const CONNECTOR_WORDS = new Set(["and"]);

// Generic trailing "noise" words retried-without on a miss, so "Cardiovascular
// System" resolves via "cardiovascular", etc. Kept small to avoid false hits.
const NOISE_WORDS = new Set([
    "system",
    "systems",
    "medicine",
    "disorders",
    "disorder",
    "conditions",
    "condition",
    "general",
    "clinical",
]);

/** Lower-case, turn punctuation into spaces, drop connector words, collapse ws. */
function normalizeLabel(raw: string): string {
    return raw
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, " ")
        .split(/\s+/)
        .filter((word) => word.length > 0 && !CONNECTOR_WORDS.has(word))
        .join(" ");
}

function stripNoise(key: string): string {
    return key
        .split(" ")
        .filter((word) => !NOISE_WORDS.has(word))
        .join(" ");
}

function buildLookup(): Map<string, TopicId> {
    const lookup = new Map<string, TopicId>();
    const add = (raw: string, id: TopicId): void => {
        const key = normalizeLabel(raw);
        // First writer wins: canonical id + name are added before synonyms, so a
        // synonym can never shadow a canonical topic name.
        if (key.length > 0 && !lookup.has(key)) {
            lookup.set(key, id);
        }
    };
    for (const topic of BLUEPRINT_TOPICS) {
        add(topic.id, topic.id);
        add(topic.name, topic.id);
        for (const synonym of SYNONYMS[topic.id]) {
            add(synonym, topic.id);
        }
    }
    return lookup;
}

const LOOKUP = buildLookup();

/**
 * Resolve a free-text QBank subject/system label to a canonical topic id, or
 * `null` when it isn't recognized (case/punctuation-insensitive). A `null`
 * result must be surfaced to the user to assign manually — never dropped.
 */
export function mapLabelToTopic(label: string): TopicId | null {
    const key = normalizeLabel(label);
    if (key.length === 0) {
        return null;
    }
    const direct = LOOKUP.get(key);
    if (direct !== undefined) {
        return direct;
    }
    const stripped = stripNoise(key);
    if (stripped.length > 0 && stripped !== key) {
        const viaStripped = LOOKUP.get(stripped);
        if (viaStripped !== undefined) {
            return viaStripped;
        }
    }
    return null;
}
