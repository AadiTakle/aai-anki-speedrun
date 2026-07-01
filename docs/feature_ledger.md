# Feature Ledger

Single source of truth for the review/rollback workflow (`docs/review_workflow.md`). One row per
feature. The human uses this to pick any `ready-for-review` feature to review; the supervisor keeps
it current as features move through their lifecycle.

## Configuration (decided knobs)

| Knob | Value | Notes |
|------|-------|-------|
| Reviewed/gold trunk | `main` | Deadline checkpoints + installers cut from here. Approved features only. |
| Surge trunk | `dev` | Where the supervisor builds ahead. All completed features, reviewed or not. |
| Review surface | **GitHub PRs** | Repo: `AadiTakle/aai-anki-speedrun`. Local review-packet fallback available. |
| Stack-depth budget | **3** | Max unreviewed features stacked on one unreviewed foundation before preferring breadth. |
| Checkpoint policy | Strict | Wed/Fri/Sun builds come from `main` (approved) unless a `dev` feature is explicitly marked low-risk. |

## Lifecycle states

`planned → in-dev → ready-for-review → in-review → approved` (or `changes-requested → fix-job → ready-for-review`)

## Branch naming

- Feature branch: `feat/<id>-<slug>` (e.g. `feat/F1-rust-queue-skeleton`)
- Stacked feature targets its parent feature's branch; retargets to `main` when the parent is approved.

## Lane key

A = Rust engine · B = Python/Qt desktop · C = Web UI (Svelte) · D = AI/eval · E = Mobile · F = Sync/infra · G = Proof/docs

---

## Active features

Wednesday slice planned (see `docs/wednesday_plan.md`). Contract not yet FROZEN; branches/PRs created
at FAN OUT. `test_charter` points to the relevant section of `docs/wednesday_plan.md` §5.

| id | title | lane | branch | pr | depends_on | state | base_commit | test_charter | review_notes |
|----|-------|------|--------|----|------------|-------|-------------|--------------|--------------|
| F1  | Topic taxonomy + card→topic crosswalk (config-backed) | A/B | `cursor/feat-f1-topic-store-1838` | #7 | – | ready-for-review | `ee35e7e` | `wednesday_plan.md` §5 (F1/F4) | 4 Rust tests green (store) |
| F4  | Per-topic memory mastery query | A | `cursor/feat-f4-mastery-query-1838` | #8 | F1 | ready-for-review | `09c27c1` | `wednesday_plan.md` §5 (F4) | 5 Rust tests green (mastery) |
| F5  | Points-at-stake / topic-aware review queue (Rust change) | A | `cursor/feat-f5-points-at-stake-1838` (+ `feat/f5-weighted-interleave`) | #9 | F1 | ready-for-review | `8717fb6` | `wednesday_plan.md` §5 (F5) | 3 Rust + 1 Py test green. **Redesigned to recency-decayed weighted interleaving** (`fea31a3`, merged to integration): dominant topic leads/recurs, similar topics interleave (no blocking). +1 interleave test; F10 review-loop test updated to assert interleaving. |
| F6  | Memory score with range + give-up rule | A(/C later) | `feat/f6-memory-score` | #10 | F4 | ready-for-review | `ce946bf` | `wednesday_plan.md` §5 (F6) | Engine done: 5 Rust + 1 Py test green; PR #10 stacked on F4. Also verified integrated (`integration/wed-f1-f5` `ec9e7d8`): full `just check` green (537 Rust / 123 py). Lane-C dashboard deferred (Fri). |
| F6c | Memory-score dashboard (range + abstain UI) | C | `feat/f6c-memory-dashboard` | #11 | F6 | ready-for-review | `e373004` | `wednesday_plan.md` §5 (F6, Lane C) | Done: 5 vitest + contract-fidelity test; PR #11 stacked on F6; integrated green (vitest 55/55, svelte/tsc clean). Stretch (viewable page + e2e) deferred → needs Lane-B `mediasrv.py is_sveltekit_page()` allowlist + Qt entry point. |
| F10 | Exam-deck review loop (import + run) | B/G | `feat/f10-review-loop` | #12 | F5 | ready-for-review | `f830d90` | `wednesday_plan.md` §1 | Done: end-to-end pytest (9/9 drain, order, undo, integrity) + demo script; PR #12 stacked on F5. Integrated green — full reconfigured `just check` passes with all Wed features (F1–F6+F6c+F10). |
| F11 | Desktop installer (macOS done; Linux clean-machine pending) | B/F | (on integration) | – | core-green | in-dev | (integration) | `wednesday_plan.md` §1/§5 (F11) | **macOS done:** `just wheels` + Briefcase build/package → ad-hoc-signed `Anki.app` + `anki-26.5-mac-apple.dmg`; repro in `proof/wednesday/INSTALLER.md`. Linux clean-machine verify still pending. |
| F12 | Mobile: iOS app on shared engine (Swift→C FFI→rslib) | E | `feat/f12-mobile-ios` | – | contract | ready-for-review | `df62a28` | `wednesday_plan.md` §5 (F12) | **Foundation done** (merged to integration): rslib cross-compiles for iOS (sim full; device static lib), C-ABI `mobile/speedrun-ffi`, real review session runs Swift→C→rslib (Rust test + Swift CLI), packaged `.xcframework`. Gap: manual Xcode `.app` wiring (documented in `mobile/README.md`). Pivoted AnkiDroid→iOS per host (Mac). |
| F13 | Proof harness: `just bench` (7h) + benchmarks | G | `feat/f13-bench-harness` | – | all | ready-for-review | `2314b93` | `wednesday_plan.md` §1 | **Done** (merged to integration): `bench`-gated 10k-card/20-topic synthetic deck + criterion benches (F5 build_queues ≈9.1ms, F4 mastery ≈7.0ms, F6 score ≈6.9ms — all well under §10) + `just bench` recipe + `proof/wednesday/BENCH.md`. Recordings still manual (need display capture). |

