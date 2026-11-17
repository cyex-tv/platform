<script lang="ts">
  import '../../tool.css';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError } from '$lib/api';
  import Mark from '$lib/components/Mark.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import Toasts from '$lib/tool/Toasts.svelte';
  import UserMenu from '$lib/tool/UserMenu.svelte';
  import { ADMIN_PERMISSIONS, loadMe, session } from '$lib/tool/session.svelte';

  let { children } = $props();
  let problem = $state<string | null>(null);

  const admin = $derived(session.me?.permissions.some((p) => ADMIN_PERMISSIONS.includes(p)) ?? false);
  const links = $derived([
    { href: '/me', label: 'Emotes', match: (p: string) => p === '/me' || p.startsWith('/me/emotes') || p === '/me/upload' },
    { href: '/me/sets', label: 'Sets', match: (p: string) => p.startsWith('/me/sets') },
    { href: '/me/channels', label: 'Channels', match: (p: string) => p.startsWith('/me/channels') },
    { href: '/me/account', label: 'Account', match: (p: string) => p.startsWith('/me/account') },
    ...(admin ? [{ href: '/admin', label: 'Admin', match: (p: string) => p.startsWith('/admin') }] : []),
  ]);

  onMount(async () => {
    try {
      await loadMe();
    } catch (e) {
      if (e instanceof ApiError && e.code === 'signed_out') {
        await goto(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`, { replaceState: true });
        return;
      }
      problem = e instanceof Error ? e.message : 'cyex.tv can’t be reached right now.';
    }
  });
</script>

<svelte:head>
  <link rel="preconnect" href="https://fonts.googleapis.com" />
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600&family=IBM+Plex+Sans:wght@400;500;600;700&display=swap" />
</svelte:head>

<div class="tool">
  <a class="skip" href="#main">Skip to content</a>
  <header class="bar">
    <div class="wrap inner">
      <a class="brand" href="/" aria-label="cyex.tv front page"><Mark /><span>cyex.tv</span></a>
      <nav aria-label="Your account">
        {#each links as l}
          <a href={l.href} aria-current={l.match(page.url.pathname) ? 'page' : undefined}>{l.label}</a>
        {/each}
      </nav>
      <div class="end">
        <ThemeToggle />
        {#if session.me}<UserMenu />{/if}
      </div>
    </div>
  </header>
  <main id="main" tabindex="-1" class="wrap">
    {#if problem}
      <div class="ui-empty" role="alert">
        <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
        <p>{problem}</p>
        <button class="ui-btn secondary" type="button" onclick={() => location.reload()}>Try again</button>
      </div>
    {:else if !session.me}
      <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span><p>Loading…</p></div>
    {:else}
      {@render children()}
    {/if}
  </main>
  <Toasts />
</div>

<style>
  .bar { position: sticky; top: 0; z-index: var(--z-panel); background: var(--bg); border-bottom: 1px solid var(--border); }
  .inner { display: flex; align-items: center; gap: 24px; padding-block: 10px; min-height: 56px; }
  .brand { display: flex; align-items: center; gap: 9px; color: var(--text); text-decoration: none; font-weight: 700; font-size: .95rem; flex: none; }
  nav { display: flex; gap: 18px; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  nav a { position: relative; padding: 8px 0; color: var(--muted); text-decoration: none; white-space: nowrap;
    font-size: .74rem; font-weight: 700; letter-spacing: .06em; text-transform: uppercase; transition: color var(--fast); }
  @media (hover: hover) and (pointer: fine) { nav a:hover { color: var(--text); } }
  nav a::after { content: ''; position: absolute; inset: auto 0 2px; height: 2px; background: var(--accent);
    scale: 0 1; transition: scale var(--base) var(--ease); }
  nav a[aria-current='page'] { color: var(--text); }
  nav a[aria-current='page']::after { scale: 1 1; }
  nav a:active { opacity: .6; transition: none; }
  .end { display: flex; align-items: center; gap: 8px; margin-left: auto; flex: none; }
  .end :global(.theme) { width: 32px; height: 32px; border-radius: var(--radius); border: 1px solid var(--border-strong); transition: background-color var(--fast), color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .end :global(.theme:hover) { background: var(--surface-hover); border-color: var(--border-strong); } }
  .end :global(.theme:active) { transform: none; opacity: .6; transition: none; }
  .skip { position: fixed; left: 12px; top: -60px; z-index: var(--z-toast); padding: 8px 12px; background: var(--text); color: var(--bg); }
  .skip:focus { top: 12px; }
  main:focus { outline: none; }
  @media (prefers-reduced-motion: reduce) { nav a::after { transition: none; } }
  @media (max-width: 640px) {
    .inner { flex-wrap: wrap; gap: 4px 16px; }
    nav { order: 3; flex-basis: 100%; }
  }
</style>
