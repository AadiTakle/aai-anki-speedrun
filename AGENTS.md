# AGENTS.md

This repository vendors the [Anki](https://github.com/ankitects/anki) desktop
source under `anki/`. The actual project (Rust core + Python/PyQt GUI +
Svelte/TypeScript frontend) lives entirely in `anki/`.

## Cursor Cloud specific instructions

- **Run everything from `anki/`.** All build/run/test/lint commands live there
  and are driven by `just` (recipes wrap a custom ninja/`runner` build system).
  See `anki/CLAUDE.md` and `anki/docs/development.md` for the canonical command
  reference; `just --list` shows all recipes.
- **Common commands (from `anki/`):** `just build` (builds pylib + qt),
  `just run` (build + launch the GUI), `just lint`, `just test` (or
  `just test-rust` / `just test-py` / `just test-ts`), `just fmt`, and
  `just check` (full format + build + checks). Web views are served at
  `http://localhost:40000/_anki/pages/` while Anki is running.
- **Self-managed toolchain.** The build downloads its own node, uv (Python),
  protoc and yarn into `anki/out/`; you don't install those manually. Host
  tools used: `just` and `n2` (both installed under `~/.cargo/bin`) and a
  rustup-managed Rust toolchain pinned by `anki/rust-toolchain.toml`
  (auto-downloaded). The first `just build` is slow (compiles the Rust core);
  later builds are incremental.
- **Translation repos are required build deps and are NOT tracked here.**
  `anki/ftl/core-repo` and `anki/ftl/qt-repo` (upstream Anki git submodules)
  must be present or the build fails at configure time (`input .git missing` /
  submodule errors), because this wrapper repo vendored Anki as plain files and
  dropped its `.git`/submodule wiring. The startup update script recreates a
  nested git repo inside `anki/` and clones these two repos so the build's
  `git submodule update` step succeeds. They show up as untracked under
  `anki/ftl/` — do **not** `git add` them into this repo; they are build
  dependencies, not source.
- **The nested `anki/.git` is intentional** (created by the update script) and
  does not interfere with this repo: files under `anki/` that are already
  tracked by `/workspace`'s git remain tracked normally. Run your git
  workflow (commits/PRs) from `/workspace`.
- **Running the GUI in this headless VM.** An X server is available on
  `DISPLAY=:1`. Launch with:
  ```
  DISPLAY=:1 QT_QPA_PLATFORM=xcb \
    QTWEBENGINE_CHROMIUM_FLAGS="--no-sandbox --remote-allow-origins=http://localhost:8080" \
    just run
  ```
  `--no-sandbox` is required — QtWebEngine aborts (SIGABRT, core dump) without
  it inside the container. On first launch Anki shows a language-selection
  dialog; pass `-b <dir>` (e.g. `just run -b /tmp/ankidata`) to use a scratch
  profile directory. Required Qt/xcb system libraries (including
  `libxcb-cursor0`, `libxcb-icccm4`, `libxcb-keysyms1`) are already installed
  in the VM image.
