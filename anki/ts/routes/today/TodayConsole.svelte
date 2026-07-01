<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

TodayConsole — the presentational body of the "Today" screen. It receives the
already-loaded, non-null Views from +page.svelte (the loader) so it never has to
reason about loading/abstention-of-existence itself. Readiness leads as the
dominant vital; Memory & Performance are the smaller inputs that feed it; then
the single Next-Action order, the acuity-ranked points-at-stake list, and the
daily-loop pathway. Honesty bar: readiness renders as a RANGE via the shared
gauge, or the honest "NOT ENOUGH INFO" flatline + reasons when abstained.
-->
<script lang="ts">
    import {
        ABSTAIN_HEADLINE,
        Chip,
        ConfidenceChip,
        formatCoverage,
        formatPercentRange,
        formatScorePoint,
        formatScoreRange,
        formatUpdatedAt,
        LoopPathway,
        PointsAtStakeRow,
        ReadinessGauge,
        readinessToGauge,
        stakeAcuity,
        VitalCard,
        VitalReadout,
    } from "$lib/speedrun";
    import type {
        CoverageMapView,
        LoopStep,
        MemoryScoreView,
        NextActionView,
        PerformanceScoreView,
        PointsAtStakeView,
        ReadinessScoreView,
    } from "$lib/speedrun";

    import { confidenceLabel, stakeNote, stakeReason } from "./presentation";

    export let readiness: ReadinessScoreView;
    export let memory: MemoryScoreView;
    export let performance: PerformanceScoreView;
    export let stakes: PointsAtStakeView;
    export let nextAction: NextActionView;
    export let coverage: CoverageMapView;
    /** Readiness target scaled score (console config; not on ScoreView). */
    export let target: number | undefined = undefined;
    /** Readiness abstain unlock rule (console config; not on ScoreView). */
    export let unlock: string | undefined = undefined;

    // The opinionated daily-loop spine. "Plan" (Today) is done; the active next
    // move is the Q-block the order card launches.
    const loopSteps: LoopStep[] = [
        { label: "Plan" },
        { label: "Q-block" },
        { label: "Auto-link" },
        { label: "Review" },
        { label: "Close" },
        { label: "Stop" },
    ];
    const loopActive = 1;

    $: gauge = readinessToGauge(readiness, { target, unlock });
    $: readinessRange = formatScoreRange(readiness);
    $: readinessPoint = formatScorePoint(readiness);
    $: readinessUpdated = formatUpdatedAt(readiness.updatedAt);
    $: memoryRange = formatPercentRange(memory);
    $: performanceRange = formatPercentRange(performance);
</script>

