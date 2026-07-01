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
| F5  | Points-at-stake / topic-aware review queue (Rust change) | A | `cursor/feat-f5-points-at-stake-1838` | #9 | F1 | ready-for-review | `8717fb6` | `wednesday_plan.md` §5 (F5) | 3 Rust + 1 Py test green |
| F6  | Memory score with range + give-up rule | A(/C later) | `feat/f6-memory-score` | #10 | F4 | ready-for-review | `ce946bf` | `wednesday_plan.md` §5 (F6) | Engine done: 5 Rust + 1 Py test green; PR #10 stacked on F4. Also verified integrated (`integration/wed-f1-f5` `ec9e7d8`): full `just check` green (537 Rust / 123 py). Lane-C dashboard deferred (Fri). |
| F6c | Memory-score dashboard (range + abstain UI) | C | `feat/f6c-memory-dashboard` | – | F6 | in-dev | `ce946bf` (on F6) | `wednesday_plan.md` §5 (F6, Lane C) | Lane-C worker dispatched (vitest + Svelte; e2e stretch) |
| F10 | Exam-deck review loop (import + run) | B/G | `feat/f10-review-loop` | – | F5 | in-dev | `8717fb6` (on F5) | `wednesday_plan.md` §1 | Lane-B/G worker dispatched (pytest review loop + demo script) |
| F11 | Linux installer (clean-machine) | B/F | `feat/F11-linux-installer` | – | core-green | planned | – | `wednesday_plan.md` §5 (F11) | – |
| F12 | Mobile: AnkiDroid fork builds + review session | E | `feat/F12-mobile-ankidroid` | – | contract | planned | – | `wednesday_plan.md` §5 (F12) | – |
| F13 | Proof harness: recordings + `just bench` skeleton | G | `feat/F13-proof-harness` | – | all | planned | – | `wednesday_plan.md` §1 | – |

## Completed / approved features

| id | title | lane | pr | approved_commit | notes |
|----|-------|------|----|-----------------|-------|
| –  | –     | –    | –  | –               | –     |

## Test-integrity & rollback reports

Defects caught in review and any test-integrity escalations are logged here (newest first), per
`docs/review_workflow.md` §7 and `docs/factory_workflow.md` §8.

| date | feature | type | summary | resolution |
|------|---------|------|---------|------------|
| 2026-06-30 | F1/F4/F5 | code-review | Review pass on PRs #6–#9: #6 CHANGES-REQUESTED (enum ownership); #7/#8/#9 MERGE-WITH-NITS. Actionable code nits: F1 no range validation on weight/weakness; F4 counts suspended/buried cards; F5 missing tie-break test. | **Fixed on integration (`0e01634`)**, each with a test, full `just check` green: F1 clamps weight≥0 / weakness∈[0,1] (non-finite→0) at the write boundary; F4 excludes suspended/buried from mastery counts; F5 adds a deterministic tie-break test. Propagate to PR branches #7/#8/#9 before individual merge to `main`. |
| 2026-06-30 | F4/F6, contract | review-deferred | Higher-cost review findings needing a deliberate contract freeze or design decision (NOT yet done). | **Deferred + flagged to human:** (a) `TopicMastery.avg_recall=0.0` overloads "no data" vs "0% recall" — needs a `recall_card_count` proto append + F6 to stop counting unbacked zeros (honesty bar); (b) `GetMemoryScore(Empty)`→dedicated request msg to avoid re-freeze; (c) move `REVIEW_CARD_ORDER_POINTS_AT_STAKE` enum ownership from F5 into the freeze (#6) at merge-down; (d) F5 gather-time selection so limit doesn't drop high-value cards (documented scope cut, `wednesday_plan.md §8`); (e) F4 N+1 card fetch → batch for the 50k-card 7a target. |
| 2026-06-30 | F1/F4/F5/F6 | fmt-gate | `just fmt`/ruff deviations lurked in `store.rs`, `mastery.rs`, `queue/builder/mod.rs`, `score.rs` (format gate never run per-branch during dev, only `just build`/`just test`). | Formatting-only normalization applied on the integration trunk (`ec9e7d8`); no logic/assertion changes. Propagate the same fmt fix to PR branches #7/#8/#9/#10 before individual merge to `main` (batched follow-up). |
