<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

TrajectoryChart — a route-local SVG chart for the Exam-Trajectory view. Three
metrics share one geometry builder:
  - "readiness": a confidence BAND over the weeks to exam, an honest ABSTAINED
    region while coverage < 50%, and a dashed exam-day PROJECTION (never a solid
    promise), against dashed TARGET + PASS reference lines.
  - "coverage" / "stability": an area + line sparkline with a dashed projection
    segment; coverage also draws the 50% "readiness unlocks" line.
Pass `compact` for the small sparklines. Colors + type come from the shared STAT
tokens via SCSS; the chart reads only the route-local illustrative time-series.
The builder returns a flat, non-nullable model so the markup stays simple.
-->
<script lang="ts">
    import {
        COVERAGE,
        COVERAGE_UNLOCK_PCT,
        NOW_INDEX,
        PASS,
        READINESS,
        STABILITY,
        TARGET,
        WEEKS,
    } from "./trajectory-mock";

    type Metric = "readiness" | "coverage" | "stability";

    export let metric: Metric = "readiness";
    export let compact = false;

    const EN_DASH = "\u2013";
    const N = WEEKS.length;

    function build(metric: Metric, compact: boolean) {
        const W = compact ? 264 : 720;
        const H = compact ? 118 : 300;
        const X0 = compact ? 10 : 58;
        const X1 = compact ? 254 : 694;
        const YT = compact ? 14 : 26;
        const YB = compact ? 98 : 250;

        const isReadiness = metric === "readiness";
        // Flatten metric-based selection to satisfy no-nested-ternary.
        function pick<T>(coverage: T, stability: T, other: T): T {
            if (metric === "coverage") {
                return coverage;
            }
            if (metric === "stability") {
                return stability;
            }
            return other;
        }
        let series: typeof COVERAGE | typeof STABILITY | never[] = [];
        if (metric === "coverage") {
            series = COVERAGE;
        } else if (metric === "stability") {
            series = STABILITY;
        }
        const vFloor = isReadiness ? 205 : 0;
        const vMax = pick(100, 28, 262);
        const unit = pick("%", "d", "");

        const xAt = (i: number): number => X0 + (i / (N - 1)) * (X1 - X0);
        const yAt = (v: number): number =>
            YB - ((v - vFloor) / (vMax - vFloor)) * (YB - YT);

        let gridVals: number[];
        if (isReadiness) {
            gridVals = [PASS, TARGET, 260];
        } else if (metric === "coverage") {
            gridVals = [25, 50, 75, 100];
        } else {
            gridVals = [7, 14, 21, 28];
        }
        const gridLines = gridVals.map((g) => ({ g, y: yAt(g), label: `${g}${unit}` }));
        const weekLabels = WEEKS.map((label, i) => ({
            x: xAt(i),
            label,
            exam: label === "Exam",
        }));

        // Readiness band pieces.
        let bandPoints = "";
        let centerPoints = "";
        let projBandPoints = "";
        let projCenterPoints = "";
        let showAbstain = false;
        let abstainX = 0;
        let abstainW = 0;
        let showNow = false;
        let nowDotX = 0;
        let nowDotY = 0;
        let nowLabelX = 0;
        let nowLabelY = 0;
        let nowLabelText = "";
        let projLabelX = 0;
        let projLabelY = 0;
        let projLabelText = "";
        let targetY = 0;
        let passY = 0;

        // Coverage / stability sparkline pieces.
        let areaPoints = "";
        let linePoints = "";
        let seriesProjPoints = "";
        let dots: { x: number; y: number }[] = [];
        let showProjDot = false;
        let projDotX = 0;
        let projDotY = 0;
        let showUnlock = false;
        let unlockY = 0;
        let nowValueX = 0;
        let nowValueY = 0;
        let nowValueText = "";

        if (isReadiness) {
            targetY = yAt(TARGET);
            passY = yAt(PASS);

            const obs = READINESS.map((r, i) => ({ r, i }))
                .filter(({ r }) => r.pt !== null && !r.projection)
                .map(({ i }) => i);
            const top = obs.map((i) => `${xAt(i)},${yAt(READINESS[i].hi as number)}`);
            const bottom = [...obs]
                .reverse()
                .map((i) => `${xAt(i)},${yAt(READINESS[i].lo as number)}`);
            bandPoints = [...top, ...bottom].join(" ");
            centerPoints = obs
                .map((i) => `${xAt(i)},${yAt(READINESS[i].pt as number)}`)
                .join(" ");

            const nowR = READINESS[NOW_INDEX];
            const projR = READINESS[NOW_INDEX + 1];
            if (nowR.pt !== null && projR.pt !== null) {
                showNow = true;
                projBandPoints =
                    `${xAt(NOW_INDEX)},${yAt(nowR.hi as number)} ` +
                    `${xAt(NOW_INDEX + 1)},${yAt(projR.hi as number)} ` +
                    `${xAt(NOW_INDEX + 1)},${yAt(projR.lo as number)} ` +
                    `${xAt(NOW_INDEX)},${yAt(nowR.lo as number)}`;
                projCenterPoints = `${xAt(NOW_INDEX)},${yAt(nowR.pt)} ${xAt(NOW_INDEX + 1)},${yAt(projR.pt)}`;
                nowDotX = xAt(NOW_INDEX);
                nowDotY = yAt(nowR.pt);
                nowLabelX = xAt(NOW_INDEX);
                nowLabelY = yAt(nowR.hi as number) - 8;
                nowLabelText = `${nowR.lo}${EN_DASH}${nowR.hi}`;
                projLabelX = xAt(NOW_INDEX + 1);
                projLabelY = yAt(projR.pt) + 16;
                projLabelText = `proj. ${projR.lo}${EN_DASH}${projR.hi}`;
            }

            const abstainCount = READINESS.filter((r) => r.pt === null).length;
            if (abstainCount > 0) {
                showAbstain = true;
                abstainX = xAt(0) - (compact ? 2 : 6);
                abstainW = xAt(abstainCount - 0.5) - abstainX;
            }
        } else {
            const idx = Array.from({ length: NOW_INDEX + 1 }, (_, i) => i);
            areaPoints =
                `${xAt(0)},${YB} ` +
                idx.map((i) => `${xAt(i)},${yAt(series[i])}`).join(" ") +
                ` ${xAt(NOW_INDEX)},${YB}`;
            linePoints = idx.map((i) => `${xAt(i)},${yAt(series[i])}`).join(" ");
            seriesProjPoints =
                `${xAt(NOW_INDEX)},${yAt(series[NOW_INDEX])} ` +
                `${xAt(NOW_INDEX + 1)},${yAt(series[NOW_INDEX + 1])}`;
            dots = idx.map((i) => ({ x: xAt(i), y: yAt(series[i]) }));
            showProjDot = true;
            projDotX = xAt(NOW_INDEX + 1);
            projDotY = yAt(series[NOW_INDEX + 1]);
            if (metric === "coverage") {
                showUnlock = true;
                unlockY = yAt(COVERAGE_UNLOCK_PCT);
            }
            nowValueX = xAt(NOW_INDEX);
            nowValueY = yAt(series[NOW_INDEX]) - 10;
            nowValueText = `${series[NOW_INDEX]}${unit} now`;
        }

        return {
            W,
            H,
            X0,
            X1,
            YT,
            YB,
            gridLines,
            weekLabels,
            isReadiness,
            bandPoints,
            centerPoints,
            projBandPoints,
            projCenterPoints,
            showAbstain,
            abstainX,
            abstainW,
            showNow,
            nowDotX,
            nowDotY,
            nowLabelX,
            nowLabelY,
            nowLabelText,
            projLabelX,
            projLabelY,
            projLabelText,
            targetY,
            passY,
            areaPoints,
            linePoints,
            seriesProjPoints,
            dots,
            showProjDot,
            projDotX,
            projDotY,
            showUnlock,
            unlockY,
            nowValueX,
            nowValueY,
            nowValueText,
        };
    }

    function titleFor(m: Metric): string {
        if (m === "readiness") {
            return "Readiness range trending toward the target by exam day";
        }
        if (m === "coverage") {
            return "Blueprint coverage growth to exam day";
        }
        return "Mean memory stability growth to exam day";
    }

    $: c = build(metric, compact);
    $: title = titleFor(metric);
