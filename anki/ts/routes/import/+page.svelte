<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT · Import → Auto-link (screen lane C3, destination "import").

Ingest a QBank / practice-test block (paste / CSV / JSON), preview the parse +
idempotent dedup, then fire the flagship ORGANIZING ACTION: one calm pulse that
settles into a quiet result — misses become unsuspended AnKing cards + opened
error-log reframes — pointing at the single next move. Built entirely on the
shared foundation ($lib/speedrun); no engine calls, illustrative data only.
-->
<script lang="ts">
    import { onDestroy } from "svelte";

    import { AppShell, Chip, StatusDot, STAT_SIGNAL, acuityLabel } from "$lib/speedrun";

    import { IMPORT_SCENARIO, type ImportFormat } from "./import-mock";

    type Phase = "idle" | "linking" | "settled";

    const scenario = IMPORT_SCENARIO;
    const { preview, autoLink } = scenario;

    // How long the single calm pulse plays before settling (kept in sync with
    // the CSS animation below). Skipped entirely under reduced-motion.
    const PULSE_MS = 900;

    const FORMATS: { id: ImportFormat; label: string }[] = [
        { id: "paste", label: "Paste" },
        { id: "csv", label: "CSV" },
        { id: "json", label: "JSON" },
    ];

    let format: ImportFormat = "paste";
    let text = scenario.samples.paste;
    // "pristine" = still showing a sample, so switching tabs may swap it; once
    // the user edits, we never clobber their input on a tab switch.
    let pristine = true;
    let phase: Phase = "idle";
    let pulseTimer: ReturnType<typeof setTimeout> | undefined;

    $: hasContent = text.trim().length > 0;

    function selectFormat(next: ImportFormat): void {
        format = next;
        if (pristine) {
            text = scenario.samples[next];
        }
    }

    function onInput(): void {
        pristine = false;
    }

    function loadSample(): void {
        text = scenario.samples[format];
        pristine = true;
        resetResult();
    }

    function clearInput(): void {
        text = "";
        pristine = false;
        resetResult();
    }

    function prefersReducedMotion(): boolean {
        return (
            typeof window !== "undefined"
            && typeof window.matchMedia === "function"
            && window.matchMedia("(prefers-reduced-motion: reduce)").matches
        );
    }

    function runImport(): void {
        if (!hasContent || phase === "linking") {
            return;
        }
        clearTimer();
        // Honor reduced-motion: settle immediately, no pulse.
        if (prefersReducedMotion()) {
            phase = "settled";
            return;
        }
        phase = "linking";
        pulseTimer = setTimeout(() => {
            phase = "settled";
            pulseTimer = undefined;
        }, PULSE_MS);
    }

    function resetResult(): void {
        clearTimer();
        phase = "idle";
    }

    function clearTimer(): void {
        if (pulseTimer !== undefined) {
            clearTimeout(pulseTimer);
            pulseTimer = undefined;
        }
    }

    onDestroy(clearTimer);
</script>

