<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

DailyLoop — the daily loop as a progressing to-do list, and the console's merged
"what to do next" surface. It reads the engine's derived DailyPlanView (Plan →
Q-block → Review → Close): completed steps check off along a connecting rail, and
the single CURRENT step expands to carry the reason + CTA (the old order card).
Review shows a live "N of M" bar. When every step is done, the loop is closed and
the card collapses to the honest rest state. Progress is derived, never a manual
checkbox — the honesty bar applies to a to-do list too.
-->
<script lang="ts">
    import type { DailyTaskView, NextActionView } from "$lib/speedrun";

    export let tasks: DailyTaskView[] = [];
    /** Enriches the Review step's reason with the real recommended block. */
    export let nextAction: NextActionView;
    /** Launches the real reviewer (handled by the console via pycmd). */
    export let onStartReview: () => void = () => {};

    // The tally tracks the actionable steps; "Close" is the passive rail cap.
    $: actionable = tasks.filter((t) => t.id !== "close");
    $: doneCount = actionable.filter((t) => t.state === "done").length;
    $: totalCount = actionable.length;
    $: pct = totalCount ? Math.round((doneCount / totalCount) * 100) : 0;
    $: current = tasks.find((t) => t.state === "current") ?? null;
    $: allDone = tasks.length > 0 && current === null;

    function reason(task: DailyTaskView): string {
        if (task.id === "review") {
            return nextAction.available && nextAction.headline
                ? nextAction.headline
                : "Front-loaded by points-at-stake, seeded with today's misses.";
        }
        if (task.id === "qblock") {
            return "Import today's UWorld / AMBOSS results — auto-links your misses so weak cards resurface.";
        }
        return "";
    }
</script>

<section class="plan" aria-label="Today's plan">
    <div class="head">
        <span class="eyebrow">Today's plan</span>
        <span class="tally">{doneCount} / {totalCount} done</span>
    </div>
    <div class="track"><span class="fill" style="width:{pct}%"></span></div>

    <ol class="loop">
        {#each tasks as task (task.id)}
            <li class="task {task.state}" class:cap={task.id === "close"}>
                <span class="marker" aria-hidden="true">
                    {#if task.state === "done"}&#10003;{/if}
                </span>
                <div class="body">
                    <div class="line">
                        <span class="label">{task.label}</span>
                        <span class="detail">{task.detail}</span>
                    </div>

                    {#if task.state === "current"}
                        <div class="active">
                            <p class="reason">{reason(task)}</p>
                            {#if task.id === "review" && task.totalCount > 0}
                                <div
                                    class="mini"
                                    role="img"
                                    aria-label="{task.doneCount} of {task.totalCount} reviewed today"
                                >
                                    <span
                                        class="mini-fill"
                                        style="width:{(task.doneCount /
                                            task.totalCount) *
                                            100}%"
                                    ></span>
                                </div>
                            {/if}
                            {#if task.id === "review"}
                                <button
                                    type="button"
                                    class="cta"
                                    on:click={onStartReview}
                                >
                                    Start review &#8594;
                                </button>
                            {:else if task.id === "qblock"}
                                <a class="cta" href="/import">Open import &#8594;</a>
                            {/if}
                        </div>
                    {/if}
                </div>
            </li>
        {/each}
    </ol>

    {#if allDone}
        <p class="closed">You've closed the loop — rest is part of the plan.</p>
    {/if}
</section>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .plan {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        padding: 14px 16px;
    }

    .head {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 8px;
        margin-bottom: 8px;
    }
    .eyebrow {
        @include stat.eyebrow;
    }
    .tally {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
    }

    .track {
        height: 4px;
        border-radius: stat.$radius-pill;
        background: stat.$line;
        overflow: hidden;
        margin-bottom: 16px;
    }
    .fill {
        display: block;
        height: 100%;
        background: stat.$primary;
        transition: width 240ms ease;
    }

    .loop {
        list-style: none;
        margin: 0;
        padding: 0;
    }

    /* Each step: a status marker in a fixed gutter + the content, with a rail
       connecting consecutive markers (the "loop" spine). */
    .task {
        position: relative;
        padding: 0 0 14px 30px;
        min-height: 20px;
    }
    .task.cap {
        padding-bottom: 0;
    }
    .task:not(.cap)::before {
        content: "";
        position: absolute;
        left: 8px;
        top: 20px;
        bottom: 0;
        width: 2px;
        background: stat.$line;
    }
    /* A completed step's rail segment reads teal. */
    .task.done:not(.cap)::before {
        background: stat.$primary;
    }

    .marker {
        position: absolute;
        left: 0;
        top: 0;
        width: 18px;
        height: 18px;
        border-radius: 50%;
        border: 2px solid stat.$line;
        background: stat.$surface;
        display: flex;
        align-items: center;
        justify-content: center;
        box-sizing: border-box;
        font-family: stat.$font-mono;
        font-size: 10px;
        line-height: 1;
        color: stat.$surface;
    }
    .task.done .marker {
        border-color: stat.$primary;
        background: stat.$primary;
    }
    .task.current .marker {
        border-color: stat.$primary;
        background: stat.$primary-wash;
    }
    .task.current .marker::after {
        content: "";
        width: 7px;
        height: 7px;
        border-radius: 50%;
        background: stat.$primary;
    }

    .line {
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 10px;
    }
    .label {
        font-family: stat.$font-body;
        font-size: 14px;
        color: stat.$ink-soft;
    }
    .task.current .label {
        color: stat.$ink;
        font-weight: 600;
    }
    .detail {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
        white-space: nowrap;
    }

    /* The current step expands: reason + (review) live bar + CTA. */
    .active {
        margin-top: 8px;
        padding: 10px 12px;
        border-inline-start: 3px solid stat.$primary;
        background: stat.$primary-wash;
        border-radius: stat.$radius-sm;
    }
    .reason {
        margin: 0 0 10px;
        font-size: 12px;
        line-height: 1.4;
        color: stat.$ink;
    }
    .mini {
        height: 6px;
        border-radius: stat.$radius-pill;
        background: stat.$line;
        overflow: hidden;
        margin-bottom: 10px;
    }
    .mini-fill {
        display: block;
        height: 100%;
        background: stat.$primary;
    }
    .cta {
        display: inline-block;
        font-family: stat.$font-mono;
        font-size: 12px;
        font-weight: 600;
        padding: 8px 16px;
        border-radius: stat.$radius-sm;
        border: 1px solid stat.$primary;
        background: stat.$primary;
        color: stat.$surface;
        cursor: pointer;
        text-decoration: none;
    }
    .cta:focus-visible {
        outline: 2px solid stat.$primary;
        outline-offset: 2px;
    }

    .closed {
        margin: 4px 0 0;
        padding-top: 12px;
        border-top: 1px solid stat.$line;
        font-size: 13px;
        color: stat.$ink-soft;
    }
</style>
