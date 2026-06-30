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

This product is built for a deliberately **narrow** audience: the **capable, high-stakes,
time-compressed Step 2 CK examinee** whose bottleneck is **information organization, not knowledge**
(the SPOV). They have already mastered the content through medical school and are highly effective
learners; they are losing weeks to resource sprawl (~4.4 disconnected tools; Insight 5), manual
error-tracking, and anxiety-driven over-study — not to missing facts.

**Who this is for (the target):**

- **Primary — learners under disproportionate score/time pressure:** International Medical Graduates
  (IMGs), DO students double-testing, and students with learning differences (ADHD/LD). They must
  score higher than peers with less time (Insight 3; Category 6), so recovering wasted study time is
  existential, not just convenient.
- **Secondary — US-MD and non-traditional students** running the same loop at lower score pressure
  (e.g. integrating prep into clinical rotations).

**Who this is *not* for (out of audience):**

- Early-stage learners still building baseline knowledge (they need content delivery — video
  courses, lectures — which this product intentionally pushes *down/out*).
- Anyone wanting more questions/content; the gold-standard QBanks already over-supply that. We
  *orchestrate* existing resources, we don't replace them (Insight 1).
- Non-Step-2 exams (out of scope per the brainlift).

**Representative persona — Aisha Rahman, 27, International Medical Graduate (IMG):**

- **Context:** Already practiced as a junior doctor abroad; now preparing for Step 2 CK to match into
  a US residency. ~10 weeks out, studying ~8 hrs/day around visa and application logistics.
- **Stakes:** Residency screening filters mean she must score **5–12 points higher** than US-MD peers
  for equal interest (§6.1 [25][26]); she targets 245+. U.S. systems-based practice and patient-safety
  content is newer to her (§6.1 [27]).
- **Current toolset (the problem):** UWorld + AnKing Step 2 + OnlineMedEd + Divine Intervention +
  NBME forms. Five disconnected tools she stitches together by hand (the ~4.4-resource sprawl;
  Insight 5).
- **Goals:** Cover the blueprint fast, convert recall into clinical reasoning, and *know* when she is
  actually ready without burning weeks on low-yield review.
- **Frustrations:**
  - Spends evenings manually unsuspending AnKing cards and copying missed questions into a Google Doc
    error log she rarely revisits (§2.3 [16]).
  - Can't tell whether her UWorld % means she's on track; UWSA overpredicts (§2.2).
  - Tempted to re-watch OnlineMedEd when anxious, even though it eats hours (§2.1; Insight 4).
  - Overwhelmed each morning by which of her five tools to open first (§5.1).

## User Stories

Each story is one **fake client** with a **unique, specific, start-to-finish workflow** through the
MVP. Feature references map to the roadmap above (F1–F9). All stories are within MVP scope — **no AI**
(no card generation, chatbot, or semantic search); every score runs with AI off.

### US-1 — Aisha Rahman (IMG, dedicated period): "cover the blueprint and trust my readiness"

*Goal:* study only high-value material and get an honest readiness range without manual bookkeeping.

1. **Onboarding.** Imports her AnKing Step 2 deck. The app builds the canonical Step 2 taxonomy and
   crosswalks her AnKing tags + UWorld subjects onto it; the coverage map reads **61%** (F1, F7).
