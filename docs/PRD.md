# Anki Step 2 Prep Speedrun - STAT

## MVP

**Scope:** The app created by modifying the Anki codebase should act as a one-stop-shop for all Step 2 preparation needs. This includes support for integrating the most popular study resources including UWorld/AMBOSS/Other QBanks and NBME/Free 120/General Practice Tests.

Other sources will be integrated into the app later, but this is all that is in scope for the MVP.

### Guiding SPOV

Every MVP feature exists to advance the brainlift's thesis (`docs/brainlift_v1.md`, **SPOV 1**):

> The only barrier preventing capable Step 2 students from cutting their study time by 33%+ is **information organization** — not knowledge.

These students have already mastered the content through med school; they waste 8.4–11.2 weeks of a ~4-month prep on fragmented, low-yield activity (video series, redundant tools, manual error-tracking across disconnected apps). The product wins by **consolidating** the proven-effective resources (QBanks, practice tests, targeted flashcards) into one workflow, **linking** them so a missed question routes straight to the review that fixes it, **focusing** effort on weak high-yield topics, and **eliminating the fluff**.

The features below are AI-free (no card generation, no chatbot, no semantic search). AI is explicitly deferred (see "Out of Scope for MVP").

### Feature Roadmap (build order + priority)

Priority legend: **P0** = critical path, must ship for the MVP to demonstrate the SPOV · **P1** = high, completes the value proposition · **P2** = in scope but may slip.

| Order | Feature | Priority | Depends on |
|:-:|---|:-:|:-:|
| 1 | Unified Topic Taxonomy & Resource Crosswalk | P0 | — |
| 2 | QBank & Practice-Test Data Ingestion | P0 | 1 |
| 3 | Missed-Question → Flashcard Linking (auto selective-unsuspension) | P0 | 1, 2 |
| 4 | Per-Topic Mastery & the Memory↔Performance Gap | P0 | 1, 2 |
| 5 | Points-at-Stake / Topic-Aware Queue (Rust engine change) | P0 | 4 |
| 6 | Three Scores (Memory / Performance / Readiness) with Ranges + Give-Up Rule | P0 | 4 |
| 7 | Blueprint Coverage Map | P1 | 1, 2 |
| 8 | Speed & Pacing Analytics | P1 | 2, 4 |
| 9 | Anti-Over-Study / Efficiency Guardrails | P2 | 4, 6 |

### Features (detailed)

#### 1. Unified Topic Taxonomy & Resource Crosswalk - P0

- **What:** One canonical, blueprint-weighted Step 2 content map, plus a curated crosswalk that maps each source's labels (UWorld system/subject, AMBOSS, NBME content areas, **AnKing deck tags**) onto it. Source-adapter pattern so new resources are added by writing one more mapping — generic, not limited to UWorld/AMBOSS/NBME.
- **Build hook:** New curated mapping (new SQLite table or `col.conf` JSON) joined on the existing AnKing tag hierarchy.
- **SPOV contribution:** This *is* "information organization" made concrete. It is the shared spine that ends resource fragmentation (Insight 5) and is what makes cross-tool tracking possible at all. Everything downstream joins on it.

#### 2. QBank & Practice-Test Data Ingestion - P0

- **What:** A generic importer (CSV/JSON/paste) that normalizes any source into canonical `QuestionAttempt` and `PracticeTestResult` records, with idempotent dedup on `(source, external_id, timestamp)`.
- **Build hook:** New tables in `rslib/src/storage/`; thin per-source adapters.
- **SPOV contribution:** Consolidates the performance data from the average student's ~4.4 disconnected resources into one store (Insight 5), replacing the manual, error-prone cross-tool tracking the brainlift flags as a core pain point. Dedup directly protects the "no double-counted reviews" requirement.

#### 3. Missed-Question → Flashcard Linking (auto selective-unsuspension) - P0

- **What:** When a QBank/practice-test question is missed, automatically surface/unsuspend the AnKing cards mapped to that topic and append to an integrated error log. The flagship organizing action.
- **Build hook:** Crosswalk lookup (F1) → card search/unsuspend ops routed through Anki's op/undo framework so undo stays correct.
- **SPOV contribution:** Automates the exact proven high-scorer workflow ("selective unsuspension" + error log ; §4.2) that is today manual and split across separate tools (Insight 5). This is the clearest embodiment of "organization cuts time": a missed vignette becomes the right review with zero overhead.

#### 4. Per-Topic Mastery & the Memory↔Performance Gap - P0

- **What:** Per canonical topic, show FSRS recall (memory) beside QBank application accuracy (performance), and flag "knows the fact, can't apply it" topics. This is the aggregation layer consumed by F5 and F6.
- **Build hook:** Mastery query over cards/revlog + `question_attempts`, joined via the crosswalk (a 7a "mastery query" candidate).
- **SPOV contribution:** Targets the rote-recall-vs-clinical-thinking gap the brainlift says wastes the most time (Insight 1). Showing where recall ≠ application points effort at genuine weak spots instead of re-memorizing already-known facts.

