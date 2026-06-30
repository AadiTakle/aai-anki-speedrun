# Anki Codebase Notes

Working notes from exploring the upstream Anki source (`ankitects/anki`) vendored at `anki/`.
Purpose: understand the structure, core features, and how to build for desktop and mobile,
and map all of that onto the requirements in `project_guidelines.md`.

> All paths below are relative to the repo root unless noted. The vendored copy lives in `anki/`.

---

## 1. Repository Structure

Anki is a multi-language monorepo: a **Rust core** (`rslib`), a **Python library + Qt GUI**
(`pylib`, `qt`), and a **TypeScript/Svelte web UI** (`ts`). The layers talk to each other through
**protobuf-defined RPCs**.

| Path | Purpose |
|------|---------|
| `anki/rslib/` | **Core Rust library** (crate `anki`): scheduler/FSRS, SQLite storage, collection, sync, import/export, search. This is "the engine." |
| `anki/rslib/proto/` | Prost-generated protobuf message types + Python/TS codegen (crate `anki_proto`). |
| `anki/rslib/sync/` | Standalone sync-server binary (crate `anki-sync-server`). |
| `anki/proto/anki/*.proto` | **Protobuf service + message definitions** — the cross-language API contract (25 files). |
| `anki/pylib/` | Python library (`import anki`). Wraps Rust via PyO3. Published as the `anki` PyPI wheel. |
| `anki/pylib/rsbridge/` | **PyO3 bridge** (crate `rsbridge`, compiled to `_rsbridge.so`) — how Python calls Rust. |
| `anki/qt/` | **PyQt6 desktop GUI** (`import aqt`). Embeds web views. Published as the `aqt` wheel. Contains the Briefcase installer (`qt/installer/`). |
| `anki/ts/` | TypeScript + SvelteKit web UI (reviewer, editor, deck options, graphs). Bundled into the Qt app at build time. |
| `anki/ftl/` | Fluent translation files + i18n codegen for Rust/TS/Python. |
| `anki/build/` | Custom build system: `configure` (defines the ninja graph), `ninja_gen`, `runner` (entrypoint). |
| `anki/tools/` | Build/run helper scripts (`build`, `build-installer`, `run.py`, etc.). |
| `anki/docs/` | Developer docs (Sphinx). See index below. |
| `anki/docs-site/` | User-facing docs (Mintlify). |
| `anki/justfile` | Command runner wrapping ninja (`just run`, `just check`, etc.). |
| `anki/out/` | Generated at build time (wheels, pyenv, node_modules, Rust target, generated protos). |

### Language layering (request flow)

```
ts/ (Svelte web UI)
   │  HTTP POST RPC (generated protobuf client)
qt/aqt/ (PyQt6 shell; hosts web views, mediasrv HTTP server)
   │  import anki; col._backend.<rpc>(...)
pylib/anki/ (Python API)  ──  pylib/rsbridge/ (PyO3)
   │  run_service_method(service, method, protobuf_bytes)
rslib/ (Rust core: scheduler, storage, sync, search)
        ▲
        │  proto/anki/*.proto  (shared schema, codegen for all three layers)
```

Key principle: **Rust owns the business logic and never calls up into Python/TS.** Everything
crosses the boundary as protobuf bytes through a single dispatch function
`run_service_method(service_idx, method_idx, input_bytes)`.

### Developer docs index (`anki/docs/`)

| File | Summary |
|------|---------|
| `development.md` | Main dev guide: prerequisites, `./run`, tests, wheels, installer, env vars. |
| `build.md` | Build-system architecture (`configure`/`ninja_gen`/`runner`) + debugging. |
| `architecture.md` | High-level split: rslib+pylib backend vs aqt+ts GUI; protobuf role. |
| `language_bridge.md` | **How to declare/call/implement an RPC across TS ↔ Python ↔ Rust** (most important for our Rust change). |
| `protobuf.md` | Protobuf conventions and pitfalls. |
| `mac.md` / `linux.md` / `windows.md` | Platform build prerequisites. |
| `editing.md` | IDE setup. |
| `testing-coverage.md` | Test commands + coverage thresholds. |
| `e2e-testing.md` | Playwright e2e tests. |
| `releasing.md` | Release/installer pipeline. |
| `syncserver/` | Self-hosted sync server Dockerfiles + README. |
| `docker/` | Dockerfile for building/running Anki over X11. |

