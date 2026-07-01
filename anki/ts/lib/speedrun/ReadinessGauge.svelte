<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

ReadinessGauge — the STAT signature. Readiness drawn as a confidence BAND on the
194–300 USMLE scaled-score axis (with PASS + TARGET markers), and an honest
"NOT ENOUGH INFO" flatline when the give-up rule fires. Every number is labeled.
Derive the `gauge` prop from a ReadinessScoreView with `readinessToGauge()`.
-->
<script lang="ts">
    import { STAT_SCALE } from "./tokens";
    import type { GaugeView } from "./types";

    export let gauge: GaugeView;
    /** Compact variant (axis-only, no value/labels) for inline/mobile use. */
    export let compact = false;
    /** Draw the band/flatline but suppress the big value + captions. */
    export let hideValue = false;

    // U+2013 EN DASH.
    const EN_DASH = "\u2013";

    const SMIN = STAT_SCALE.min; // 194
    const SMAX = STAT_SCALE.max; // 300
    const PASS = STAT_SCALE.pass; // 214
    const X0 = 46;
    const X1 = 498;

    // Abstain box spans a fixed, illustrative slice of the axis.
    const ABSTAIN_LOW = 200;
    const ABSTAIN_HIGH = 292;

    const sx = (s: number): number => X0 + ((s - SMIN) / (SMAX - SMIN)) * (X1 - X0);

    $: height = compact ? 44 : 104;
    $: axisY = compact ? 26 : 70;
    $: confident =
        gauge.mode === "confident" &&
        gauge.low != null &&
        gauge.high != null &&
        gauge.point != null;
    $: showValue = !compact && !hideValue;
    $: abstainMid = (sx(ABSTAIN_LOW) + sx(ABSTAIN_HIGH)) / 2;
    $: title = confident
        ? `Readiness ${gauge.point} (range ${gauge.low}${EN_DASH}${gauge.high})`
        : "Readiness abstaining \u2014 insufficient data";
</script>

<svg
    class="stat-gauge"
    viewBox="0 0 520 {height}"
    width="100%"
    {height}
    preserveAspectRatio="xMidYMid meet"
    role="img"
>
    <title>{title}</title>

    <!-- axis -->
    <line class="axis" x1={X0} y1={axisY} x2={X1} y2={axisY} stroke-width="2" />

    <!-- pass marker -->
    <line
        class="pass"
        x1={sx(PASS)}
        y1={axisY - 10}
        x2={sx(PASS)}
        y2={axisY + 10}
        stroke-width="1"
        stroke-dasharray="2 3"
    />

    <!-- target marker -->
    {#if gauge.target != null}
        <polygon
            class="target"
            points="{sx(gauge.target) - 5},{axisY - 16} {sx(gauge.target) + 5},{axisY -
                16} {sx(gauge.target)},{axisY - 8}"
        />
    {/if}

    {#if confident}
        <rect
            class="band"
            x={sx(gauge.low ?? 0)}
            y={axisY - 9}
            width={Math.max(2, sx(gauge.high ?? 0) - sx(gauge.low ?? 0))}
            height="18"
            rx="4"
            fill-opacity="0.18"
            stroke-opacity="0.55"
        />
        <line
            class="point-line"
            x1={sx(gauge.point ?? 0)}
            y1={axisY - 16}
            x2={sx(gauge.point ?? 0)}
            y2={axisY + 14}
            stroke-width="2"
        />
        {#if showValue}
            <text
                class="txt value"
                x={sx(gauge.point ?? 0)}
                y={axisY - 22}
                font-size="17"
                text-anchor="middle"
            >
                {gauge.point}
            </text>
            <text
                class="txt caption"
                x={sx(gauge.point ?? 0)}
                y={axisY + 30}
                font-size="12"
                text-anchor="middle"
            >
                {gauge.low}{EN_DASH}{gauge.high}
            </text>
        {/if}
    {:else}
        <rect
            class="abstain-box"
            x={sx(ABSTAIN_LOW)}
            y={axisY - 10}
            width={sx(ABSTAIN_HIGH) - sx(ABSTAIN_LOW)}
            height="20"
            rx="5"
            stroke-width="1.5"
            stroke-dasharray="4 4"
        />
        <text
            class="txt abstain"
            x={abstainMid}
            y={axisY + (compact ? 4 : 5)}
            font-size={compact ? 11 : 13}
            letter-spacing="0.12em"
            text-anchor="middle"
        >
            NOT ENOUGH INFO
        </text>
        {#if showValue && gauge.unlock}
            <text
                class="txt caption"
                x={abstainMid}
                y={axisY + 30}
                font-size="11"
                text-anchor="middle"
            >
                {gauge.unlock}
            </text>
        {/if}
    {/if}

    {#if !compact}
        <text
            class="txt tick"
            x={sx(PASS)}
            y={axisY + 30}
            font-size="10"
            text-anchor="middle"
        >
            PASS {PASS}
        </text>
        {#if gauge.target != null}
            <text
                class="txt target-label"
                x={sx(gauge.target)}
                y={axisY - 22}
                font-size="10"
                text-anchor="middle"
            >
                TARGET {gauge.target}
            </text>
        {/if}
        <text class="txt tick" x={X0} y={axisY + 30} font-size="10" text-anchor="start">
            {SMIN}
        </text>
        <text class="txt tick" x={X1} y={axisY + 30} font-size="10" text-anchor="end">
            {SMAX}
        </text>
    {/if}
</svg>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-gauge {
        display: block;
    }

    .axis {
        stroke: stat.$line;
    }
    .pass {
        stroke: stat.$ink-soft;
    }
    .target {
        fill: stat.$stable;
    }
    .band {
        fill: stat.$primary;
        stroke: stat.$primary;
    }
    .point-line {
        stroke: stat.$primary;
    }
    .abstain-box {
        fill: none;
        stroke: stat.$muted;
    }

    .txt {
        font-family: stat.$font-mono;
        font-variant-numeric: tabular-nums;
    }
    .value {
        fill: stat.$ink;
        font-weight: 600;
    }
    .caption,
    .tick {
        fill: stat.$ink-soft;
    }
    .abstain {
        fill: stat.$muted;
    }
    .target-label {
        fill: stat.$stable;
    }
</style>
