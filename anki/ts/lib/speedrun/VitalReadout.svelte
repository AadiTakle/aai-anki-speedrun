<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

VitalReadout — a small monospace "input" readout (label / value / sub), used for
the Memory and Performance vitals that feed readiness. Pass a pre-formatted
range string as `value` (e.g. "68–74%"); pass `muted` for an abstained input.
-->
<script lang="ts">
    export let label: string;
    /** Pre-formatted value, e.g. "68–74%". */
    export let value: string;
    export let sub: string | null = null;
    /** Render as an abstained / insufficient input. */
    export let muted = false;
</script>

<div class="stat-readout" class:muted>
    <span class="label">{label}</span>
    <span class="value">{value}</span>
    {#if sub}
        <span class="sub">{sub}</span>
    {/if}
</div>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-readout {
        display: flex;
        flex-direction: column;
        gap: 2px;
        flex: 1;
        min-width: 0;
        font-family: stat.$font-mono;
        font-variant-numeric: tabular-nums;
    }

    .label {
        font-size: 10px;
        letter-spacing: 0.12em;
        color: stat.$ink-soft;
    }
    .value {
        font-size: 15px;
        font-weight: 600;
        color: stat.$ink;
    }
    .sub {
        font-size: 9px;
        color: stat.$ink-soft;
    }

    .stat-readout.muted .value {
        color: stat.$muted;
    }
</style>
