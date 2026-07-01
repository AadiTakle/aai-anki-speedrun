<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

ConfidenceChip — the readiness confidence badge ("Low confidence",
"Medium · pacing-adjusted", "Not enough info"…). A thin semantic wrapper over
Chip: pass `muted` for the abstain state so the dot reads as insufficient data.
-->
<script lang="ts">
    import Chip from "./Chip.svelte";
    import type { Acuity } from "./types";

    export let label: string;
    /** Convenience for the abstain state: muted dot instead of the watch dot. */
    export let muted = false;
    /** Override the dot acuity directly if needed. */
    export let acuity: Acuity = "watch";

    $: dot = muted ? "muted" : acuity;
</script>

<Chip {dot}>{label}</Chip>
