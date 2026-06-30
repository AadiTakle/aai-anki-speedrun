# Review & Rollback Workflow (Stacked PRs + Patch Queue)

Companion to `docs/factory_workflow.md`. The factory doc says *how features get built* (lanes,
contract freeze, TDD). This doc says *how features get reviewed and corrected* without ever
stalling forward development.

Durable context: future edits should follow this.

## 1. Goals

1. **Continuous surge.** Development never blocks waiting for human review.
2. **Per-feature review.** Every feature push is a self-contained PR the human can review on its own.
3. **Review the past, anytime.** The human can come back to any older completed feature and review it while new (possibly dependent) features are already in progress.
4. **Safe rollback/fix.** If the human flags a defect in a completed feature, it can be fixed or rolled back, and the fix **propagates** through every feature that was stacked on top of it.

## 2. Two-track model

> **Concrete branch names in this repo:** the reviewed/gold trunk is the existing **`main`** branch
> (so wherever this doc says `reviewed/main`, the actual branch is `main`), and the surge trunk is
> **`dev`**. Both are bootstrapped and pushed to `origin`.

Two long-lived pointers, so review state is decoupled from dev progress:

```
 reviewed/main  ──●────●─────────●───────────────►   (human-APPROVED only; gold)
                  F1   F2        F5
                          \
 dev (surge)    ──●────●───●────●────●────●────●──►   (everything built; may be unreviewed)
                  F1   F2  F3   F4   F5   F6   F7
                                ▲
                          I keep building here
```

- **`reviewed/main`** — contains only features the human has approved. This is the "trustworthy" line used for demos, installers, and deadline checkpoints.
- **`dev`** — where the supervisor surges. Holds every completed feature, reviewed or not, in dependency order.

A feature being unreviewed never stops me from building the next one on `dev`.

## 3. A feature IS a PR — lifecycle state machine

Each feature is one branch + one PR with a tracked state:

```
planned → in-dev → ready-for-review → in-review → approved ──► merges to reviewed/main
                                          │
                                          └─► changes-requested ──► fix-job (§7) ──► back to ready-for-review
```

| State | Meaning | Who advances it |
|-------|---------|-----------------|
| `planned` | Scoped in the ledger; contract + test charter defined | supervisor |
| `in-dev` | Lane worker building it (TDD, isolated worktree) | worker |
| `ready-for-review` | TDD-green, gated, PR opened, landed on `dev`, in review queue | supervisor |
| `in-review` | Human is looking at it | human |
| `approved` | Human approved → fast-forwarded onto `reviewed/main` | human → supervisor |
| `changes-requested` | Human found a defect → triggers fix + propagation | human → supervisor |

A feature reaches `ready-for-review` only after passing the factory's gates (§7 of factory doc) and
the strict test-integrity rule (§8). **Review is about product/design correctness, not "do the tests
pass" — that's already guaranteed.**

## 4. Stacking for dependencies (how I surge on dependent work)

When F4 depends on F3, F4's branch is **stacked on F3's branch**, not on `reviewed/main`. So I keep
building the dependency chain before any of it is reviewed.

```
reviewed/main
   └─ feat/F3-topic-weights        (PR #3 → targets reviewed/main)
        └─ feat/F4-points-queue    (PR #4 → targets feat/F3)   ← diff shows ONLY F4's delta
             └─ feat/F5-dashboard  (PR #5 → targets feat/F4)
```

- Each stacked PR **targets its parent feature's branch**, so the human reviews only that feature's
  incremental diff — not a giant blob.
- When a parent is approved and merged to `reviewed/main`, the child PR is **auto-retargeted** to
  `reviewed/main` and restacked (`git rebase --update-refs`).
- The ledger records `depends_on` for every feature, so the dependency graph is explicit and
  machine-checkable.

## 5. The ledger — single source of truth

