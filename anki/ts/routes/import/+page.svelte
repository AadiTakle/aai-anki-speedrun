<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT · Import → Auto-link (screen lane C3, destination "import").

Ingest a QBank's aggregate "performance by subject/system" block (paste any
QBank — UWorld etc. — or add custom rows), map each subject to a canonical
blueprint topic (auto-mapped, user-correctable), then fire the flagship
ORGANIZING ACTION: ImportQbankAggregate + RelinkMisses over the shared engine
($lib/speedrun), one calm pulse that settles into the real returned summary.

Honesty bar: every outcome number comes from the RPC response. On a backend
error we surface it inline — we NEVER fall back to fabricated results, and rows
the user left unmapped are excluded from the import and clearly reported.
-->
<script lang="ts">
    import { importQbankAggregate, relinkMisses } from "@generated/backend";
    import { onDestroy, onMount } from "svelte";

    import {
        acuityFromWeakness,
        acuityLabel,
        AppShell,
        BLUEPRINT_TOPICS,
        Chip,
        mapLabelToTopic,
        parseQbankReport,
        STAT_SIGNAL,
        StatusDot,
        topicDisplayName,
    } from "$lib/speedrun";

    import { DEFAULT_SOURCE, SAMPLE_PASTE } from "./import-mock";

    type Phase = "idle" | "linking" | "settled";

    /** One editable preview row: a QBank subject mapped to a blueprint topic. */
    interface PreviewRow {
        label: string;
        /** Canonical topic id, or "" when unassigned (must be resolved to import). */
        topicId: string;
        correct: number;
        total: number;
        /** True when the topic was auto-mapped from the label. */
        auto: boolean;
        /** True for a user-added manual row (label + counts are editable). */
        custom: boolean;
    }

    interface ImportedTopic {
        topicId: string;
        name: string;
        correct: number;
        total: number;
        pct: number;
        acuity: "critical" | "watch" | "stable" | "muted";
    }

    interface ImportSummary {
        topicsImported: number;
        totalQuestions: number;
        excluded: number;
        topics: ImportedTopic[];
    }

    // How long the single calm pulse plays before settling (kept in sync with the
    // CSS animation below). Skipped entirely under reduced-motion.
    const PULSE_MS = 900;

    let source = DEFAULT_SOURCE;
    let text = SAMPLE_PASTE;
    let previewRows: PreviewRow[] = [];
    let parseWarnings: string[] = [];
    let parsed = false;

    let phase: Phase = "idle";
    let importError: string | null = null;
    let summary: ImportSummary | null = null;
    let pulseTimer: ReturnType<typeof setTimeout> | undefined;

    $: hasContent = text.trim().length > 0;
    $: mappedRows = previewRows.filter((row) => row.topicId !== "");
    $: unmappedRows = previewRows.filter((row) => row.topicId === "");
    $: resolvedRows = mappedRows.filter(
        (row) => row.total > 0 && row.correct >= 0 && row.correct <= row.total,
    );
    $: canImport = resolvedRows.length > 0 && phase !== "linking";

    function rowReady(row: PreviewRow): boolean {
        return (
            row.topicId !== "" &&
            row.total > 0 &&
            row.correct >= 0 &&
            row.correct <= row.total
        );
    }

    // Row status dot (flattened; see no-nested-ternary).
    function rowAcuity(row: PreviewRow): "critical" | "watch" | "stable" {
        if (row.topicId === "") {
            return "critical";
        }
        if (!rowReady(row)) {
            return "watch";
        }
        return "stable";
    }

    function parseBlock(): void {
        const result = parseQbankReport(text);
        previewRows = result.rows.map((row) => {
            const topic = mapLabelToTopic(row.label);
            return {
                label: row.label,
                topicId: topic ?? "",
                correct: row.correct,
                total: row.total,
                auto: topic !== null,
                custom: false,
            };
        });
        parseWarnings = result.warnings;
        parsed = true;
        resetResult();
    }

    function onInput(): void {
        // Editing the block invalidates a stale parse until re-parsed.
        importError = null;
    }

    function loadSample(): void {
        source = DEFAULT_SOURCE;
        text = SAMPLE_PASTE;
        parseBlock();
    }

    function clearInput(): void {
        text = "";
        previewRows = [];
        parseWarnings = [];
        parsed = false;
        resetResult();
    }

    function addCustomRow(): void {
        previewRows = [
            ...previewRows,
            { label: "", topicId: "", correct: 0, total: 0, auto: false, custom: true },
        ];
        parsed = true;
        resetResult();
    }

    function removeRow(index: number): void {
        previewRows = previewRows.filter((_, i) => i !== index);
        resetResult();
    }

    // Re-trigger derived reactivity after an in-place edit to a bound row field.
    function touch(): void {
        previewRows = [...previewRows];
        importError = null;
    }

    /** Collapse resolved rows to one QbankTopicResult per topic (sum duplicates). */
    function aggregateByTopic(
        rows: PreviewRow[],
    ): { topicId: string; correct: number; total: number }[] {
        const byTopic = new Map<
            string,
            { topicId: string; correct: number; total: number }
        >();
        for (const row of rows) {
            const existing = byTopic.get(row.topicId);
            if (existing) {
                existing.correct += row.correct;
                existing.total += row.total;
            } else {
                byTopic.set(row.topicId, {
                    topicId: row.topicId,
                    correct: row.correct,
                    total: row.total,
                });
            }
        }
        return [...byTopic.values()];
    }

    function toImportedTopics(
        rows: { topicId: string; correct: number; total: number }[],
    ): ImportedTopic[] {
        return (
            rows
                .map((row) => {
                    const accuracy = row.total > 0 ? row.correct / row.total : 0;
                    return {
                        topicId: row.topicId,
                        name: topicDisplayName(row.topicId),
                        correct: row.correct,
                        total: row.total,
                        pct: Math.round(accuracy * 100),
                        acuity: acuityFromWeakness(1 - accuracy),
                    };
                })
                // Weakest topics first — where the points-at-stake are highest.
                .sort((a, b) => a.pct - b.pct)
        );
    }

    function prefersReducedMotion(): boolean {
        return (
            typeof window !== "undefined" &&
            typeof window.matchMedia === "function" &&
            window.matchMedia("(prefers-reduced-motion: reduce)").matches
        );
    }

    function settle(): void {
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

    async function runImport(): Promise<void> {
        if (!canImport) {
            return;
        }
        clearTimer();
        importError = null;
        phase = "linking";

        const aggregate = aggregateByTopic(resolvedRows);
        const excluded = unmappedRows.length;
        try {
            // The flagship organizing action, over the shared engine: replace this
            // source's aggregate rows, then turn misses into the focused queue.
            const response = await importQbankAggregate({
                source: source.trim() || DEFAULT_SOURCE,
                rows: aggregate,
            });
            await relinkMisses({});

            summary = {
                topicsImported: response.topicsImported,
                totalQuestions: response.totalQuestions,
                excluded,
                topics: toImportedTopics(aggregate),
            };
            settle();
        } catch (error) {
            // Honesty bar: never fabricate a result — surface the failure.
            importError = error instanceof Error ? error.message : String(error);
            summary = null;
            phase = "idle";
        }
    }

    function resetResult(): void {
        clearTimer();
        phase = "idle";
        importError = null;
        summary = null;
    }

    function clearTimer(): void {
        if (pulseTimer !== undefined) {
            clearTimeout(pulseTimer);
            pulseTimer = undefined;
        }
    }

    onMount(parseBlock);
    onDestroy(clearTimer);
</script>

<AppShell active="import">
    <Chip slot="badges" dot="stable">SYNCED</Chip>

    <div class="page">
        <header class="intro">
            <p class="eyebrow">Ingest · the organizing action</p>
            <h1 class="headline">
                Import your QBank subjects — watch your misses organize themselves
            </h1>
            <p class="lede">
                Paste any QBank's per-subject scorecard (UWorld and most banks only
                export aggregate counts, not questions). STAT maps each subject to a
                blueprint topic, then unsuspends the cards behind your weakest topics —
                one calm pulse, then quiet.
            </p>
        </header>

        <!-- 1 · INPUT — a source label + one flexible paste box -->
        <section class="panel" aria-labelledby="ingest-h">
            <div class="panel-head">
                <h2 id="ingest-h" class="panel-title">Source block</h2>
                <Chip>aggregate · by subject</Chip>
            </div>

            <label class="field-label" for="source-input">QBank source</label>
            <input
                id="source-input"
                class="text-input"
                type="text"
                spellcheck="false"
                placeholder="UWorld"
                bind:value={source}
            />

            <label class="field-label spaced" for="block-input">
                Paste “performance by subject / system”
            </label>
            <textarea
                id="block-input"
                class="block-input"
                rows="8"
                spellcheck="false"
                bind:value={text}
                on:input={onInput}
            ></textarea>

            <div class="input-actions">
                <button type="button" class="btn ghost" on:click={loadSample}>
                    Load sample block
                </button>
                <button
                    type="button"
                    class="btn ghost"
                    on:click={clearInput}
                    disabled={!hasContent && previewRows.length === 0}
                >
                    Clear
                </button>
                <span class="spacer"></span>
                <button
                    type="button"
                    class="btn primary"
                    on:click={parseBlock}
                    disabled={!hasContent}
                >
                    Parse block
                </button>
            </div>
        </section>

        <!-- 2 · PREVIEW + MAP — auto-mapped topics, user-correctable -->
        {#if parsed}
            <section class="panel" aria-labelledby="parse-h">
                <div class="panel-head">
                    <h2 id="parse-h" class="panel-title">Preview &amp; map</h2>
                    <Chip dot={unmappedRows.length > 0 ? "critical" : "stable"}>
                        {resolvedRows.length} ready · {unmappedRows.length} to assign
                    </Chip>
                </div>

                <p class="parse-summary">
                    <span class="stat-num">{previewRows.length}</span>
                    rows
                    <span class="sep">·</span>
                    <span class="stat-num">{mappedRows.length}</span>
                    mapped
                    <span class="sep">·</span>
                    <span class="stat-num crit">{unmappedRows.length}</span>
                    unassigned
                </p>

                {#if previewRows.length > 0}
                    <ul class="rows" aria-label="Parsed subjects">
                        {#each previewRows as row, i (i)}
                            <li class="prow" class:unmapped={row.topicId === ""}>
                                <StatusDot acuity={rowAcuity(row)} size={8} />

                                {#if row.custom}
                                    <input
                                        class="cell-input label-input"
                                        type="text"
                                        placeholder="Custom subject"
                                        aria-label="Custom subject label"
                                        bind:value={row.label}
                                    />
                                {:else}
                                    <span class="label" title={row.label}>
                                        {row.label}
                                    </span>
                                {/if}

                                <select
                                    class="select"
                                    class:assign={row.topicId === ""}
                                    aria-label="Blueprint topic for {row.label ||
                                        'custom row'}"
                                    bind:value={row.topicId}
                                    on:change={touch}
                                >
                                    <option value="">— assign topic —</option>
                                    {#each BLUEPRINT_TOPICS as topic (topic.id)}
                                        <option value={topic.id}>{topic.name}</option>
                                    {/each}
                                </select>

                                {#if row.custom}
                                    <span class="counts editable">
                                        <input
                                            class="cell-input num"
                                            type="number"
                                            min="0"
                                            aria-label="Correct"
                                            bind:value={row.correct}
                                            on:input={touch}
                                        />
                                        <span class="of">/</span>
                                        <input
                                            class="cell-input num"
                                            type="number"
                                            min="0"
                                            aria-label="Total"
                                            bind:value={row.total}
                                            on:input={touch}
                                        />
                                    </span>
                                {:else}
                                    <span class="counts">
                                        <span class="stat-num">{row.correct}</span>
                                        /
                                        <span class="stat-num">{row.total}</span>
                                    </span>
                                {/if}

                                <button
                                    type="button"
                                    class="row-remove"
                                    aria-label="Remove row"
                                    title="Remove row"
                                    on:click={() => removeRow(i)}
                                >
                                    ×
                                </button>
                            </li>
                        {/each}
                    </ul>
                {:else}
                    <p class="dedup-note">
                        No subject rows parsed. Add a custom row, or paste a
                        “performance by subject” block above.
                    </p>
                {/if}

                <div class="input-actions">
                    <button type="button" class="btn ghost" on:click={addCustomRow}>
                        + Add custom row
                    </button>
                    <span class="spacer"></span>
                    <button
                        type="button"
                        class="btn primary"
                        on:click={runImport}
                        disabled={!canImport}
                    >
                        Import &amp; auto-link
                    </button>
                </div>

                {#if unmappedRows.length > 0}
                    <p class="dedup-note assign-note">
                        <span class="stat-num crit">{unmappedRows.length}</span>
                        unassigned row{unmappedRows.length === 1 ? "" : "s"} will be excluded
                        from the import — assign a topic to include them.
                    </p>
                {/if}

                {#if parseWarnings.length > 0}
                    <details class="warnings">
                        <summary>
                            {parseWarnings.length} line{parseWarnings.length === 1
                                ? ""
                                : "s"} skipped while parsing
                        </summary>
                        <ul>
                            {#each parseWarnings as warning (warning)}
                                <li>{warning}</li>
                            {/each}
                        </ul>
                    </details>
                {/if}
            </section>
        {/if}

        <!-- 3 · THE AUTO-LINK MOMENT — one calm pulse, then the real summary -->
        <section
            class="result"
            class:idle={phase === "idle" && importError === null}
            class:pulsing={phase === "linking"}
            class:settled={phase === "settled"}
            class:errored={importError !== null}
            aria-live="polite"
        >
            {#if importError !== null}
                <div class="result-head">
                    <p class="eyebrow crit">Import failed</p>
                    <button type="button" class="linklike" on:click={resetResult}>
                        Dismiss
                    </button>
                </div>
                <p class="error-copy">
                    The engine rejected the import, so nothing was changed. No result is
                    shown — we don't fabricate one.
                </p>
                <p class="error-detail">{importError}</p>
            {:else if phase === "idle"}
                <div class="result-idle">
                    <StatusDot acuity="muted" size={9} />
                    <p class="idle-copy">
                        Map your subjects, then auto-link to unsuspend the cards behind
                        your weakest topics.
                    </p>
                </div>
            {:else if summary !== null}
                <span class="pulse-ring" aria-hidden="true"></span>

                <div class="result-head">
                    <p class="eyebrow accent">
                        {phase === "linking" ? "Organizing…" : "Organized · just now"}
                    </p>
                    {#if phase === "settled"}
                        <button type="button" class="linklike" on:click={resetResult}>
                            Dismiss
                        </button>
                    {/if}
                </div>

                <p class="result-headline">
                    <span class="stat-num big">{summary.topicsImported}</span>
                    topics imported
                    <span class="sep">·</span>
                    <span class="stat-num big">{summary.totalQuestions}</span>
                    questions logged from {source.trim() || DEFAULT_SOURCE}
                </p>

                <div class="topic-chips">
                    {#each summary.topics as topic (topic.topicId)}
                        <span
                            class="topic-chip"
                            style="color: {STAT_SIGNAL[
                                topic.acuity
                            ]}; border-color: {STAT_SIGNAL[topic.acuity]}"
                            title="{acuityLabel(topic.acuity)} · {topic.pct}% correct"
                        >
                            {topic.name} · {topic.correct}/{topic.total} · {topic.pct}%
                        </span>
                    {/each}
                </div>
                <p class="chips-caption">
                    Where your performance landed · weakest topics first.
                    {#if summary.excluded > 0}
                        <span class="crit">
                            {summary.excluded} unassigned row{summary.excluded === 1
                                ? ""
                                : "s"} excluded.
                        </span>
                    {/if}
                </p>

                {#if phase === "settled"}
                    <p class="settled-copy">
                        Your next move is one review session — the queue is already
                        built from your weakest topics.
                    </p>
                    <div class="result-actions">
                        <a class="btn primary" href="/reviewer">
                            Start targeted review →
                        </a>
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
        &.crit {
            color: stat.$critical;
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

    .field-label {
        @include stat.eyebrow;
        display: block;
        margin-bottom: 6px;

        &.spaced {
            margin-top: 14px;
        }
    }

    .text-input {
        width: 100%;
        box-sizing: border-box;
        padding: 9px 12px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        color: stat.$ink;
        font-family: stat.$font-mono;
        font-size: 0.82rem;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -1px;
        }
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

        &.crit {
            color: stat.$critical;
        }
        &.big {
            color: inherit;
        }
    }

    .sep {
        opacity: 0.5;
        padding: 0 2px;
    }

    .crit {
        color: stat.$critical;
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
        &.unmapped {
            background: stat.$critical-wash;
        }
    }

    .prow .label {
        color: stat.$ink;
        font-size: 0.82rem;
        flex: 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .cell-input {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        color: stat.$ink;
        font-family: stat.$font-mono;
        font-size: 0.78rem;
        padding: 4px 8px;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -1px;
        }
    }

    .cell-input.label-input {
        flex: 1;
        min-width: 0;
    }

    .cell-input.num {
        width: 52px;
        text-align: end;
    }

    .select {
        font-family: stat.$font-mono;
        font-size: 0.75rem;
        padding: 4px 8px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        color: stat.$ink;
        max-width: 46%;

        &.assign {
            border-color: stat.$critical;
            color: stat.$critical;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -1px;
        }
    }

    .counts {
        @include stat.readout;
        font-size: 0.78rem;
        color: stat.$ink-soft;
        white-space: nowrap;

        &.editable {
            display: inline-flex;
            align-items: center;
            gap: 4px;
        }
    }

    .of {
        color: stat.$ink-soft;
    }

    .row-remove {
        border: none;
        background: none;
        cursor: pointer;
        color: stat.$ink-soft;
        font-size: 1.1rem;
        line-height: 1;
        padding: 0 2px;

        &:hover {
            color: stat.$critical;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    .dedup-note {
        color: stat.$ink-soft;
        font-size: 0.78rem;
        line-height: 1.5;
        margin: 10px 0 0;
    }

    .assign-note {
        color: stat.$ink;
    }

    .warnings {
        margin-top: 10px;
        font-size: 0.76rem;
        color: stat.$ink-soft;

        summary {
            cursor: pointer;
            font-family: stat.$font-mono;
            letter-spacing: 0.03em;
        }
        ul {
            margin: 6px 0 0;
            padding-inline-start: 18px;
        }
        li {
            margin: 2px 0;
            font-family: stat.$font-mono;
            word-break: break-word;
        }
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

    .result.errored {
        border-color: stat.$critical;
        border-inline-start-color: stat.$critical;
        background: stat.$critical-wash;
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

    .error-copy {
        color: stat.$ink;
        font-size: 0.9rem;
        line-height: 1.5;
        margin: 10px 0 0;
        max-width: 56ch;
    }

    .error-detail {
        @include stat.readout;
        font-size: 0.76rem;
        color: stat.$critical;
        margin: 8px 0 0;
        word-break: break-word;
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
