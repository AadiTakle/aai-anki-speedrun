# Skills & Rules Index

The repeatable SOPs and tools that speed up and de-risk development on this repo. Two mechanisms:

1. **Project rules** — `.cursor/rules/*.mdc` at the repo root. Cursor auto-attaches these by their
   `globs`/`description` (in the desktop IDE and for cloud agents). They are the project's "skills."
2. **Plugin skills** — `SKILL.md` files provided by an installed Cursor plugin (machine-local, not in
   this repo). Read them at their absolute path when relevant.

> Note: upstream Anki ships its own rules under `anki/.cursor/rules/` (`building.md`, `i18n.md`) — keep
> those; our project rules live at the repo root to avoid upstream churn.

## Project rules (this repo, `.cursor/rules/`)

| Rule | When it applies | Purpose |
|------|-----------------|---------|
| `add-backend-rpc.mdc` | editing `anki/proto`, `anki/rslib`, `anki/pylib`, `anki/ts` | The exact recipe to add/change a backend RPC or proto message across Rust↔Python↔TS, undo-safe, with the regen + test steps. The most-repeated, most-error-prone operation. |
| `factory-lane-worker.mdc` | any feature implementation | TDD loop, scope/isolation, the strict test-integrity rule, and per-lane gates (condensed from `factory_workflow.md` §4/§7/§8). |
| `proof-and-eval-tooling.mdc` | Lane D (AI/eval) or Lane G (proof) | Specs for the benchmark harness (7h), held-out eval + calibration, leakage check (7e), and crash/offline tests (7g). Deps land at the owning lane's FREEZE. |

## Plugin skills (machine-local; use when relevant)

| Skill | Use for | Notes |
|-------|---------|-------|
| `xcode-project-setup` | **Mobile Lane E (iOS), on a Mac** | Safely adds Swift Package Manager dependencies / links files to an `.xcodeproj` via a bundled Swift script (no Ruby, no manual `.pbxproj` editing). Requires macOS + Swift (`swift --version`). For our iOS path we wrap the Rust engine as a static lib via C-FFI/UniFFI and add it as a local SPM package; this skill handles the Xcode wiring. |
| `firebase-*` (auth, firestore, hosting, etc.) | only if we adopt Firebase | **Out of scope** for the MVP (no Firebase dependency planned). Listed for awareness only. |

The `xcode-project-setup` skill is currently at:
`~/.cursor/plugins/cache/cursor-public/3777/<hash>/skills/xcode-project-setup/SKILL.md` (path/hash
varies per machine; locate with a file search). It only works on macOS, so it's a **local-Mac task**,
not something the Linux cloud VM can run.

## Per-lane tooling status (deferred, see `proof-and-eval-tooling.mdc`)

| Tool | Lane | Status | Lands at |
|------|------|--------|----------|
| `criterion` + `just bench` + synthetic 50k-deck generator (7h) | G | spec'd | benchmark slice FREEZE |
| `ml/` eval env (numpy/scipy/scikit-learn/matplotlib) + calibration (Brier/log-loss) | D | spec'd | Friday AI freeze |
| leakage-check script (7e) | D | spec'd | Friday AI freeze |
| crash/offline harness (7g) | G/F | spec'd | proof slice |

Heavy deps (Cargo.lock entries, a Python ML env) are intentionally **not** installed yet — they're
added by the supervisor at the owning lane's contract freeze so we don't carry unused tooling.
