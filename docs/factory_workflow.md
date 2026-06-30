# Factory Workflow: Supervised Multi-Agent Development

How development on this repo is organized. The supervisor (the orchestrating agent) decomposes
work along the codebase's natural seams, freezes a contract, fans out isolated worker subagents,
and integrates their output through a disciplined merge train. **All work is test-driven (TDD)
with a strict test-integrity rule (§8).**

This document is durable context: future edits should follow it. For how completed features are
reviewed, rolled back, and how dependent work surges ahead of human review, see
`docs/review_workflow.md`.

---

## 1. Core insight

Anki is a layered system glued by **one explicit contract: `anki/proto/anki/*.proto`**. Every
cross-layer call is `run_service_method(service, method, protobuf_bytes)`. If the contract is
frozen first, the Rust engine, Python/Qt desktop, Svelte web UI, AI pipeline, mobile client, and
sync infra can all be built **in parallel against a stable interface**.

Prime directive: **contract-first, then fan out.**

---

## 2. Design principles

1. **Seams over files.** Agents own disjoint architectural seams (directory sets), never the same file concurrently.
2. **Isolation by default.** Each worker runs in its own git worktree + branch (via `best-of-n-runner`) with its own build dir. Zero file contention.
3. **The contract is sacred and centrally owned.** Only the supervisor edits `proto/`, `Cargo.lock`, `package.json`/`yarn.lock`, and other cross-cutting lockfiles.
4. **Test-driven and self-verifying.** Tests are written/confirmed *before* implementation. A lane is not "done" until its tests are green (§7, §8).
5. **Async supervision.** Workers run in the background and notify on completion; the supervisor stays unblocked.

---

## 3. Lanes (org chart)

```
                         ┌──────────────────────────┐
                         │   SUPERVISOR              │
                         │  - owns proto/ contract   │
                         │  - owns merge train       │
                         │  - owns lockfiles + CI    │
                         │  - owns the test charter  │
                         └─────────────┬─────────────┘
                                       │ dispatch (isolated worktrees)
   ┌───────────┬───────────┬──────────┼──────────┬───────────┬───────────┐
   ▼           ▼           ▼           ▼          ▼           ▼           ▼
 LANE A      LANE B      LANE C      LANE D     LANE E      LANE F      LANE G
 Rust        Python/     Web UI      AI +       Mobile      Sync /      Proof /
 engine      Qt desktop  (Svelte)    eval       (AnkiDroid  infra       docs /
 rslib/      pylib/+qt/  ts/         ml/+ai/    or FFI)     rslib/sync  recordings
```

| Lane | Owns (writes) | Reads only |
|------|---------------|------------|
| A — Engine | `rslib/src/scheduler/`, `rslib/src/storage/` | `proto/` |
| B — Desktop | `pylib/anki/`, `qt/aqt/` | `proto/`, generated bindings |
| C — Web UI | `ts/` | `proto/` |
| D — AI/eval | new `ml/` or `ai/` dir + scripts | held-out data, `proto/` |
| E — Mobile | `mobile/` (FFI crate) or the AnkiDroid fork | `rslib/`, `proto/` |
| F — Sync/infra | `rslib/sync/`, docker, server config | `proto/` |
| G — Proof | `docs/`, recordings, benchmark harness | everything (read-only) |

---

## 4. Unit of work: an isolated job

Every dispatch is a job spec handed to a `best-of-n-runner` subagent (own branch + worktree):

```
Job:
  lane:        A (engine)
  branch:      lane/engine/points-at-stake-queue
  scope:       ONLY rslib/src/scheduler/queue/, rslib/src/storage/card/mod.rs
  contract:    proto frozen at commit <hash>
  tests-first: write/confirm the failing tests BEFORE implementation (§7)
  deliverable: impl that makes the agreed tests pass + undo-safe (OpChanges)
  gate:        `just test-rust` green, `just check:clippy` clean
  return:      diff summary + FULL test output + files touched + any test-integrity reports (§8)
  forbidden:   editing proto/, lockfiles, anything outside scope,
               weakening/deleting tests (§8)
```

