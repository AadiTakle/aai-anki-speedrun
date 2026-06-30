# Wednesday Slice Plan (Core works, no AI)

Supervisor PLAN artifact for the **Wednesday** deadline (`docs/project_guidelines.md` §6). Defines the
vertical slices (cut along lane seams), the **contract delta** to FREEZE, and the **acceptance-test
charter** per slice. Workflow: `docs/factory_workflow.md`; review/stacking: `docs/review_workflow.md`;
feature states tracked in `docs/feature_ledger.md`.

Grounded in the engine map (see `docs/codebase_notes.md` §2–3). No AI this deadline (Lane D dark).

## 1. Wednesday definition of done (from §6)

Desktop: Anki builds from source ✓ (done); a **real Rust engine change** working end-to-end (diff +
≥3 Rust unit tests + 1 Python test, undo-safe, no corruption); a **review loop** on the exam deck; a
**memory model** with an honest score (a **range** + the **give-up rule**); a Linux **installer** that
runs on a clean machine. Mobile: a phone app that **builds** and runs a **real review session** on the
shared engine with the exam deck (two-way sync NOT required Wednesday). Proof: commit hash +
clean-build recording, test output, clean-install recording, phone-review recording.

**Exam:** USMLE Step 2 CK (3-digit scaled score; readiness calibrated to NBME/UWSA later).

## 2. Design decisions (locked before FREEZE)

