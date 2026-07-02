<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT · Error Log / Differential (screen lane C3, destination "errors").

Every miss becomes a reasoning artifact, not a re-read. Each entry is built
around one prompt — "How would the vignette need to change for [your wrong
answer] to be the right answer?" — with a one-line takeaway you write and mark,
an error-type tag (knowledge / reasoning / misread), and confusion-pair
grouping. Built on the shared foundation ($lib/speedrun); illustrative data only.
-->
<script lang="ts">
    import { AppShell, Chip, StatusDot } from "$lib/speedrun";
    import type { Acuity } from "$lib/speedrun";

    import {
        ERROR_ENTRIES,
        KIND_META,
        type ErrorEntry,
        type ErrorKind,
    } from "./errors-mock";

    const entries = ERROR_ENTRIES;

    type Filter = ErrorKind | "all";

    const FILTERS: { id: Filter; label: string }[] = [
        { id: "all", label: "All" },
        { id: "reasoning", label: "Reasoning" },
        { id: "knowledge", label: "Knowledge" },
        { id: "misread", label: "Misread" },
    ];

    // Per-entry local state (no persistence needed for the mock).
    let filter: Filter = "all";
    const takeaways: Record<string, string> = Object.fromEntries(
        entries.map((e) => [e.id, e.takeaway]),
    );
    let marked: Record<string, boolean> = Object.fromEntries(
        entries.map((e) => [e.id, false]),
    );
    let expanded: Record<string, boolean> = { [entries[0].id]: true };

    interface Group {
        confusion: string;
        acuity: Acuity;
        entries: ErrorEntry[];
    }

    const RANK: Record<Acuity, number> = { critical: 3, watch: 2, stable: 1, muted: 0 };

    function groupAcuity(list: ErrorEntry[]): Acuity {
        return list.reduce<Acuity>(
            (worst, e) => (RANK[e.acuity] > RANK[worst] ? e.acuity : worst),
            "muted",
        );
    }

    function groupByConfusion(list: ErrorEntry[]): Group[] {
        const byPair = new Map<string, ErrorEntry[]>();
        for (const e of list) {
            const arr = byPair.get(e.confusion);
            if (arr) {
                arr.push(e);
            } else {
                byPair.set(e.confusion, [e]);
            }
        }
        return [...byPair.entries()].map(([confusion, groupEntries]) => ({
            confusion,
            acuity: groupAcuity(groupEntries),
            entries: groupEntries,
        }));
    }

    function toggle(id: string): void {
        expanded = { ...expanded, [id]: !expanded[id] };
    }

    function markTakeaway(id: string): void {
        if ((takeaways[id] ?? "").trim().length > 0) {
            marked = { ...marked, [id]: true };
        }
    }

    function reopen(id: string): void {
        marked = { ...marked, [id]: false };
    }

    function kindCount(k: Filter): number {
        return k === "all"
            ? entries.length
            : entries.filter((e) => e.kind === k).length;
    }

    $: visible = filter === "all" ? entries : entries.filter((e) => e.kind === filter);
    $: groups = groupByConfusion(visible);
    $: openCount = entries.filter((e) => !marked[e.id]).length;
</script>

