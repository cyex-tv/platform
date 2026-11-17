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
  // The signed-in pages (`(tool)`) bring their own header in the cyex-app style, and a page whose
  // load returns `chrome: false` (the docs at /developers) brings its own header and footer.
  const tool = $derived(page.route.id?.startsWith('/(tool)') || page.data.chrome === false);

  // Only the browser has the session cookie; signed out is the default. The tool layout loads
  // the account itself, and loadMe() shares one request between the two.
  $effect(() => {
    if (!tool) loadMe().catch(() => {});
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

{#if tool}
  {@render children()}
{:else}
  <a class="skip" href="#main">Skip to content</a>
  <AppHeader />
  <div class="progress {bar}" aria-hidden="true"></div>
  <main id="main" tabindex="-1">
    {@render children()}
  </main>
  <Footer slim={page.data.footer === 'slim'} />
  <Toasts />
{/if}
<CommandPalette />

<style>
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
