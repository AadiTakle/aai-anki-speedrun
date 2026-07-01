# Speedrun mobile (F12) — iOS on the shared Rust engine

This lane delivers the **foundation for a phone app that runs a real review
session on the shared Anki engine (`rslib`)** — not a JS/Swift reimplementation
of the scheduler. On macOS/Apple Silicon the mobile path is **iOS: Swift ↔
`rslib` over a C FFI** (see `AGENTS.md` / `docs/codebase_notes.md` §5, "Option B
— custom native FFI crate").

Everything here is new code under `mobile/`. Nothing in `anki/` was modified.

## What's here

```
mobile/
├── speedrun-ffi/            # Rust crate: minimal C ABI over rslib (the engine)
│   ├── Cargo.toml           #   standalone workspace; path-dep on ../../anki/rslib
│   ├── .cargo/config.toml   #   points PROTOC at anki's bundled protoc
│   ├── rust-toolchain.toml  #   pins 1.92.0 (same as the engine)
│   ├── include/
│   │   ├── speedrun_ffi.h   #   hand-written C header (the contract)
│   │   └── module.modulemap #   clang module for Swift/xcframework import
│   ├── src/lib.rs           #   open / counts / next-card / answer / seed
│   └── tests/review_loop.rs #   end-to-end review session through the C ABI
└── ios/                     # Swift client
    ├── Package.swift        #   SwiftPM: SpeedrunFFI (C) + SpeedrunKit (Swift)
    ├── Sources/SpeedrunFFI/module.modulemap   # re-uses the crate's header
    ├── Sources/SpeedrunKit/SpeedrunEngine.swift  # idiomatic Swift wrapper
    ├── Sources/SpeedrunKit/ReviewView.swift      # SwiftUI review screen
    ├── Sources/speedrun-demo/main.swift          # host CLI review loop (opt-in)
    ├── App/SpeedrunApp.swift # iOS app entry (for an Xcode app target)
    └── build-xcframework.sh  # builds SpeedrunFFI.xcframework (device + sim)
```

The C ABI (see `speedrun-ffi/include/speedrun_ffi.h`) is deliberately minimal —
just enough for a review session:

| C function | rslib call it drives |
|---|---|
| `speedrun_open(path)` / `speedrun_close` | `CollectionBuilder::build()` |
| `speedrun_counts(...)` | `Collection::get_queued_cards` (new/learning/review) |
| `speedrun_next_card(...)` | `Collection::get_next_card` + `render_existing_card().question()` |
| `speedrun_answer_card(id, ease, ms)` | `get_scheduling_states` + `answer_card` (undo-safe transaction) |
| `speedrun_add_basic_note(front, back)` | `add_note` (demo/seed convenience) |
| `speedrun_version()` | `anki::version::buildhash()` |

## Feasibility probe (this machine)

```
$ xcode-select -p
/Applications/Xcode.app/Contents/Developer
$ xcodebuild -version
Xcode 26.6
Build version 17F113
$ swift --version
swift-driver version: 1.148.6 Apple Swift version 6.3.3 (swiftlang-6.3.3.1.3 clang-2100.1.1.101)
Target: arm64-apple-macosx26.0
$ rustc --version           # via rust-toolchain.toml (pins 1.92.0, same as the engine)
rustc 1.92.0 (ded5c06cf 2025-12-08)
$ rustup target list --toolchain 1.92.0 --installed
aarch64-apple-darwin
aarch64-apple-ios          # added: rustup target add aarch64-apple-ios
aarch64-apple-ios-sim      # added: rustup target add aarch64-apple-ios-sim
```

Conclusion: the toolchain is complete — Xcode + Swift present, and both iOS Rust
targets install and compile the engine.

## Build & run

All commands are run from `mobile/`. The engine's proto build step needs
`protoc`; `speedrun-ffi/.cargo/config.toml` already points `PROTOC` at the
bundled `anki/out/extracted/protoc/bin/protoc`, so a plain `cargo build` works.

### 1. Rust FFI (host) — compile + prove a review session runs

```bash
cd speedrun-ffi
cargo build          # builds libspeedrun_ffi.{a,dylib} for the host
cargo test           # runs the end-to-end review-loop test through the C ABI
```

### 2. Cross-compile the engine for iOS

```bash
cd speedrun-ffi
cargo build --target aarch64-apple-ios-sim               # simulator (arm64)
cargo rustc --lib --crate-type staticlib \
      --target aarch64-apple-ios                          # device (arm64) static lib
```

Device note: a *full* `cargo build --target aarch64-apple-ios` also links a
`cdylib`, which currently fails at link time on `blake3`'s NEON assembly symbol
`___chkstk_darwin`. That link step is irrelevant for an app (apps link the
`.a`), so the device build uses `cargo rustc --crate-type staticlib` to build
only the static library, which succeeds.

### 3. Package as an XCFramework (for Xcode)

```bash
cd ios
./build-xcframework.sh          # -> ios/build/SpeedrunFFI.xcframework (gitignored)
```

Produces a framework with `ios-arm64` (device) and `ios-arm64-simulator` slices,
each carrying the C header + module map so Swift can `import SpeedrunFFI`.

### 4. Swift binding — compile the wrapper + UI

```bash
cd ios
swift build                     # compiles SpeedrunKit against the C module map
```

### 5. Prove it from Swift (host CLI)

```bash
cd speedrun-ffi && cargo build              # produce target/debug/libspeedrun_ffi.a
cd ../ios
SPEEDRUN_BUILD_DEMO=1 swift run speedrun-demo
```

Runs a review session **through Swift → C ABI → rslib** on the host.

### 6. Run the app in the iOS Simulator (Xcode, GUI step)

1. `./ios/build-xcframework.sh`
2. In Xcode: create an iOS App target, drag in `ios/build/SpeedrunFFI.xcframework`
   (Embed & Sign), add the local SwiftPM package `ios/` (product `SpeedrunKit`),
   and add `ios/App/SpeedrunApp.swift` as the app's `@main`.
3. Build & run on an iOS Simulator. `SpeedrunApp` seeds a few Step-2 demo cards
   on first launch and shows `ReviewView`.

## Gate output (verbatim, captured on this machine)

**`cargo build` (host `aarch64-apple-darwin`):**
```
   Compiling anki v0.0.0 (/Users/atakle/aai-anki-speedrun/anki/rslib)
   Compiling speedrun-ffi v0.1.0 (/Users/atakle/aai-anki-speedrun/mobile/speedrun-ffi)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.69s
```

**`cargo test` (review session on the shared engine):**
```
running 2 tests
test open_bad_path_reports_error ... ok
test review_session_runs_on_shared_engine ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

**`cargo build --target aarch64-apple-ios-sim` (iOS simulator):**
```
   Compiling anki v0.0.0 (/Users/atakle/aai-anki-speedrun/anki/rslib)
   Compiling fsrs v5.2.0
   Compiling speedrun-ffi v0.1.0 (/Users/atakle/aai-anki-speedrun/mobile/speedrun-ffi)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.13s
```

**`cargo rustc --crate-type staticlib --target aarch64-apple-ios` (iOS device static lib):**
```
   Compiling speedrun-ffi v0.1.0 (/Users/atakle/aai-anki-speedrun/mobile/speedrun-ffi)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.37s
# libspeedrun_ffi.a -> architecture: arm64
```

**`xcodebuild -create-xcframework`:**
```
xcframework successfully written out to: .../mobile/ios/build/SpeedrunFFI.xcframework
SpeedrunFFI.xcframework/{Info.plist, ios-arm64/{Headers,libspeedrun_ffi.a}, ios-arm64-simulator/{Headers,libspeedrun_ffi.a}}
```

**`swift build` (SwiftPM binding):**
```
[3/5] Compiling SpeedrunKit SpeedrunEngine.swift
[4/5] Compiling SpeedrunKit ReviewView.swift
[5/5] Emitting module SpeedrunKit
Build complete! (4.54s)
```

**`SPEEDRUN_BUILD_DEMO=1 swift run speedrun-demo` (Swift → C → rslib):**
```
shared engine build: bb741ecd
seeded -> new=2 learning=0 review=0
Q: Which artery supplies the SA node in most people?
  answered Good -> new=1 learning=1 review=0
Q: First-line management of stable narrow-complex SVT?
  answered Good -> new=0 learning=2 review=0
Q: Which artery supplies the SA node in most people?
  answered Good -> new=0 learning=1 review=0
Q: First-line management of stable narrow-complex SVT?
  answered Good -> new=0 learning=0 review=0
review session complete; 4 card(s) answered on the shared engine.
```
(Linking the host CLI also prints `ld: warning: object file ... built for newer
'macOS' version (26.5) than being linked (12.0)` — harmless min-version warnings
from the host SDK; the link and run succeed.)

## What works / what's blocked / next steps

**Works**
- The **shared engine cross-compiles for iOS** (simulator arm64: full build;
  device arm64: static lib) — `fsrs`, `burn`, bundled SQLite (`rusqlite`),
  `zstd`, `reqwest`, `tokio`, etc. all build for iOS.
- A **minimal C ABI** over `rslib` (open / counts / next-card / answer / seed),
  with a hand-written header + clang module map.
- A **real review session runs through the C ABI** — proven by the Rust
  integration test *and* by the Swift host CLI (Swift → C → rslib): seed, pull
  the engine-rendered question, and answer via the actual scheduler transaction.
- The engine packages into an **`.xcframework`** (device + simulator slices).
- The **Swift wrapper + SwiftUI review screen compile** (`swift build`).

**Blocked / not done in this pass**
- **A fully wired, runnable `.xcodeproj` iOS app.** SwiftPM cannot emit an iOS
  `.app`, and creating/opening an Xcode project is a GUI step not scriptable in
  this headless environment. All the pieces are here (xcframework + SwiftPM
  package + SwiftUI screen + `@main` app file); §6 above is the remaining manual
  wiring, which I could not execute/screenshot here.
- **iOS device `cdylib`** link fails on `blake3`'s `___chkstk_darwin` (NEON asm).
  Worked around for the static lib (what apps need); a proper fix is a
  `blake3`/deployment-target tweak or `blake3` `pure` feature.
- **Two-way sync** was explicitly out of scope for Wednesday (F12 = build + run a
  review session).

**Next steps**
1. Wire and record the Simulator app run (§6) to close F12's "runs a real review
   session on the phone" line.
2. Add `answer`-side rendering (`render_existing_card().answer()`) to the ABI so
   the UI can reveal the back of the card.
3. Import a real Step 2 deck (`.apkg`) via `rslib`'s import path instead of the
   seed helper.
4. Android: the same crate already builds a `cdylib`; add a JNI shim + NDK
   targets (`aarch64-linux-android`, …) for the AnkiDroid-style path.

## Implementation notes

- **Standalone workspace.** `speedrun-ffi/Cargo.toml` has an empty `[workspace]`
  so it does **not** join the vendored Anki cargo workspace. Building it only
  writes a separate `mobile/speedrun-ffi/Cargo.lock` + `target/`; it never
  touches `anki/Cargo.toml` / `anki/Cargo.lock` (supervisor-owned).
- **tokio feature unification.** `rslib` uses tokio `io-util`/`fs` methods but,
  inside the Anki workspace, relies on feature unification from sibling crates to
  enable them. A standalone build loses that, so `speedrun-ffi` depends on
  `tokio` with `features = ["full"]` purely to re-unify those features up to
  `rslib` (we don't call tokio directly).
- **`CARGO_TARGET_DIR`.** In this build environment the target dir is redirected
  to a cache; normal checkouts build into `mobile/speedrun-ffi/target/`. The
  `-L` path in `Package.swift` and `build-xcframework.sh` assume the normal
  location.
- **License:** AGPL-3.0-or-later (same as Anki), since this links `rslib`.