- **Our additions live in a new `proto/anki/speedrun.proto` + new `SpeedrunService`**, so upstream
  Anki `.proto` files are touched as little as possible (only one enum variant in
  `deck_config.proto`). This keeps the future upstream merge cheap (guideline 7a "list files touched
  / merge difficulty").
- **No collection schema migration for the MVP.** The topic taxonomy, card→topic crosswalk, and
  per-topic weakness/weights are stored as JSON in the existing `col.conf` (config table). This is
  sync-safe and undo-safe (config changes already flow through the op framework) and matches PRD F1's
  "new SQLite table OR `col.conf` JSON" hook. A dedicated table can replace this post-MVP if needed.
- **Points-at-stake ordering is a Rust post-sort of the gathered review vec** when the new
  `REVIEW_CARD_ORDER_POINTS_AT_STAKE` order is selected. Today only new cards are sorted in Rust
  (`queue/builder/sorting.rs::sort_new`); we add the symmetric `sort_review` path. Due cards are still
  gathered via the existing SQL (`storage/card/mod.rs::for_each_due_card_in_active_decks`), then
  reordered by `blueprint_weight(topic) * weakness(topic)` desc, with the existing `fnvhash` tie-break
  for determinism.
- **Wednesday weakness is seeded via an RPC** (`SetTopicWeights`), not real QBank ingestion — real
  ingestion is F2 (Friday+). This makes F5 independently testable now.
- **Memory score only for Wednesday** (F6 partial). Performance + readiness scores arrive Fri/Sun.
- **Give-up rule (stated):** the memory score **abstains** (returns `abstained=true`, no point
  estimate) until there are **≥200 graded reviews** AND **≥50% topic coverage**. Tunable; frozen in
  the test charter.

## 3. Slices → lanes → jobs

| id | feature (Wed scope) | lane | owns (writes) | depends_on |
|----|---------------------|------|---------------|------------|
| F1 | Topic taxonomy + card→topic crosswalk (config-backed; seeded from AnKing tags) | A/B | `rslib/src/.../speedrun/` (new module), `pylib/anki/` wrapper | — |
| F4 | Per-topic **memory** mastery query (FSRS recall + mastered count per topic) | A | `rslib/src/.../speedrun/` query | F1 |
| F5 | **Points-at-stake / topic-aware review queue** (the mandatory Rust change) | A | `rslib/src/scheduler/queue/builder/`, `storage/card/mod.rs` | F1 |
| F6 | **Memory score** with range + give-up rule | A/C | `rslib/src/.../speedrun/` score, `ts/` dashboard panel | F4 |
| F10 | Exam-deck review loop (import AnKing Step 2 / sample deck; run loop) | B/G | proof scripts, fixtures | F5 |
| F11 | Linux installer that runs on a clean machine | B/F | `tools/` invocation, packaging notes | core green |
| F12 | Mobile: AnkiDroid fork builds + real review session on shared engine | E | AnkiDroid fork pinned to our `rslib`/`proto` | contract |
| F13 | Proof harness: clean-build + install + phone recordings; `just bench` skeleton | G | `docs/`, recordings | all |

(IDs F1/F4/F5/F6 keep the PRD numbering; F10–F13 are Wednesday delivery/proof slices.)

## 4. Contract delta to FREEZE (supervisor-only)

1. **`proto/anki/deck_config.proto`** — add one enum variant to `DeckConfig.Config.ReviewCardOrder`:
   `REVIEW_CARD_ORDER_POINTS_AT_STAKE = 13;` (append; do not renumber existing).
2. **New `proto/anki/speedrun.proto`** — `SpeedrunService` with:
   - `SetTopicWeights(SetTopicWeightsRequest) returns (collection.OpChanges)` — set canonical topics,
     blueprint weights, card→topic crosswalk, and per-topic weakness (stored in `col.conf`).
   - `GetTopicMastery(GetTopicMasteryRequest) returns (TopicMasteryResponse)` — per-topic mastered
     count + average FSRS recall (memory mastery; F4).
   - `GetMemoryScore(GetMemoryScoreRequest) returns (MemoryScore)` — F6.
   - Messages: `Topic{ id, name, blueprint_weight }`, `TopicWeakness{ topic_id, weakness }`,
     `CardTopic{ card_id, topic_id }`, `TopicMastery{ topic_id, mastered, total, avg_recall }`,
     `MemoryScore{ bool abstained, double point, double low, double high, double coverage_pct,
     repeated string reasons, int64 updated_at }`.
3. **Service registration** — wire `SpeedrunService` into the backend service index so codegen emits
   Rust traits + Python/TS bindings (per `anki/docs/language_bridge.md`). Supervisor handles this in
   FREEZE; regenerate with `just check`.

Tag the contract (e.g. `contract/wed-v1`) after `just check` is green on trunk.

## 5. Acceptance-test charter (frozen with the contract)

**F5 — points-at-stake queue (Lane A gate: `just test-rust` + `just check:clippy`):**
- R1: with topics weighted and per-topic weakness seeded, and due cards mapped to topics, the
  points-at-stake order returns review cards sorted by `weight*weakness` descending; deterministic
  tie-break via existing hash.
- R2: cards whose topic has no weight/weakness fall back gracefully (ordered last, default order);
  no panic, no skipped cards.
- R3: undo-safe / no corruption — building + answering through the points-at-stake queue then undo
  restores prior state; `pragma integrity_check` == ok.
- P1 (Python, Lane B gate `just test-py`): via `col` API, `set_topic_weights(...)`, select the
  points-at-stake order, build the queue, assert the first card is the highest points-at-stake card.

**F4 — mastery query (Lane A):** per-topic mastered count + avg recall correct from cards/revlog
joined to the crosswalk; empty topic → zeros; runs without error on a few-hundred-card fixture.

**F6 — memory score + give-up (Lane A + C):** abstains (`abstained=true`, no point) when graded
reviews < 200 OR coverage < 50%; otherwise returns a point estimate with a non-degenerate `[low,high]`
range, `coverage_pct`, populated `reasons`, and `updated_at`. C: dashboard renders the memory score
with its range and the abstain state (vitest + one Playwright check).

**F12 — mobile (Lane E):** AnkiDroid fork builds for an emulator/device and runs a real review
session on the exam deck through the shared `rslib`. Gate: build + recorded review session.

**Integration (supervisor):** `just check` green on trunk after each merge; review-loop smoke; the
Wednesday recordings captured by Lane G.

## 6. Merge-train order (Wednesday)

`contract (proto + enum + service)` → **A** (F1 store → F4 query → F5 queue → F6 score) →
**B** (Python wrappers + review loop) → **C** (dashboard panel) → **E** (mobile) → **F** (installer)
→ **G** (proof). Run `just check` on trunk after each merge.

## 7. Stacking plan (review_workflow §4)

```
main
 └─ feat/F1-topic-crosswalk
      ├─ feat/F4-mastery-query
      │    └─ feat/F6-memory-score        (+ Lane C dashboard panel)
      └─ feat/F5-points-at-stake-queue    (the Rust change; front-of-queue for review)
feat/F12-mobile-ankidroid                 (independent, off the frozen contract)
feat/F11-linux-installer                  (independent, off core-green)
```

Stack-depth budget = 3 (engine is high-risk → F5 and F1 are pushed to the front of the review queue).
Cloud-agent branches use the session prefix while preserving the `feat/<id>-<slug>` intent; the real
id/branch is recorded in `docs/feature_ledger.md`.

## 8. Risks / long poles (start first)

- **Mobile (F12)** is greenfield here (no Android/iOS scaffolding in `anki/`; see
  `docs/codebase_notes.md` §5). AnkiDroid fork + NDK/JNI build is the long pole — kick it off at
  FREEZE, in parallel.
- **Limits × reorder interaction (F5):** due cards are gathered under per-deck limits before the Rust
  post-sort; for Wednesday we reorder the gathered set (limits respected as today) and document the
  behavior. Revisit if it biases high-value cards out of the gathered window.
- **Installer on a clean machine (F11):** validate the Linux tarball in a clean container, not just
  the build host.
