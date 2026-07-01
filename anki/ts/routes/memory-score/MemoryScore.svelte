<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script lang="ts">
    import { memoryScoreDisplay, type MemoryScoreLike } from "./lib";

    // Accepts the raw F6 score (the generated MemoryScore RPC result is
    // structurally assignable to MemoryScoreLike).
    export let score: MemoryScoreLike;

    $: display = memoryScoreDisplay(score);
</script>

<div class="memory-score">
    <h1 class="title">Memory</h1>

    {#if display.mode === "scored"}
        <div class="score" data-testid="score">
            <div class="range">{display.rangeLabel}</div>
            <div class="caption">
                likely range · best estimate {display.point}
            </div>
        </div>
    {:else}
        <div class="abstain" data-testid="abstain">
            <div class="abstain-headline">{display.headline}</div>
            <div class="caption">
                We won't show a score until there's enough evidence to back it.
            </div>
        </div>
    {/if}

    <div class="coverage" data-testid="coverage">
        Blueprint coverage: <strong>{display.coverageLabel}</strong>
    </div>

    {#if display.reasons.length > 0}
        <div class="reasons">
            <div class="reasons-heading">Why</div>
            <ul>
                {#each display.reasons as reason}
                    <li>{reason}</li>
                {/each}
            </ul>
        </div>
    {/if}
</div>

<style lang="scss">
    .memory-score {
        max-width: 30em;
        margin: 2em auto;
        font-size: var(--font-size);
        color: var(--fg);
    }

    .title {
        font-size: 1.1em;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        opacity: 0.7;
        margin-bottom: 0.25em;
    }

    .range {
        font-size: 3rem;
        font-weight: 700;
        line-height: 1.1;
    }

    .abstain-headline {
        font-size: 1.75rem;
        font-weight: 600;
        line-height: 1.2;
    }

    .caption {
        opacity: 0.7;
        margin-top: 0.25em;
    }

    .coverage {
        margin-top: 1em;
        padding-top: 0.75em;
        border-top: 1px solid var(--border);
    }

    .reasons {
        margin-top: 1em;
    }

    .reasons-heading {
        text-transform: uppercase;
        letter-spacing: 0.06em;
        font-size: 0.8em;
        opacity: 0.7;
        margin-bottom: 0.25em;
    }

    .reasons ul {
        margin: 0;
        padding-left: 1.2em;
    }
</style>
