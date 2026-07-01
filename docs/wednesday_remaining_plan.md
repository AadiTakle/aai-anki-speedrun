# Wednesday — Remaining Work Plan

Supervisor PLAN for closing the **Wednesday** deadline (`docs/project_guidelines.md` §6,
`docs/wednesday_plan.md` §1). The core slice is **on `main` @ `78b6003`** and green. This plan
covers only what's left to hit the full definition-of-done, decomposed along lane seams.

## 1. Definition-of-done: done vs. remaining

| DoD item (`wednesday_plan.md` §1) | State |
|---|---|
| Desktop builds from source | ✅ |
| **Real Rust engine change** end-to-end (≥3 Rust + 1 Py test, undo-safe, no corruption) | ✅ F5 (points-at-stake + interleaving) |
| **Review loop** on the exam deck | ✅ F10 |
| **Memory model**, honest score (range + give-up rule) | ✅ F6 (+ F4, F6-C) |
| **Linux installer** that runs on a **clean machine** | ❌ **W1** (macOS `.app`/`.dmg` done; Linux pending) |
| Mobile: phone app that **builds** + runs a **real review session** on the shared engine | 🟡 **W2** (iOS FFI review session runs via Swift; runnable `.app` pending) |
| **Proof**: commit hash + clean-build recording, test output, clean-install recording, phone-review recording | 🟡 **W3** (commit hash + test output ✅; recordings pending) |

No contract/proto change is required for any P0 item, so **no FREEZE needed** — lanes can fan out
directly off `main`.

## 2. Remaining slices

### P0 — DoD gaps (must land today)

**W1 — Linux installer on a clean machine (Lane B/F).**
- Build the Linux installer via `qt/tools/build_installer.py … build`/`package` **inside a Linux
  container** (the mac-template path is macOS; Linux uses `linux-template`, already vendored).
- Verify it launches on a **fresh** container (no build deps) — that's the "clean machine" bar
  (challenge 7g/installer). Capture the run.
- Owns: `docs/`, a `proof/wednesday/` build+verify script + a `Dockerfile`/compose for the clean box.
- Risk/prereq: **needs Docker on the Mac**. First step is a feasibility probe (`docker version`);
  if absent, fall back to documenting the exact clean-container steps + producing the artifact on
  any available Linux.
- Acceptance: a Linux installer artifact + a recorded/scripted clean-container launch that opens Anki
  with AI off.

**W2 — Runnable mobile app (Lane E).**
- Finish the iOS path so mobile *builds and runs a review session as an app* (not just the FFI CLI):
  generate a real `.xcodeproj`/simulator target linking `mobile/speedrun-ffi`'s `.xcframework`, run a
  review session in the iOS Simulator.
- Owns: `mobile/ios/` only (no `anki/` changes).
- Risk/prereq: Xcode is present (F12 probe confirmed 26.6); SwiftPM can't emit an `.app`, so this
  needs `.pbxproj` generation (xcodegen or hand-wired). Stop-and-report if the Simulator build wall
  is hit; the FFI review session already satisfies the "shared engine" bar as a fallback.
- Acceptance: Simulator screen recording of a review session driven by `rslib` via FFI.

**W3 — Proof bundle + recordings (Lane G).**
- Assemble the Wednesday proof packet: commit hash (`78b6003`), **clean-build** log/recording,
  **test output** (full `just check` + `feature_proof.py`), **clean-install** recording (from W1),
  **phone-review** recording (from W2), and the `just bench` numbers (F13).
- Owns: `proof/wednesday/` (a `PROOF.md` index + capture scripts).
- Honest constraint: **video capture of GUI/phone needs a display** — I produce all
  scriptable/terminal artifacts + a capture checklist; the human records the 2 screen videos (or we
  use the iOS Simulator's built-in recorder for W2).
- Acceptance: `proof/wednesday/PROOF.md` linking every required artifact.

### P1 — High-value polish (makes the demo/recordings clickable)

**W4 — `Tools → Speedrun` menu (Lane B/C).**
- A new API-enabled `AnkiWebViewKind.MEMORY_SCORE` + a dialog that opens the memory-score page, and a
  **Tools → Speedrun** submenu: **Memory Score** (opens the dialog) + **Seed sample data** (runs the
  realistic seed). Removes the Debug-Console/URL friction entirely.
- Owns: `anki/qt/aqt/` (+ the enum in `webview.py`, allowlist already done), a small `speedrun.py`
  dialog module; no proto.
- Acceptance: clicking the menu opens the score in-app (API access works) and seeds with one click;
  `just check` green.

### P2 — Hardening (deferred review items; only if time)

**W5 — engine hardening.** `GetMemoryScore(Empty)` → dedicated request message (avoids a future
re-freeze; proto change → supervisor FREEZE); F5 gather-time selection (so the daily limit can't drop
high-value cards); F4 N+1 → batch fetch for the 50k-card target. Each is a small stacked branch with
tests.

## 3. Execution order & parallelism

Disjoint seams → fan out in parallel off `main`:
- **W1 (infra/Docker)**, **W2 (iOS)**, **W4 (Qt/ts)** are independent → parallel background lane
  workers.
- **W3 (proof)** depends on W1 + W2 outputs → assemble last (but its scriptable parts start now).
- **W5** is optional, stacked, after P0.

Gate each with its lane check; integrate onto `main` (via `integration` FF or direct) as they land;
run `just check` after any that touch `anki/`.

## 4. Risks / long poles
- **Docker availability** (W1) — probe first; without it, the clean-machine *verification* can't run
  locally (artifact still producible).
- **iOS `.app` wiring** (W2) — the one known wall from F12; Simulator run is the target, FFI CLI is
  the fallback proof.
- **Screen recordings** (W3) — not automatable headlessly; human-in-the-loop for the 2 videos.