<div class="today">
    <!-- Readiness — the dominant vital -->
    <VitalCard label="Readiness · exam-day projection" emphasis="dominant" updated={readinessUpdated}>
        {#if readiness.abstained}
            <div class="readiness-abstain">{ABSTAIN_HEADLINE}</div>
        {:else}
            <div class="readiness-value">
                <span class="point">{readinessPoint}</span>
                <span class="scale">
                    <span class="scale-label">SCALED SCORE · 194–300</span>
                    <span class="range">range {readinessRange}</span>
                </span>
            </div>
        {/if}

        <div class="chips">
            <ConfidenceChip label={confidenceLabel(readiness)} muted={readiness.abstained} />
            <Chip>{formatCoverage(readiness.coveragePct)} covered</Chip>
            {#if target != null}
                <Chip>target {target}</Chip>
            {/if}
        </div>

        <div class="gauge">
            <ReadinessGauge {gauge} hideValue />
        </div>

        {#if readiness.reasons.length > 0}
            <ul class="reasons">
                {#each readiness.reasons as reason (reason)}
                    <li>{reason}</li>
                {/each}
            </ul>
        {/if}
    </VitalCard>

    <div class="lower">
        <div class="col-a">
            <!-- What feeds readiness — the smaller inputs -->
            <VitalCard label="What feeds readiness" emphasis="input">
                <div class="inputs">
                    <VitalReadout
                        label="MEMORY"
                        value={memoryRange ?? "\u2014"}
                        sub="FSRS recall"
                        muted={memory.abstained}
                    />
                    <span class="input-divider" aria-hidden="true"></span>
                    <VitalReadout
                        label="PERFORMANCE"
                        value={performanceRange ?? "\u2014"}
                        sub="QBank accuracy"
                        muted={performance.abstained}
                    />
                </div>
            </VitalCard>

            <!-- The single Next-Action order -->
            {#if nextAction.available}
                <section class="order" aria-label="Next action">
                    <span class="order-eyebrow">NEXT — DO THIS NOW</span>
                    <h2 class="order-headline">{nextAction.headline}</h2>
                    {#if nextAction.meta}
                        <p class="order-meta">{nextAction.meta}</p>
                    {/if}
                    <div class="order-actions">
                        <button type="button" class="btn primary">Start block →</button>
                        <button type="button" class="btn ghost">Not now</button>
                    </div>
                </section>
            {:else}
                <section class="order rest" aria-label="Next action">
                    <span class="order-eyebrow muted">NOTHING DUE · REST</span>
                    <h2 class="order-headline">You're caught up. Rest is part of the plan.</h2>
                </section>
            {/if}
        </div>

        <div class="col-b">
            <!-- Today's focus — points at stake -->
            <section class="block" aria-label="Today's focus — points at stake">
                <div class="block-head">
                    <span class="eyebrow">Today's focus · points at stake</span>
                </div>
                <div class="stakes">
                    {#each stakes.topics as topic, i (topic.topicId)}
                        <PointsAtStakeRow
                            topic={topic.name}
                            reason={stakeReason(topic)}
                            acuity={stakeAcuity(topic)}
                            weight={topic.points}
                            note={stakeNote(topic, coverage)}
                            divider={i > 0}
                        />
                    {/each}
                </div>
            </section>

            <!-- The opinionated daily-loop spine -->
            <section class="block" aria-label="Daily loop">
                <div class="block-head">
                    <span class="eyebrow">Daily loop</span>
                    <span class="spine-tag">the opinionated spine</span>
                </div>
                <LoopPathway steps={loopSteps} activeIndex={loopActive} numbered />
            </section>
        </div>
    </div>
</div>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .today {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .lower {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .col-a,
    .col-b {
        display: flex;
        flex-direction: column;
        gap: 14px;
        min-width: 0;
    }

    /* Readiness — dominant vital ------------------------------------------- */
    .readiness-value {
        display: flex;
        align-items: baseline;
        gap: 12px;
        margin-bottom: 10px;
    }
    .point {
        font-family: stat.$font-display;
        font-variant-numeric: tabular-nums;
        font-size: 40px;
        font-weight: 700;
        line-height: 1;
        color: stat.$ink;
    }
    .scale {
        display: flex;
        flex-direction: column;
        gap: 1px;
    }
    .scale-label {
        @include stat.readout;
        font-size: 10px;
        letter-spacing: 0.08em;
        color: stat.$ink-soft;
    }
    .range {
        @include stat.readout;
        font-size: 13px;
        color: stat.$ink;
    }
    .readiness-abstain {
        font-family: stat.$font-display;
        font-size: 24px;
        font-weight: 700;
        line-height: 1.1;
        color: stat.$muted;
        margin-bottom: 10px;
    }

    .chips {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        margin-bottom: 10px;
    }
    .gauge {
        margin: 2px 0 10px;
    }

    .reasons {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .reasons li {
        position: relative;
        padding-inline-start: 14px;
        font-size: 11px;
        color: stat.$ink-soft;
    }
    .reasons li::before {
        content: "·";
        position: absolute;
        inset-inline-start: 4px;
    }

    /* What feeds readiness — inputs ---------------------------------------- */
    .inputs {
        display: flex;
        align-items: stretch;
    }
    .input-divider {
        width: 1px;
        margin: 0 14px;
        background: stat.$line;
    }

    /* The Next-Action order ------------------------------------------------ */
    .order {
        border-inline-start: 3px solid stat.$primary;
        background: stat.$primary-wash;
        border-radius: stat.$radius-sm;
        padding: 14px 16px;
    }
    .order.rest {
        border-inline-start-color: stat.$muted;
        background: transparent;
        border: 1px solid stat.$line;
    }
    .order-eyebrow {
        @include stat.readout;
        font-size: 10px;
        letter-spacing: 0.14em;
        color: stat.$primary;
    }
    .order-eyebrow.muted {
        color: stat.$muted;
    }
    .order-headline {
        font-family: stat.$font-display;
        font-size: 20px;
        font-weight: 700;
        line-height: 1.18;
        color: stat.$ink;
        margin: 6px 0 4px;
    }
    .order-meta {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
        margin: 0;
    }
    .order-actions {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        margin-top: 12px;
    }
    .btn {
        font-family: stat.$font-mono;
        font-size: 12px;
        border-radius: stat.$radius-sm;
        padding: 8px 16px;
        border: 1px solid transparent;
        cursor: pointer;
    }
    .btn.primary {
        background: stat.$primary;
        border-color: stat.$primary;
        color: stat.$surface;
        font-weight: 600;
    }
    .btn.ghost {
        background: transparent;
        border-color: stat.$line;
        color: stat.$ink-soft;
    }
    .btn:focus-visible {
        outline: 2px solid stat.$primary;
        outline-offset: 2px;
    }

    /* Focus + loop blocks -------------------------------------------------- */
    .block-head {
        display: flex;
        align-items: baseline;
        gap: 8px;
        margin-bottom: 6px;
    }
    .eyebrow {
        @include stat.eyebrow;
    }
    .spine-tag {
        @include stat.readout;
        margin-inline-start: auto;
        font-size: 10px;
        color: stat.$ink-soft;
    }
    .stakes {
        display: flex;
        flex-direction: column;
    }

    /* Desktop: readiness leads full-width, then a two-column console that
       collapses back to a single column on phones (thumb-reach). */
    @media (min-width: stat.$bp-compact) {
        .lower {
            display: grid;
            grid-template-columns: 1.05fr 0.95fr;
            gap: 16px;
            align-items: start;
        }
    }
</style>
