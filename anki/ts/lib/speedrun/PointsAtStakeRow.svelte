<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

PointsAtStakeRow — one weak high-yield topic in the "Today's focus" list: an
acuity marker + topic + reason, with an optional note badge and the point
weight. Rows draw a top hairline by default; set `divider={false}` on the first.
Derive `acuity` from the topic's weakness with `acuityFromWeakness()`.
-->
<script lang="ts">
    import Chip from "./Chip.svelte";
    import { formatStakePoints } from "./display";
    import StatusDot from "./StatusDot.svelte";
    import type { Acuity } from "./types";

    export let topic: string;
    export let reason: string;
    export let acuity: Acuity = "muted";
    /** Point weight: a number renders "N pts", a string renders as-is, null "—". */
    export let weight: number | string | null = null;
    /** Optional trailing badge, e.g. "Paused" / "Untouched". */
    export let note: string | null = null;
    /** Draw a top hairline separator (disable on the first row). */
    export let divider = true;

    $: weightLabel =
        typeof weight === "number" ? formatStakePoints(weight) : (weight ?? "\u2014");
</script>

<div class="stat-stake" class:divider>
    <StatusDot {acuity} />
    <span class="topic" class:muted={acuity === "muted"}>{topic}</span>
    <span class="reason">{reason}</span>
    <span class="spacer"></span>
    {#if note}
        <Chip>{note}</Chip>
    {/if}
    <span class="weight">{weightLabel}</span>
</div>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-stake {
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 8px 0;
        font-family: stat.$font-body;

        &.divider {
            border-top: 1px solid stat.$line;
        }
    }

    .topic {
        font-size: 13px;
        font-weight: 600;
        color: stat.$ink;

        &.muted {
            color: stat.$ink-soft;
        }
    }
    .reason {
        font-size: 12px;
        color: stat.$ink-soft;
    }
    .spacer {
        flex: 1;
    }
    .weight {
        font-family: stat.$font-mono;
        font-variant-numeric: tabular-nums;
        font-size: 11px;
        color: stat.$ink-soft;
        white-space: nowrap;
    }
</style>
