<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

STAT "Today" console (screen lane C1) — the loader. Pulls every score/signal the
console needs through the shared `$lib/speedrun` adapter in onMount, then hands
the non-null Views to <TodayConsole/>. The mock adapter selects a persona via
`?persona=US-1..US-4` (US-2 is the honest-abstain state); default US-1.

Built on the read-only shared foundation — nothing under lib/speedrun is edited.
-->
<script lang="ts">
    import { onMount } from "svelte";

    import {
        activePersona,
        AppShell,
        Chip,
        getCoverageMap,
        getMemoryScore,
        getNextAction,
        getPerformanceScore,
        getPointsAtStake,
        getReadinessScore,
    } from "$lib/speedrun";
    import type {
        CoverageMapView,
        MemoryScoreView,
        NextActionView,
        PerformanceScoreView,
        PointsAtStakeView,
        ReadinessScoreView,
    } from "$lib/speedrun";

    import TodayConsole from "./TodayConsole.svelte";

    let error: string | null = null;

    let readiness: ReadinessScoreView | null = null;
    let memory: MemoryScoreView | null = null;
    let performance: PerformanceScoreView | null = null;
    let stakes: PointsAtStakeView | null = null;
    let nextAction: NextActionView | null = null;
    let coverage: CoverageMapView | null = null;

    // Readiness TARGET + abstain unlock rule are console config, not carried on
    // ScoreView; read from the seeded persona (exported from the barrel) so they
    // stay consistent with getReadinessScore()'s active persona.
    let target: number | undefined;
    let unlock: string | undefined;

    onMount(async () => {
        try {
            const persona = activePersona();
            target = persona.readinessTarget;
            unlock = persona.readinessUnlock;

            [readiness, memory, performance, stakes, nextAction, coverage] =
                await Promise.all([
                    getReadinessScore(),
                    getMemoryScore(),
                    getPerformanceScore(),
                    getPointsAtStake(),
                    getNextAction(),
                    getCoverageMap(),
                ]);
        } catch (e) {
            error = String(e);
        }
    });
</script>

<AppShell active="today">
    <Chip slot="badges" dot="stable">SYNCED</Chip>

    {#if error}
        <div class="state error" role="alert">Couldn't load Today: {error}</div>
    {:else if !readiness || !memory || !performance || !stakes || !nextAction || !coverage}
        <div class="state loading">Acquiring signal…</div>
    {:else}
        <TodayConsole
            {readiness}
            {memory}
            {performance}
            {stakes}
            {nextAction}
            {coverage}
            {target}
            {unlock}
        />
    {/if}
</AppShell>

<style lang="scss">
    @use "$lib/speedrun/tokens" as stat;

    .state {
        padding: 24px 4px;
        font-family: stat.$font-mono;
        font-size: 13px;
        color: stat.$ink-soft;
    }
    .state.error {
        color: stat.$critical;
    }
</style>