---

## 2. Core Features / Engine Internals

### Scheduler & FSRS (memory model)

- **FSRS** (the memory algorithm) is an external crate (`fsrs = "5.2.0"`) integrated under
  `anki/rslib/src/scheduler/fsrs/`:
  - `memory_state.rs` — compute/update memory state (stability, difficulty) from the review log.
  - `params.rs` — parameter optimization (`FSRS::compute_parameters`).
  - `rescheduler.rs`, `retention.rs`, `simulator.rs`.
- Card state transitions: `anki/rslib/src/scheduler/states/` (`review.rs`, `relearning.rs`, …).
- Answering a card (applies grade, computes next interval): `anki/rslib/src/scheduler/answering/`.
- **This is Anki's built-in answer to "Memory" in the three-scores model.** Performance and
  Readiness are *not* in here — we build those.

### Queue building (review ordering) — prime target for the Rust change

- Entry: `Collection::get_queues()` → `Collection::build_queues()` in
  `anki/rslib/src/scheduler/queue/builder/mod.rs`.
- Gather phase (pulls due/new/learning cards from SQLite): `queue/builder/gathering.rs`.
- Sort phase: `queue/builder/sorting.rs`; new/review interleaving in `intersperser.rs`.
- SQL ordering for review cards (FSRS retrievability/difficulty variants):
  `anki/rslib/src/storage/card/mod.rs` (`ReviewCardOrder` enum, `review_order_sql`).
- This is exactly where guideline **7a (points-at-stake queue / topic-aware scheduling)** would hook in.

### Storage / collection

- SQLite layer: `anki/rslib/src/storage/` (`sqlite.rs`, `card/`, `note/`, `deck/`, …).
- Base schema: `anki/rslib/src/storage/schema11.sql` (collection format `collection.anki2`).
- Collection state + transactions: `anki/rslib/src/collection/mod.rs`.

### Backend dispatch

- Generated service traits: `anki/rslib/src/services.rs` (includes generated `backend.rs`).
- Backend entry / init: `anki/rslib/src/backend/mod.rs` (`init_backend`, `run_service_method`).

---

## 3. The Rust ↔ Python Boundary (needed for guideline 7a)

Anki uses **PyO3** (not C FFI) on desktop. Every backend call is `(service_idx, method_idx,
protobuf_bytes) → protobuf_bytes`.

### Service pattern (important)

Every domain defines **two** services in its `.proto`:

- `FooService` → implemented on **`Collection`** (needs an open DB).
- `BackendFooService` → implemented on **`Backend`** (may work without a collection).
- A method in `FooService` but not `BackendFooService` is auto-delegated:
  `Backend::method()` → `self.with_col(|col| FooService::method(col, …))`.

### Codegen pipeline

```
proto/anki/*.proto
  ├─► rslib/proto/build.rs (prost)      → Rust message types (anki_proto)
  ├─► rslib/build.rs                    → OUT_DIR/backend.rs (traits + dispatch)
  ├─► rslib/proto/python.rs             → out/pylib/anki/_backend_generated.py
  └─► rslib/proto/typescript.rs         → out/ts/lib/generated/backend.ts
```

After editing a `.proto`, run `just check` (or rebuild `:rslib:proto` + `:pylib:rsbridge`) so the
generated code recompiles.

### End-to-end example (existing RPC `GetQueuedCards`)

1. **Proto** — `anki/proto/anki/scheduler.proto`: `rpc GetQueuedCards(...) returns (QueuedCards);`
2. **Rust trait** — generated into `OUT_DIR/backend.rs`, included by `rslib/src/services.rs`.
3. **Rust impl** — `anki/rslib/src/scheduler/service/mod.rs` (`fn get_queued_cards`).
4. **PyO3 dispatch** — `anki/pylib/rsbridge/lib.rs` (`Backend.command`).
5. **Generated Python** — `out/pylib/anki/_backend_generated.py`.
6. **Python call site** — `anki/pylib/anki/scheduler/v3.py` (`col._backend.get_queued_cards(...)`).

