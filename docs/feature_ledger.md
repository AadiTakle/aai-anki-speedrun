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

_No features in progress yet. The first feature is added here when planning the Wednesday slice._

| id | title | lane | branch | pr | depends_on | state | base_commit | test_charter | review_notes |
|----|-------|------|--------|----|------------|-------|-------------|--------------|--------------|
| –  | –     | –    | –      | –  | –          | –     | –           | –            | –            |

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
