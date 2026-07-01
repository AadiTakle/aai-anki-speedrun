<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT · Targeted-review reviewer (screen lane C2).

An illustrative STAT reviewer SCREEN over seeded, route-local cards — NOT the
real scheduler. It KEEPS Anki's question → reveal-answer → grade flow and the
Again/Hard/Good/Easy grades (+ 1–4 / Space shortcuts, FSRS intervals) and
CHANGES what's around them: the session is seeded by today's misses, front-loaded
by points-at-stake, and interleaved across systems (SessionHud); each card shows
its provenance and, on the answer side, the miss reframe/differential
(ReframePanel); pacing is visible per card.

Built on the shared foundation ($lib/speedrun) — this lane never edits it.
-->
<script lang="ts">
    import { onDestroy, onMount } from "svelte";

    import { AppShell, Chip, StatusDot } from "$lib/speedrun";

    import { pacingAcuity, SESSION_CARDS, toRuns, type Grade } from "./cards-mock";
    import GradeBar from "./GradeBar.svelte";
    import ReframePanel from "./ReframePanel.svelte";
    import SessionHud from "./SessionHud.svelte";

    const cards = SESSION_CARDS;

    let index = 0;
    let face: "prompt" | "answer" = "prompt";
    let done = false;
    let elapsed = 0; // seconds on the current card
    let startedAt = 0; // ms timestamp the current prompt began
    let graded: { id: string; grade: Grade }[] = [];
    let chromeNote = "";

    $: card = cards[index];
    $: overTarget = card ? elapsed > card.targetSeconds : false;
    $: answeredPacing = card ? pacingAcuity(elapsed, card.targetSeconds) : "stable";
    // When the session is done, mark every order-bar segment as reviewed.
    $: hudIndex = done ? cards.length : index;

    function clock(): number {
        return typeof performance !== "undefined" ? performance.now() : Date.now();
    }

    let timer: ReturnType<typeof setInterval> | undefined;

    onMount(() => {
        startedAt = clock();
        timer = setInterval(() => {
            if (face === "prompt" && !done) {
                elapsed = (clock() - startedAt) / 1000;
            }
        }, 250);
    });

    onDestroy(() => {
        if (timer) {
            clearInterval(timer);
        }
    });

    function reveal(): void {
        if (face !== "prompt" || done) {
            return;
        }
        elapsed = (clock() - startedAt) / 1000; // freeze time-to-answer
        chromeNote = "";
        face = "answer";
    }

    function backToPrompt(): void {
        if (face !== "answer") {
            return;
        }
        face = "prompt";
        startedAt = clock() - elapsed * 1000; // resume the timer where it paused
    }

    function grade(g: Grade): void {
        if (face !== "answer" || !card || done) {
            return;
        }
        graded = [...graded, { id: card.id, grade: g }];
        chromeNote = "";
        if (index + 1 >= cards.length) {
            done = true;
            return;
        }
        index += 1;
        face = "prompt";
        elapsed = 0;
        startedAt = clock();
    }

    function restart(): void {
        index = 0;
        face = "prompt";
        done = false;
        elapsed = 0;
        graded = [];
        chromeNote = "";
        startedAt = clock();
    }

    function keptChrome(what: string): void {
        chromeNote = `${what} is kept from Anki — inactive in this illustrative mock.`;
    }

    function buttonFocused(): boolean {
        return typeof document !== "undefined" && document.activeElement?.tagName === "BUTTON";
    }

    function onKeydown(event: KeyboardEvent): void {
        if (done) {
            return;
        }
        const { key } = event;
        if (key === " ") {
            // Space is Anki's advance key: reveal, then rate with the default.
            event.preventDefault();
            if (face === "prompt") {
                reveal();
            } else {
                grade("good");
            }
            return;
        }
        if (key === "Enter" && !buttonFocused()) {
            event.preventDefault();
            if (face === "prompt") {
                reveal();
            } else {
                grade("good");
            }
            return;
        }
        if (face === "answer") {
            if (key === "1") {
                grade("again");
            } else if (key === "2") {
                grade("hard");
            } else if (key === "3") {
                grade("good");
            } else if (key === "4") {
                grade("easy");
            }
        }
    }
</script>

<svelte:window on:keydown={onKeydown} />

