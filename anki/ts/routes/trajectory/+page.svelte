<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT "Exam trajectory" — the motivation centerpiece (screen lane C1). The Today
gauge is a snapshot; this is that gauge over the weeks to exam day. It motivates
with GENUINE progress — a narrowing readiness RANGE closing on the target,
growing blueprint coverage, and rising memory stability — never a vanity number.
Early weeks ABSTAIN honestly (coverage < 50%); the exam-day point is a
PROJECTION, not a promise. The memory↔performance gap ranks "what to fix next".

The longitudinal series is route-local, illustrative scenario data (US-1 Aisha,
per STAT system-design §4) — the shared adapter exposes point-in-time Views, not
history, so the time-series lives in ./trajectory-mock. Built on the read-only
shared foundation; nothing under lib/speedrun is edited.
-->
<script lang="ts">
    import { AppShell, Chip, StatusDot } from "$lib/speedrun";
    import type { Acuity } from "$lib/speedrun";

    import TrajectoryChart from "./TrajectoryChart.svelte";
    import {
        COVERAGE,
        NOW_INDEX,
        rankGap,
        READINESS,
        STABILITY,
        TARGET,
        TRAJECTORY_SCENARIO,
    } from "./trajectory-mock";

    const now = READINESS[NOW_INDEX];
    const coverageNow = COVERAGE[NOW_INDEX];
    const stabilityNow = STABILITY[NOW_INDEX];

    const ranked = rankGap();
    const fixNext = ranked[0];

    // Acuity for a gap row: the biggest gap is what to fix first (critical); a
    // small gap is "strong on both" and a pause candidate (muted); the rest are
    // on watch. The number + word always ride with the color.
    function gapAcuity(gap: number, isFixNext: boolean): Acuity {
        if (isFixNext) {
            return "critical";
        }
        return gap <= 3 ? "muted" : "watch";
    }

    const gapCaption =
        `Biggest gap = "knows it, can't apply it (yet)." ${fixNext.topic} leads the fix-next queue ` +
        `(recall ${fixNext.memory}% vs application ${fixNext.performance}%).`;
</script>

