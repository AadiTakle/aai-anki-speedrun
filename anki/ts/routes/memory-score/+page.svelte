<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
-->
<script lang="ts">
    import { getMemoryScore } from "@generated/backend";
    import { onMount } from "svelte";

    import type { MemoryScoreLike } from "./lib";
    import MemoryScore from "./MemoryScore.svelte";

    let score: MemoryScoreLike | null = null;
    let error: string | null = null;

    onMount(async () => {
        try {
            // The frozen F6 RPC takes no arguments (generic.Empty).
            score = await getMemoryScore({});
        } catch (e) {
            error = String(e);
        }
    });
</script>

<div class="page">
    {#if error}
        <div class="error">Couldn't load the memory score: {error}</div>
    {:else if score}
        <MemoryScore {score} />
    {:else}
        <div class="loading">Loading…</div>
    {/if}
</div>

<style lang="scss">
    .page {
        padding: 1em;
        color: var(--fg);
        background: var(--canvas);
        min-height: 100vh;
    }

    .error {
        color: var(--fg-red, red);
    }

    .loading {
        opacity: 0.7;
    }
</style>