<AppShell active="reviewer">
    <svelte:fragment slot="badges">
        <Chip dot="stable">SYNCED</Chip>
        <Chip>MOCK · SEEDED</Chip>
    </svelte:fragment>

    <div class="reviewer">
        <SessionHud {cards} index={hudIndex} {elapsed} />

        {#if done}
            <section class="complete" aria-live="polite">
                <span class="complete-eyebrow">Session complete</span>
                <p class="complete-head">
                    {graded.length} miss-seeded {graded.length === 1 ? "card" : "cards"} reviewed
                </p>
                <p class="complete-sub">
                    Every card earned its place — each came from a real QBank miss, ordered by
                    points-at-stake and interleaved across systems. Grades flowed through Anki's FSRS
                    unchanged.
                </p>
                <button type="button" class="btn primary" on:click={restart}>Restart session</button>
            </section>
        {:else if card}
            <section class="card-panel">
                <!-- provenance — why this card is here (STAT-only) -->
                <div class="provenance">
                    <span class="prov-tag">Unsuspended from your miss</span>
                    <Chip>{card.provenance.source}</Chip>
                    <Chip dot={card.acuity}>{card.provenance.topic}</Chip>
                    <Chip>{card.provenance.system}</Chip>
                    <span class="prov-age">{card.provenance.ageLabel}</span>
                </div>

                <!-- the card face — Anki's question → answer rendering, KEPT -->
                <div class="card-face" aria-live="polite">
                    {#if face === "answer"}
                        <span class="face-label">Answer</span>
                        <p class="face-body">
                            {#each toRuns(card.answer) as run, i (i)}{#if run.strong}<strong>{run.text}</strong>{:else}{run.text}{/if}{/each}
                        </p>
                    {:else}
                        <span class="face-label">Question</span>
                        <p class="face-body">
                            {#each toRuns(card.question) as run, i (i)}{#if run.strong}<strong>{run.text}</strong>{:else}{run.text}{/if}{/each}
                        </p>
                    {/if}
                </div>

                {#if face === "answer"}
                    <ReframePanel {card} />
                {:else if overTarget}
                    <!-- pacing awareness: accurate-but-slow hint (STAT-only) -->
                    <p class="pacing-hint">
                        <StatusDot acuity="watch" />
                        Accurate-but-slow territory — you're past the ~{card.targetSeconds}s target for
                        this item. Note your pace, then reveal.
                    </p>
                {/if}

                {#if chromeNote}
                    <p class="chrome-note">{chromeNote}</p>
                {/if}

                <!-- bottom bar — Edit / Show-answer|grades / More, reframed -->
                <div class="bar">
                    {#if face === "answer"}
                        <div class="grade-head">
                            <span class="grade-label">Grade — FSRS intervals, unchanged from Anki</span>
                            <span class="answered" title="Time to answer this card">
                                <StatusDot acuity={answeredPacing} />
                                answered in {Math.round(elapsed)}s
                            </span>
                            <button type="button" class="link" on:click={backToPrompt}>
                                ‹ back to prompt
                            </button>
                        </div>
                        <GradeBar intervals={card.intervals} onGrade={grade} />
                    {:else}
                        <div class="prompt-bar">
                            <button
                                type="button"
                                class="btn ghost"
                                title="Edit card"
                                on:click={() => keptChrome("Editing the card")}
                            >
                                Edit
                            </button>
                            <button type="button" class="btn primary show" on:click={reveal}>
                                Show answer
                            </button>
                            <button
                                type="button"
                                class="btn ghost"
                                title="More card actions"
                                on:click={() => keptChrome("The More menu (bury / suspend / flag)")}
                            >
                                More ▾
                            </button>
                        </div>
                        <p class="hint">
                            <span class="kbd">Space</span> reveals · <span class="kbd">1</span>–<span class="kbd">4</span> grade — kept from Anki
                        </p>
                    {/if}
                </div>
            </section>
        {/if}

        <!-- explicit keeps-vs-changes, so the overhaul is legible -->
        <details class="diff">
            <summary>What's different from stock Anki</summary>
            <div class="diff-cols">
                <div class="diff-col">
                    <span class="diff-title kept">Preserved from Anki</span>
                    <ul>
                        <li>FSRS scheduling + the Again / Hard / Good / Easy grades and intervals</li>
                        <li>Question → reveal-answer → grade flow and card rendering</li>
                        <li>The 1–4 grade shortcuts and Space to reveal / rate</li>
                        <li>Revlog, undo, and sync (the proven core)</li>
                    </ul>
                </div>
                <div class="diff-col">
                    <span class="diff-title changed">Changed for STAT</span>
                    <ul>
                        <li>Selection: cards seeded by today's QBank misses, not a deck's due pile</li>
                        <li>Order: front-loaded by points-at-stake, interleaved across systems</li>
                        <li>Framing: provenance line + the miss reframe on the answer side</li>
                        <li>Chrome: deck picker &amp; raw counts → a focused session HUD + pacing</li>
                    </ul>
                </div>
            </div>
        </details>
    </div>
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .reviewer {
        display: flex;
        flex-direction: column;
        gap: 14px;
        max-width: 880px;
        margin-inline: auto;
    }

    // ---- card panel -------------------------------------------------------
    .card-panel {
        display: flex;
        flex-direction: column;
        gap: 12px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        padding: 14px;
    }

    .provenance {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 8px;
    }

    .prov-tag {
        @include stat.readout;
        font-size: 10px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: stat.$primary;
    }

    .prov-age {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }

    .card-face {
        display: flex;
        flex-direction: column;
        gap: 6px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$paper;
        padding: 18px;
        min-height: 108px;
    }

    .face-label {
        @include stat.eyebrow;
    }

    .face-body {
        margin: 0;
        font-family: stat.$font-body;
        font-size: 16px;
        line-height: 1.55;
        color: stat.$ink;

        strong {
            font-weight: 700;
        }
    }

    .pacing-hint {
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 0;
        font-family: stat.$font-body;
        font-size: 12px;
        line-height: 1.4;
        color: stat.$watch;
    }

    .chrome-note {
        margin: 0;
        font-family: stat.$font-mono;
        font-size: 11px;
        color: stat.$ink-soft;
    }

    // ---- bottom bar -------------------------------------------------------
    .bar {
        display: flex;
        flex-direction: column;
        gap: 10px;
        border-top: 1px solid stat.$line;
        padding-top: 12px;
    }

    .grade-head {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 8px;
    }

    .grade-label {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }

    .answered {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
        margin-inline-start: auto;
    }

    .link {
        @include stat.readout;
        font-size: 11px;
        color: stat.$primary;
        background: none;
        border: none;
        padding: 0;
        cursor: pointer;

        &:hover {
            text-decoration: underline;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    .prompt-bar {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .btn {
        font-family: stat.$font-mono;
        font-size: 12px;
        border-radius: stat.$radius-sm;
        padding: 8px 14px;
        cursor: pointer;

        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }
    }

    .btn.ghost {
        border: 1px solid stat.$line;
        background: stat.$surface;
        color: stat.$ink-soft;

        &:hover {
            border-color: stat.$ink-soft;
            color: stat.$ink;
        }
    }

    .btn.primary {
        border: 1px solid stat.$primary;
        background: stat.$primary;
        color: stat.$surface;
        font-weight: 600;

        &:hover {
            filter: brightness(1.05);
        }
    }

    .show {
        flex: 1;
        padding-block: 10px;
    }

    .hint {
        margin: 0;
        font-family: stat.$font-body;
        font-size: 11px;
        color: stat.$ink-soft;
    }

    .kbd {
        @include stat.readout;
        font-size: 10px;
        border: 1px solid stat.$line;
        border-radius: 3px;
        padding: 0 4px;
        color: stat.$ink-soft;
    }

    // ---- session-complete -------------------------------------------------
    .complete {
        display: flex;
        flex-direction: column;
        gap: 8px;
        align-items: flex-start;
        border: 1px solid stat.$line;
        border-left: 3px solid stat.$primary;
        border-radius: stat.$radius-md;
        background: stat.$primary-wash;
        padding: 18px;
    }

    .complete-eyebrow {
        @include stat.eyebrow;
        color: stat.$primary;
    }

    .complete-head {
        margin: 0;
        font-family: stat.$font-display;
        font-size: 20px;
        font-weight: 700;
        color: stat.$ink;
    }

    .complete-sub {
        margin: 0;
        max-width: 56ch;
        font-family: stat.$font-body;
        font-size: 13px;
        line-height: 1.5;
        color: stat.$ink-soft;
    }

    // ---- keeps vs changes -------------------------------------------------
    .diff {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        padding: 10px 14px;
    }

    summary {
        @include stat.readout;
        font-size: 11px;
        letter-spacing: 0.04em;
        color: stat.$ink;
        cursor: pointer;
    }

    .diff-cols {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 20px;
        margin-top: 12px;
    }

    .diff-title {
        display: block;
        @include stat.readout;
        font-size: 10px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        margin-bottom: 6px;

        &.kept {
            color: stat.$primary;
        }
        &.changed {
            color: stat.$ink-soft;
        }
    }

    .diff ul {
        margin: 0;
        padding-inline-start: 18px;
        display: flex;
        flex-direction: column;
        gap: 5px;
    }

    .diff li {
        font-family: stat.$font-body;
        font-size: 12px;
        line-height: 1.45;
        color: stat.$ink-soft;
    }

    @media (max-width: stat.$bp-compact) {
        .diff-cols {
            grid-template-columns: 1fr;
            gap: 14px;
        }
        // Full-bleed card: cancel AppShell's 16px content gutter so the card
        // runs edge-to-edge and the grade row is a full-width thumb target.
        .card-panel {
            margin-inline: -16px;
            border-inline: none;
            border-radius: 0;
        }
        .card-face {
            padding: 16px;
        }
        .face-body {
            font-size: 15px;
        }
    }
</style>
