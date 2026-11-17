<script lang="ts">
  import '../app.css';
  import { untrack } from 'svelte';
  import { navigating, page } from '$app/state';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import AppHeader from '$lib/shell/AppHeader.svelte';
  import Toasts from '$lib/tool/Toasts.svelte';
  import { loadMe } from '$lib/tool/session.svelte';

  let { children } = $props();
  // One shell for every route: AppHeader, main, Footer. Pages (and the signed-in group's layout)
  // only fill main, with RailLayout or ColumnLayout. A page's load may return `footer: 'slim'`
  // (the docs) to end the page without the gap above the footer.

  // Only the browser has the session cookie; signed out is the default. The signed-in layout
  // asks too (to send you to sign in), and loadMe() shares one request between the two.
  $effect(() => {
    loadMe().catch(() => {});
  });

  // A thin bar under the nav while a page loads: it shows only after a short delay, crawls to
  // 85%, then fills and fades when the page arrives.
  let bar = $state<'idle' | 'run' | 'done'>('idle');
  $effect(() => {
    const going = !!navigating.to;
    const now = untrack(() => bar);
    if (going) {
      if (now !== 'done') {
        bar = 'run';
        return;
      }
      // Still fading from the last load: start again from empty, not backwards from full.
      bar = 'idle';
      const f = requestAnimationFrame(() => requestAnimationFrame(() => (bar = 'run')));
      return () => cancelAnimationFrame(f);
    }
    if (now !== 'run') return;
    bar = 'done';
    const t = setTimeout(() => (bar = 'idle'), 400);
    return () => clearTimeout(t);
  });
</script>

<a class="skip" href="#main">Skip to content</a>
<AppHeader />
<div class="progress {bar}" aria-hidden="true"></div>
<main id="main" tabindex="-1">
  {@render children()}
</main>
<Footer slim={page.data.footer === 'slim'} />
<Toasts />
<CommandPalette />

<style>
  main:focus { outline: none; }
  .progress { position: fixed; top: calc(var(--nav-h) + env(safe-area-inset-top, 0px)); left: 0; right: 0; height: 2px;
    z-index: var(--z-nav); pointer-events: none; background: var(--accent); transform-origin: 0 50%;
    opacity: 0; transform: scaleX(0); }
  .progress.run { opacity: 1; transform: scaleX(.85);
    transition: opacity var(--dur-color) ease-out var(--delay-progress), transform var(--dur-progress) var(--ease) var(--delay-progress); }
  .progress.done { opacity: 0; transform: scaleX(1);
    transition: transform var(--dur-color) ease-out, opacity var(--dur-color) ease-out var(--dur-color); }
  /* Reduced motion: no crawl, the bar only fades in and out. */
  @media (prefers-reduced-motion: reduce) { .progress, .progress.run, .progress.done { transform: none; } }
  @media (forced-colors: active) { .progress { background: Highlight; } }
</style>
