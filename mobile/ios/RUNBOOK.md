# Speedrun iOS — RUNBOOK (W2, Lane E)

A **runnable iOS Simulator app** that runs a real Anki review session on the
**shared Rust engine** (`anki/rslib`) through the existing C FFI
(`mobile/speedrun-ffi`) → Swift wrapper (`SpeedrunKit`) → SwiftUI (`ReviewView`).
No scheduling logic lives in Swift: the queue, FSRS scheduling and SQLite
storage all run in Rust, on the phone.

```
SwiftUI ReviewView ─▶ SpeedrunKit (SpeedrunEngine) ─▶ C ABI (speedrun_ffi.h)
                                                        └─▶ anki/rslib  (SHARED ENGINE)
```

## What this produces

- `build/SpeedrunFFI.xcframework` — the shared engine compiled for iOS
  (`ios-arm64` device + `ios-arm64-simulator` static-lib slices). *(gitignored;
  rebuild with `./build-xcframework.sh`)*
- `Speedrun.xcodeproj` — generated from `project.yml` by **XcodeGen**, with two
  targets:
  - **SpeedrunKit** (framework) — the Swift wrapper + `ReviewView`; links the
    xcframework, so `import SpeedrunFFI` resolves to its clang module, and pulls
    in the system libs the Rust staticlib needs.
  - **SpeedrunApp** (application, `@main`) — seeds a few Step‑2 cards on first
    launch and shows `ReviewView`.
- `proof/review_session.mp4` — a Simulator screen recording of a full review
  session on the shared engine (plus `launch_state.png` / `end_state.png`).

## Prerequisites (confirmed on this machine)

- macOS, Apple Silicon; **Xcode 26.6** (`xcodebuild -version`).
- Rust toolchain **1.92.0**, pinned by `../speedrun-ffi/rust-toolchain.toml`.
- iOS Rust targets:
  ```bash
  rustup target add aarch64-apple-ios-sim aarch64-apple-ios
  ```
- **XcodeGen**: `brew install xcodegen`
- **iOS Simulator runtime** — needed to *boot* a simulator (Xcode ships the
  compile SDK but not the runtime). One-time, ~8.5 GB:
  ```bash
  xcodebuild -downloadPlatform iOS
  ```

## 1 — Build the engine xcframework

Run from `mobile/ios/`:

```bash
# The anki engine's proto build needs protoc. speedrun-ffi/.cargo/config.toml
# points PROTOC at ../../anki/out/extracted/protoc/bin/protoc; if THIS checkout
# hasn't run the full Anki build, point PROTOC at any Anki checkout that has it:
export PROTOC=/Users/atakle/aai-anki-speedrun/anki/out/extracted/protoc/bin/protoc

# If CARGO_TARGET_DIR is redirected to a shared cache in your shell, unset it so
# the build is self-contained in mobile/speedrun-ffi/target/ (what the scripts
# expect). A stale shared cache can break libsqlite3-sys codegen.
unset CARGO_TARGET_DIR

./build-xcframework.sh          # -> build/SpeedrunFFI.xcframework
```

## 2 — Generate the Xcode project

`project.yml` is the source of truth; the committed `Speedrun.xcodeproj` is
generated from it. Regenerate any time with:

```bash
xcodegen generate
```

## 3 — Build for the Simulator

```bash
xcodebuild -project Speedrun.xcodeproj -scheme SpeedrunApp \
  -configuration Debug \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  -derivedDataPath build/DerivedData build
# => ** BUILD SUCCEEDED **
```

Pick any available device from `xcrun simctl list devices available`.

## 4 — Boot, install, launch, and record a review session

```bash
UDID=$(xcrun simctl list devices available | awk -F '[()]' '/iPhone 17 \(/{print $2; exit}')
APP=build/DerivedData/Build/Products/Debug-iphonesimulator/SpeedrunApp.app

xcrun simctl boot "$UDID"; xcrun simctl bootstatus "$UDID"
xcrun simctl install "$UDID" "$APP"

# record while the app auto-drives a review session on the shared engine
xcrun simctl io "$UDID" recordVideo --codec h264 --force proof/review_session.mp4 &
REC=$!
sleep 2
xcrun simctl launch "$UDID" com.speedrun.ios --autodemo
sleep 20
kill -INT $REC; wait $REC          # SIGINT finalizes the .mp4
```

- **Interactive run** (tap Again / Hard / Good / Easy yourself): launch WITHOUT
  `--autodemo`.
- `--autodemo` makes the app auto-answer each engine-served card "Good" — every
  step is a real `engine.answer()` / `nextCard()` / `counts()` call on `rslib`,
  just paced with pauses so each state is visible for the recording. `simctl`
  can't inject taps headlessly, so this is how the session is driven for proof.
- To re-record from a clean `New = 3` state, `xcrun simctl uninstall "$UDID"
  com.speedrun.ios` first (clears the app's on-device collection).

## What works

- The shared `rslib` **cross-compiles and links into an iOS app** (simulator
  arm64): FSRS, burn, bundled SQLite, zstd, reqwest, tokio, etc.
- The app **launches on the Simulator** and renders the engine's due counts and
  the **engine-rendered question**; answering runs the **real FSRS scheduler
  transaction** (undo-safe) in Rust.
- Full session proven in `proof/review_session.mp4`: 3 seeded Step‑2 cards →
  answered Good through their learning steps (**6 scheduler answers**) → counts
  `New 3→0`, Learn cycles → **"All caught up."**

## Remaining gaps / notes

- **Simulator only.** A real *device* install needs signing/provisioning (not
  attempted here). The device `cdylib` link still hits blake3's
  `___chkstk_darwin`; the app path uses the **staticlib** (the device slice
  builds fine as a staticlib, and is included in the xcframework).
- The **engine build hash shows blank** in the footer for a standalone `cargo`
  build (the `buildhash` is only injected by Anki's ninja build) — cosmetic.
- The ABI renders only the **question** (no back/answer reveal yet) and seeds
  cards via the FFI helper, matching the current `speedrun_ffi.h` contract.
  Two‑way sync is out of scope for W2.
```
