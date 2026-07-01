// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// The five STAT destinations. They ARE the daily loop (Today · Review · Import
// · Errors · Trajectory) — there is no deck browser / add-card sprawl / raw
// stats maze. AppShell renders these; screen lanes each own one `+page.svelte`.
//
// Each `href` must have a matching entry in `is_sveltekit_page()` in
// anki/qt/aqt/mediasrv.py for a fresh load of the route to be served the SPA
// fallback (the allowlist is centralized so screen lanes never touch it).

import type { Destination, DestinationId } from "./types";

export const STAT_DESTINATIONS: readonly Destination[] = [
    { id: "today", label: "Today", href: "/today", caption: "plan" },
    { id: "reviewer", label: "Review", href: "/reviewer", caption: "retrieve" },
    { id: "import", label: "Import", href: "/import", caption: "organize" },
    { id: "errors", label: "Errors", href: "/errors", caption: "reframe" },
    { id: "trajectory", label: "Trajectory", href: "/trajectory", caption: "recalibrate" },
] as const;

export function destinationById(id: DestinationId): Destination {
    // Non-null: DestinationId is exhaustively covered by STAT_DESTINATIONS.
    return STAT_DESTINATIONS.find((d) => d.id === id)!;
}