2. **8:00 — plan.** Dashboard shows three ranged scores; readiness is **low-confidence** ("244,
   239–250, 61% coverage") and "Today's focus" surfaces her highest **points-at-stake** topics —
   Renal, Cardiology (F5, F6, F7).
3. **8:05 — retrieval.** Does a timed, randomized 40-question UWorld block.
4. **9:30 — ingest + auto-link.** Pastes/imports the results; her 12 misses **auto-unsuspend** the 31
   mapped AnKing cards and open 12 error-log entries with the reframe prompt (F2, F3).
5. **9:35 — targeted review.** Anki session (~120 cards) **front-loaded by points-at-stake** and
   seeded with today's misses, interleaved across systems (F5, F3).
6. **10:15 — close the loop.** Per-topic memory↔performance updates (Renal 58%→64%); a pacing flag
   appears on acid-base (*accurate but 95 s/question*) (F4, F8).
7. **10:20 — stop.** A guardrail pauses Endocrine ("strong on memory *and* application") so she
   doesn't over-review (F9).
8. **1:00 — phone.** Between errands she clears 15 due reviews offline on her phone; they sync to the
   desktop (mobile + sync).
9. **Weekly.** Imports an NBME Form 14 score; the readiness range tightens and next week re-prioritizes
   her weakest high-yield sections (F6 readiness, F7).

*End state:* she touched only high-value material, never hand-managed unsuspension/error logs, and has
a readiness range she can act on.

### US-2 — Marcus Bell (DO, double-testing, on core rotations): "don't show me a score I haven't earned"

*Goal:* make minimal rotation downtime count, and only trust readiness once it's backed by data.

1. **Setup.** Sets exam = Step 2 CK; imports UWorld history + a partial AnKing subset. Coverage is
   **38%** with ~120 graded reviews, so readiness **abstains**: *"No score yet — needs ≥50% coverage
   and ≥200 graded reviews"* (F6 give-up rule, F7).
2. **Micro-blocks.** During Internal Medicine downtime he does 15-question shelf-mode UWorld blocks
   and imports each (F2).
3. **Auto-link.** Misses auto-link to the mapped cards and build his error log; no manual tracking
   (F3).
4. **Focus.** The points-at-stake queue front-loads high-yield IM topics (55–65% of the exam) so his
   scarce time hits the heaviest-weighted material first (F5).
5. **Coverage nudge.** The coverage map flags an untouched high-weight section (Pediatrics) and keeps
   readiness abstaining until he addresses it — a 10k-card deck that skips a section is not "ready"
   (F7, challenge 7c).
6. **Earned score.** After ~3 weeks, coverage crosses 50% and reviews exceed 200; readiness
   **un-abstains** with a deliberately wide range and its reasons listed (F6).
7. **Sync.** Reviews done on his phone during call shifts sync back to the desktop (mobile + sync).

*End state:* effort routed to the heaviest gaps under severe time pressure; the score appears only
once it is honestly supported.

### US-3 — Priya Nair (US-MD with ADHD / learning difference): "tell me the one next thing, and stop me from spiraling"

*Goal:* remove decision overhead and prevent anxiety-driven over-study.

1. **Single next action.** Each morning the dashboard collapses choice paralysis to **one** "best next
   action" plus the 3 highest points-at-stake topics — no five-app decision (F5, daily loop §1).
2. **Short interleaved review.** She reviews short interleaved blocks seeded by recent misses, never a
   blanket deck pass (F3, F5).
3. **Pacing.** Pacing analytics flag accurate-but-slow topics (Biostatistics at 95 s/question) and
   feed timing into readiness, so a topic counts as ready only when **fast *and* accurate** (F8).
4. **Anti-over-study.** Guardrails pause topics already strong on both memory and performance and call
   diminishing returns, countering the over-study failure mode (F9, Insight 4).
5. **Mobile.** She does focused micro-sessions between classes offline; they sync when she's back
   online (mobile + sync).

*End state:* low cognitive overhead, no over-review spiral, pacing-aware readiness.

### US-4 — Ben Carter (US-MD, secondary user, mid-clerkship): "turn rotations into continuous prep"

*Goal:* integrate longitudinal prep into clinical rotations without manual cross-tool tracking.

1. **Rotation-first.** He studies during clerkships, mostly on his phone during hospital downtime
   (mobile).
2. **Shelf-mode micro-blocks.** He runs shelf-mode QBank micro-blocks and imports them; misses
   **selectively unsuspend only the mapped cards** — never the whole deck (F2, F3, §4.2 workflow).
3. **Stay sharp on high-yield.** Points-at-stake keeps Internal Medicine high-yield fresh ahead of his
   dedicated period (Medicine-last sequencing) (F5).
4. **One source of truth.** Phone reviews sync to the desktop, where he checks the per-topic
   memory↔performance gap weekly (F4, sync).

*End state:* clerkships become continuous, organized prep; one connected workflow instead of 4–5
disconnected tools.

## Tech Stack

Speedrun keeps Anki's layered architecture and adds a Rust `speedrun` engine module, a phone client on
the **same** engine, and a Step 2 dashboard — all glued by the single protobuf contract. AI/eval is a
deferred lane and the product runs fully with AI off.

| Layer | Technology | Role |
|-------|------------|------|
| Desktop client | PyQt6 shell (`aqt`) hosting Svelte/TS web views via the `mediasrv` HTTP server | Main app shell + UI |
| Web UI | Svelte + TypeScript (`ts/`): reviewer, editor, **Step 2 dashboard** (3 scores, coverage, pacing) | Rendered views + dashboards |
| Mobile client | AnkiDroid fork (Kotlin/Java + JNI); iOS via C-FFI/UniFFI (option) | Phone companion on the shared engine |
| Shared engine | Rust `rslib` (crate `anki`) incl. new **`speedrun`** module | Scheduler/FSRS, storage, sync, points-at-stake order, mastery + scores |
| Contract seam | `proto/anki/*.proto` + new `speedrun.proto` (`SpeedrunService`) → prost / PyO3 / TS codegen | One protobuf API across all layers |
| Bridges | PyO3 (`_rsbridge.so`, desktop), JNI (Android), C-FFI/UniFFI (iOS) | Native calls into Rust |
| Data | `collection.anki2` (SQLite) + `col.conf` JSON | Cards/notes/revlog + taxonomy/crosswalk/weights/attempts |
| Resource ingestion (F2) | CSV/JSON/paste importers (UWorld · AMBOSS · NBME/Free120) → `QuestionAttempt`/`PracticeTestResult` | Consolidate performance data (AI-free) |
| Sync | `anki-sync-server` (Rust/Axum), hub-and-spoke over HTTP | Two-way device sync |
| Build & tooling | `just` → n2/ninja → `runner`; Rust 1.92 (cargo), uv/Python, node/yarn, protoc; Briefcase installer | Build, dev, packaging |
| AI / eval (deferred, post-MVP) | `ml/` pipeline: card-gen, held-out eval, baselines, leakage check (behind a flag; off by default) | Friday+ lane D |

**Full architecture diagram (mermaid):** see [`docs/tech_stack.md`](./tech_stack.md).