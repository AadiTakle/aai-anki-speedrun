<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

LoopPathway — the opinionated daily-loop spine as a strip of pills. Steps before
`activeIndex` read as done (solid teal), the active step as teal wash, and later
steps as upcoming (hairline). `numbered` prefixes 1..n; `loop` closes the strip
with a ↺; per-step `caption`s render beneath each pill when present.
-->
<script lang="ts">
    import type { LoopStep } from "./types";

    export let steps: LoopStep[] = [];
    export let activeIndex = 0;
    export let numbered = false;
    /** Close the strip with a recurring ↺ instead of ending flat. */
    export let loop = false;
</script>

<div class="stat-loop">
    {#each steps as step, i (i)}
        <div class="step">
            <div class="pillwrap">
                <span
                    class="pill"
                    class:done={i < activeIndex}
                    class:active={i === activeIndex}
                    class:upcoming={i > activeIndex}
                >
                    {#if numbered}{i + 1}
                    {/if}{step.label}
                </span>
                {#if step.caption}
                    <span class="caption">{step.caption}</span>
                {/if}
            </div>
            {#if i < steps.length - 1}
                <span class="arrow" aria-hidden="true">&#8594;</span>
            {:else if loop}
                <span class="arrow loop" aria-hidden="true">&#8634;</span>
            {/if}
        </div>
    {/each}
</div>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-loop {
        display: flex;
        flex-wrap: wrap;
        align-items: flex-start;
        gap: 6px;
    }

    .step {
        display: flex;
        align-items: center;
        gap: 6px;
    }

    .pillwrap {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2px;
    }

    .pill {
        font-family: stat.$font-mono;
        font-size: 11px;
        padding: 4px 10px;
        border-radius: stat.$radius-pill;
        border: 1px solid stat.$line;
        color: stat.$ink-soft;
        background: transparent;
        white-space: nowrap;

        &.done {
            border-color: stat.$primary;
            background: stat.$primary;
            color: stat.$surface;
        }
        &.active {
            border-color: stat.$primary;
            background: stat.$primary-wash;
            color: stat.$primary;
        }
        &.upcoming {
            border-color: stat.$line;
            color: stat.$ink-soft;
        }
    }

    .caption {
        font-family: stat.$font-mono;
        font-size: 9px;
        color: stat.$ink-soft;
    }

    .arrow {
        color: stat.$ink-soft;
        font-size: 12px;

        &.loop {
            color: stat.$primary;
        }
    }
</style>
