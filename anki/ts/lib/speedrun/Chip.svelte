<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

Chip — the small monospace pill used everywhere for badges and status
(device, "SYNCED", "AI OFF", coverage, target…). This is the canvas's MockChip.
Label goes in the default slot; pass `dot` to prepend a round acuity dot.
-->
<script lang="ts">
    import StatusDot from "./StatusDot.svelte";
    import type { Acuity } from "./types";

    /** Optional leading status dot. */
    export let dot: Acuity | null = null;
    /** Visual weight: hairline (default), teal wash, or solid teal. */
    export let tone: "line" | "wash" | "solid" = "line";
</script>

<span class="stat-chip" class:wash={tone === "wash"} class:solid={tone === "solid"}>
    {#if dot}
        <StatusDot acuity={dot} shape="round" size={6} />
    {/if}
    <slot />
</span>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-chip {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        padding: 2px 8px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-pill;
        background: stat.$surface;
        color: stat.$ink-soft;
        font-family: stat.$font-mono;
        font-size: 10px;
        letter-spacing: 0.06em;
        white-space: nowrap;

        &.wash {
            border-color: stat.$primary;
            background: stat.$primary-wash;
            color: stat.$primary;
        }
        &.solid {
            border-color: stat.$primary;
            background: stat.$primary;
            color: stat.$surface;
        }
    }
</style>