#### 5. Points-at-Stake / Topic-Aware Queue (Rust engine change) - P0

- **What:** A new review order that prioritizes cards by `blueprint_weight * weakness` (weakness from QBank performance), bringing weak high-yield topics back sooner while keeping FSRS intervals valid. The mandatory real Rust change; ships to both desktop and phone.
- **Build hook:** `rslib/src/scheduler/queue/builder/` + `ReviewCardOrder` in `storage/card/mod.rs`, exposed via a new protobuf RPC; >=3 Rust tests + 1 Python test; undo-safe.
- **SPOV contribution:** Converts organized weakness data into focused daily action; the mechanism that recovers wasted reps and delivers the 33% speedup at the core of the SPOV. Focuses the capable learner on what moves their score.

#### 6. Three Scores (Memory / Performance / Readiness) with Ranges + Give-Up Rule - P0

- **What:** Memory (FSRS), Performance (QBank-derived, e.g. Beta-Binomial per topic for a built-in range), Readiness (blueprint-weighted, calibrated against NBME/UWSA), each shown as a range with reasons, last-updated, and an explicit abstain rule when data is insufficient.
- **Build hook:** Classical stats over F4 outputs; calibrate to imported practice-test scaled scores; new RPC + dashboard in `ts/`.
- **SPOV contribution:** Honest, ranged readiness lets these capable students *stop* over-preparing — directly countering the "overzealous students perform worse" failure mode (Insight 4) and anxiety-driven resource hoarding (Insights 4–5). Predictors are already known to be accurate (Insight 2), so surfacing them honestly is high-value and low-risk.

#### 7. Blueprint Coverage Map - P1

- **What:** Percentage of the official Step 2 outline actually covered by the deck + QBank exposure, by weighted section; readiness abstains when coverage is below a stated line.
- **Build hook:** Query over crosswalk coverage; feeds F6's give-up rule (satisfies challenge 7c).
- **SPOV contribution:** Makes "what's done vs what's left" explicit so study stays targeted rather than scattered, and blocks false confidence — reinforcing organization over volume.

#### 8. Speed & Pacing Analytics - P1

- **What:** Per-question timing from QBank/tests; flag accurate-but-slow topics; pacing and fatigue view; feed median time into readiness so a topic only counts as ready when fast *and* accurate.
- **Build hook:** Timing fields on `question_attempts`; pacing panel in dashboard.
- **SPOV contribution:** Addresses the timed, 9-hour endurance reality of the exam (Subcategory 5.2) and the explicit "speed" half of the product goal — ensuring efficiency gains never come at the cost of finishing on time.

#### 9. Anti-Over-Study / Efficiency Guardrails — P2

- **What:** Stop over-reviewing topics that are strong on *both* FSRS and QBank, flag redundant/low-value study, and nudge users away from passive video fallback toward active retrieval.
- **Build hook:** Rules over F4/F6 signals; surfaced as dashboard nudges and queue suppression.
- **SPOV contribution:** Directly attacks the 8.4–11.2 wasted weeks the SPOV quantifies by disincentivizing the inefficient fluff (video series, redundant over-review; Insight 4) — the "eliminate the fluff" mandate, made into product behavior.

### Out of Scope for MVP

- **AI features** (deferred to a later phase, per project deadlines): AI card generation from sources, AI summarization of video/podcast content, chatbots, and semantic/vector search. The MVP must give a score and run fully with AI switched off.
- Additional resource integrations beyond the QBanks and practice tests named above.

## Daily Workflow

The app is opinionated about study order: it actively promotes the practice methods the evidence supports and pushes the time-wasting ones down or out. This ordering is the product's expression of the SPOV, organizing effort toward what works is the lever that recovers the ~33% of wasted study time.

### The order the app promotes practice in (and the evidence why)

**1st - QBank questions, timed/randomized (active retrieval).** The day is built around active retrieval on clinical vignettes, not passive review.

- Active QBank retrieval is the dominant methodology of successful students; high scorers do **40–80 questions/day** in **timed, randomized (interleaved)** blocks to match exam pacing and create productive difficulty (Knowledge Tree §1.0, Source 1 [30]).
- UWorld is used by **90%+** of students [1]; AMBOSS users average **+10.4 points** [3]. Experts rank QBanks and practice tests as the top two methods (Insight 4).
- The app orchestrates these banks rather than replacing them (Insight 1).

**2nd - Targeted flashcard review of misses, never blanket deck review.** Flashcards come second, and only on what was missed.

- Daily Anki correlates with higher Step 1 scores but shows **no independent correlation with Step 2 CK** - *"unless used selectively to review personal Qbank mistakes"* (§1.3 [9][10][11]).
- Top scorers use a **"selective unsuspension"** workflow: unsuspend only the cards tied to missed questions (§4.2 [1]). The app automates this (Feature 3).

