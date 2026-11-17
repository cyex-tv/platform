<script lang="ts" module>
  export type NavUser = { displayName: string; avatar?: string | null };
</script>

<script lang="ts">
  // The site's top bar: sticky, 64px, transparent at the top and glass once scrolled (solid
  // under reduced transparency or more contrast). Brand, Emotes and Developers, a search field
  // that opens the command palette, the theme toggle, Sign in or the account menu, and the one
  // accent button. 720px and narrower: brand, a search button and a menu sheet.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { signOut } from '$lib/api';
  import BottomSheet from './BottomSheet.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Mark from './Mark.svelte';
  import Popover from './Popover.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import { openPalette, shortcutHint } from './palette.svelte';

  let { me = null, onsignout }: { me?: NavUser | null; onsignout?: () => void } = $props();

  let scrolled = $state(false);
  let menu = $state(false);
  let account = $state(false);
  let hint = $state('Ctrl K');
  let broken = $state(false);

  onMount(() => {
    hint = shortcutHint();
    const onScroll = () => (scrolled = window.scrollY > 8);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });

  const links = [
    { href: '/search', label: 'Emotes', match: (p: string) => p.startsWith('/search') || p.startsWith('/emotes') },
    { href: '/developers', label: 'Developers', match: (p: string) => p.startsWith('/developers') },
  ];
  const mine = [
    { href: '/me', label: 'Your emotes' },
    { href: '/me/sets', label: 'Your sets' },
    { href: '/me/channels', label: 'Your channels' },
    { href: '/me/account', label: 'Account' },
  ];
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);

  async function leave() {
    account = false;
    menu = false;
    try {
      await signOut();
    } catch {}
    onsignout?.();
    await goto('/', { invalidateAll: true });
  }

  function search() {
    menu = false;
    openPalette();
  }
</script>