### Recipe: add a new backend RPC

1. Add message(s) + `rpc` to a service in `anki/proto/anki/<domain>.proto`.
2. `just check` to regenerate bindings.
3. Implement the trait method in `rslib/src/<domain>/service.rs` (or `scheduler/service/mod.rs`).
4. (Optional) add a Python wrapper in `pylib/anki/...`.
5. Add Rust `#[test]`s + a Python test (see §6).

### Tests

- **Rust unit tests**: co-located `#[cfg(test)]` modules across `rslib/src/**`; shared helpers in
  `anki/rslib/src/tests.rs` (`Collection::new()`, in-memory collections). Run via `just test-rust`
  (uses `cargo nextest`).
- **Python tests that call Rust**: `anki/pylib/tests/` (e.g. `test_schedv3.py` opens a real
  collection and calls `col.sched.*`, which hits Rust through `_backend`). Run via `just test-py`.
- This directly satisfies guideline 7a's "3 Rust unit tests + 1 test that calls it from Python."

---

## 4. Building the Desktop App

### Toolchain / prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | **1.92.0** | pinned in `anki/rust-toolchain.toml`; install via rustup. |
| Python | **≥3.12** (`.python-version` = 3.13.13) | building from source needs 3.12+. |
| Ninja or **N2** | Ninja 1.10+ / N2 | install N2 via `anki/tools/install-n2`. |
| just | optional | `brew install just`. |
| Node, Yarn, protoc, uv | auto-downloaded by the build | overridable via `*_BINARY` env vars. |

Manual installs: rustup, N2/Ninja, (optionally) just, plus platform tools — on macOS: Xcode +
CLI tools, and `git`/`rsync` (Homebrew); audio uses `mpv`/`lame`. See `anki/docs/mac.md`.

### Build & run from source

```bash
cd anki
just run             # build pylib + qt, then launch Anki (dev mode)
just run-optimized   # RELEASE=1 ./run
just build           # ninja pylib qt (build only)
just check           # build + all lint/tests
```

Equivalent direct scripts: `./run` (macOS/Linux), `.\run.bat` (Windows), `./ninja pylib qt`.

`./run` sets `ANKIDEV=1`, serves the dev web UI at `http://localhost:40000/_anki/pages/`, then
launches via `tools/run.py`.

### Wheels & installer (guideline: "installer that runs on a clean machine")

```bash
cd anki
./tools/build            # → out/wheels/*.whl  (anki + aqt wheels; RELEASE=2)
tools/build-installer    # → out/installer/dist/  (Windows MSI / macOS .dmg / Linux tarball)
```

The installer is Briefcase-based (`anki/qt/installer/`, `anki/qt/tools/build_installer.py`).

---

## 5. Building for Mobile (the hard part)

**Key finding: the upstream `anki/` repo does NOT build a mobile-native library and has no
Android/iOS scaffolding.** It only builds `rsbridge` as a **PyO3 desktop extension**. There are
**zero** `extern "C"`, `#[no_mangle]`, `uniffi`, `cbindgen`, JNI, `.aar`, or gradle references, and
no Android/iOS Rust targets in the build system. The Briefcase config explicitly marks iOS/Android
`supported = false`.

What *does* exist and is reusable:

- `rslib` is a normal Rust library (crate `anki`, `publish = false`) — it can be cross-compiled.
- A stable cross-language contract: `run_service_method(service, method, protobuf_bytes)` +
  `init_backend(BackendInit)`.
- AnkiDroid-oriented support already in-tree: `anki/proto/anki/ankidroid.proto`
  (`AnkidroidService`), `anki/rslib/src/ankidroid/` (paged DB access for JNI-style consumers),
  `anki/rslib/src/backend/dbproxy.rs`. All protos set `java_multiple_files = true` (Java/Kotlin
  codegen intent).

### Realistic options to get the engine on a phone

- **Option A — Build on AnkiDroid (recommended for Android).** AnkiDroid (`ankidroid/Anki-Android`,
  separate AGPL repo) already cross-compiles this `rslib` for Android ABIs and bridges it via JNI
  (historically a `rsdroid`/`.aar` layer that is NOT in this repo). We'd pin AnkiDroid to our
  `proto/` + `rslib` revision so our Rust change ships to the phone too. This is the path the
  guidelines explicitly bless ("build on AnkiDroid … or run Anki's Rust backend on the device").