**3rd - Error-log reframing of each miss.** Every wrong answer becomes a reasoning artifact, not just a re-read.

- High scorers keep an error log asking *"how would the vignette need to change for this wrong answer to be correct?"* (§4.2 [16]).
- Plateaus occur when students label every miss a knowledge gap and miss the underlying reasoning error (§5.3 [24]).

**Periodic (≈weekly) - calibrated practice tests, for readiness only.**

- NBME/UWSA forms are the only calibrated readiness gauges (NBME Form 14 *r*=0.92; UWSA2 *r*=0.89, overpredicts ~3.2 pts; Free 120 *r*=0.85) (§1.5; Insight 2). They anchor the readiness score; over-testing just burns prep time.

**Pushed down / out - passive video and resource sprawl.**

- Using **3+ video resources correlates with significantly lower** Step 2 scores (§2.1 [14]); video is the most time-consuming, lowest-yield modality.
- Using **too many resources makes students perform worse** (Insight 4; §5.1 [17][15]). The average student spreads effort across **4.4 resources, ~1.4 of them video -> 8.4–11.2 wasted weeks** the app aims to reclaim (Insight 5 / SPOV).

### The daily loop

1. **Check the plan (~2 min):** dashboard shows the three scores (with ranges), coverage, and "Today's focus" = highest points-at-stake topics + the single best next action. *(Features 5–7)*
2. **Do the recommended QBank block:** timed and randomized in UWorld/AMBOSS; the app set the size and topics. *(feeds Feature 2)*
3. **Auto-ingest + auto-link:** results import, and every miss unsuspends the mapped cards and opens an error-log entry with the reframe prompt. *(Features 2 → 3)*
4. **Targeted review:** an Anki session front-loaded by points-at-stake and seeded with today's misses, interleaved across systems. *(Features 5, 3)*
5. **Close the loop:** performance and the memory↔performance gap update, pacing flags accurate-but-slow topics, and the readiness range refreshes. *(Features 4, 6, 8)*
6. **Know when to stop:** guardrails pause topics strong on both memory and application and call diminishing returns — countering over-study. *(Feature 9)*

**Weekly beat:** a full practice test recalibrates readiness, refreshes the coverage map, and re-prioritizes the next week's points-at-stake topics.

## User Persona

> Primary target persona, created from the brainlift's spiky insight that capable-but-high-stakes, time-pressured learners — especially IMGs, DOs, and students with learning differences — are the prime beneficiaries of a "speedrun" workflow (Insight 3; Category 6). Secondary users (US-MD and non-traditional students) run the same loop at lower score pressure.

**Aisha Rahman — 27, International Medical Graduate (IMG)**

- **Context:** Already practiced as a junior doctor abroad; now preparing for Step 2 CK to match into a US residency. ~10 weeks out, studying ~8 hrs/day around visa and application logistics.
- **Stakes:** Residency screening filters mean she must score **5–12 points higher** than US-MD peers for equal interest (§6.1 [25][26]); she targets 245+. U.S. systems-based practice and patient-safety content is newer to her (§6.1 [27]).
- **Current toolset (the problem):** UWorld + AnKing Step 2 + OnlineMedEd + Divine Intervention + NBME forms. Five disconnected tools she stitches together by hand (the ~4.4-resource sprawl; Insight 5).
- **Goals:** Cover the blueprint fast, convert recall into clinical reasoning, and *know* when she is actually ready without burning weeks on low-yield review.
- **Frustrations:**
  - Spends evenings manually unsuspending AnKing cards and copying missed questions into a Google Doc error log she rarely revisits (§2.3 [16]).
  - Can't tell whether her UWorld % means she's on track; UWSA overpredicts (§2.2).
  - Tempted to re-watch OnlineMedEd when anxious, even though it eats hours (§2.1; Insight 4).
  - Overwhelmed each morning by which of her five tools to open first (§5.1).

### A Day in the Life: Aisha (dedicated period)

- **8:00** — Opens the app. *"Readiness 244 (239–250), low confidence - 61% coverage. Today: Renal + Cardiology are your highest-value gaps."*
- **8:05** — Does a timed 40-question random block in UWorld.
- **9:30** — The companion extension syncs her results: 12 misses → 31 AnKing cards auto-unsuspended, 12 error-log entries created with reframe prompts.
- **9:35** — Anki session (~120 cards), front-loaded with the renal/cardio misses, interleaved across systems.
- **10:15** — Dashboard updates: Renal performance 58% → 64%; a pacing flag appears on acid-base (*accurate but 95s/question*).
- **10:20** — Guardrail: *"Endocrine is strong on memory and application — paused from today's queue."* She stops there instead of over-reviewing.
- **1:00 (downtime)** — On her phone between errands, she clears 15 due reviews; they sync back to the desktop automatically.
- **Sunday** — Takes NBME Form 14 and imports the score. The readiness range tightens and next week's focus shifts to her weakest high-yield sections.