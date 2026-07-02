# HANDOFF → backend/integration agent: STAT UI overhaul is finished on its branch

**From:** the UI agent (Lane C). **Status:** ✅ done on my side, **awaiting your wave‑2 + integration.**
This file is intentionally **untracked** (a note on the desk) so it doesn't touch your merge
train. Delete it once you've absorbed it. Please add the ledger row (below) when you integrate.

## What's ready

- **Branch / worktree:** `feat/ui-overhaul` @ `8ff029a`, checked out at
  `/Users/atakle/aai-anki-speedrun-ui`. Based off `integration/next-slice` @ `a568cfb`.
- **Not merged, not yet type‑checked together** (deferred to the merge‑train `just check` in your
  provisioned env — the isolated worktree lacks `ftl/core-repo` + `ftl/qt-repo`).
- **Frontend only.** No `proto/`, `rslib`, `pylib`, generated, or lockfile changes. The only
  non‑`ts/` edit is the **`mediasrv.py` sveltekit page allowlist** (allowlist entries only —
  RPC‑exposure lines untouched).

### Files (35, all on `feat/ui-overhaul`)
- **Foundation** `anki/ts/lib/speedrun/` (18): `tokens.ts` + `_tokens.scss`, `types.ts` (mirrors
  `speedrun.proto` as `*View`), `speedrunClient.ts` (typed adapter), `display.ts`(+`.test.ts`),
  `personas.ts`, `nav.ts`, `index.ts` (barrel), and components `AppShell`, `ReadinessGauge`,
  `VitalCard`, `VitalReadout`, `PointsAtStakeRow`, `LoopPathway`, `Chip`, `ConfidenceChip`,
  `StatusDot`.
- **Screens** `anki/ts/routes/`: `today/`, `trajectory/`, `reviewer/`, `import/`, `errors/`.
- **`anki/qt/aqt/mediasrv.py`**: `is_sveltekit_page()` allowlist += `today, reviewer, import,
  errors, trajectory`.

## The decoupling seam (why wiring is cheap)

The UI runs today on a **mock adapter**. Every score/signal goes through
`anki/ts/lib/speedrun/speedrunClient.ts`, and each function has a single
`// TODO(swap): replace mock with <fn> from "@generated/backend"` marker. Wiring each RPC is a
one‑line swap. `getMemoryScore` is already wired to the real RPC (with a mock fallback).

## Integration steps for you (once your part is done)

1. **Merge** `feat/ui-overhaul` → `integration/next-slice`.
2. **Run the full `just check`** (merge‑train gate) in the provisioned worktree; fix any TS/Svelte
   errors on the branch (the three screen lanes were built against the `svelte-check`‑clean
   foundation and self‑reviewed, but were **not** compiled together).
3. **Swap mocks → real RPCs** at the `// TODO(swap)` seams in `speedrunClient.ts`. Confirm the
   adapter's `*View` interfaces in `types.ts` match the actual proto message shapes; reconcile if
   they drifted:
   - `getMemoryScore` — already real.
   - `getPerformanceScore` — RPC merged (`feat/performance-score`) → swap.
   - `getPointsAtStake` — RPC merged (`feat/points-at-stake-rpc`) → swap.
   - `import` screen action — wire to the merged **F2** ingestion RPC (`feat/f2-qbank-ingest`).
   - `getCoverageMap`, `getNextAction`, and the error‑log/relink flow — wire when **wave 2**
     (`feat/coverage-map`, `feat/next-action-hero`, `feat/f3-relink-misses`) lands.
4. **GUI entry:** routes are allowlisted; make sure the Tools menu (`feat/w4-speedrun-menu`) links
   the five pages. Viewable at `http://localhost:40000/_anki/pages/{today,reviewer,import,errors,trajectory}`.
5. **Data:** import a QBank CSV via F2 so memory/performance/points‑at‑stake populate for real.
   **Readiness must keep abstaining** ("NOT ENOUGH INFO") until calibration exists — honesty bar.

## Flags from the screen lanes (decide during integration)

- **`/import` route:** the foundation added a **new STAT `/import`** allowlist entry (distinct from
  Anki's `import-page` / `import-csv` / `import-anki-package`, which ingest decks). The import lane
  used the new `/import` intentionally (it ingests QBank *attempts*, not decks). If you'd rather
  reuse `import-page`, update `nav.ts` and drop the `import` allowlist entry.
- **Not on the contract yet (UI worked around locally, flagging for a future freeze):**
  - Readiness `target` / `unlock` / confidence descriptor aren't on `ReadinessScoreView` — Today
    reads them from `personas.ts`. Consider adding them to the proto.
  - No readiness/coverage **time‑series** — the Trajectory screen uses route‑local illustrative
    data. A `GetReadinessHistory`‑style RPC would make it live.
  - No **per‑topic memory** — the memory↔performance gap bars use route‑local recall. Per‑topic
    recall on the performance topics (or a `TopicMemoryView`) would make it live.
- **Minor:** `AppShell` content padding is hardcoded (reviewer full‑bleed uses `-16px`); a
  `--stat-content-pad` var would be cleaner. No shared `Button`/`Segmented` primitive — screens
  styled those locally.

## Suggested `docs/feature_ledger.md` row (add on integration)

```
| Fui | STAT UI overhaul (Today/Reviewer/Import/Errors/Trajectory + `$lib/speedrun` foundation) | C | feat/ui-overhaul | – | F2/PERF/PAS (+wave2 for full wiring) | ready-for-integration | 8ff029a | UI-OVERHAUL-HANDOFF.md | frontend-only; mock adapter w/ swap seams; needs merge-train `just check` |
```

— UI agent, branches finished. Ping me (via the human) when wave 2 is in and I'll do the adapter
swaps + validation if you'd like, or you can run the steps above directly.
