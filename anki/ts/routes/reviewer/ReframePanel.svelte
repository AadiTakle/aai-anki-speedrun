<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

ReframePanel — the miss REFRAME / differential, surfaced on the answer side (a
STAT-only addition; stock Anki has no error log). It forces a differential from
the learner's actual wrong answer — "how would the vignette need to change for
[your wrong answer] to be right?" — and shows their one-line takeaway. Feedback
lives on the same screen as the answer, when reasoning is freshest.
-->
<script lang="ts">
    import { reframePrompt, type ReviewCard } from "./cards-mock";

    export let card: ReviewCard;

    $: prompt = reframePrompt(card);
</script>

<aside class="reframe" aria-label="Error-log reframe">
    <div class="head">
        <span class="tag">Your error-log reframe</span>
        <span class="chose">
            you chose <span class="wrong">"{card.reframe.wrongAnswer}"</span>
        </span>
    </div>

    <p class="prompt">{prompt}</p>
    <p class="differential">{card.reframe.differential}</p>

    <div class="takeaway">
        <span class="takeaway-label">Your one-line takeaway</span>
        <p class="takeaway-body">"{card.reframe.takeaway}"</p>
    </div>
</aside>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .reframe {
        border-left: 3px solid stat.$critical;
        background: stat.$critical-wash;
        border-radius: stat.$radius-sm;
        padding: 12px 14px;
    }

    .head {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 6px 10px;
    }

    .tag {
        @include stat.readout;
        font-size: 10px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: stat.$critical;
    }

    .chose {
        @include stat.readout;
        font-size: 11px;
        color: stat.$ink-soft;
    }

    .wrong {
        color: stat.$critical;
        text-decoration: line-through;
    }

    .prompt {
        margin: 8px 0 0;
        font-family: stat.$font-body;
        font-size: 14px;
        font-weight: 600;
        line-height: 1.45;
        color: stat.$ink;
    }

    .differential {
        margin: 6px 0 0;
        font-family: stat.$font-body;
        font-size: 13px;
        line-height: 1.5;
        color: stat.$ink-soft;
    }

    .takeaway {
        margin-top: 10px;
        border: 1px solid stat.$line;
        border-radius: stat.$radius-sm;
        background: stat.$surface;
        padding: 8px 12px;
    }

    .takeaway-label {
        @include stat.readout;
        font-size: 9px;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: stat.$ink-soft;
    }

    .takeaway-body {
        margin: 3px 0 0;
        font-family: stat.$font-body;
        font-style: italic;
        font-size: 13px;
        line-height: 1.45;
        color: stat.$ink;
    }
</style>
