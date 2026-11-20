<script lang="ts">
  import '../app.css';
  import { untrack } from 'svelte';
  import { navigating, page } from '$app/state';
  import Footer from '$lib/components/Footer.svelte';
  import AppHeader from '$lib/shell/AppHeader.svelte';
  import { titleTruncated } from '$lib/shell/truncated';
  import Toasts from '$lib/tool/Toasts.svelte';
  import { syncMotion } from '$lib/shell/motion.svelte';
  import { watchDpr } from '$lib/dpr.svelte';
  import { loadMe, session } from '$lib/tool/session.svelte';
  import UploadDialog from '$lib/upload/UploadDialog.svelte';
  import { openUpload, opensDialog } from '$lib/upload/upload.svelte';

  let { children } = $props();
  // One shell for every route: AppHeader, main, Footer. Pages (and the signed-in group's layout)
  // only fill main, with RailLayout or ColumnLayout. A page's load may return `footer: 'slim'`
  // (the docs) to end the page without the gap above the footer.

  // Only the browser has the session cookie; signed out is the default. The signed-in layout
  // asks too (to send you to sign in), and loadMe() shares one request between the two.
  // The reduced-motion setting, for the pictures that choose a still by it.
  $effect(syncMotion);
  // The screen's pixel ratio, for the pictures that pick their file in script (no srcset).
  $effect(watchDpr);
  $effect(() => {
    loadMe().catch(() => {});
  });

  // Signed in, every link to the upload page (the header's Upload, an empty "Your emotes", help
  // answers) opens the upload dialog over the current page instead. Ctrl/⌘-click and middle
  // click still open the page in a new tab; signed out, the link goes to sign in as before.
  function uploadLinks(e: MouseEvent) {
    if (!session.me) return;
    const link = (e.target as Element | null)?.closest?.('a[href]');
    if (!(link instanceof HTMLAnchorElement)) return;
    if (!opensDialog(e, link, new URL(location.href))) return;
    e.preventDefault();
    openUpload();
  }
  // Capture, so this runs before the router's own click handler (which skips prevented clicks).
  $effect(() => {
    document.addEventListener('click', uploadLinks, true);
    return () => document.removeEventListener('click', uploadLinks, true);
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

<svelte:document onpointerover={titleTruncated} />

<a class="skip" href="#main">Skip to content</a>
{#if page.data.skipTo}<a class="skip" href="#{page.data.skipTo}">Skip to contents</a>{/if}
<AppHeader />
<div class="progress {bar}" aria-hidden="true"></div>
<main id="main" tabindex="-1">
  {@render children()}
</main>
<Footer slim={page.data.footer === 'slim'} />
<Toasts />
{#if session.me}<UploadDialog />{/if}

<style>
  main:focus { outline: none; }
  /* While something on the page is still loading (a loading state, a grid fetching), the footer waits, hidden
     but in place: a page growing under a visible footer would shove it down. */
  :global(main:has([aria-busy='true']) + .footer) { visibility: hidden; }
  .progress { position: fixed; top: calc(var(--nav-h) + 1px + env(safe-area-inset-top, 0px)); left: 0; right: 0; height: 2px;
    z-index: var(--z-nav); pointer-events: none; background: var(--accent); transform-origin: 0 50%;
    opacity: 0; transform: scaleX(0); }
  .progress.run { opacity: 1; transform: scaleX(.85);
    transition: opacity var(--dur-color) ease-out var(--delay-progress), transform var(--dur-progress) linear var(--delay-progress); }
  .progress.done { opacity: 0; transform: scaleX(1);
    transition: transform var(--dur-color) ease-out, opacity var(--dur-color) ease-out var(--dur-color); }
  /* Reduced motion: no crawl and no static full-width bar; the bar stays hidden. */
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .progress { display: none; } }
  :global(html[data-motion='reduce']) .progress { display: none; }
  @media (forced-colors: active) { .progress { background: Highlight; } }
</style>
