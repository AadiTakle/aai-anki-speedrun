# STAT (Speedrun) — USMLE Step 2 CK study app

Fork of [Anki](https://github.com/ankitects/anki) (vendored under `anki/`) built into **STAT** — a desktop + mobile study app for **one exam: USMLE Step 2 CK**. The shared Rust engine (`anki/rslib`) powers scheduling, scoring, and ingestion; desktop is PyQt + SvelteKit webviews; mobile is SwiftUI over the same engine via FFI.

**Canonical product/spec docs:** [`docs/PRD.md`](docs/PRD.md) · [`docs/project_guidelines.md`](docs/project_guidelines.md) · [`docs/wednesday_plan.md`](docs/wednesday_plan.md) · [`docs/feature_ledger.md`](docs/feature_ledger.md) · [`AGENTS.md`](AGENTS.md)

> **Active development:** wave-2 integration (F2/F3/performance/UI, etc.) lands on branch **`integration/next-slice`**. The Wednesday proof bundle on **`main`** (`069dc55` at last check) is the graded checkpoint; newer RPCs and screens may exist only on the integration branch until merged.

> **Graders (60 s):** on `main`, build the installable macOS **`Anki.app`** ([below](#built-apps-for-demo--grading)), launch it (not `just run`), use **Tools → Speedrun → Seed sample data (demo)**, then open **Today → Study (native reviewer) → Memory Score**; phone proof is [`mobile/ios/proof/review_session.mp4`](mobile/ios/proof/review_session.mp4) or rebuild the Simulator **Release** app ([mobile](#mobile-ios--built-app-for-demo)).

---

## Repository layout

| Path | What it is |
|------|------------|
| `anki/` | Vendored Anki monorepo — **all build commands run from here** (`just …`) |
| `anki/rslib/` | Shared Rust engine (scheduler, speedrun module, sync) |
| `anki/proto/anki/speedrun.proto` | `SpeedrunService` RPC contract |
| `anki/pylib/`, `anki/qt/`, `anki/ts/` | Python bindings, desktop shell, SvelteKit UI |
| `mobile/speedrun-ffi/` | C ABI → `rslib` for iOS |
| `mobile/ios/` | SwiftUI app + xcframework build scripts |
| `proof/wednesday/` | Wednesday proof bundle (logs, installers, Linux clean-machine) |
| `docs/` | PRD, deadlines, factory workflow, feature ledger |

---

## Prerequisites

**All platforms**

- [`just`](https://github.com/casey/just) and [`n2`](https://github.com/evmar/n2) (typically under `~/.cargo/bin`)
- Rust toolchain pinned by `anki/rust-toolchain.toml` (rustup auto-downloads)

**Desktop (macOS or Linux)**

- Host build tools only — Node, Python (uv), protoc, and yarn are **downloaded into `anki/out/`** on first `just build`.

**Desktop build deps (not tracked in git)**

The Anki translation submodules must exist or configure fails:

```bash
# From anki/ — the startup/update script normally clones these:
#   anki/ftl/core-repo
#   anki/ftl/qt-repo
```

Do **not** commit `anki/ftl/*`; they are build dependencies.

**Mobile (macOS + Apple Silicon)**

- Xcode + iOS Simulator runtime (`xcodebuild -downloadPlatform iOS`)
- Rust iOS targets: `rustup target add aarch64-apple-ios-sim aarch64-apple-ios`
- [XcodeGen](https://github.com/yonaskolb/XcodeGen): `brew install xcodegen`
- A built `protoc` from the desktop tree: `anki/out/extracted/protoc/bin/protoc` (run `just build` in `anki/` once)

---

## Desktop — build, run, test

All commands from **`anki/`**.

### First-time build

```bash
cd anki
just build          # slow first time (~minutes); incremental after
```

### Run the GUI (development)

```bash
just run            # build + launch
just run -b /tmp/ankidata   # scratch profile (skip language wizard on repeat runs)
```

**Headless / CI VM** (needs X server):

```bash
DISPLAY=:1 QT_QPA_PLATFORM=xcb \
QTWEBENGINE_CHROMIUM_FLAGS="--no-sandbox --remote-allow-origins=http://localhost:8080" \
just run -b /tmp/ankidata
```

### Run tests

```bash
just test           # full suite
just test-rust      # rslib (includes speedrun + F5 queue tests)
just test-py        # pylib + qt
just test-ts        # SvelteKit / vitest

# Targeted speedrun engine tests
cargo test -p anki --lib speedrun::
cargo test -p anki --lib scheduler::queue::builder::tests::points_at_stake

# Full gate before merge
just check
```

### Benchmark (challenge 7h skeleton)

```bash
just bench          # criterion: F5 build_queues, F4 topic_mastery, F6 memory_score
```

See [`proof/wednesday/BENCH.md`](proof/wednesday/BENCH.md).

### STAT console in the running app

With Anki running, web routes are served at (when allowlisted in [`mediasrv.py`](anki/qt/aqt/mediasrv.py)):

| Screen | URL | Notes |
|--------|-----|-------|
| Today | `http://localhost:40000/_anki/pages/today` | Allowlisted |
| Reviewer | `http://localhost:40000/_anki/pages/reviewer` | Allowlisted |
| Import | `http://localhost:40000/_anki/pages/import` | Allowlisted |
| Memory score | `http://localhost:40000/_anki/pages/memory-score` | Allowlisted |
| Errors | `http://localhost:40000/_anki/pages/errors` | Route exists; temporarily **not** allowlisted on `integration/next-slice` |
| Trajectory | `http://localhost:40000/_anki/pages/trajectory` | Route exists; temporarily **not** allowlisted on `integration/next-slice` |

**Qt entry points:** `Tools → Speedrun` submenu — wired in [`anki/qt/aqt/speedrun.py`](anki/qt/aqt/speedrun.py), registered from [`anki/qt/aqt/main.py`](anki/qt/aqt/main.py). First-run onboarding wizard: `maybe_offer_onboarding`.

**Seed demo data without real imports:**

```bash
cd anki
PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 \
  ./out/pyenv/bin/python ../proof/wednesday/seed_in_app.py
```

Or use **Tools → Speedrun → Seed sample data (demo)** in the GUI.

### Desktop installers

| Platform | Doc | Reproduce |
|----------|-----|-----------|
| **macOS** `.app` + `.dmg` | [`proof/wednesday/INSTALLER.md`](proof/wednesday/INSTALLER.md) | `just wheels` then `qt/tools/build_installer.py … build/package` |
| **Linux** clean machine | [`proof/wednesday/linux/README.md`](proof/wednesday/linux/README.md) | `./build_and_verify.sh all` (container build + clean runtime verify) |

Artifacts land under `anki/out/` (gitignored).

### Built apps for demo / grading

Use these paths for screen recordings and grading — **not** `just run` (dev tree) or Debug Console scripts.

#### macOS desktop (`.app` / `.dmg`)

From `anki/` (first run: `just build` once so the toolchain + `protoc` exist; ~30–90 min):

```bash
cd anki

# Production wheels (RELEASE=1 — minified web assets, same as release pipeline)
just wheels
# -> out/wheels/anki-26.5-cp310-abi3-macosx_12_0_arm64.whl
# -> out/wheels/aqt-26.5-py3-none-any.whl

# Briefcase .app bundle (ad-hoc signed)
./out/pyenv/bin/python qt/tools/build_installer.py --version 26.5 build \
  --aqt_wheel  "$(pwd)/out/wheels/aqt-26.5-py3-none-any.whl" \
  --anki_wheel "$(pwd)/out/wheels/anki-26.5-cp310-abi3-macosx_12_0_arm64.whl" \
  --skip_fcitx
# -> out/installer/build/anki/macos/app/Anki.app

# Optional distributable disk image
./out/pyenv/bin/python qt/tools/build_installer.py --version 26.5 package
# -> out/installer/dist/anki-26.5-mac-apple.dmg
```

**Launch the built app** (fresh demo profile — avoids your daily Anki data):

```bash
DEMO_PROFILE=/tmp/stat-demo-profile
rm -rf "$DEMO_PROFILE"
open -a "$(pwd)/out/installer/build/anki/macos/app/Anki.app" --args -b "$DEMO_PROFILE"
# Or mount the .dmg, drag Anki to Applications, then:
# open -a /Applications/Anki.app --args -b "$DEMO_PROFILE"
```

First launch shows the language chooser once; pick **English**. STAT lives under **Tools → Speedrun**.

#### Linux desktop (clean-machine tarball)

Built **on Linux** (or via Docker on macOS — see [`proof/wednesday/linux/README.md`](proof/wednesday/linux/README.md)):

```bash
# From repo root — slow first compile inside container
proof/wednesday/linux/build_and_verify.sh all
# Production tarball (optional, slower):
# RELEASE=2 proof/wednesday/linux/build_and_verify.sh build
```

Artifact: `anki/out/installer/dist/anki-26.5-linux-<arch>.tar.zst`. Extract and run the bundled launcher (not system `anki`):

```bash
tar -xf anki-26.5-linux-aarch64.tar.zst
cd anki-linux
./anki -b /tmp/stat-demo-profile
```

#### Mobile iOS (Simulator **Release** app)

The phone proof is a **built `.app` installed on the Simulator**, not `just run` or a Swift REPL. Full steps: [`mobile/ios/RUNBOOK.md`](mobile/ios/RUNBOOK.md).

```bash
# One-time: desktop build for protoc
cd anki && just build

cd ../mobile/ios
export PROTOC="$(pwd)/../../anki/out/extracted/protoc/bin/protoc"
unset CARGO_TARGET_DIR
./build-xcframework.sh
xcodegen generate

# Release build (what you install on Simulator for demo)
xcodebuild -project Speedrun.xcodeproj -scheme SpeedrunApp \
  -configuration Release \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  -derivedDataPath build/DerivedData build
# -> build/DerivedData/Build/Products/Release-iphonesimulator/SpeedrunApp.app

UDID=$(xcrun simctl list devices available | awk -F '[()]' '/iPhone 17 \(/{print $2; exit}')
APP=build/DerivedData/Build/Products/Release-iphonesimulator/SpeedrunApp.app
xcrun simctl boot "$UDID" 2>/dev/null; xcrun simctl bootstatus "$UDID"
xcrun simctl uninstall "$UDID" com.speedrun.ios 2>/dev/null   # fresh New=3 seed
xcrun simctl install "$UDID" "$APP"
xcrun simctl launch "$UDID" com.speedrun.ios                   # interactive taps
```

Device installs need signing/provisioning; Wednesday scope is **Simulator only**.

### Wednesday proof scripts (real backend)

From `anki/`:

```bash
# Self-checking interactivity proof (all features compute from inputs)
PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 \
  ./out/pyenv/bin/python ../proof/wednesday/feature_proof.py

# End-to-end review loop (F5 interleaving, drain, undo, integrity)
PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 \
  ./out/pyenv/bin/python ../proof/wednesday/review_loop_demo.py
```

Index: [`proof/wednesday/PROOF.md`](proof/wednesday/PROOF.md).

---

## Mobile (iOS) — build, run, test

Android / AnkiDroid is **deferred**; the Wednesday phone proof is **iOS Simulator** on the shared engine.

**Overview:** [`mobile/README.md`](mobile/README.md) · **Step-by-step:** [`mobile/ios/RUNBOOK.md`](mobile/ios/RUNBOOK.md)

### Quick path (Simulator)

```bash
# 1 — Desktop build once (for protoc)
cd anki && just build

# 2 — Engine xcframework
cd ../mobile/ios
export PROTOC=/path/to/aai-anki-speedrun/anki/out/extracted/protoc/bin/protoc
unset CARGO_TARGET_DIR
./build-xcframework.sh          # -> build/SpeedrunFFI.xcframework

# 3 — Xcode project + app
xcodegen generate
xcodebuild -project Speedrun.xcodeproj -scheme SpeedrunApp \
  -configuration Debug \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  -derivedDataPath build/DerivedData build

# 4 — Install & launch
UDID=$(xcrun simctl list devices available | awk -F '[()]' '/iPhone 17 \(/{print $2; exit}')
APP=build/DerivedData/Build/Products/Debug-iphonesimulator/SpeedrunApp.app
xcrun simctl boot "$UDID"
xcrun simctl install "$UDID" "$APP"
xcrun simctl launch "$UDID" com.speedrun.ios        # interactive
# xcrun simctl launch "$UDID" com.speedrun.ios --autodemo   # proof recording
```

**Proof artifact:** [`mobile/ios/proof/review_session.mp4`](mobile/ios/proof/review_session.mp4) — 3 seeded cards, 6 real scheduler answers on `rslib`, counts drain to “All caught up.”

**Engine seam:** `mobile/speedrun-ffi/` (C ABI) → `SpeedrunKit` → `SpeedrunApp` SwiftUI `ReviewView`.

**Rust tests (shared with desktop):** same `cargo test -p anki --lib speedrun::` — no separate mobile scheduler.

---

## Wednesday checklist → codebase map

Source of truth for requirements: [`docs/wednesday_plan.md`](docs/wednesday_plan.md) §1 (Definition of Done) and §2 (Feature acceptance charter). Graded rules: [`docs/project_guidelines.md`](docs/project_guidelines.md) §2, §6, §11.

### §1 — Definition of Done (deadline checklist)

| # | Requirement | Where it lives | Tests / proof |
|---|-------------|----------------|---------------|
| 1 | **Installable desktop app from source, AI off** | Build: `just build` / `just run`. Installers: [`proof/wednesday/INSTALLER.md`](proof/wednesday/INSTALLER.md), [`proof/wednesday/linux/`](proof/wednesday/linux/). No AI path in MVP scoring. | Linux clean-container verify; macOS `.app`/`.dmg` build logs |
| 2 | **Two apps, one shared engine, sync-capable** | Engine: `anki/rslib/`. Desktop: PyO3 `anki/pylib/`. Mobile: `mobile/speedrun-ffi/` → iOS. Sync infra: `anki/rslib/sync/` (two-app demo = Friday 7b). | iOS proof video; same `rslib` tests on both sides |
| 3 | **Real Rust engine change (not UI-only)** | **F5** points-at-stake queue: [`anki/rslib/src/scheduler/queue/builder/mod.rs`](anki/rslib/src/scheduler/queue/builder/mod.rs) (`sort_review_by_points_at_stake`). Proto enum: [`anki/proto/anki/deck_config.proto`](anki/proto/anki/deck_config.proto) `REVIEW_CARD_ORDER_POINTS_AT_STAKE = 13`. Weight×weakness from [`anki/rslib/src/speedrun/`](anki/rslib/src/speedrun/) store + mastery. | 5× `points_at_stake_*` Rust tests in `mod.rs`; Python binding test below |
| 4 | **≥3 Rust tests + 1 Python test, undo-safe, no corruption** | Rust: F5 tests + F1/F4/F6 in `rslib`. Python: [`anki/pylib/tests/test_speedrun.py`](anki/pylib/tests/test_speedrun.py) `test_points_at_stake_queue_orders_highest_first`. Undo: `points_at_stake_answer_then_undo_is_safe`. Integrity: review loop demo. | `cargo test … points_at_stake`; `just test-py`; [`proof/wednesday/review_loop_demo.py`](proof/wednesday/review_loop_demo.py) |
| 5 | **Desktop + phone each complete a review loop on shared engine** | Desktop loop: exam deck + points-at-stake ordering → standard reviewer ([`anki/qt/aqt/speedrun.py`](anki/qt/aqt/speedrun.py) `setup_speedrun_reviewer`). Phone: [`mobile/ios/App/`](mobile/ios/App/) + FFI. | [`anki/pylib/tests/test_speedrun_review_loop.py`](anki/pylib/tests/test_speedrun_review_loop.py); [`mobile/ios/proof/review_session.mp4`](mobile/ios/proof/review_session.mp4) |
| 6 | **Three scores as ranges + give-up/abstain when insufficient** | **Memory (F6):** [`anki/rslib/src/speedrun/score.rs`](anki/rslib/src/speedrun/score.rs). **Performance:** [`anki/rslib/src/speedrun/performance.rs`](anki/rslib/src/speedrun/performance.rs). **Readiness:** RPC stub in [`anki/rslib/src/speedrun/service.rs`](anki/rslib/src/speedrun/service.rs) (`get_readiness_score` — currently always abstains; full calibration TBD). RPCs: [`anki/proto/anki/speedrun.proto`](anki/proto/anki/speedrun.proto). UI: [`anki/ts/routes/today/`](anki/ts/routes/today/), [`memory-score/`](anki/ts/routes/memory-score/), [`trajectory/`](anki/ts/routes/trajectory/). | `test_memory_score_abstains_on_fresh_collection`; performance tests in `rslib`; [`feature_proof.py`](proof/wednesday/feature_proof.py) abstain→scored progression for memory |
| 7 | **AI off for MVP** | Scores from FSRS + blueprint + ingest only; no LLM in scoring path. | Feature proof runs with `ANKI_TEST_MODE=1`, no AI deps |
| 8 | **Proof bundle: commit, test logs, clean build/install, phone video** | [`proof/wednesday/PROOF.md`](proof/wednesday/PROOF.md), [`proof/wednesday/logs/`](proof/wednesday/logs/), Linux tarball, iOS mp4, [`BENCH.md`](proof/wednesday/BENCH.md) | Regenerate commands in PROOF.md |

### §2 — Feature acceptance charter

| ID | Feature | Implementation | Contract (RPC / proto) | Tests | UI / desktop entry |
|----|---------|----------------|------------------------|-------|-------------------|
| **A — F6** | Memory score (range + abstain) | [`anki/rslib/src/speedrun/score.rs`](anki/rslib/src/speedrun/score.rs) | `GetMemoryScore` in [`speedrun.proto`](anki/proto/anki/speedrun.proto) | `score.rs` tests; [`test_speedrun.py`](anki/pylib/tests/test_speedrun.py); [`feature_proof.py`](proof/wednesday/feature_proof.py) | [`anki/ts/routes/memory-score/+page.svelte`](anki/ts/routes/memory-score/+page.svelte); Tools → Speedrun |
| **B — F5** | Points-at-stake / topic-aware queue | [`scheduler/queue/builder/mod.rs`](anki/rslib/src/scheduler/queue/builder/mod.rs); weakness weights from speedrun store | `deck_config.proto` order enum; queue via normal scheduler | 5× Rust `points_at_stake_*`; [`test_speedrun.py`](anki/pylib/tests/test_speedrun.py) | Exam deck review order; [`reviewer/`](anki/ts/routes/reviewer/) vitals |
| **C — F4** | Per-topic mastery query | [`anki/rslib/src/speedrun/mastery.rs`](anki/rslib/src/speedrun/mastery.rs) | `GetTopicMastery` | `mastery.rs` tests; pylib speedrun tests; feature proof | Today / trajectory topic breakdown |
| **D — F1** | Topic store + crosswalk (persist, undo-safe) | [`anki/rslib/src/speedrun/store.rs`](anki/rslib/src/speedrun/store.rs), [`service.rs`](anki/rslib/src/speedrun/service.rs) | Store RPCs on `SpeedrunService` | `store.rs` / service tests; feature proof undo | Import + onboarding crosswalk |
| **E — F10** | Desktop review loop (interleave, drain, undo) | Scheduler v3 + F5 ordering; [`anki/qt/aqt/speedrun.py`](anki/qt/aqt/speedrun.py) reviewer hook | Uses collection scheduler, not a separate RPC | [`test_speedrun_review_loop.py`](anki/pylib/tests/test_speedrun_review_loop.py); [`review_loop_demo.py`](proof/wednesday/review_loop_demo.py) | Built-in Anki reviewer on exam deck |
| **F — F4c** | Mastery dashboard (web) | TS client [`anki/ts/lib/speedrun/speedrunClient.ts`](anki/ts/lib/speedrun/speedrunClient.ts) | `GetTopicMastery` | [`speedrunClient.test.ts`](anki/ts/lib/speedrun/speedrunClient.test.ts) | [`today/`](anki/ts/routes/today/), [`trajectory/`](anki/ts/routes/trajectory/) |
| **G — F12** | Mobile review session (shared engine) | [`mobile/speedrun-ffi/`](mobile/speedrun-ffi/), [`mobile/ios/`](mobile/ios/) | C header `speedrun_ffi.h` | Same Rust `rslib` tests; manual sim proof | `SpeedrunApp` → `ReviewView` |
| **H — F11** | Desktop installer | [`anki/qt/tools/build_installer.py`](anki/qt/tools/build_installer.py), mac template under `anki/qt/installer/` | N/A | Linux [`build_and_verify.sh`](proof/wednesday/linux/build_and_verify.sh) | Ship `Anki.app` / `.dmg` / Linux tarball |
| **I — F13** | Engine benchmark harness | [`anki/rslib/src/speedrun/bench.rs`](anki/rslib/src/speedrun/bench.rs) | N/A | Criterion via `just bench` | [`proof/wednesday/BENCH.md`](proof/wednesday/BENCH.md) |
| **Fob** | First-run onboarding wizard | [`anki/qt/aqt/speedrun.py`](anki/qt/aqt/speedrun.py) (`run_setup_wizard`, `maybe_offer_onboarding`) | Ingest + crosswalk RPCs | [`anki/qt/tests/test_speedrun_onboarding.py`](anki/qt/tests/test_speedrun_onboarding.py) (12 tests) | Auto-offered on first launch |

### Wave 2 / integration branch (beyond Wednesday minimum)

These are in active merge on **`integration/next-slice`**; see [`docs/feature_ledger.md`](docs/feature_ledger.md) for current state.

| ID | Feature | Engine / contract | Tests | UI |
|----|---------|-------------------|-------|-----|
| **F2** | QBank performance ingest | [`anki/rslib/src/speedrun/qbank.rs`](anki/rslib/src/speedrun/qbank.rs), ingest RPCs in proto | [`test_speedrun_ingest.py`](anki/pylib/tests/test_speedrun_ingest.py), [`test_speedrun_qbank.py`](anki/pylib/tests/test_speedrun_qbank.py) | [`import/`](anki/ts/routes/import/) |
| **F2c** | Aggregate QBank rollups | qbank module + proto | qbank py tests | Today performance vital |
| **F3** | Relink misses | [`anki/rslib/src/speedrun/relink.rs`](anki/rslib/src/speedrun/relink.rs) | [`test_speedrun_relink.py`](anki/pylib/tests/test_speedrun_relink.py) | [`errors/`](anki/ts/routes/errors/) |
| **F7** | Performance score | [`performance.rs`](anki/rslib/src/speedrun/performance.rs) | [`test_speedrun_performance.py`](anki/pylib/tests/test_speedrun_performance.py) | Today / trajectory |
| **F8** | Readiness score | Stub: [`service.rs`](anki/rslib/src/speedrun/service.rs) `get_readiness_score` (honest abstain until NBME/UWSA calibration) | Contract in proto; impl pending | Today gauge (abstain = “NOT ENOUGH INFO”) |
| **F9** | Next action / daily plan | [`next_action.rs`](anki/rslib/src/speedrun/next_action.rs), [`daily.rs`](anki/rslib/src/speedrun/daily.rs) | `rslib` tests | Today console |
| **Coverage map** | Blueprint coverage | [`coverage.rs`](anki/rslib/src/speedrun/coverage.rs) | `rslib` tests | Trajectory |
| **Focus mode** | Topic-scoped queue | [`focus.rs`](anki/rslib/src/speedrun/focus.rs) | [`test_speedrun_focus.py`](anki/pylib/tests/test_speedrun_focus.py) | Reviewer |
| **UI overhaul** | STAT console shell | [`anki/ts/lib/speedrun/`](anki/ts/lib/speedrun/) components + routes | vitest in `lib/speedrun/*.test.ts` | All `_anki/pages/*` routes; allowlist [`mediasrv.py`](anki/qt/aqt/mediasrv.py) |

**Backend ↔ UI handoff** (when wiring mock adapters): [`docs/UI-OVERHAUL-HANDOFF.md`](docs/UI-OVERHAUL-HANDOFF.md).

### Graded challenges (7a–7h) — pointers

| Challenge | Topic | Primary location |
|-----------|-------|------------------|
| **7a** | Real engine change | F5 + F6 above |
| **7b** | Two-app sync | `anki/rslib/sync/`; self-hosted: `cargo install --path rslib/sync` → `anki-sync-server` ([`AGENTS.md`](AGENTS.md)) |
| **7c** | Three honest scores | F6 + performance + readiness modules |
| **7d** | AI off + held-out eval | Out of MVP scope for scoring; eval tooling → future `ml/` lane |
| **7e** | No test-data leakage | Readiness/performance gating; abstain rules in score modules |
| **7f** | Ablation study | Proof/eval lane (post-MVP) |
| **7g** | Crash / offline | Anki collection + sync resilience (existing Anki paths) |
| **7h** | Benchmark | F13 [`bench.rs`](anki/rslib/src/speedrun/bench.rs), `just bench` |

---

## Test inventory (speedrun-focused)

| Suite | Location | Approx. count |
|-------|----------|---------------|
| Rust `speedrun` module + F5 queue | `anki/rslib/src/speedrun/**`, `scheduler/queue/builder/mod.rs` | ~80 `#[test]` |
| Python speedrun + onboarding | `anki/pylib/tests/test_speedrun*.py`, `anki/qt/tests/test_speedrun_onboarding.py` | ~27 `def test_` |
| TypeScript speedrun | `anki/ts/lib/speedrun/*.test.ts` | 4 files |
| Proof scripts | `proof/wednesday/feature_proof.py`, `review_loop_demo.py` | integration |

---

## Parallel development / worktrees

Feature lanes use **isolated git worktrees** (e.g. `feat/f2-qbank-ingest`, `feat/ui-overhaul`) to avoid collision while a backend agent integrates on **`integration/next-slice`**. Before testing a specific feature, check out the branch named in [`docs/feature_ledger.md`](docs/feature_ledger.md) or merge integration.

---

## License

AGPL-3.0-or-later (Anki upstream). See [`anki/LICENSE`](anki/LICENSE).
