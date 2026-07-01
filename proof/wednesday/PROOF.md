# Wednesday — Proof Bundle

Evidence packet for the Wednesday deadline (`docs/project_guidelines.md` §6,
`docs/wednesday_plan.md` §1). The core slice is on `main` @ **`78b6003`** and green.

| Required proof | Status | Artifact |
|---|---|---|
| **Commit hash** | ✅ | `main` @ `78b6003` (`logs/commit.txt`) |
| **Test output** | ✅ | `logs/feature_proof.txt` (real-backend, all pass) + full `just check` (see below) |
| **Clean-build** | ✅ log / 🟡 recording | `just build` from source succeeds (~75 s first build; incremental after). Screen recording = capture-checklist item. |
| **Clean-install (Linux, clean machine)** | ✅ | `proof/wednesday/linux/` — `anki-26.5-linux-aarch64.tar.zst` (191 MB) built in a container, then launched on a **separate clean container** (runtime libs + xvfb only): `Anki 26.05`, engine creates a collection AI-off, GUI opens + exits 0. Repro: `build_and_verify.sh all`. |
| **Phone review session** | ✅ | `mobile/ios/proof/review_session.mp4` — iOS Simulator app: 3 seeded cards → 6 real scheduler answers on shared `rslib` (New 3→0), "All caught up" (W2). |
| **Benchmark (7h)** | ✅ | `just bench`: build_queues ≈9.1 ms, topic_mastery ≈7.0 ms, memory_score ≈6.9 ms (10k-card deck; all < §10 targets). |
| **macOS installer** | ✅ | `Anki.app` + `anki-26.5-mac-apple.dmg` (`proof/wednesday/INSTALLER.md`). |

## Automated test coverage (repeatable)
- **Feature interactivity proof** (real backend, self-checking) — `logs/feature_proof.txt`:
  ```
  RESULT: ALL CHECKS PASSED — every feature computed from its inputs.
  ```
  Run: `cd anki && PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 ./out/pyenv/bin/python ../proof/wednesday/feature_proof.py`
- **Rust engine:** `cargo test -p anki --lib speedrun::` → 17 passed; `points_at_stake` → 5 passed.
- **Full gate:** `just check` green — Rust (537) / Python (123) / TypeScript (55) / clippy / fmt / mypy / lints.
- **End-to-end review loop:** `proof/wednesday/review_loop_demo.py` (points-at-stake interleaving, 9/9 drain, undo, `integrity_check == ok`).

## What each feature proves (see the interactivity proof output)
- **F5** points-at-stake: queue order *changes when weights change* (X-first ↔ Z-first), interleaved.
- **F6** memory score: *abstain → abstain → scored range* as coverage goes 0% → 20% → 60% (honesty bar).
- **F4** mastery: mastered/total/recall computed; suspended/buried excluded; `recall_card_count` backs recall.
- **F1** store: topics persist and `undo()` reverts (no corruption).

## Capture checklist (the two screen recordings)
Not automatable headlessly — capture on a display:
1. **Clean-build:** screen-record `just build` from a clean `out/` (or attach the build log).
2. **Clean-install (Linux):** produced by W1 in a clean container (xvfb) — see `proof/wednesday/linux/`.
3. **Phone review:** produced by W2 via the iOS Simulator recorder — `mobile/ios/proof/`.
