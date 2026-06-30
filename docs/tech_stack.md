# Speedrun Tech Stack

The full technology stack for Speedrun (the Step 2 CK study app built on the Anki fork). Linked from
the Tech Stack section of `docs/PRD.md`. It keeps Anki's layered architecture — clients talk to one
shared Rust engine across a single **protobuf contract** — and adds a Rust `speedrun` engine module, a
phone client on the *same* engine, a Step 2 dashboard, resource ingestion, and a deferred AI/eval
lane. The product runs fully with **AI off**.

See `docs/codebase_notes.md` for where each piece lives in the source tree, and
`docs/factory_workflow.md` for how the lanes map onto this stack.

```mermaid
flowchart TB
  subgraph Clients["Client apps"]
    Desktop["Desktop app<br/>PyQt6 shell (aqt)<br/>hosts web views via mediasrv HTTP"]
    WebUI["Web UI<br/>Svelte + TypeScript (ts/)<br/>reviewer · editor · Step 2 dashboard"]
    Mobile["Mobile app<br/>AnkiDroid fork (Kotlin/JNI)<br/>or iOS via C-FFI"]
  end

  subgraph Contract["Cross-language contract (proto seam)"]
    Proto["proto/anki/*.proto<br/>+ speedrun.proto (SpeedrunService)"]
    Codegen["codegen<br/>prost (Rust) · _backend_generated.py · backend.ts"]
    Proto --> Codegen
  end

  subgraph Bridges["Language bridges"]
    PyO3["PyO3 rsbridge<br/>(_rsbridge.so) — desktop"]
    JNI["JNI / rsdroid — Android"]
    CFFI["C-FFI / UniFFI — iOS (option)"]
  end

  subgraph Engine["Shared Rust engine — rslib (crate anki)"]
    Sched["scheduler + FSRS<br/>(memory model)"]
    Speedrun["speedrun module (new)<br/>points-at-stake order · topic mastery<br/>memory/performance/readiness scores · give-up rule"]
    Storage["storage (SQLite)<br/>cards · notes · revlog · config"]
    Search["search · import/export"]
    SyncCore["sync client + server logic"]
  end

  subgraph Data["Per-device data"]
    Col["collection.anki2 (SQLite)"]
    Conf["col.conf JSON<br/>taxonomy · crosswalk · weights · weakness · QBank attempts"]
    Media["media folder"]
  end

  subgraph Ingest["Resource ingestion (F2, AI-free)"]
    Importers["CSV/JSON/paste importers<br/>UWorld · AMBOSS · NBME/Free120"]
    Records["QuestionAttempt · PracticeTestResult"]
  end

  subgraph Sync["Sync infra"]
    SyncServer["anki-sync-server (Rust/Axum)<br/>hub-and-spoke over HTTP"]
  end

  subgraph AIeval["AI / eval — DEFERRED (post-MVP, runs with AI off)"]
    ML["ml/ pipeline<br/>card-gen · eval harness · baselines · leakage check"]
  end

  subgraph Build["Build & tooling"]
    Just["just -> n2/ninja -> runner"]
    Toolchain["Rust 1.92 (cargo) · uv/Python · node/yarn · protoc"]
    Installer["Briefcase installer (desktop)"]
  end

  WebUI -->|"HTTP POST protobuf"| Desktop
  Desktop --> PyO3
  Mobile --> JNI
  Mobile --> CFFI
  PyO3 --> Engine
  JNI --> Engine
  CFFI --> Engine
  Codegen --> WebUI
  Codegen --> PyO3
  Codegen --> JNI
  Engine --> Contract
  Speedrun --> Storage
  Sched --> Storage
  Storage --> Col
  Storage --> Conf
  Storage --> Media
  Importers --> Records
  Records --> Conf
  SyncCore --> SyncServer
  Desktop -. sync .-> SyncServer
  Mobile -. sync .-> SyncServer
  ML -. "future, behind flag" .-> Speedrun
  Build --> Engine
  Installer --> Desktop
```

## Layer notes

- **One engine, many clients.** Desktop (PyO3) and mobile (JNI / C-FFI) both call the *same* `rslib`
  via `run_service_method(service, method, protobuf_bytes)`. The mandatory Rust change (points-at-stake
  queue) therefore ships to both platforms automatically.
- **Contract-first.** `proto/anki/*.proto` plus our additive `speedrun.proto` are the only
  cross-layer API; codegen produces Rust traits, Python `_backend_generated.py`, and TS `backend.ts`.
  Our additions are isolated in `speedrun.proto` to keep upstream merges cheap.
- **No schema migration for the MVP.** Topic taxonomy, card→topic crosswalk, blueprint weights,
  per-topic weakness, and imported QBank attempts are stored as JSON in `col.conf` (sync- and
  undo-safe). A dedicated table can replace this post-MVP.
- **Sync is hub-and-spoke**, not peer-to-peer: each device merges with `anki-sync-server`; the
  conflict rule is last-write-wins by modification time (see `docs/codebase_notes.md` §6).
- **AI is deferred.** The `ml/` lane (card generation, eval, baselines, leakage check) attaches behind
  a feature flag after the MVP; the core scores and queue run with AI switched off.