<AppShell active="trajectory">
    <Chip slot="badges" dot="stable">SYNCED</Chip>

    <div class="trajectory">
        <header class="page-head">
            <h1 class="title">Exam trajectory</h1>
            <p class="subtitle">
                Your readiness range closing on the target — honestly. Wide when data is
                thin, narrowing as coverage and reviews accrue. The exam-day point is a
                projection, not a promise.
            </p>
        </header>

        <!-- Signature: the readiness band over the weeks to exam day -->
        <section class="card">
            <div class="card-head">
                <span class="card-title">Readiness trajectory to exam day</span>
                <span class="head-chips">
                    <Chip>{TRAJECTORY_SCENARIO}</Chip>
                    {#if now.lo != null && now.hi != null}
                        <Chip dot="stable">now {now.lo}–{now.hi}</Chip>
                    {/if}
                    <Chip>target {TARGET}</Chip>
                </span>
            </div>

            <TrajectoryChart metric="readiness" />

            <div class="axis-labels">
                <span>x: weeks to exam day</span>
                <span>y: USMLE scaled score (194–300)</span>
            </div>

            <div class="legend">
                <span class="legend-item">
                    <span class="sw band" aria-hidden="true"></span>
                    observed range
                </span>
                <span class="legend-item">
                    <span class="sw dash" aria-hidden="true"></span>
                    projection, not a promise
                </span>
                <span class="legend-item">
                    <span class="sw dash stable" aria-hidden="true"></span>
                    target {TARGET}
                </span>
                <span class="legend-item">
                    <span class="sw muted" aria-hidden="true"></span>
                    abstained (coverage &lt; 50%)
                </span>
            </div>

            <p class="caption">
                Scenario: {TRAJECTORY_SCENARIO} · illustrative, not live data · range + projection;
                abstains before 50% coverage.
            </p>
        </section>

        <!-- Progress sparklines: coverage growth + memory stability -->
        <div class="sparks">
            <section class="card spark">
                <div class="card-head">
                    <span class="card-title">Blueprint coverage</span>
                    <span class="spark-now">{coverageNow}% now</span>
                </div>
                <TrajectoryChart metric="coverage" compact />
                <p class="caption">
                    y: coverage (%) · readiness unlocks ≥ 50%. Illustrative.
                </p>
            </section>

            <section class="card spark">
                <div class="card-head">
                    <span class="card-title">Memory stability</span>
                    <span class="spark-now">{stabilityNow} d now</span>
                </div>
                <TrajectoryChart metric="stability" compact />
                <p class="caption">
                    y: mean FSRS stability (days) — how long recall survives.
                    Illustrative.
                </p>
            </section>
        </div>

        <!-- Memory ↔ performance gap: what to fix next -->
        <section class="card">
            <div class="card-head">
                <span class="card-title">
                    Memory ↔ performance gap — what to fix next
                </span>
                <span class="head-chips">
                    <span class="key">
                        <span class="sw bar-mem" aria-hidden="true"></span>
                        recall
                    </span>
                    <span class="key">
                        <span class="sw bar-perf" aria-hidden="true"></span>
                        applied
                    </span>
                </span>
            </div>

            <div class="gap-rows">
                {#each ranked as t (t.topic)}
                    {@const isFix = t === fixNext}
                    <div class="gap-row" class:fix={isFix}>
                        <div class="gap-row-head">
                            <StatusDot acuity={gapAcuity(t.gap, isFix)} />
                            <span class="gap-topic">{t.topic}</span>
                            <span class="spacer"></span>
                            <span class="gap-delta">
                                {t.gap >= 0 ? "+" : ""}{t.gap} gap{isFix
                                    ? " · fix next"
                                    : ""}
                            </span>
                        </div>
                        <div class="metric-line">
                            <span class="metric-key">recall</span>
                            <span class="bar-track">
                                <span class="bar mem" style="width: {t.memory}%"></span>
                            </span>
                            <span class="metric-val">{t.memory}%</span>
                        </div>
                        <div class="metric-line">
                            <span class="metric-key">applied</span>
                            <span class="bar-track">
                                <span
                                    class="bar perf"
                                    style="width: {t.performance}%"
                                ></span>
                            </span>
                            <span class="metric-val">{t.performance}%</span>
                        </div>
                    </div>
                {/each}
            </div>

            <p class="caption">
                {gapCaption} Endocrine is strong on both → a candidate to pause. Scenario:
                {TRAJECTORY_SCENARIO} · illustrative.
            </p>
        </section>
    </div>
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .trajectory {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    .page-head {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .title {
        font-family: stat.$font-display;
        font-size: 22px;
        font-weight: 700;
        letter-spacing: -0.01em;
        color: stat.$ink;
        margin: 0;
    }
    .subtitle {
        font-size: 13px;
        color: stat.$ink-soft;
        margin: 0;
        max-width: 62ch;
    }

    .card {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        padding: 14px 16px;
    }

    .card-head {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 8px;
        margin-bottom: 8px;
    }
    .card-title {
        font-family: stat.$font-display;
        font-size: 15px;
        font-weight: 700;
        color: stat.$ink;
    }
    .head-chips {
        margin-inline-start: auto;
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 6px;
    }
    .spark-now {
        @include stat.readout;
        margin-inline-start: auto;
        font-size: 12px;
        font-weight: 600;
        color: stat.$ink;
    }

    .axis-labels {
        display: flex;
        flex-wrap: wrap;
        gap: 14px;
        margin-top: 4px;
    }
    .axis-labels span {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }

    .legend {
        display: flex;
        flex-wrap: wrap;
        gap: 12px;
        margin-top: 8px;
    }
    .legend-item,
    .key {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }
    .sw {
        display: inline-block;
        width: 14px;
        height: 8px;
        border-radius: 2px;
        flex-shrink: 0;
    }
    .sw.band {
        background: stat.$primary;
        opacity: 0.3;
    }
    .sw.muted {
        background: stat.$muted;
        opacity: 0.35;
    }
    .sw.dash {
        height: 0;
        border-top: 2px dashed stat.$primary;
        border-radius: 0;
    }
    .sw.dash.stable {
        border-top-color: stat.$stable;
    }
    .sw.bar-mem {
        background: stat.$primary;
    }
    .sw.bar-perf {
        background: stat.$ink-soft;
    }

    .caption {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
        margin: 8px 0 0;
        line-height: 1.5;
    }

    .sparks {
        display: flex;
        flex-direction: column;
        gap: 14px;
    }

    /* Memory ↔ performance gap ------------------------------------------- */
    .gap-rows {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }
    .gap-row {
        display: flex;
        flex-direction: column;
        gap: 4px;
        padding-top: 12px;
        border-top: 1px solid stat.$line;
    }
    .gap-row:first-child {
        padding-top: 0;
        border-top: none;
    }
    .gap-row-head {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .gap-topic {
        font-size: 13px;
        font-weight: 600;
        color: stat.$ink;
    }
    .gap-row.fix .gap-topic {
        color: stat.$critical;
    }
    .spacer {
        flex: 1;
    }
    .gap-delta {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }
    .gap-row.fix .gap-delta {
        color: stat.$critical;
    }

    .metric-line {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .metric-key {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
        width: 52px;
        flex-shrink: 0;
    }
    .bar-track {
        flex: 1;
        height: 10px;
        border-radius: 3px;
        background: stat.$paper;
        border: 1px solid stat.$line;
        overflow: hidden;
    }
    .bar {
        display: block;
        height: 100%;
        border-radius: 2px;
    }
    .bar.mem {
        background: stat.$primary;
    }
    .bar.perf {
        background: stat.$ink-soft;
    }
    .metric-val {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
        width: 40px;
        text-align: end;
        flex-shrink: 0;
    }

    /* Two-up sparklines + gap beside the chart on wider viewports; single
       column on phones. */
    @media (min-width: stat.$bp-compact) {
        .sparks {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 14px;
        }
    }
</style>