<AppShell active="errors">
    <Chip slot="badges" dot="stable">SYNCED</Chip>

    <div class="page">
        <header class="intro">
            <p class="eyebrow">Reframe · the differential</p>
            <h1 class="headline">Every miss becomes a reasoning artifact</h1>
            <p class="lede">
                Not a re-read. Each miss opens around one prompt — <em>
                    "How would the vignette need to change for your wrong answer to be
                    right?"
                </em>
                — so you capture the distinction the exam actually tests, tag the error type,
                and keep it linked to the cards it unsuspended.
            </p>
        </header>

        <div class="controls">
            <Chip dot={openCount > 0 ? "critical" : "stable"}>
                {openCount} open · grouped by confusion
            </Chip>
            <span class="spacer"></span>
            <div class="segmented" role="group" aria-label="Filter by error type">
                {#each FILTERS as f (f.id)}
                    <button
                        type="button"
                        class="seg"
                        class:active={filter === f.id}
                        aria-pressed={filter === f.id}
                        on:click={() => (filter = f.id)}
                    >
                        {f.label} ({kindCount(f.id)})
                    </button>
                {/each}
            </div>
        </div>

        {#if groups.length === 0}
            <p class="empty">No {filter} errors logged.</p>
        {/if}

        {#each groups as group (group.confusion)}
            <section class="group" aria-label={"Confusion pair: " + group.confusion}>
                <div class="group-head">
                    <StatusDot acuity={group.acuity} size={8} />
                    <span class="group-title">{group.confusion}</span>
                    <span class="group-count">
                        {group.entries.length}
                        {group.entries.length === 1 ? "entry" : "entries"}
                    </span>
                </div>

                <div class="entries">
                    {#each group.entries as entry (entry.id)}
                        <article class="entry" class:logged={marked[entry.id]}>
                            <button
                                type="button"
                                class="entry-head"
                                aria-expanded={!!expanded[entry.id]}
                                aria-controls={"body-" + entry.id}
                                on:click={() => toggle(entry.id)}
                            >
                                <StatusDot acuity={entry.acuity} size={9} />
                                <span class="entry-topic">{entry.topic}</span>
                                <span class="diff">
                                    <span class="chose">{entry.chosen}</span>
                                    <span class="arrow" aria-hidden="true">→</span>
                                    <span class="answer">{entry.correct}</span>
                                </span>
                                <span class="entry-spacer"></span>
                                {#if marked[entry.id]}
                                    <span class="logged-flag">
                                        <StatusDot
                                            acuity="stable"
                                            size={7}
                                            shape="round"
                                        /> logged
                                    </span>
                                {/if}
                                <span class="kind-tag">
                                    {KIND_META[entry.kind].label}
                                </span>
                                <span class="cards">{entry.cards} cards</span>
                                <span
                                    class="chevron"
                                    class:open={expanded[entry.id]}
                                    aria-hidden="true"
                                >
                                    ▾
                                </span>
                            </button>

                            {#if expanded[entry.id]}
                                <div class="entry-body" id={"body-" + entry.id}>
                                    <div class="reframe">
                                        <p class="eyebrow accent">Reframe prompt</p>
                                        <p class="reframe-q">
                                            How would the vignette need to change for
                                            <strong>"{entry.chosen}"</strong>
                                            to be the right answer?
                                        </p>
                                    </div>

                                    <div class="takeaway">
                                        {#if marked[entry.id]}
                                            <p class="eyebrow">
                                                Your one-line takeaway
                                            </p>
                                            <blockquote class="takeaway-locked">
                                                {takeaways[entry.id]}
                                            </blockquote>
                                            <button
                                                type="button"
                                                class="linklike"
                                                on:click={() => reopen(entry.id)}
                                            >
                                                Reopen to edit
                                            </button>
                                        {:else}
                                            <label
                                                class="eyebrow"
                                                for={"takeaway-" + entry.id}
                                            >
                                                Your one-line takeaway
                                            </label>
                                            <textarea
                                                id={"takeaway-" + entry.id}
                                                class="takeaway-input"
                                                rows="2"
                                                placeholder="What one distinction would have flipped this?"
                                                bind:value={takeaways[entry.id]}
                                            ></textarea>
                                            <div class="takeaway-actions">
                                                <button
                                                    type="button"
                                                    class="btn primary"
                                                    disabled={(
                                                        takeaways[entry.id] ?? ""
                                                    ).trim().length === 0}
                                                    on:click={() =>
                                                        markTakeaway(entry.id)}
                                                >
                                                    Mark takeaway
                                                </button>
                                            </div>
                                        {/if}
                                    </div>

                                    <div class="meta">
                                        <span class="meta-tag">
                                            <StatusDot
                                                acuity={KIND_META[entry.kind].acuity}
                                                size={7}
                                            />
                                            {KIND_META[entry.kind].label} — {entry.kindDetail}
                                        </span>
                                        <span class="meta-spacer"></span>
                                        <span class="meta-source">{entry.source}</span>
                                        <span class="meta-cards">
                                            linked {entry.cards} cards →
                                        </span>
                                    </div>
                                </div>
                            {/if}
                        </article>
                    {/each}
                </div>
            </section>
        {/each}
    </div>
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .page {
        display: flex;
        flex-direction: column;
        gap: 16px;
        max-width: 820px;
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
        max-width: 68ch;

        em {
            color: stat.$ink;
            font-style: italic;
        }
    }

    // --- controls ----------------------------------------------------------
    .controls {
        display: flex;
        align-items: center;
        gap: 10px;
        flex-wrap: wrap;
    }

    .spacer {
        flex: 1;
    }

    .segmented {
        display: inline-flex;
        gap: 6px;
        flex-wrap: wrap;
    }

    .seg {
        font-family: stat.$font-mono;
        font-size: 0.7rem;
        letter-spacing: 0.03em;
        padding: 5px 12px;
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
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    .empty {
        color: stat.$ink-soft;
        font-size: 0.9rem;
        margin: 0;
    }

    // --- confusion group ---------------------------------------------------
    .group {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .group-head {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 2px 2px 0;
    }

    .group-title {
        @include stat.readout;
        font-size: 0.72rem;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: stat.$ink;
        font-weight: 600;
    }

    .group-count {
        @include stat.readout;
        font-size: 0.68rem;
        color: stat.$ink-soft;
    }

    .entries {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    // --- entry -------------------------------------------------------------
    .entry {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        overflow: hidden;
    }

    .entry.logged {
        border-inline-start: 3px solid stat.$stable;
    }

    .entry-head {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        text-align: start;
        padding: 11px 14px;
        border: none;
        background: none;
        cursor: pointer;
        color: stat.$ink;

        &:hover {
            background: stat.$paper;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -2px;
        }
    }

    .entry-topic {
        font-family: stat.$font-body;
        font-weight: 700;
        font-size: 0.9rem;
        color: stat.$ink;
        white-space: nowrap;
    }

    .diff {
        @include stat.readout;
        font-size: 0.68rem;
        display: inline-flex;
        align-items: center;
        gap: 6px;
        min-width: 0;

        .chose {
            color: stat.$critical;
            text-decoration: line-through;
        }
        .arrow {
            color: stat.$ink-soft;
        }
        .answer {
            color: stat.$stable;
        }
    }

    .entry-spacer {
        flex: 1;
    }

    .logged-flag {
        @include stat.readout;
        font-size: 0.66rem;
        color: stat.$stable;
        display: inline-flex;
        align-items: center;
        gap: 4px;
        white-space: nowrap;
    }

    .kind-tag {
        @include stat.readout;
        font-size: 0.64rem;
        letter-spacing: 0.04em;
        padding: 3px 8px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-pill;
        color: stat.$ink-soft;
        white-space: nowrap;
    }

    .cards {
        @include stat.readout;
        font-size: 0.68rem;
        color: stat.$primary;
        white-space: nowrap;
    }

    .chevron {
        color: stat.$ink-soft;
        transition: transform 120ms ease;
        font-size: 0.7rem;

        &.open {
            transform: rotate(180deg);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .chevron {
            transition: none;
        }
    }

    // --- expanded body -----------------------------------------------------
    .entry-body {
        padding: 0 14px 14px;
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .reframe {
        border-inline-start: 3px solid stat.$primary;
        background: stat.$primary-wash;
        border-radius: stat.$radius-sm;
        padding: 10px 12px;
    }

    .reframe-q {
        color: stat.$ink;
        font-size: 0.9rem;
        line-height: 1.45;
        margin: 4px 0 0;

        strong {
            font-weight: 700;
        }
    }

    .takeaway {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .takeaway-input {
        width: 100%;
        box-sizing: border-box;
        resize: vertical;
        padding: 10px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        color: stat.$ink;
        font-family: stat.$font-body;
        font-size: 0.88rem;
        line-height: 1.45;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: -1px;
        }
    }

    .takeaway-locked {
        margin: 0;
        padding: 8px 12px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        color: stat.$ink;
        font-size: 0.88rem;
        font-style: italic;
        line-height: 1.45;
    }

    .takeaway-actions {
        display: flex;
        gap: 8px;
    }

    .btn {
        font-family: stat.$font-mono;
        font-size: 0.74rem;
        font-weight: 600;
        padding: 7px 14px;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$line;
        background: stat.$surface;
        color: stat.$ink;
        cursor: pointer;

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

    .linklike {
        align-self: flex-start;
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

    // --- meta row ----------------------------------------------------------
    .meta {
        display: flex;
        align-items: center;
        gap: 10px;
        flex-wrap: wrap;
        padding-top: 10px;
        border-top: 1px solid stat.$line;
    }

    .meta-tag {
        @include stat.readout;
        font-size: 0.68rem;
        color: stat.$ink-soft;
        display: inline-flex;
        align-items: center;
        gap: 6px;
    }

    .meta-spacer {
        flex: 1;
    }

    .meta-source,
    .meta-cards {
        @include stat.readout;
        font-size: 0.68rem;
        white-space: nowrap;
    }

    .meta-source {
        color: stat.$ink-soft;
    }

    .meta-cards {
        color: stat.$primary;
    }

    // On phones the dense header wraps to keep every token thumb-legible.
    @media (max-width: stat.$bp-compact) {
        .entry-head {
            flex-wrap: wrap;
        }
        .diff {
            flex-basis: 100%;
            order: 3;
        }
        .entry-spacer {
            display: none;
        }
    }
</style>