<header class="nav" class:scrolled>
  <a class="brand" href="/" aria-label="cyex.tv home">
    <Mark />
    <span>cyex.tv</span>
  </a>

  <nav class="links" aria-label="Main">
    {#each links as l}
      <a href={l.href} aria-current={l.match(page.url.pathname) ? 'page' : undefined}>{l.label}</a>
    {/each}
  </nav>

  <button class="search" type="button" onclick={search} aria-label="Search emotes, channels and pages ({hint})" aria-haspopup="dialog">
    <Icon name="search" />
    <span class="ph">Search emotes</span>
    <kbd>{hint}</kbd>
  </button>

  <div class="end">
    <ThemeToggle />
    {#if me}
      <Popover label="Your account" align="end" width="220px" bind:open={account}>
        {#snippet trigger(props)}
          <button class="who" type="button" {...props}>
            {#if me.avatar && !broken}
              <img src={me.avatar} alt="" width="24" height="24" onerror={() => (broken = true)} />
            {:else}
              <span class="blank" aria-hidden="true">{[...me.displayName][0]?.toUpperCase() ?? '?'}</span>
            {/if}
            <span class="who-name">{me.displayName}</span>
            <Icon name="chevron-down" size={14} />
          </button>
        {/snippet}
        <div class="menu-list">
          {#each mine as m}
            <a href={m.href} onclick={() => (account = false)}>{m.label}</a>
          {/each}
          <hr />
          <button type="button" onclick={leave}><Icon name="log-out" />Sign out</button>
        </div>
      </Popover>
    {:else}
      <a class="btn btn-s" href={signin}>Sign in</a>
    {/if}
    <a class="btn btn-primary btn-s" href="/#install">Get the extension</a>
  </div>

  <div class="phone">
    <IconButton icon="search" label="Search" tip="none" onclick={search} />
    <IconButton icon="menu" label="Menu" tip="none" aria-expanded={menu} aria-haspopup="dialog" onclick={() => (menu = true)} />
  </div>
</header>

<BottomSheet bind:open={menu} label="Menu" title={false}>
  <nav class="sheet-links" aria-label="Main">
    {#each links as l}
      <a href={l.href} aria-current={l.match(page.url.pathname) ? 'page' : undefined} onclick={() => (menu = false)}>{l.label}</a>
    {/each}
    {#if me}
      <p class="sheet-who">Signed in as {me.displayName}</p>
      {#each mine as m}
        <a href={m.href} onclick={() => (menu = false)}>{m.label}</a>
      {/each}
      <button type="button" onclick={leave}>Sign out</button>
    {/if}
  </nav>
  <div class="sheet-row">
    <span>Theme</span>
    <ThemeToggle />
  </div>
  {#snippet footer()}
    {#if !me}<a class="btn" href={signin} onclick={() => (menu = false)}>Sign in</a>{/if}
    <a class="btn btn-primary" href="/#install" onclick={() => (menu = false)}>Get the extension</a>
  {/snippet}
</BottomSheet>

<style>
  .nav { position: sticky; top: 0; z-index: var(--z-nav); flex: none; display: flex; align-items: center; gap: 12px;
    height: calc(var(--nav-h) + env(safe-area-inset-top, 0px)); padding-top: env(safe-area-inset-top, 0px);
    padding-inline: max(var(--page-pad), (100% - var(--page-max)) / 2 + var(--page-pad), env(safe-area-inset-left, 0px));
    transition: background-color var(--dur-move); }
  .nav.scrolled { background: color-mix(in srgb, var(--bg) 78%, transparent);
    -webkit-backdrop-filter: blur(14px) saturate(1.4); backdrop-filter: blur(14px) saturate(1.4); }
  .nav::after { content: ''; position: absolute; inset: 100% 0 auto; height: 18px; pointer-events: none;
    background: linear-gradient(color-mix(in srgb, var(--bg) 55%, transparent), transparent);
    opacity: 0; transition: opacity var(--dur-move); }
  .nav.scrolled::after { opacity: 1; }

  .brand { flex: none; display: flex; align-items: center; gap: 9px; margin-right: 10px;
    font-size: 17px; font-weight: 600; letter-spacing: -.01em; }
  .links { display: flex; gap: 2px; }
  .links a { padding: 8px 10px; font-size: var(--t-nav); color: var(--muted); border-radius: var(--radius-btn);
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .links a[aria-current='page'] { color: var(--text); }
  .links a:active { transform: scale(var(--press)); }

  .search { flex: 0 1 300px; min-width: 0; margin-left: auto; display: flex; align-items: center; gap: 10px; height: 40px; padding: 0 6px 0 12px;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface); color: var(--muted);
    font: inherit; font-size: var(--t-nav); cursor: text; text-align: left;
    transition: border-color var(--dur-color), transform var(--dur-press) ease-out; }
  .search:active { transform: scale(.99); }
  .ph { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  kbd { flex: none; display: inline-grid; place-items: center; height: 24px; padding: 0 7px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); background: var(--surface-2); font-size: 12px; font-weight: 500; line-height: 1; color: var(--muted); }

  .end { flex: none; display: flex; align-items: center; gap: 10px; }
  .end .btn-s { height: 40px; }
  .who { display: flex; align-items: center; gap: 8px; height: 40px; max-width: 200px; padding: 0 10px 0 7px;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface); color: var(--text);
    font: inherit; font-size: var(--t-nav); font-weight: 500; cursor: pointer;
    transition: background-color var(--dur-color), border-color var(--dur-color), transform var(--dur-press) ease-out; }
  .who:active { transform: scale(var(--press)); }
  .who img, .blank { flex: none; width: 24px; height: 24px; border-radius: 50%; object-fit: cover; }
  .blank { display: inline-grid; place-items: center; background: var(--surface-2); color: var(--muted); font-size: 11px; font-weight: 600; }
  .who-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who :global(svg) { color: var(--muted); }

  .menu-list { display: grid; }
  .menu-list a, .menu-list button { display: flex; align-items: center; gap: 10px; min-height: 38px; padding: 0 10px;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-ui);
    text-align: left; cursor: pointer; transition: background-color var(--dur-color); }
  .menu-list :global(svg) { color: var(--muted); }
  .menu-list hr { width: 100%; margin: 6px 0; border: 0; border-top: 1px solid var(--border); }

  .phone { display: none; margin-left: auto; gap: 4px; }

  .sheet-links { display: grid; }
  .sheet-links a, .sheet-links button { display: flex; align-items: center; min-height: 48px; padding: 0; border: 0;
    border-bottom: 1px solid var(--border); background: none; color: var(--text); font: inherit; font-size: 17px; text-align: left; cursor: pointer; }
  .sheet-links a[aria-current='page'] { font-weight: 600; }
  .sheet-who { margin: 20px 0 4px; font-size: var(--t-small); color: var(--muted); }
  .sheet-row { display: flex; align-items: center; justify-content: space-between; min-height: 56px; font-size: 17px; }

  @media (hover: hover) and (pointer: fine) {
    .links a:hover { color: var(--text); }
    .search:hover { border-color: var(--muted); }
    .who:hover { background: var(--surface-2); }
    .menu-list a:hover, .menu-list button:hover { background: var(--surface-2); }
  }
  @media (max-width: 1100px) { .search kbd { display: none; } .search { flex-basis: 220px; } }
  @media (max-width: 1000px) { .search .ph { display: none; } .search { flex: 0 0 40px; padding: 0; justify-content: center; } .who { max-width: 140px; } }
  @media (max-width: 720px) {
    .links, .search, .end { display: none; }
    .phone { display: flex; }
    .brand { margin-right: 0; }
  }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .nav.scrolled { background: var(--bg); -webkit-backdrop-filter: none; backdrop-filter: none; border-bottom: 1px solid var(--border-strong); }
    .nav::after { display: none; }
  }
</style>