- **Option B — Custom native FFI crate (Android + iOS from one core).** Add a new workspace crate
  (e.g. `mobile/rsbridge_ffi/`) with `crate-type = ["cdylib", "staticlib"]` exposing a small C ABI
  that wraps `init_backend` + `run_service_method`. Then JNI for Android, Swift/`staticlib` (or
  UniFFI) for iOS. Cross-compile targets (`aarch64-linux-android`, `aarch64-apple-ios`, …) are all
  **new** — none exist in the repo today. More work, but one core for both platforms and matches the
  guideline's iOS-via-C-FFI suggestion.

- **Option C — PyO3 on mobile.** Not realistic (embedded CPython on iOS/Android app stores).

- **Option D — Sync-only thin client.** Insufficient alone: offline review/scheduling needs the
  engine on-device.

### Mobile key files

| Topic | Path |
|------|------|
| PyO3 desktop bridge (pattern to mirror for FFI) | `anki/pylib/rsbridge/lib.rs` |
| RPC dispatch | `anki/rslib/rust_interface.rs` (`run_service_method`) |
| Backend init | `anki/rslib/src/backend/mod.rs` (`init_backend`) |
| AnkiDroid proto | `anki/proto/anki/ankidroid.proto` |
| AnkiDroid Rust support | `anki/rslib/src/ankidroid/` |

---

## 6. Sync Architecture (guidelines §3, 7b)

**Model: hub-and-spoke over HTTP, not peer-to-peer.** Each device keeps a local
`collection.anki2`, works offline, and syncs against a central server (AnkiWeb or a self-hosted
server). Two devices never merge directly — they each merge with the server.

### Where it lives

| Path | Role |
|------|------|
| `anki/rslib/src/sync/mod.rs` | Sync root module. |
| `anki/rslib/src/sync/collection/` | Collection sync protocol + merge logic. |
| `anki/rslib/src/sync/http_client/` | `HttpSyncClient` (default endpoint `https://sync.ankiweb.net/`). |
| `anki/rslib/src/sync/http_server/` | **Built-in self-hostable server** (`SimpleServer`, Axum). |
| `anki/rslib/src/sync/media/` | Media sync (`/msync/` endpoints). |
| `anki/rslib/src/backend/sync.rs` | Protobuf bridge (`BackendSyncService`). |
| `anki/proto/anki/sync.proto` | Sync RPC definitions. |

### Protocol (normal sync sequence)

`meta` → `start` (exchange deletion graves) → `applyChanges` (notetypes/decks/tags/config) →
`chunk` (pull) → `applyChunk` (push notes/cards/revlog, 250/chunk) → `sanityCheck2` → `finish`
(bump USN, set last-sync). Wrapped in a SQLite transaction; rolls back + `abort` on failure.

### Conflict model (guideline 7b — "your conflict rule")

- **Change tracking via USN** (Update Sequence Number, `Usn(i32)` in `rslib/src/types.rs`). Locally
  modified rows get `usn = -1`; after sync they're set to `server_usn + 1`.
- **Normal sync = last-write-wins by modification time** (`mtime`/`mod`). Concurrent edits to the
  same note/card resolve silently — newer `mtime` wins (`collection/chunks.rs`,
  `add_or_update_*_if_newer`). Revlog is append-only. Tags union. Notetype field/template count
  changes force a `ResyncRequired`.
- **Full sync** (whole-DB upload/download) triggers when schema timestamps (`scm`) diverge; the user
  picks a direction. Not an incremental merge.
- So Anki's built-in conflict rule is "last writer by timestamp wins, mediated by the server." For
  guideline 7b we either document/use this rule or implement a stricter one.

### Self-hosting the sync server (useful for our two-app demo)

```bash
# From a source checkout:
cd anki
cargo install --path rslib/sync
SYNC_USER1=user:pass anki-sync-server        # serves http://0.0.0.0:8080/

# Or from the Python package:
SYNC_USER1=user:pass python -m anki.syncserver

# Or from desktop Anki:
SYNC_USER1=user:pass anki --syncserver
```

