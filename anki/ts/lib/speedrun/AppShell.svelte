<!--
Copyright: Ankitects Pty Ltd and contributors
License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

AppShell — the responsive, mobile-first frame every STAT screen renders inside.
Desktop: a sticky top bar (STAT wordmark + horizontal nav + trust badges).
Mobile (< 768px): the nav collapses to a thumb-reach bottom tab bar. The shell
sets the STAT design tokens (`--stat-*`) and the clinical light surface on its
subtree, so slotted screen content inherits the typography + palette and can
theme off `var(--stat-*)`.

Slots:
  - default: the screen's content.
  - badges:  extra trust badges for the top bar (device, SYNCED/OFFLINE). The
             always-on "AI OFF" badge is rendered for you.
Props:
  - active:  the current DestinationId (highlights its nav entry).
  - fluid:   full-bleed content instead of the centered 1120px console column.
  - aiOff:   show the always-on "AI OFF" badge (default true; MVP runs AI off).
-->
<script lang="ts">
    import Chip from "./Chip.svelte";
    import { STAT_DESTINATIONS } from "./nav";
    import type { DestinationId } from "./types";

    export let active: DestinationId | null = null;
    export let fluid = false;
    export let aiOff = true;
</script>

<div class="stat-shell">
    <header class="topbar">
        <span class="wordmark">STAT</span>
        <span class="rule" aria-hidden="true"></span>
        <nav class="desktop-nav" aria-label="STAT sections">
            {#each STAT_DESTINATIONS as d (d.id)}
                <a
                    class="nav-link"
                    class:current={active === d.id}
                    href={d.href}
                    aria-current={active === d.id ? "page" : undefined}
                >
                    {d.label}
                </a>
            {/each}
        </nav>
        <span class="badges">
            <slot name="badges" />
            {#if aiOff}
                <Chip>AI OFF</Chip>
            {/if}
        </span>
    </header>

    <main class="content" class:fluid>
        <slot />
    </main>

    <nav class="tabbar" aria-label="STAT sections">
        {#each STAT_DESTINATIONS as d (d.id)}
            <a
                class="tab"
                class:current={active === d.id}
                href={d.href}
                aria-current={active === d.id ? "page" : undefined}
            >
                {d.label}
            </a>
        {/each}
    </nav>
</div>

<style lang="scss">
    @use "./tokens" as stat;

    .stat-shell {
        @include stat.root;

        display: flex;
        flex-direction: column;
        min-height: 100vh;
        min-height: 100dvh;
        font-family: stat.$font-body;
        color: stat.$ink;
        background: stat.$paper;
    }

    .topbar {
        position: sticky;
        top: 0;
        z-index: 2;
        display: flex;
        align-items: center;
        gap: 10px;
        padding: 10px 16px;
        border-bottom: 1px solid stat.$line;
        background: stat.$surface;
    }

    .wordmark {
        font-family: stat.$font-display;
        font-size: 16px;
        font-weight: 800;
        letter-spacing: 0.16em;
        color: stat.$ink;
    }

    .rule {
        width: 1px;
        height: 14px;
        background: stat.$line;
    }

    .desktop-nav {
        display: none; // mobile-first: hidden until the compact breakpoint
        align-items: center;
        gap: 2px;
    }

    .nav-link {
        padding: 3px 8px;
        border-radius: stat.$radius-sm;
        font-family: stat.$font-mono;
        font-size: 11px;
        letter-spacing: 0.06em;
        color: stat.$ink-soft;
        text-decoration: none;

        &:hover {
            color: stat.$ink;
        }
        &.current {
            color: stat.$surface;
            background: stat.$primary;
        }
    }

    .badges {
        margin-inline-start: auto;
        display: flex;
        align-items: center;
        gap: 8px;
    }

    .content {
        flex: 1;
        padding: 16px;

        &:not(.fluid) {
            width: 100%;
            max-width: 1120px;
            margin-inline: auto;
        }
    }

    // Thumb-reach bottom tab bar — the mobile nav.
    .tabbar {
        position: sticky;
        bottom: 0;
        z-index: 2;
        display: flex;
        border-top: 1px solid stat.$line;
        background: stat.$surface;
    }

    .tab {
        flex: 1;
        padding: 10px 4px;
        border-top: 2px solid transparent;
        text-align: center;
        font-family: stat.$font-mono;
        font-size: 10px;
        letter-spacing: 0.04em;
        color: stat.$ink-soft;
        text-decoration: none;

        &.current {
            color: stat.$primary;
            border-top-color: stat.$primary;
        }
    }

    // Visible keyboard focus — a teal ring on every destination.
    .nav-link:focus-visible,
    .tab:focus-visible {
        outline: 2px solid stat.$primary;
        outline-offset: -2px;
    }

    @media (min-width: stat.$bp-compact) {
        .desktop-nav {
            display: flex;
        }
        .tabbar {
            display: none;
        }
    }
</style>
