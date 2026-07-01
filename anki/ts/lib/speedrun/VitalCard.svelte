<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

VitalCard — the clinical surface card used across STAT screens. An eyebrow label
+ optional "updated" meta (or a `meta` slot) over a slotted body. Use
`emphasis="dominant"` for the readiness-dominant vital (compose a ReadinessGauge
+ big value inside it); `emphasis="input"` for the smaller memory/performance
inputs. Flat by intent — no shadows, no gradients.
-->
<script lang="ts">
    export let label: string | null = null;
    /** Right-aligned meta, e.g. "updated 2h ago". Ignored if the `meta` slot is used. */
    export let updated: string | null = null;
    export let emphasis: "default" | "dominant" | "input" = "default";
</script>

<section
    class="stat-vital-card"
    class:dominant={emphasis === "dominant"}
    class:input={emphasis === "input"}
>
    {#if label || updated || $$slots.meta}
        <header class="head">
            {#if label}<span class="eyebrow">{label}</span>{/if}
            <span class="meta">
                {#if $$slots.meta}
                    <slot name="meta" />
                {:else if updated}
                    {updated}
                {/if}
            </span>
        </header>
    {/if}
    <slot />
</section>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-vital-card {
        display: block;
        padding: 14px 16px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        color: stat.$ink;

        &.dominant {
            padding: 16px;
        }
        &.input {
            padding: 10px 12px;
            background: stat.$paper;
        }
    }

    .head {
        display: flex;
        align-items: baseline;
        gap: 10px;
        margin-bottom: 8px;
    }

    .eyebrow {
        font-family: stat.$font-mono;
        font-size: 10px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: stat.$ink-soft;
    }

    .meta {
        margin-inline-start: auto;
        font-family: stat.$font-mono;
        font-size: 10px;
        color: stat.$ink-soft;
        white-space: nowrap;
    }
</style>