<AppShell active="import">
    <Chip slot="badges" dot="stable">SYNCED</Chip>

    <div class="page">
        <header class="intro">
            <p class="eyebrow">Ingest · the organizing action</p>
            <h1 class="headline">Import a Q-block — watch your misses organize themselves</h1>
            <p class="lede">
                Drop a QBank or practice-test block. STAT normalizes it into attempts, dedups
                idempotently, then unsuspends exactly the cards your misses map to and opens an
                error-log reframe for each — one calm pulse, then quiet.
            </p>
        </header>

        <!-- 1 · INPUT — one importer, three source adapters -->
        <section class="panel" aria-labelledby="ingest-h">
            <div class="panel-head">
                <h2 id="ingest-h" class="panel-title">Source block</h2>
                <Chip>{scenario.source}</Chip>
            </div>

            <div class="segmented" role="group" aria-label="Import format">
                {#each FORMATS as f (f.id)}
                    <button
                        type="button"
                        class="seg"
                        class:active={format === f.id}
                        aria-pressed={format === f.id}
                        on:click={() => selectFormat(f.id)}
                    >
                        {f.label}
                    </button>
                {/each}
            </div>

            <label class="field-label" for="block-input">Paste your {format.toUpperCase()} export</label>
            <textarea
                id="block-input"
                class="block-input"
                rows="8"
                spellcheck="false"
                bind:value={text}
                on:input={onInput}
            ></textarea>

            <div class="input-actions">
                <button type="button" class="btn ghost" on:click={loadSample}>Load sample block</button>
                <button type="button" class="btn ghost" on:click={clearInput} disabled={!hasContent}>
                    Clear
                </button>
                <span class="spacer"></span>
                <button type="button" class="btn primary" on:click={runImport} disabled={!hasContent}>
                    Import &amp; auto-link
                </button>
            </div>
        </section>

        <!-- 2 · PARSE + IDEMPOTENT DEDUP preview -->
        {#if hasContent}
            <section class="panel" aria-labelledby="parse-h">
                <div class="panel-head">
                    <h2 id="parse-h" class="panel-title">Parsed preview</h2>
                    <Chip dot="stable">dedup (source, id, ts)</Chip>
                </div>

                <p class="parse-summary">
                    <span class="stat-num">{preview.parsed}</span> parsed
                    <span class="sep">·</span>
                    <span class="stat-num">{preview.fresh}</span> new
                    <span class="sep">·</span>
                    <span class="stat-num dup">{preview.duplicates}</span> duplicates skipped
                </p>

                <ul class="rows" aria-label="Parsed rows (excerpt)">
                    {#each preview.rows as row, i (i)}
                        <li class="prow" class:dup={row.duplicate}>
                            <StatusDot
                                acuity={row.duplicate ? "muted" : row.correct ? "stable" : "critical"}
                                size={8}
                            />
                            <span class="mono id">{row.externalId}</span>
                            <span class="topic">{row.topic}</span>
                            <span class="mono secs">{row.seconds}s</span>
                            <span
                                class="status"
                                class:ok={row.correct && !row.duplicate}
                                class:miss={!row.correct && !row.duplicate}
                                class:skip={row.duplicate}
                            >
                                {row.duplicate ? "deduped" : row.correct ? "correct" : "MISS"}
                            </span>
                        </li>
                    {/each}
                </ul>

                <p class="dedup-note">
                    Showing {preview.rows.length} of {preview.parsed} rows · illustrative. {scenario.dedupNote}
                </p>
            </section>
        {/if}

        <!-- 3 · THE AUTO-LINK MOMENT — one calm pulse, then quiet -->
        <section
            class="result"
            class:idle={phase === "idle"}
            class:pulsing={phase === "linking"}
            class:settled={phase === "settled"}
            aria-live="polite"
        >
            {#if phase === "idle"}
                <div class="result-idle">
                    <StatusDot acuity="muted" size={9} />
                    <p class="idle-copy">
                        Import a block to organize your misses into a review queue.
                    </p>
                </div>
            {:else}
                <span class="pulse-ring" aria-hidden="true"></span>

                <div class="result-head">
                    <p class="eyebrow accent">
                        {phase === "linking" ? "Organizing…" : "Organized · just now"}
                    </p>
                    {#if phase === "settled"}
                        <button type="button" class="linklike" on:click={resetResult}>Undo import</button>
                    {/if}
                </div>

                <p class="result-headline">
                    <span class="stat-num big">{autoLink.misses}</span> misses →
                    <span class="stat-num big">{autoLink.cardsUnsuspended}</span> AnKing cards unsuspended
                    <span class="sep">·</span>
                    <span class="stat-num big">{autoLink.reframesOpened}</span> error-log reframes opened
                </p>

                <div class="topic-chips">
                    {#each autoLink.topics as t (t.topicId)}
                        <span
                            class="topic-chip"
                            style="color: {STAT_SIGNAL[t.acuity]}; border-color: {STAT_SIGNAL[t.acuity]}"
                            title="{acuityLabel(t.acuity)} · {t.cards} cards mapped"
                        >
                            {t.name} · {t.misses}
                            {t.misses === 1 ? "miss" : "misses"} → {t.cards} cards
                        </span>
                    {/each}
                </div>
                <p class="chips-caption">Where your {autoLink.misses} misses landed · one reframe each.</p>

                {#if phase === "settled"}
                    <p class="settled-copy">
                        Nothing else demands attention. Your next move is one review session — the queue
                        is already built.
                    </p>
                    <div class="result-actions">
                        <a class="btn primary" href="/reviewer">Start targeted review →</a>
                        <a class="btn ghost" href="/errors">Open error log →</a>
                    </div>
                {/if}
            {/if}
        </section>
    </div>
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .page {
        display: flex;
        flex-direction: column;
        gap: 20px;
        max-width: 760px;
        margin-inline: auto;
    }

    .eyebrow {
        @include stat.eyebrow;
        margin: 0;

        &.accent {
            color: stat.$primary;
        }
    }

    .headline {
        font-family: stat.$font-display;
        font-weight: 800;
        letter-spacing: -0.01em;
        font-size: clamp(1.35rem, 4vw, 1.85rem);
        line-height: 1.15;
        color: stat.$ink;
        margin: 6px 0 0;
    }

    .lede {
        color: stat.$ink-soft;
        font-size: 0.95rem;
        line-height: 1.5;
        margin: 10px 0 0;
        max-width: 62ch;
    }

    .intro {
        margin: 0;
    }

    // --- panels ------------------------------------------------------------
    .panel {
        background: stat.$surface;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        padding: 16px;
    }

    .panel-head {
        display: flex;
        align-items: center;
        gap: 10px;
        margin-bottom: 12px;
    }

    .panel-title {
        font-family: stat.$font-display;
        font-weight: 700;
        font-size: 0.95rem;
        color: stat.$ink;
        margin: 0;
        margin-inline-end: auto;
    }

    // --- segmented control -------------------------------------------------
    .segmented {
        display: inline-flex;
        gap: 6px;
        margin-bottom: 12px;
    }

    .seg {
        font-family: stat.$font-mono;
        font-size: 0.72rem;
        letter-spacing: 0.04em;
        padding: 5px 14px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        color: stat.$ink-soft;
        cursor: pointer;

        &:hover {
            color: stat.$ink;
        }
        &.active {
            border-color: stat.$primary;
            background: stat.$primary-wash;
            color: stat.$primary;
        }
    }

    .field-label {
        @include stat.eyebrow;
        display: block;
        margin-bottom: 6px;
    }

    .block-input {
        width: 100%;
        box-sizing: border-box;
        resize: vertical;
        min-height: 150px;
        padding: 12px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        color: stat.$ink;
        font-family: stat.$font-mono;
        font-size: 0.8rem;
        line-height: 1.55;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -1px;
        }
    }

    // --- buttons -----------------------------------------------------------
    .input-actions,
    .result-actions {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-top: 12px;
        flex-wrap: wrap;
    }

    .spacer {
        flex: 1;
    }

    .btn {
        font-family: stat.$font-mono;
        font-size: 0.78rem;
        font-weight: 600;
        padding: 8px 16px;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$line;
        background: stat.$surface;
        color: stat.$ink;
        cursor: pointer;
        text-decoration: none;
        display: inline-flex;
        align-items: center;

        &:disabled {
            opacity: 0.45;
            cursor: not-allowed;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    .btn.primary {
        background: stat.$primary;
        border-color: stat.$primary;
        color: stat.$surface;

        &:not(:disabled):hover {
            filter: brightness(1.04);
        }
    }

    .btn.ghost {
        color: stat.$ink-soft;

        &:not(:disabled):hover {
            color: stat.$ink;
            border-color: stat.$ink-soft;
        }
    }

    .linklike {
        border: none;
        background: none;
        padding: 0;
        font-family: stat.$font-mono;
        font-size: 0.72rem;
        color: stat.$primary;
        cursor: pointer;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    // --- parse preview -----------------------------------------------------
    .parse-summary {
        @include stat.readout;
        font-size: 0.75rem;
        letter-spacing: 0.04em;
        text-transform: uppercase;
        color: stat.$ink-soft;
        margin: 0 0 10px;
    }

    .stat-num {
        color: stat.$ink;
        font-weight: 600;

        &.dup {
            color: stat.$stable;
        }
        &.big {
            color: inherit;
        }
    }

    .sep {
        opacity: 0.5;
        padding: 0 2px;
    }

    .rows {
        list-style: none;
        margin: 0;
        padding: 0;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        overflow: hidden;
    }

    .prow {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 7px 12px;
        border-top: 1px solid stat.$line;

        &:first-child {
            border-top: none;
        }
        &.dup {
            opacity: 0.55;
        }
        &.dup .id,
        &.dup .topic {
            text-decoration: line-through;
        }
    }

    .mono {
        @include stat.readout;
        font-size: 0.75rem;
    }

    .prow .id {
        color: stat.$ink-soft;
        min-width: 68px;
    }

    .prow .topic {
        color: stat.$ink;
        font-size: 0.82rem;
        flex: 1;
    }

    .prow .secs {
        color: stat.$ink-soft;
    }

    .prow .status {
        @include stat.readout;
        font-size: 0.68rem;
        letter-spacing: 0.04em;
        width: 58px;
        text-align: end;

        &.ok {
            color: stat.$stable;
        }
        &.miss {
            color: stat.$critical;
        }
        &.skip {
            color: stat.$muted;
        }
    }

    .dedup-note {
        color: stat.$ink-soft;
        font-size: 0.78rem;
        line-height: 1.5;
        margin: 10px 0 0;
    }

    // --- the auto-link result ---------------------------------------------
    .result {
        position: relative;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$line;
        border-inline-start: 3px solid stat.$line;
        background: stat.$surface;
        padding: 16px;
    }

    .result.idle {
        border-inline-start-color: stat.$muted;
    }

    .result.pulsing,
    .result.settled {
        border-color: stat.$primary;
        border-inline-start-color: stat.$primary;
        background: stat.$primary-wash;
    }

    .result-idle {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .idle-copy {
        color: stat.$ink-soft;
        font-size: 0.9rem;
        margin: 0;
    }

    .result-head {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 10px;
    }

    .result-headline {
        font-family: stat.$font-display;
        font-weight: 700;
        line-height: 1.25;
        color: stat.$ink;
        font-size: clamp(1.05rem, 3.2vw, 1.3rem);
        margin: 10px 0 0;
    }

    .topic-chips {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
        margin-top: 12px;
    }

    .topic-chip {
        @include stat.readout;
        font-size: 0.7rem;
        padding: 4px 10px;
        border: 1px solid;
        border-radius: stat.$radius-pill;
        background: stat.$surface;
        white-space: nowrap;
    }

    .chips-caption {
        color: stat.$ink-soft;
        font-size: 0.76rem;
        margin: 8px 0 0;
    }

    .settled-copy {
        color: stat.$ink-soft;
        font-size: 0.88rem;
        line-height: 1.5;
        margin: 12px 0 0;
        max-width: 56ch;
    }

    // One calm pulse — a single expanding hairline ring, no shadow/gradient.
    .pulse-ring {
        position: absolute;
        inset: -1px;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$primary;
        opacity: 0;
        pointer-events: none;
    }

    .result.pulsing .pulse-ring {
        animation: stat-pulse 900ms ease-out 1 both;
    }

    @keyframes stat-pulse {
        0% {
            opacity: 0;
            transform: scale(0.985);
        }
        45% {
            opacity: 0.55;
        }
        100% {
            opacity: 0;
            transform: scale(1.015);
        }
    }

    // Reduced motion: never pulse (the JS also settles instantly).
    @media (prefers-reduced-motion: reduce) {
        .result.pulsing .pulse-ring {
            animation: none;
        }
    }

    @media (min-width: stat.$bp-compact) {
        .result {
            padding: 20px;
        }
    }
</style>