"Wholly separate parts" holds because scope is enforced by both directory ownership and physical
worktree isolation.

---

## 5. The supervisor loop (per deadline cycle: Wed → Fri → Sun)

```
0. PLAN        Decompose deadline into vertical slices; cut each along lane seams.
               Define the contract delta AND the acceptance tests for each slice.

1. FREEZE      Supervisor edits proto/ + lockfiles, regenerates bindings, commits +
               tags the contract. Also commits/agrees the acceptance test charter.
               ── the only serialized step; everything downstream parallelizes ──

2. FAN OUT     Dispatch N background jobs, each pinned to the frozen contract,
               each in its own worktree, each TDD (tests first).

3. MONITOR     Stay free. Completion notifications arrive per lane. On a blocker:
               (a) resume that agent with guidance, or (b) adjust contract + re-freeze.
               Test-integrity reports (§8) are triaged here and escalated to the human.

4. GATE        Each returned lane must be self-green. Bugbot / security-review on
               risky lanes (engine, sync, AI input).

5. MERGE TRAIN Integrate in dependency order: contract → engine (A) → binding
               consumers (B, C, E) → AI (D) → infra (F) → proof (G).
               Run `just check` on trunk after each merge.

6. VERIFY      Cross-cutting checks no single lane can do alone: two-app sync test
               (7b), benchmark (7h), crash/offline (7g).

7. ITERATE     Failures spawn small targeted fix-jobs (resume where possible).
               Loop back to 2 for the next slice/deadline.
```

Only **step 1 (freeze)** and **step 5 (merge train)** are serial. All expensive work runs parallel
in step 2.

---

## 6. Conflict avoidance

- **`proto/` churn** → supervisor-only; frozen + tagged before fan-out.
- **Generated code** (`out/`, `_backend_generated.py`, `backend.ts`) → never committed by workers; regenerated deterministically during the merge train.
- **`Cargo.lock` / `yarn.lock`** → supervisor-owned. Workers file a "dep request" in their return message; supervisor batches into the freeze step.

---

## 7. TDD discipline (how every lane works)

Every lane follows red → green → refactor, and the **acceptance tests are agreed up front** so
"done" is objective, not vibes.

1. **Tests first.** Before implementing, the worker writes (or confirms supervisor-provided)
   tests that encode the slice's acceptance criteria. These tests must **fail first** for the right
   reason (red), proving they actually exercise the new behavior.
2. **Implement to green.** The worker writes the minimum code to make those tests pass, plus the
   lane gate (`just test-rust` / `test-py` / `vitest` / eval cutoff).
3. **Refactor under green.** Cleanup only while tests stay green.
4. **Return the evidence.** The worker returns the **full test output** (not a summary claim) so the
   supervisor can confirm red→green actually happened.
5. **Test charter is part of the contract.** Just like `proto/`, the agreed acceptance tests for a
   slice are frozen by the supervisor at FREEZE. A lane cannot quietly redefine its own success
   criteria.

Per-lane gates:

| Lane | Gate |
|------|------|
| A — Engine | `just test-rust` + `just check:clippy`; undo-safe / no corruption assertions |
| B — Desktop | `just test-py` (Python tests calling into Rust) |
| C — Web UI | `vitest` + Playwright e2e |
| D — AI/eval | held-out accuracy + leakage check pass a **pre-registered** cutoff |
| E — Mobile | device/emulator build + a real review session |
| Integration | `just check` on trunk, then sync/benchmark/crash tests |

---

## 8. Test-integrity rule (STRICT — non-negotiable)

The point of TDD here is that **passing tests are a trustworthy signal of progress.** That only
holds if tests are not quietly weakened to go green. Therefore:

### Hard rules for every worker subagent

