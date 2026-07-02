<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT · Reviewer — the custom review UI over REAL cards.

Anki's scheduler stays authoritative: this page only displays the card and relays
the chosen rating. The Python bridge (aqt/speedrun.py) feeds each real card via
`window.speedrunSetCard(...)` and applies grades through the real
build_answer/answer_card (so intervals, undo, and integrity stay correct). Cards
come in points-at-stake order, and the answer side shows a real "why this card
matters" (its topic + your weakness). Built on the shared foundation
($lib/speedrun); it never edits it.
-->
<script lang="ts">
    import { onDestroy, onMount } from "svelte";

    import { AppShell } from "$lib/speedrun";

    interface Counts {
        new: number;
        learning: number;
        review: number;
    }
    interface CardData {
        question: string;
        answer: string;
        topicName: string;
        weakness: number;
        buttons: string[];
        counts: Counts;
    }

    const GRADE_LABELS = ["Again", "Hard", "Good", "Easy"];

    let card: CardData | null = null;
    let showingAnswer = false;
    let done = false;

    $: weaknessPct = card ? Math.round(card.weakness * 100) : 0;

    function send(message: string): void {
        const cmd = (globalThis as { pycmd?: (msg: string) => void }).pycmd;
        if (cmd) {
            cmd(message);
        }
    }

    function reveal(): void {
        showingAnswer = true;
    }

    function grade(ease: number): void {
        showingAnswer = false;
        send(`speedrunReviewAnswer:${ease}`);
    }

    function onKeydown(event: KeyboardEvent): void {
        if (done || card === null) {
            return;
        }
        if (!showingAnswer) {
            if (event.key === " " || event.key === "Enter") {
                event.preventDefault();
                reveal();
            }
            return;
        }
        if (event.key === " " || event.key === "Enter" || event.key === "3") {
            event.preventDefault();
            grade(3);
        } else if (event.key === "1" || event.key === "2" || event.key === "4") {
            grade(Number(event.key));
        }
    }

    onMount(() => {
        const bridge = window as unknown as {
            speedrunSetCard?: (c: CardData) => void;
            speedrunSessionDone?: () => void;
        };
        bridge.speedrunSetCard = (c: CardData): void => {
            card = c;
            showingAnswer = false;
            done = false;
        };
        bridge.speedrunSessionDone = (): void => {
            card = null;
            done = true;
        };
        window.addEventListener("keydown", onKeydown);
        send("speedrunReviewNext");
    });

    onDestroy(() => {
        window.removeEventListener("keydown", onKeydown);
    });
</script>

<AppShell active="reviewer">
    <div class="reviewer">
        {#if done}
            <div class="rest">
                <h1>Caught up</h1>
                <p>No cards due right now — rest is part of the plan.</p>
            </div>
        {:else if card === null}
            <div class="loading">Loading your next card…</div>
        {:else}
            <div class="hud">
                <span class="count new">{card.counts.new} new</span>
                <span class="count learn">{card.counts.learning} learning</span>
                <span class="count due">{card.counts.review} review</span>
            </div>

            {#if card.topicName}
                <div class="why">
                    <span class="why-eyebrow">Why this card</span>
                    <div class="why-reason">
                        <span class="why-topic">{card.topicName}</span>
                        <span class="why-sep" aria-hidden="true">·</span>
                        <span class="why-weak">
                            <span class="why-weak-label">weakness</span>
                            <span class="why-weak-num">{weaknessPct}%</span>
                        </span>
                    </div>
                </div>
            {/if}

            <div class="card-face">
                {#if showingAnswer}
                    {@html card.answer}
                {:else}
                    {@html card.question}
                {/if}
            </div>

            {#if showingAnswer}
                <div class="grades">
                    {#each GRADE_LABELS as label, i (label)}
                        <button
                            type="button"
                            class="grade g{i + 1}"
                            on:click={() => grade(i + 1)}
                        >
                            <span class="g-label">{label}</span>
                            <span class="g-ivl">{card.buttons[i] ?? ""}</span>
                        </button>
                    {/each}
                </div>
            {:else}
                <button type="button" class="show" on:click={reveal}>
                    Show answer
                </button>
            {/if}
        {/if}
    </div>
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .reviewer {
        max-width: 760px;
        margin: 0 auto;
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .loading,
    .rest {
        padding: 48px 8px;
        text-align: center;
        color: stat.$ink-soft;
    }
    .rest h1 {
        margin: 0 0 6px;
        color: stat.$ink;
    }

    .hud {
        display: flex;
        gap: 12px;
        font-family: stat.$font-mono;
        font-size: 11px;
        color: stat.$ink-soft;
    }
    .count.new {
        color: stat.$primary;
    }

    /* A rationale callout: an uppercase kicker over one reason line (subject +
       datum). Teal lives only on the border so the text stays ink/ink-soft. */
    .why {
        display: flex;
        flex-direction: column;
        gap: 3px;
        padding: 8px 12px;
        border-inline-start: 3px solid stat.$primary;
        background: stat.$primary-wash;
        border-radius: stat.$radius-sm;
    }
    .why-eyebrow {
        @include stat.eyebrow;
    }
    .why-reason {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 8px;
    }
    .why-topic {
        font-family: stat.$font-body;
        font-size: 15px;
        font-weight: 600;
        color: stat.$ink;
    }
    .why-sep {
        color: stat.$line;
    }
    .why-weak {
        display: inline-flex;
        align-items: baseline;
        gap: 5px;
        @include stat.readout;
        font-size: 12px;
        color: stat.$ink-soft;
        white-space: nowrap;
    }
    .why-weak-num {
        font-weight: 600;
        color: stat.$ink;
    }

    .card-face {
        min-height: 180px;
        padding: 24px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        color: stat.$ink;
        font-size: 18px;
        line-height: 1.5;
    }

    .show {
        align-self: center;
        padding: 10px 28px;
        font-size: 15px;
        font-weight: 600;
        cursor: pointer;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$primary;
        background: stat.$primary;
        color: stat.$surface;
    }

    .grades {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 8px;
    }
    .grade {
        display: flex;
        flex-direction: column;
        gap: 2px;
        padding: 10px 8px;
        cursor: pointer;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$line;
        background: stat.$surface;
        color: stat.$ink;
    }
    .grade:hover {
        border-color: stat.$primary;
    }
    .g-label {
        font-weight: 600;
    }
    .g-ivl {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
    }
    .grade.g1 .g-label {
        color: stat.$critical;
    }
    .grade.g4 .g-label {
        color: stat.$primary;
    }
</style>
