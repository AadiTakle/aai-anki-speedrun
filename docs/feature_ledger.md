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
| F1  | Topic taxonomy + card→topic crosswalk (config-backed) | A/B | `feat/F1-topic-crosswalk` | – | – | planned | – | `wednesday_plan.md` §5 (F1/F4) | – |
| F4  | Per-topic memory mastery query | A | `feat/F4-mastery-query` | – | F1 | planned | – | `wednesday_plan.md` §5 (F4) | – |
| F5  | Points-at-stake / topic-aware review queue (Rust change) | A | `feat/F5-points-at-stake-queue` | – | F1 | planned | – | `wednesday_plan.md` §5 (F5) | – |
| F6  | Memory score with range + give-up rule | A/C | `feat/F6-memory-score` | – | F4 | planned | – | `wednesday_plan.md` §5 (F6) | – |
| F10 | Exam-deck review loop (import + run) | B/G | `feat/F10-review-loop` | – | F5 | planned | – | `wednesday_plan.md` §1 | – |
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
| –    | –       | –    | –       | –          |
