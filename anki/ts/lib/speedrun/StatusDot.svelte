<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

StatusDot — the small acuity marker (square by default, as on the points-at-stake
rows and alerts; round for chip dots). Purely decorative: color is never the only
signal, so it is aria-hidden and always paired with a label by the consumer.
-->
<script lang="ts">
    import type { Acuity } from "./types";

    export let acuity: Acuity = "muted";
    export let shape: "square" | "round" = "square";
    /** Marker size in px. */
    export let size = 10;
</script>

<span
    class="stat-dot"
    class:round={shape === "round"}
    class:critical={acuity === "critical"}
    class:watch={acuity === "watch"}
    class:stable={acuity === "stable"}
    class:muted={acuity === "muted"}
    style="--stat-dot-size: {size}px"
    aria-hidden="true"
></span>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-dot {
        display: inline-block;
        flex-shrink: 0;
        width: var(--stat-dot-size, 10px);
        height: var(--stat-dot-size, 10px);
        border-radius: 2px;
        background: stat.$muted;

        &.round {
            border-radius: stat.$radius-pill;
        }
        &.critical {
            background: stat.$critical;
        }
        &.watch {
            background: stat.$watch;
        }
        &.stable {
            background: stat.$stable;
        }
        &.muted {
            background: stat.$muted;
        }
    }
</style>
