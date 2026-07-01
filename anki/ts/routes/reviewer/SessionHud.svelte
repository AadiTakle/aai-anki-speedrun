<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

SessionHud — replaces Anki's deck picker + new/learn/review counts with a
focused targeted-session header: progress, the miss seed, the interleave
indicator, live pacing, and a points-at-stake order bar. The order bar makes the
"front-loaded by points-at-stake" selection legible; the current card is
highlighted, graded cards read as done, upcoming as dimmed.
-->
<script lang="ts">
    import { Chip } from "$lib/speedrun";

    import { pacingAcuity, sessionSystems, type ReviewCard } from "./cards-mock";

    export let cards: ReviewCard[];
    /** 0-based index of the current card. */
    export let index: number;
    /** Seconds elapsed on the current card (live). */
    export let elapsed = 0;

    $: total = cards.length;
    $: position = Math.min(index + 1, total);
    $: systems = sessionSystems(cards);
    $: current = cards[index];
    $: elapsedWhole = Math.round(elapsed);
    $: pacing = current ? pacingAcuity(elapsed, current.targetSeconds) : "stable";
    $: slow = pacing === "watch";
</script>

<section class="hud" aria-label="Targeted review session">
    <div class="line">
        <span class="eyebrow">Targeted session</span>
        <span class="progress">{position} / {total} cards</span>
        <span class="rule" aria-hidden="true"></span>
        <Chip dot="critical">SEEDED BY {total} MISSES</Chip>
        <Chip>INTERLEAVED · {systems.length} SYSTEMS</Chip>
        <span class="spacer"></span>
        {#if current}
            <Chip dot={pacing}>
                {slow ? "SLOW" : "PACING"} {elapsedWhole}s · target {current.targetSeconds}s
            </Chip>
        {/if}
    </div>

    <div class="order">
        <span class="order-label">Order</span>
        <div class="bars" role="img" aria-label="Cards ordered by points at stake, highest first">
            {#each cards as card, i (card.id)}
                <span
                    class="bar"
                    class:critical={card.acuity === "critical"}
                    class:watch={card.acuity === "watch"}
                    class:stable={card.acuity === "stable"}
                    class:muted={card.acuity === "muted"}
                    class:done={i < index}
                    class:current={i === index}
                    class:upcoming={i > index}
                    title="{card.provenance.topic} · {card.points} pts"
                ></span>
            {/each}
        </div>
        <span class="order-caption">points-at-stake</span>
    </div>
</section>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .hud {
        border: 1px solid stat.$line;
        border-radius: stat.$radius-md;
        background: stat.$surface;
        padding: 10px 14px;
    }

    .line {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 8px;
    }

    .eyebrow {
        @include stat.eyebrow;
    }

    .progress {
        @include stat.readout;
        font-size: 12px;
        font-weight: 600;
        color: stat.$ink;
    }

    .rule {
        width: 1px;
        height: 12px;
        background: stat.$line;
    }

    .spacer {
        flex: 1;
    }

    .order {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-top: 10px;
    }

    .order-label,
    .order-caption {
        @include stat.readout;
        font-size: 9px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: stat.$ink-soft;
        white-space: nowrap;
    }

    .bars {
        display: flex;
        flex: 1;
        gap: 4px;
    }

    .bar {
        flex: 1;
        height: 6px;
        border-radius: 2px;
        background: stat.$muted;

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

        // Selection state modulates opacity; hue always encodes acuity.
        &.done {
            opacity: 0.28;
        }
        &.upcoming {
            opacity: 0.5;
        }
        &.current {
            opacity: 1;
            outline: 2px solid stat.$ink;
            outline-offset: 1px;
        }
    }

    @media (max-width: stat.$bp-compact) {
        .spacer {
            display: none;
        }
    }
</style>