## Completed / approved features

Merged to `main` @ `78b6003` on 2026-07-01 by fast-forwarding `integration/wed-f1-f5`
(the superset of the whole stack + all fixes). Stack PRs #6 (merged) and #7–#12 (closed as
superseded — their commits are on `main`).

| id | title | lane | pr | approved_commit | notes |
|----|-------|------|----|-----------------|-------|
| FREEZE | Wednesday contract: SpeedrunService | – | #6 | on `main` | proto + service + enum |
| F1 | Topic taxonomy + crosswalk store (col.conf, undo-safe) | A/B | #7 | on `main` | + range clamping (review fix) |
| F4 | Per-topic memory mastery query | A | #8 | on `main` | + suspended/buried exclusion + `recall_card_count` (honesty fix) |
| F5 | Points-at-stake review queue (Rust engine change) | A | #9 | on `main` | recency-decayed weighted interleaving |
| F6 | Memory score (range + give-up rule) | A | #10 | on `main` | abstains on <200 reviews / <50% coverage / no recall data |
| F6c | Memory-score dashboard (range + abstain UI) | C | #11 | on `main` | + served page + deck-options dropdown (GUI wiring) |
| F10 | Exam-deck review loop (proof) | B/G | #12 | on `main` | end-to-end pytest + demo |
| F11 | Desktop installer (macOS + Linux, clean-machine verified) | B/F | – | on `main` (+W1 on integration) | macOS `.app`/`.dmg`; **Linux `.tar.zst` built in-container + verified on a clean container (xvfb, AI off) — W1** (`proof/wednesday/linux/`). |
| F12 | Mobile: iOS app on shared engine (FFI) | E | – | on `main` (+W2 on integration) | **Runnable iOS Simulator app done (W2):** XcodeGen project, `.app` builds, recorded review session on `rslib` (`mobile/ios/proof/review_session.mp4`). Device signing/provisioning still out of scope. |
| F13 | Benchmark harness (`just bench`, challenge 7h) | G | – | on `main` | recordings pending |

## Test-integrity & rollback reports

Defects caught in review and any test-integrity escalations are logged here (newest first), per
`docs/review_workflow.md` §7 and `docs/factory_workflow.md` §8.

| date | feature | type | summary | resolution |
|------|---------|------|---------|------------|
| 2026-06-30 | F5 | design-change | Product owner: pure points-at-stake descending sort produced *blocked* practice (all of one topic back-to-back), losing interleaving (a learning-science principle). | ✅ Implemented **recency-decayed weighted interleaving** (`fea31a3`): effective score = `base(topic) × slots_since_topic_last_shown` (base = weight×weakness), reset to 0 on pick; unmapped/zero-base last; deterministic. Dominant topic leads/recurs, similar topics rotate. F10 review-loop test's block assertion rewritten to an interleaving assertion (supervisor-approved intentional behavior change, not a weakening; all drain/undo/integrity assertions kept). `deck_config` enum comment refreshed. Merged to integration. |
| 2026-06-30 | F4/F6, contract | honesty-fix | Resolved deferred item (a): `TopicMastery.avg_recall=0.0` overloaded "no FSRS data" vs a real "0% recall", which could depress the F6 score with an unbacked number (honesty bar). | Appended `TopicMastery.recall_card_count = 5` (proto, non-breaking); F4 populates it; F6 averages recall only over recall-backed covered topics and adds an abstain guard when no covered topic has recall data. +2 tests (speedrun module now 17 Rust). Full reconfigured `just check` green. Applied on integration — propagate to the stack at merge-down (touches freeze/F4/F6). |
| 2026-06-30 | F1/F4/F5 | code-review | Review pass on PRs #6–#9: #6 CHANGES-REQUESTED (enum ownership); #7/#8/#9 MERGE-WITH-NITS. Actionable code nits: F1 no range validation on weight/weakness; F4 counts suspended/buried cards; F5 missing tie-break test. | **Fixed on integration (`0e01634`)**, each with a test, full `just check` green: F1 clamps weight≥0 / weakness∈[0,1] (non-finite→0) at the write boundary; F4 excludes suspended/buried from mastery counts; F5 adds a deterministic tie-break test. Propagate to PR branches #7/#8/#9 before individual merge to `main`. |
| 2026-06-30 | F4/F6, contract | review-deferred | Higher-cost review findings needing a deliberate contract freeze or design decision (NOT yet done). | **Deferred + flagged to human:** (a) ✅ RESOLVED (see honesty-fix row above) — `TopicMastery.avg_recall=0.0` "no data" vs "0% recall"; (b) `GetMemoryScore(Empty)`→dedicated request msg to avoid re-freeze; (c) move `REVIEW_CARD_ORDER_POINTS_AT_STAKE` enum ownership from F5 into the freeze (#6) at merge-down; (d) F5 gather-time selection so limit doesn't drop high-value cards (documented scope cut, `wednesday_plan.md §8`); (e) F4 N+1 card fetch → batch for the 50k-card 7a target. |
| 2026-06-30 | F1/F4/F5/F6 | fmt-gate | `just fmt`/ruff deviations lurked in `store.rs`, `mastery.rs`, `queue/builder/mod.rs`, `score.rs` (format gate never run per-branch during dev, only `just build`/`just test`). | Formatting-only normalization applied on the integration trunk (`ec9e7d8`); no logic/assertion changes. Propagate the same fmt fix to PR branches #7/#8/#9/#10 before individual merge to `main` (batched follow-up). |
