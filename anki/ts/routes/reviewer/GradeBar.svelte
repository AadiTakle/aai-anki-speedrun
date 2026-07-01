<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

GradeBar — Anki's Again / Hard / Good / Easy grade row, KEPT verbatim: the same
four grades, their FSRS next-interval labels, the 1–4 shortcut keys, and Good as
the default. STAT changes selection/order/framing, never the grade flow itself.
On mobile the buttons become big thumb-reach targets along the bottom.
-->
<script lang="ts">
    import type { Grade, GradeIntervals } from "./cards-mock";

    export let intervals: GradeIntervals;
    /** Called with the chosen grade; the parent advances to the next card. */
    export let onGrade: (grade: Grade) => void;

    interface GradeDef {
        key: Grade;
        label: string;
        num: number;
        interval: string;
        critical?: boolean;
        isDefault?: boolean;
    }

    let grades: GradeDef[] = [];
    $: grades = [
        {
            key: "again",
            label: "Again",
            num: 1,
            interval: intervals.again,
            critical: true,
        },
        { key: "hard", label: "Hard", num: 2, interval: intervals.hard },
        {
            key: "good",
            label: "Good",
            num: 3,
            interval: intervals.good,
            isDefault: true,
        },
        { key: "easy", label: "Easy", num: 4, interval: intervals.easy },
    ];
</script>

<div class="grades" role="group" aria-label="Grade this card">
    {#each grades as g (g.key)}
        <div class="cell">
            <span class="interval">{g.interval}</span>
            <button
                type="button"
                class="grade"
                class:critical={g.critical}
                class:default={g.isDefault}
                title="{g.label} — shortcut {g.num}"
                on:click={() => onGrade(g.key)}
            >
                <span class="label">{g.label}</span>
                <span class="key" aria-hidden="true">{g.num}</span>
            </button>
            {#if g.isDefault}
                <span class="default-note">default</span>
            {/if}
        </div>
    {/each}
</div>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .grades {
        display: flex;
        gap: 8px;
    }

    .cell {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 3px;
    }

    .interval {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
    }

    .grade {
        position: relative;
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        padding: 9px 0;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        color: stat.$ink;
        font-family: stat.$font-body;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;

        &:hover {
            border-color: stat.$ink-soft;
        }
        &:focus-visible {
            outline: 2px solid stat.$primary;
            outline-offset: 2px;
        }

        // Again reads as a miss; the rest stay neutral, as in stock Anki.
        &.critical {
            border-color: stat.$critical;
            color: stat.$critical;

            &:hover {
                border-color: stat.$critical;
                background: stat.$critical-wash;
            }
        }

        // Good is Anki's default answer for a review card — flat teal border
        // (no shadow/gradient); paired with a "default" caption so color is
        // never the sole signal.
        &.default {
            border-color: stat.$primary;
            color: stat.$primary;
        }
    }

    .default-note {
        @include stat.readout;
        font-size: 9px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: stat.$primary;
    }

    .key {
        @include stat.readout;
        font-size: 10px;
        color: stat.$ink-soft;
        border: 1px solid stat.$line;
        border-radius: 3px;
        padding: 0 4px;
        line-height: 1.4;
    }

    // Mobile: big thumb-reach targets along the bottom of the card.
    @media (max-width: stat.$bp-compact) {
        .grades {
            gap: 6px;
        }
        .grade {
            flex-direction: column;
            gap: 2px;
            padding: 14px 0;
            font-size: 15px;
        }
        .interval {
            font-size: 11px;
        }
        .key {
            display: none;
        }
    }
</style>