Server env vars: `SYNC_USER1..N` (required `user:pass`), `SYNC_BASE` (data dir,
default `~/.syncserver`), `SYNC_HOST` (`0.0.0.0`), `SYNC_PORT` (`8080`), `PASSWORDS_HASHED`,
`MAX_SYNC_PAYLOAD_MEGS` (`100`). Docker images in `anki/docs/syncserver/`.

**Point a client at it**: Preferences → custom sync URL (stored as `customSyncUrl` in
`qt/aqt/profiles.py`), e.g. `http://192.168.1.200:8080/`. Legacy clients use `SYNC_ENDPOINT` /
`SYNC_ENDPOINT_MEDIA` env vars.

### Synced tables

`notes`, `cards`, `revlog`, `graves` (tombstones), `notetypes`, `decks`, `deck_config`, `tags`,
`config`. The `col` table holds sync metadata: `mod`, `scm`, `ls` (last sync), `usn`, `crt`, `ver`.
Media syncs separately with its own USN.

---

## 7. Mapping to `project_guidelines.md`

| Requirement | Where it lands in this codebase |
|-------------|---------------------------------|
| **Real Rust change** (7a: points-at-stake queue / topic-aware scheduling / mastery query) | `rslib/src/scheduler/queue/builder/` + `rslib/src/storage/card/mod.rs`; expose via new RPC in `proto/anki/scheduler.proto`. |
| **New protobuf message called from Python** | `proto/anki/*.proto` → regenerate → impl in `rslib/.../service.rs` → call from `pylib/anki/...`. |
| **3 Rust unit tests + 1 Python test** | Rust `#[cfg(test)]` in the touched module; Python test in `pylib/tests/`. |
| **Undo still works / no corruption** | Operations go through the collection op/undo framework (`rslib/src/collection/`, op changes); must return `OpChanges`. |
| **Memory model + give-up rule** | FSRS already provides memory (`rslib/src/scheduler/fsrs/`); the give-up/abstain logic + ranges are new code we add (likely a new backend RPC + dashboard). |
| **Performance & Readiness scores** | Not in Anki — net-new models + new protobuf RPCs + dashboard UI (`ts/`). |
| **Two apps, one engine, sync** (§3, 7b) | Desktop uses `rslib` via PyO3; phone reuses `rslib` via AnkiDroid (Option A) or new FFI crate (Option B); both sync through self-hosted `anki-sync-server`. |
| **Coverage map / abstain** (7c) | New query over deck/topic tags + dashboard logic. |
| **Desktop installer + phone build, AI-off mode** | `tools/build-installer`; mobile per chosen option; AI must be feature-flagged off. |
| **License: AGPL-3.0-or-later + credit Anki** | Upstream is AGPL-3.0 (some BSD-3-Clause parts); keep `anki/LICENSE` and attribution. |

### Biggest risks / unknowns (flagged for follow-up)

1. **Mobile is greenfield here.** No Android/iOS build exists in this repo. The fastest path that
   shares the *real* engine is forking/pinning **AnkiDroid** to our `rslib`. We should validate the
   AnkiDroid build (Gradle + NDK + JNI) early — the guidelines warn teams who leave the mobile/Rust
   build until Thursday won't finish.
2. **Conflict rule** is last-write-wins by timestamp. 7b's "same card on both devices offline" test
   needs us to either document this clearly or strengthen it.
3. **Three separate scores** (memory/performance/readiness) and their AI/eval pipeline are entirely
   new — Anki only gives us memory (FSRS) and the engine to hang the rest on.

---

## 8. Quick Command Reference

```bash
# Desktop dev
cd anki && just run

# Full checks (lint + Rust/Python/TS tests)
cd anki && just check

# Rust tests only / Python tests only
cd anki && just test-rust
cd anki && just test-py

# Wheels + installer
cd anki && ./tools/build              # → out/wheels/
cd anki && tools/build-installer      # → out/installer/dist/

# Self-hosted sync server
cd anki && cargo install --path rslib/sync && SYNC_USER1=me:pw anki-sync-server
```