A tracked file `docs/feature_ledger.md` (table or JSON) is the index the human uses to "come back"
to any feature. One row per feature:

| field | example |
|-------|---------|
| `id` | F4 |
| `title` | Points-at-stake review queue |
| `lane` | A (engine) |
| `branch` | `feat/F4-points-queue` |
| `pr` | #4 |
| `depends_on` | [F3] |
| `state` | ready-for-review |
| `base_commit` | `<hash on dev>` |
| `test_charter` | link to frozen acceptance tests |
| `review_notes` | (human fills) |

The human opens the ledger, picks **any** feature in `ready-for-review` (newest or oldest — order is
their choice), and reviews its PR. The **review queue** is just "all rows where state =
ready-for-review," sortable by dependency depth so foundational features can be reviewed first.

## 6. Staying unblocked (async review)

- I push each finished feature to `dev`, open its PR, set `ready-for-review`, and **immediately start
  the next feature**. I never `AwaitShell` on a human review.
- When the human approves or requests changes (a notification / message to me), I handle it in the
  **next supervisor loop tick** — interleaved with ongoing dev, not blocking it.
- Approvals are processed in dependency order: approving F3 fast-forwards it to `reviewed/main` and
  retargets F4's PR.

## 7. Change-requested / rollback / fix propagation (the hard part)

When the human flags a defect in an already-completed feature (say **F3**, which F4 and F5 are
stacked on), the supervisor runs the **patch-queue restack**:

```
1. MARK      Ledger: F3 → changes-requested. Compute downstream set from depends_on
             graph = {F4, F5}. Freeze new dev branches that would extend this stack.

2. REPRODUCE Spawn a fix-job on feat/F3 (TDD-FIRST): write a FAILING test that
             reproduces the reported defect (red), confirming the bug is real and
             captured. This new test joins F3's charter.

3. FIX       Implement on feat/F3 until the new test + all existing F3 tests are green.
             (Test-integrity rule still applies — no weakening existing tests.)

4. RESTACK   git rebase --update-refs propagates the updated F3 down the stack:
             feat/F4 and feat/F5 are replayed on the new F3.

5. RE-GATE   Re-run each downstream feature's gate (just test-rust / test-py / vitest…)
             on its restacked branch. Any feature that no longer passes is flagged
             and gets its own small fix-job.

6. REPORT    Tell the human: what F3 fix was, which downstream features were
             restacked, which needed follow-up fixes, and the new test that locks
             the bug out forever. F3 returns to ready-for-review (re-review just the
             fix delta).
```

### Rollback variants

- **Soft fix (default):** patch + restack as above. The feature stays; the bug-reproducing test
  prevents regression.
- **Hard rollback (wrong direction):** if a feature must be removed entirely, `git revert` its merge
  on `reviewed/main` (if already approved) or drop its branch from the stack, then restack dependents.
  **Dependents that are now logically invalid are flagged to the human**, not silently mutated — a
  dependent built on a removed foundation may need rescoping, which is a human decision.
- **Conflict during restack:** the supervisor resolves mechanical conflicts; if a conflict changes
  behavior or acceptance criteria, it's escalated to the human (same spirit as the test-integrity
  escalation in factory §8).

### Why a bug always becomes a test first

Per the rubric's honesty bar, every human-caught defect is converted into a **failing test before the
fix** (step 2). This means the human's review effort is permanently encoded — the same bug can't
silently come back, and the fix is proven, not asserted.

## 8. Surge guardrails (so I don't over-build on shaky ground)

Surging ahead on unreviewed, dependent features is powerful but risky: if a foundational feature is
rejected, everything stacked on it churns. Controls:

- **Review-priority by fan-in.** Features that many others depend on (contract-level, engine,
  schema) are pushed to the **front of the review queue** so the human reviews them early, before the
  stack on top gets deep.