</script>

<svg
    class="chart"
    viewBox="0 0 {c.W} {c.H}"
    width="100%"
    height={c.H}
    preserveAspectRatio="xMidYMid meet"
    role="img"
>
    <title>{title}</title>

    {#each c.gridLines as gl (gl.g)}
        <line class="grid" x1={c.X0} y1={gl.y} x2={c.X1} y2={gl.y} />
        {#if !compact}
            <text class="txt tick" x={c.X0 - 8} y={gl.y + 4} text-anchor="end">
                {gl.label}
            </text>
        {/if}
    {/each}

    <line class="axis" x1={c.X0} y1={c.YB} x2={c.X1} y2={c.YB} />

    {#if !compact}
        {#each c.weekLabels as wl (wl.label)}
            <text
                class="txt tick"
                class:exam={wl.exam}
                x={wl.x}
                y={c.YB + 18}
                text-anchor="middle"
            >
                {wl.label}
            </text>
        {/each}
    {/if}

    {#if c.isReadiness}
        <line
            class="ref target"
            x1={c.X0}
            y1={c.targetY}
            x2={c.X1}
            y2={c.targetY}
            stroke-dasharray="6 4"
        />
        <line
            class="ref pass"
            x1={c.X0}
            y1={c.passY}
            x2={c.X1}
            y2={c.passY}
            stroke-dasharray="2 3"
        />
        {#if !compact}
            <text class="txt ref-target" x={c.X1} y={c.targetY - 6} text-anchor="end">
                TARGET {TARGET}
            </text>
            <text class="txt tick" x={c.X1} y={c.passY - 5} text-anchor="end">
                PASS {PASS}
            </text>
        {/if}

        {#if c.showAbstain}
            <rect
                class="abstain-region"
                x={c.abstainX}
                y={c.YT}
                width={c.abstainW}
                height={c.YB - c.YT}
            />
            {#if !compact}
                <text
                    class="txt abstain"
                    x={c.abstainX + c.abstainW / 2}
                    y={c.YT + 44}
                    text-anchor="middle"
                >
                    ABSTAINED
                </text>
                <text
                    class="txt abstain-sub"
                    x={c.abstainX + c.abstainW / 2}
                    y={c.YT + 60}
                    text-anchor="middle"
                >
                    coverage &lt; 50%
                </text>
            {/if}
        {/if}

        {#if c.bandPoints}<polygon class="band" points={c.bandPoints} />{/if}
        {#if c.projBandPoints}<polygon
                class="proj-band"
                points={c.projBandPoints}
                stroke-dasharray="4 3"
            />{/if}
        {#if c.centerPoints}<polyline class="center" points={c.centerPoints} />{/if}
        {#if c.projCenterPoints}<polyline
                class="proj-center"
                points={c.projCenterPoints}
                stroke-dasharray="4 3"
            />{/if}

        {#if c.showNow}
            <circle class="now-dot" cx={c.nowDotX} cy={c.nowDotY} r={compact ? 3 : 4} />
            {#if !compact}
                <text
                    class="txt now-value"
                    x={c.nowLabelX}
                    y={c.nowLabelY}
                    text-anchor="middle"
                >
                    {c.nowLabelText}
                </text>
                <text
                    class="txt proj-label"
                    x={c.projLabelX}
                    y={c.projLabelY}
                    text-anchor="middle"
                >
                    {c.projLabelText}
                </text>
            {/if}
        {/if}
    {:else}
        {#if c.showUnlock}
            <line
                class="ref unlock"
                x1={c.X0}
                y1={c.unlockY}
                x2={c.X1}
                y2={c.unlockY}
                stroke-dasharray="6 4"
            />
            {#if !compact}
                <text
                    class="txt ref-unlock"
                    x={c.X1}
                    y={c.unlockY - 6}
                    text-anchor="end"
                >
                    READINESS UNLOCKS ≥ 50%
                </text>
            {/if}
        {/if}
        {#if c.areaPoints}<polygon class="area" points={c.areaPoints} />{/if}
        {#if c.linePoints}<polyline class="line" points={c.linePoints} />{/if}
        {#if c.seriesProjPoints}<polyline
                class="proj-center"
                points={c.seriesProjPoints}
                stroke-dasharray="4 3"
            />{/if}
        {#each c.dots as d, i (i)}<circle
                class="dot"
                cx={d.x}
                cy={d.y}
                r={compact ? 1.8 : 2.5}
            />{/each}
        {#if c.showProjDot}<circle
                class="proj-dot"
                cx={c.projDotX}
                cy={c.projDotY}
                r={compact ? 2.4 : 3}
                stroke-dasharray="2 2"
            />{/if}
        {#if !compact}
            <text
                class="txt now-value"
                x={c.nowValueX}
                y={c.nowValueY}
                text-anchor="middle"
            >
                {c.nowValueText}
            </text>
        {/if}
    {/if}
</svg>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .chart {
        display: block;
    }

    .grid {
        stroke: stat.$line;
        stroke-width: 1;
    }
    .axis {
        stroke: stat.$ink-soft;
        stroke-width: 1.5;
    }

    .txt {
        font-family: stat.$font-mono;
        font-variant-numeric: tabular-nums;
    }
    .tick {
        fill: stat.$ink-soft;
        font-size: 10px;
    }
    .tick.exam {
        fill: stat.$ink;
        font-weight: 700;
    }

    .ref {
        stroke-width: 1.5;
    }
    .ref.target {
        stroke: stat.$stable;
    }
    .ref.pass {
        stroke: stat.$ink-soft;
        stroke-width: 1;
    }
    .ref.unlock {
        stroke: stat.$watch;
    }
    .ref-target {
        fill: stat.$stable;
        font-size: 10px;
    }
    .ref-unlock {
        fill: stat.$watch;
        font-size: 9px;
    }

    .abstain-region {
        fill: stat.$muted;
        fill-opacity: 0.08;
    }
    .abstain {
        fill: stat.$muted;
        font-size: 10px;
        letter-spacing: 0.08em;
    }
    .abstain-sub {
        fill: stat.$muted;
        font-size: 8.5px;
    }

    .band {
        fill: stat.$primary;
        fill-opacity: 0.16;
    }
    .proj-band {
        fill: none;
        stroke: stat.$primary;
        stroke-opacity: 0.5;
        stroke-width: 1;
    }
    .center,
    .proj-center,
    .line {
        fill: none;
        stroke: stat.$primary;
        stroke-width: 2;
    }
    .now-dot,
    .dot {
        fill: stat.$primary;
    }
    .now-value {
        fill: stat.$ink;
        font-size: 11px;
        font-weight: 600;
    }
    .proj-label {
        fill: stat.$primary;
        font-size: 9px;
    }

    .area {
        fill: stat.$primary;
        fill-opacity: 0.12;
    }
    .proj-dot {
        fill: none;
        stroke: stat.$primary;
    }
</style>