- **You may NOT edit, weaken, skip, `#[ignore]`, comment out, or delete a test to make it pass.**
  If your change breaks a test, the default assumption is **your code is wrong**, not the test.
- **You may NOT loosen an assertion** (e.g. widening a tolerance, removing a case, changing an
  expected value) to accommodate your implementation.
- **A red test that you cannot make green within your scope is a STOP condition**, not a license to
  modify the test. Stop and report back to the supervisor.

### The ONLY permitted reason to change a test

A test may be changed **only** when it is genuinely **outdated/incorrect** — i.e. the project is
being developed correctly per the agreed spec, but the test encodes a stale or wrong expectation.
Even then, the worker does **not** unilaterally change it. The worker must:

1. **Stop** and not touch the test yet.
2. **Report a test-integrity report** to the supervisor containing:
   - the test name + file path,
   - why it is believed outdated (cite the spec / contract / acceptance criteria),
   - the proposed change, and
   - evidence the implementation is otherwise correct (other green tests, behavior trace).
3. **Wait for supervisor approval.**

### Escalation chain

```
Worker hits a failing test it believes is outdated
        │  (does NOT edit the test)
        ▼
Supervisor receives the test-integrity report, evaluates against the frozen
contract + test charter
        │
        ├─ clearly the worker's bug      → bounce back: fix the code, test stays
        ├─ genuinely outdated, low-risk  → supervisor approves + amends the
        │                                   frozen test charter, re-freezes
        └─ ambiguous / changes scope or  → ESCALATE TO THE HUMAN with the report,
           acceptance criteria              do not proceed until resolved
```

**Any change to a frozen acceptance test that affects scope, acceptance criteria, or the meaning of
a deliverable is escalated to the human before it is applied.** Test-integrity reports are surfaced
in the supervisor's status updates so they are never silently absorbed.

### Why this strictness

The grading rubric rewards *honest, re-runnable* evidence and penalizes misleading results
(guideline §11: leaked test data zeroes a score; made-up numbers are an automatic fail). A worker
that edits a failing test to look green is the software equivalent of a made-up number. The
test-integrity rule makes that path structurally unavailable.

---

## 9. Mapping to deadlines

- **Wednesday (core, no AI):** Lanes A, B, C, E, F run in parallel off one freeze. Lane D is dark (no model calls). Each lands TDD-green.
- **Friday (AI + sync):** Lane D activates with eval/baseline/leakage gates; E adds two-way sync; AI wired behind a feature flag (must run with AI off).
- **Sunday (prove + ship):** Lane G dominates — calibration charts, the 3-build ablation, installers, recordings — consuming frozen, test-green output from all lanes.

---

## 10. Why this is the best design here

1. **Exploits Anki's protobuf seam** — turns a multi-language monorepo into ~7 independently buildable, independently testable units.
2. **Worktree isolation** eliminates the dominant multi-agent failure mode (agents clobbering a shared tree).
3. **Async fan-out** matches deadline pressure; long poles (mobile, FSRS optimization, eval runs) run concurrently from hour one.
4. **TDD + strict test-integrity** make "green" a trustworthy progress signal and keep the evidence honest, which is exactly what the rubric grades.
5. **Graceful degradation** — a blocked/failing lane is an isolated branch, not a stalled trunk.

### Rejected alternatives
- **Single sequential agent:** too slow for Wed/Fri/Sun.
- **Many agents on a shared tree:** constant merge breakage on `proto/`, generated code, lockfiles.
- **Pure vertical slices (full-stack per agent):** every agent competes for the same files across layers. The hybrid (vertical *planning*, horizontal *execution* behind a frozen contract + frozen test charter) wins.

---

## 11. Open tuning knob

The **mobile lane (E)**: AnkiDroid-fork vs. a new in-tree FFI crate changes whether E is a
separate-repo lane or a workspace-crate lane, which affects its isolation model and merge-train
position. Lock this before the first freeze. Current lean: AnkiDroid for Android speed.
