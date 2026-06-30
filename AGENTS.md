# AGENTS.md

This repo (`AadiTakle/aai-anki-speedrun`) is a fork of [Anki](https://github.com/ankitects/anki)
(vendored under `anki/`) being built into **"Speedrun"**, a desktop + mobile study app for **one
exam: USMLE Step 2 CK**. Read this file first, then the canonical docs in `docs/` before doing any
work.

## Canonical docs (read before acting)

| Doc | What it defines |
|-----|-----------------|
| `docs/project_guidelines.md` | The mission, the hard rules, deadlines (Wed/Fri/Sun), graded challenges (7a–7h), grading rubric + hard limits. **The spec.** |
| `docs/PRD.md` | The MVP for Step 2 CK: scope, the 9 features (F1–F9) with priorities + build order, the daily study loop, the user persona. **What we build first.** |
| `docs/brainlift_v1.md` | The thesis (SPOV) + evidence base every feature must serve. |
| `docs/factory_workflow.md` | **How work is built:** supervisor + lane workers, contract-first proto freeze, TDD, isolated worktrees, merge train. |
| `docs/review_workflow.md` | **How work is reviewed/corrected:** `main` vs `dev`, stacked PRs, fix propagation. |
| `docs/feature_ledger.md` | Live single-source-of-truth table of every feature + its state. Keep it current. |
| `docs/codebase_notes.md` | Map of the Anki codebase → where each requirement lands (engine, proto seam, sync, mobile). |

If anything here conflicts with `docs/`, the docs win for product/workflow decisions; this file's
**Cursor Cloud specific instructions** win for environment/setup mechanics.

## Project goals (the rules that cannot break)

From `docs/project_guidelines.md` §2 and §11 — internalize these; they are graded:

- **Real Rust engine change**, not just Python/Qt screens (MVP target: F5 points-at-stake /
  topic-aware queue in `anki/rslib/`), exposed via a new `proto/` RPC, with ≥3 Rust tests + 1
  Python test, undo-safe, no collection corruption.
- **Two apps, one shared engine, that sync** — desktop (PyO3) + phone (AnkiDroid or FFI), both
  running the same `rslib`. No rewriting the scheduler in JS/Swift.
- **Three separate scores — memory, performance, readiness — each shown as a range**, never one
  blended number, each with reasons, last-updated, coverage %, and an explicit **give-up / abstain
  rule** when data is insufficient.
- **Honesty bar (auto-fail if broken):** never show a made-up/unbacked readiness number; never let
  test data leak into training (zeroes that score). Honest "we can't score yet" beats a pretty
  guess.
- **Held-out, re-runnable evaluation;** AI outputs trace to a named source, beat a simple baseline,
  and pass a **pre-registered** cutoff before users see them.
- **Study feature tested by ablation** (full app vs feature-off vs plain Anki, equal study time).
- **Both apps ship installable and run with AI switched off.** AI is **out of scope for the MVP**
  (no card-gen, chatbot, or semantic search yet) — the core must give a score with AI off.
- **License:** AGPL-3.0-or-later, crediting Anki (some parts BSD-3-Clause). Keep `anki/LICENSE`.

The guiding SPOV (`docs/brainlift_v1.md`): *the barrier for capable Step 2 students is information
**organization**, not knowledge.* Every feature must consolidate/link/focus the proven resources
(QBanks, practice tests, targeted flashcards) to recover the ~33% wasted study time.

## How I operate: the factory (supervisor model)

I act as the **supervisor** in `docs/factory_workflow.md`. Prime directive: **contract-first, then
fan out.**

1. **PLAN** — decompose the current deadline slice along lane seams; define the `proto/` delta and
   the acceptance tests up front.
2. **FREEZE** (only serial step I own) — edit `anki/proto/`, lockfiles (`Cargo.lock`,
   `yarn.lock`/`package.json`), regenerate bindings, commit + tag the contract and the test charter.
3. **FAN OUT** — dispatch background **lane workers** as `best-of-n-runner` subagents, each in its
   own git worktree + branch, each pinned to the frozen contract, each **TDD (tests first, must fail
   for the right reason before implementing)**.
4. **MONITOR** async — stay unblocked; resume workers on blockers or re-freeze the contract.
5. **GATE** — each lane returns self-green with **full test output** (not a summary).
6. **MERGE TRAIN** — integrate in dependency order: contract → A (engine) → B/C/E (binding
   consumers) → D (AI/eval) → F (infra) → G (proof); run `just check` on trunk after each merge.
7. **VERIFY** cross-cutting: two-app sync (7b), benchmark (7h), crash/offline (7g).

**Lanes & ownership (workers own disjoint seams, never the same file):**
A = engine `anki/rslib/{scheduler,storage}` · B = desktop `anki/{pylib/anki,qt/aqt}` ·
C = web UI `anki/ts` · D = AI/eval (new `ml/`/`ai/`) · E = mobile (`mobile/` FFI or AnkiDroid
fork) · F = sync/infra `anki/rslib/sync` + docker · G = proof `docs/` + recordings/benchmarks.
**Only the supervisor edits `anki/proto/`, lockfiles, and generated code.**

### Test-integrity rule (STRICT, non-negotiable — `docs/factory_workflow.md` §8)

A worker may **never** edit, weaken, skip, `#[ignore]`, comment out, or delete a test to make it
pass. A red test it can't make green in scope is a **STOP-and-report** condition. The only path to
changing a test is a **test-integrity report** to the supervisor (cite spec, propose change, prove
the impl is otherwise correct) and supervisor approval; anything touching scope/acceptance criteria
escalates to the human. Passing tests must stay a trustworthy signal — this mirrors the rubric's
honesty bar.

## How work is reviewed: stacked PRs (`docs/review_workflow.md`)

- **Two trunks:** `main` = gold/reviewed (deadline checkpoints + installers cut from here, approved
  features only); `dev` = surge trunk (every completed feature, reviewed or not). Both exist on
  `origin`.
- **A feature = one branch + one PR**, tracked in `docs/feature_ledger.md`
  (`planned → in-dev → ready-for-review → in-review → approved`). Branch name `feat/<id>-<slug>`;
  dependent features **stack** on their parent's branch so each PR shows only its own delta.
- **Never block `dev` on human review.** Push the feature, open its PR, set `ready-for-review`,
  start the next. Approvals fast-forward onto `main` and retarget/restack children.
- **Every human-caught defect becomes a failing test first**, then the fix; propagate down the
  stack with `git rebase --update-refs` and re-gate each dependent.
- **Surge guardrails:** stack-depth budget = 3 unreviewed deep; contract/engine/sync features get
  front-of-queue review and smaller stacks.
- Keep `docs/feature_ledger.md` updated as the single source of truth as features move state.

> Cloud-agent note: when running as a Cursor cloud agent, the session may require a branch prefix
> (e.g. `cursor/…`). Preserve the `feat/<id>-<slug>` intent in the branch name and record the real
> feature id/branch in the ledger.

## Skills & rules

Repeatable SOPs live as project rules in `.cursor/rules/*.mdc` (auto-attached by Cursor) and are
indexed in `docs/skills.md`. Consult them when relevant:

- `add-backend-rpc.mdc` — the exact recipe to add/change a backend RPC or proto message across
  Rust↔Python↔TS (undo-safe, with regen + test steps). Use for any `SpeedrunService` work.
- `factory-lane-worker.mdc` — the TDD loop, scope/isolation, strict test-integrity rule, and per-lane
  gates, condensed for every implementer.
- `proof-and-eval-tooling.mdc` — specs for the benchmark (7h), eval/calibration, leakage (7e), and
  crash/offline (7g) tooling (deps land at the owning lane's FREEZE).

For the **iOS lane on a Mac**, use the `xcode-project-setup` plugin skill (SPM/Xcode wiring; macOS +
Swift only). See `docs/skills.md` for the plugin-skill index.

## Cursor Cloud specific instructions

- **Run everything from `anki/`.** All build/run/test/lint commands live there and are driven by
  `just` (recipes wrap a custom ninja/`runner` build system). See `anki/CLAUDE.md` and
  `anki/docs/development.md` for the canonical command reference; `just --list` shows all recipes.
  `docs/codebase_notes.md` maps the codebase and has a quick command reference.
- **Common commands (from `anki/`):** `just build` (builds pylib + qt), `just run` (build + launch
  the GUI), `just lint`, `just test` (or `just test-rust` / `just test-py` / `just test-ts`),
  `just fmt`, and `just check` (full format + build + checks; run before marking work done). Web
  views are served at `http://localhost:40000/_anki/pages/` while Anki is running.
- **Self-managed toolchain.** The build downloads its own node, uv (Python), protoc and yarn into
  `anki/out/`; you don't install those manually. Host tools used: `just` and `n2` (both under
  `~/.cargo/bin`) and a rustup-managed Rust toolchain pinned by `anki/rust-toolchain.toml`
  (auto-downloaded). The first `just build` is slow (compiles the Rust core); later builds are
  incremental. After editing a `.proto`, run `just check` so generated bindings recompile.
- **Translation repos are required build deps and are NOT tracked here.** `anki/ftl/core-repo` and
  `anki/ftl/qt-repo` (upstream Anki git submodules) must be present or the build fails at configure
  time (`input .git missing` / submodule errors), because this wrapper repo vendored Anki as plain
  files and dropped its `.git`/submodule wiring. The startup update script recreates a nested git
  repo inside `anki/` and clones these two repos so the build's `git submodule update` step
  succeeds. They show up as untracked under `anki/ftl/` — do **not** `git add` them into this repo;
  they are build dependencies, not source.
- **The nested `anki/.git` is intentional** (created by the update script) and does not interfere
  with this repo: files under `anki/` that are already tracked by `/workspace`'s git remain tracked
  normally. Run your git workflow (commits/PRs) from `/workspace`.
- **Running the GUI in this headless VM.** An X server is available on `DISPLAY=:1`. Launch with:
  ```
  DISPLAY=:1 QT_QPA_PLATFORM=xcb \
    QTWEBENGINE_CHROMIUM_FLAGS="--no-sandbox --remote-allow-origins=http://localhost:8080" \
    just run
  ```
  `--no-sandbox` is required — QtWebEngine aborts (SIGABRT, core dump) without it inside the
  container. On first launch Anki shows a language-selection dialog; pass `-b <dir>` (e.g.
  `just run -b /tmp/ankidata`) to use a scratch profile directory. Required Qt/xcb system libraries
  (including `libxcb-cursor0`, `libxcb-icccm4`, `libxcb-keysyms1`) are already installed in the VM
  image.
- **Self-hosted sync server** (for the two-app sync demo, challenge 7b): from `anki/`,
  `cargo install --path rslib/sync` then `SYNC_USER1=user:pass anki-sync-server` (serves
  `http://0.0.0.0:8080/`). Point clients at it via a custom sync URL. Details in
  `docs/codebase_notes.md` §6.