- **Stack-depth budget.** A tunable max number of *unreviewed* features that may be stacked on a
  single unreviewed foundation (default: **3**). Past that, I prefer breadth (independent features in
  other lanes) over extending the risky stack, and I nudge the human to review the foundation.
- **Risk tiering.** High-risk lanes (engine, sync, AI) get smaller stacks; low-risk leaf features
  (docs, UI polish) can stack freely.
- **Contract changes are always reviewed first.** Because `proto/` is the seam everything depends on,
  any feature that changes the contract is flagged top-priority for review.

## 9. Git mechanics & commands

```bash
# Start a stacked feature (F4 on F3)
git checkout feat/F3-topic-weights
git checkout -b feat/F4-points-queue
# ... worker builds in an isolated worktree ...

# Publish for review
git push -u origin feat/F4-points-queue
gh pr create --base feat/F3-topic-weights --head feat/F4-points-queue \
  --title "F4: points-at-stake review queue" --body "<what/why/test plan/demo>"

# Human approves F3 → advance reviewed/main and restack the children
git checkout reviewed/main && git merge --ff-only feat/F3-topic-weights
gh pr edit 4 --base reviewed/main          # retarget child
git rebase --update-refs reviewed/main     # restack F4, F5...

# Human reports a bug in F3 → fix + propagate
git checkout feat/F3-topic-weights
#   (fix-job: failing test first, then fix)
git rebase --update-refs feat/F3-topic-weights   # propagate down the stack
# re-run each downstream gate, report impact
```

`--update-refs` is what makes the whole stack move as a unit when a base feature changes.

> Note: PRs live in the user's repo (`AadiTakle/aai-anki-speedrun`). If GitHub PRs aren't desired,
> the same model works locally with branches + a "review packet" (diff + test output + demo notes)
> per feature; only the review surface changes, not the mechanics.

## 10. How this composes with the factory

- A **feature** is the unit a lane produces; the factory builds it (TDD, isolated worktree, gates),
  this workflow reviews and corrects it.
- `dev` is the surge trunk the factory fans out from; `reviewed/main` is what deadline checkpoints
  (Wed/Fri/Sun) and installers are cut from.
- The **frozen test charter** (factory §7) is what a feature's tests are pinned to; human review can
  *amend* that charter (a reviewed change), which then re-freezes — and downstream features restack.
- The **test-integrity escalation** (factory §8) and this doc's **rollback escalation** share one
  rule: mechanical fixes are automatic; anything touching scope or acceptance criteria goes to the
  human.

## 11. Why this is the best design here

1. **Throughput + correctness, decoupled.** Stacked PRs let me build dependent features now; async
   review lets the human verify on their own schedule. Neither waits on the other.
2. **Reviewable units.** Each PR shows only its own delta, so even deep dependency chains stay
   reviewable feature-by-feature.
3. **Fixes propagate deterministically.** `--update-refs` restacking + per-feature gates mean a fix
   to a foundation is *provably* re-validated through everything built on it — not hand-waved.
4. **Bugs become permanent tests.** Every human-caught defect is locked out by a regression test
   before the fix lands, matching the rubric's honesty bar.
5. **Bounded blast radius.** Surge guardrails (fan-in priority, stack-depth budget, risk tiering)
   keep an early rejection from cascading into massive rework.

### Rejected alternatives
- **Block dev on review:** safest, but kills the surge — fails the core requirement.
- **One giant integration branch, review at the end:** no per-feature review; a late-found
  foundational bug is catastrophic to untangle.
- **Independent (non-stacked) feature branches off main:** can't express dependencies, so dependent
  features can't be built until their parent is merged — re-blocks the surge.

## 12. Open tuning knobs
- **Stack-depth budget** (default 3 unreviewed deep) — raise for low-risk lanes, lower for engine/sync.
- **Review surface** — real GitHub PRs (default) vs local review packets.
- **Checkpoint policy** — whether deadline builds must come strictly from `reviewed/main` or may
  include `dev` features marked low-risk.
