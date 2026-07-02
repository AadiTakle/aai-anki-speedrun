// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

// The STAT destinations — the daily loop (Today · Review · Import). "Review"
// launches Anki's real reviewer (actual cards). The Errors and Trajectory
// screens are temporarily unlinked (their routes still exist): both showed
// illustrative data — Errors needs per-question QBank data, and Trajectory needs
// a readiness/coverage time-series we don't record yet — so neither can be real.
//
// Each `href` must have a matching entry in `is_sveltekit_page()` in
// anki/qt/aqt/mediasrv.py for a fresh load of the route to be served the SPA
// fallback (the allowlist is centralized so screen lanes never touch it).

import type { Destination, DestinationId } from "./types";

export const STAT_DESTINATIONS: readonly Destination[] = [
    { id: "today", label: "Today", href: "/today", caption: "plan" },
    { id: "reviewer", label: "Review", href: "/reviewer", caption: "retrieve" },
    { id: "import", label: "Import", href: "/import", caption: "organize" },
] as const;

export function destinationById(id: DestinationId): Destination {
    // Non-null: DestinationId is exhaustively covered by STAT_DESTINATIONS.
    return STAT_DESTINATIONS.find((d) => d.id === id)!;
}
