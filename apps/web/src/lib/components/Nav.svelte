<script lang="ts" module>
  export type NavUser = { displayName: string; avatar?: string | null; platform?: string | null; admin?: boolean };
</script>

<script lang="ts">
  // The site's top bar: sticky, 64px, transparent at the top and glass once scrolled (solid
  // under reduced transparency or more contrast). Brand and Emotes on the left; on the right a
  // search field that opens the command palette (hidden on /search, whose own field is the
  // search there), the theme toggle and then either Sign in + Get the extension, or Upload +
  // the account menu. 720px and narrower: brand, a search button and a menu sheet.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { signOut } from '$lib/api';
  import { clean, iso } from '$lib/format';
  import { notify } from '$lib/tool/notices.svelte';
  import Avatar from './Avatar.svelte';
  import BottomSheet from './BottomSheet.svelte';
  import Icon, { type IconName } from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Mark from './Mark.svelte';
  import Popover from './Popover.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import { openPalette } from './palette.svelte';

  let { me = null, onsignout }: { me?: NavUser | null; onsignout?: () => void } = $props();

  let scrolled = $state(false);
  let menu = $state(false);
  let account = $state(false);

  onMount(() => {
    const onScroll = () => (scrolled = window.scrollY > 8);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });

  type Item = { href: string; label: string; icon: IconName };
  const browse = (p: string) => p.startsWith('/search') || p.startsWith('/emotes');
  const links = [{ href: '/search', label: 'Emotes', match: browse }];
  // The menu sheet spells the same places out, plus the docs (the bar has no room for them).
  const sheetLinks = [
    { href: '/search', label: 'Browse emotes', match: browse },
    { href: '/developers', label: 'Developers', match: (p: string) => p.startsWith('/developers') },
  ];
  // The signed-in destinations, grouped: what you make, then the account itself.
  const yours: Item[] = [
    { href: '/me', label: 'Your emotes', icon: 'smile' },
    { href: '/me/upload', label: 'Upload', icon: 'upload' },
    { href: '/me/sets', label: 'Sets', icon: 'layers' },
    { href: '/me/channels', label: 'Channels', icon: 'tv' },
  ];
  const settings = $derived<Item[]>([
    { href: '/me/account', label: 'Account', icon: 'user' },
    ...(me?.admin ? [{ href: '/admin', label: 'Admin', icon: 'shield' as const }] : []),
  ]);

  const onSearch = $derived(page.url.pathname.startsWith('/search'));
  const onSignin = $derived(page.url.pathname === '/signin');
  const name = $derived(clean(me?.displayName));
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);
  const platform = $derived(me?.platform ? `${me.platform.charAt(0)}${me.platform.slice(1).toLowerCase()}` : null);

  async function leave() {
    account = false;
    menu = false;
    try {
      await signOut();
    } catch {
      // Still signed in: stay on the page and say so.
      notify('Signing out didn’t work. Try again in a moment.', true);
      return;
    }
    onsignout?.();
    await goto('/', { invalidateAll: true });
  }

  function search() {
    menu = false;
    openPalette();
  }
</script>

{#snippet avatar(size: number)}
  <Avatar src={me?.avatar} {name} {size} />
{/snippet}

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

  <span class="grow"></span>

  {#if !onSearch}
    <button class="search" type="button" onclick={search} aria-label="Search emotes, channels and pages" aria-haspopup="dialog">
      <Icon name="search" />
      <span class="ph">Search emotes</span>
    </button>
  {/if}

  <div class="end">
    <ThemeToggle />
    {#if me}
      <a class="btn btn-primary btn-s upload" href="/me/upload"><Icon name="upload" />Upload</a>
      <Popover label="Your account" align="end" width="260px" bind:open={account}>
        {#snippet trigger(props)}
          <button class="who" type="button" {...props} aria-label="Your account: {iso(name)}">
            {@render avatar(26)}
            <span class="who-name"><bdi>{name}</bdi></span>
            <Icon name="chevron-down" size={14} />
          </button>
        {/snippet}
        <div class="menu-head">
          {@render avatar(36)}
          <span class="menu-id">
            <strong><bdi>{name}</bdi></strong>
            {#if platform}<span>Signed in with {platform}</span>{/if}
          </span>
        </div>
        <div class="menu-list">
          {#each yours as m}
            <a href={m.href} aria-current={page.url.pathname === m.href ? 'page' : undefined} onclick={() => (account = false)}><Icon name={m.icon} />{m.label}</a>
          {/each}
          <hr />
          {#each settings as m}
            <a href={m.href} aria-current={page.url.pathname === m.href ? 'page' : undefined} onclick={() => (account = false)}><Icon name={m.icon} />{m.label}</a>
          {/each}
          <a href="/#install" onclick={() => (account = false)}><Icon name="download" />Get the extension</a>
          <hr />
          <button type="button" onclick={leave}><Icon name="log-out" />Sign out</button>
        </div>
      </Popover>
    {:else}
      {#if !onSignin}<a class="btn btn-s" href={signin}>Sign in</a>{/if}
      <a class="btn btn-primary btn-s" href="/#install">Get the extension</a>
    {/if}
  </div>

  <div class="phone">
    {#if !onSearch}<IconButton icon="search" label="Search" tip="none" onclick={search} />{/if}
    <button class="menu-btn" type="button" aria-label="Menu" aria-expanded={menu} aria-haspopup="dialog" onclick={() => (menu = true)}>
      {#if me}{@render avatar(28)}{:else}<Icon name="menu" size={18} />{/if}
    </button>
  </div>
</header>

<BottomSheet bind:open={menu} label="Menu">
  {#if me}
    <div class="sheet-who">
      {@render avatar(40)}
      <span class="menu-id">
        <strong><bdi>{name}</bdi></strong>
        {#if platform}<span>Signed in with {platform}</span>{/if}
      </span>
    </div>
    <nav class="sheet-grid" aria-label="Yours">
      {#each yours as m}
        <a href={m.href} aria-current={page.url.pathname === m.href ? 'page' : undefined} onclick={() => (menu = false)}><Icon name={m.icon} size={20} />{m.label}</a>
      {/each}
    </nav>
  {/if}
  <nav class="sheet-links" aria-label="Main">
    {#each sheetLinks as l}
      <a href={l.href} aria-current={l.match(page.url.pathname) ? 'page' : undefined} onclick={() => (menu = false)}>{l.label}</a>
    {/each}
    {#if me}
      {#each settings as m}
        <a href={m.href} aria-current={page.url.pathname === m.href ? 'page' : undefined} onclick={() => (menu = false)}>{m.label}</a>
      {/each}
    {/if}
    <div class="sheet-row">
      <span>Theme</span>
      <ThemeToggle />
    </div>
    {#if me}<button class="out" type="button" onclick={leave}>Sign out</button>{/if}
  </nav>
  {#snippet footer()}
    {#if !me && !onSignin}<a class="btn" href={signin} onclick={() => (menu = false)}>Sign in</a>{/if}
    <a class="btn btn-primary" href="/#install" onclick={() => (menu = false)}>Get the extension</a>
  {/snippet}
</BottomSheet>

<style>
  .nav { position: sticky; top: 0; z-index: var(--z-nav); flex: none; display: flex; align-items: center; gap: 12px;
    height: calc(var(--nav-h) + env(safe-area-inset-top, 0px)); padding-top: env(safe-area-inset-top, 0px);
    padding-inline: max(var(--page-pad), (100% - var(--page-max)) / 2 + var(--page-pad)); isolation: isolate; }
  /* The glass is its own layer that fades in: opacity is cheap to animate, a blur and a tint
     aren't, and the bar itself never becomes a containing block for fixed children. */
  .nav::before { content: ''; position: absolute; inset: 0; z-index: -1; pointer-events: none;
    background: color-mix(in srgb, var(--bg) 78%, transparent);
    -webkit-backdrop-filter: blur(14px) saturate(1.4); backdrop-filter: blur(14px) saturate(1.4);
    opacity: 0; transition: opacity var(--dur-move); }
  .nav.scrolled::before { opacity: 1; }
  .nav::after { content: ''; position: absolute; inset: 100% 0 auto; height: 18px; pointer-events: none;
    background: linear-gradient(color-mix(in srgb, var(--bg) 55%, transparent), transparent);
    opacity: 0; transition: opacity var(--dur-move); }
  .nav.scrolled::after { opacity: 1; }

  .brand { flex: none; display: flex; align-items: center; min-height: 40px; gap: 9px; margin-right: 10px;
    font-size: 17px; font-weight: 600; letter-spacing: -.01em; }
  .links { display: flex; gap: 2px; }
  .links a { padding: 8px 10px; font-size: var(--t-nav); color: var(--muted); border-radius: var(--radius-btn);
    transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .links a[aria-current='page'] { color: var(--text); }
  .links a:active { transform: scale(var(--press)); }
  .grow { flex: 1; }

  /* One calm field: icon, placeholder, nothing else. */
  .search { flex: 0 1 260px; min-width: 0; display: flex; align-items: center; gap: 10px; height: 38px; padding: 0 12px;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: var(--surface); color: var(--muted);
    font: inherit; font-size: var(--t-nav); cursor: text; text-align: left;
    transition: border-color var(--dur-color), color var(--dur-color); }
  .ph { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .end { flex: none; display: flex; align-items: center; gap: 8px; }
  .end :global(.theme) { width: 38px; height: 38px; }
  .upload { gap: 8px; padding: 0 14px 0 12px; }
  .who { display: flex; align-items: center; gap: 8px; height: 38px; max-width: 200px; padding: 0 8px 0 5px;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: transparent; color: var(--text);
    font: inherit; font-size: var(--t-nav); font-weight: 500; cursor: pointer;
    transition: background-color var(--dur-color), border-color var(--dur-color), transform var(--dur-press) ease-out; }
  .who:active { transform: scale(var(--press)); }
  .who[aria-expanded='true'] { background: var(--surface-2); border-color: var(--border-strong); }
  .who-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who :global(svg) { color: var(--muted); }

  .menu-head { display: flex; align-items: center; gap: 12px; padding: 6px 10px 12px; margin-bottom: 4px; border-bottom: 1px solid var(--border); }
  .menu-id { display: grid; min-width: 0; line-height: 1.3; }
  .menu-id strong { font-size: var(--t-ui); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .menu-id span { font-size: var(--t-fine); color: var(--muted); }
  .menu-list { display: grid; }
  .menu-list a, .menu-list button { display: flex; align-items: center; gap: 12px; min-height: 38px; padding: 0 10px;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-ui);
    text-align: left; cursor: pointer; transition: background-color var(--dur-color); }
  .menu-list a[aria-current='page'] { background: var(--surface-2); }
  .menu-list :global(svg) { color: var(--muted); }
  .menu-list hr { width: 100%; margin: 4px 0; border: 0; border-top: 1px solid var(--border); }

  .phone { display: none; margin-left: auto; align-items: center; gap: 4px; }
  .menu-btn { display: inline-grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); cursor: pointer; transition: transform var(--dur-press) ease-out; }
  .menu-btn:active { transform: scale(var(--press)); }
  @media (pointer: coarse) { .menu-btn { width: 44px; height: 44px; } }

  .sheet-who { display: flex; align-items: center; gap: 12px; padding: 4px 0 16px; }
  .sheet-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; padding-bottom: 12px; }
  .sheet-grid a { display: flex; flex-direction: column; align-items: flex-start; justify-content: space-between; gap: 14px; min-height: 76px;
    padding: 12px 14px; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-2);
    font-size: var(--t-ui); font-weight: 500; transition: transform var(--dur-press) ease-out; }
  .sheet-grid a:active { transform: scale(var(--press)); }
  .sheet-grid a[aria-current='page'] { border-color: var(--border-strong); }
  .sheet-grid :global(svg) { color: var(--muted); }
  .sheet-links { display: grid; }
  .sheet-links a, .sheet-links button { display: flex; align-items: center; min-height: 50px; padding: 0; border: 0;
    border-bottom: 1px solid var(--border); background: none; color: var(--text); font: inherit; font-size: 17px; text-align: left; cursor: pointer; }
  .sheet-links a[aria-current='page'] { font-weight: 600; }
  .sheet-row { display: flex; align-items: center; justify-content: space-between; min-height: 56px; font-size: 17px; border-bottom: 1px solid var(--border); }
  .sheet-links .out { color: var(--muted); border-bottom: 0; }

  @media (hover: hover) and (pointer: fine) {
    .links a:hover { color: var(--text); }
    .search:hover { border-color: var(--border-strong); color: var(--text); }
    .who:hover { background: var(--surface-2); }
    .menu-list a:hover, .menu-list button:hover { background: var(--surface-2); }
    .menu-btn:hover { color: var(--text); }
  }
  @media (max-width: 1000px) {
    .search .ph { display: none; } .search { flex: 0 0 38px; padding: 0; justify-content: center; }
    .who-name { display: none; } .who { padding-right: 6px; }
  }
  @media (max-width: 720px) {
    .links, .search, .end, .grow { display: none; }
    .phone { display: flex; }
    .brand { margin-right: 0; }
  }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .nav::before { background: var(--bg); -webkit-backdrop-filter: none; backdrop-filter: none; border-bottom: 1px solid var(--border-strong); }
    .nav::after { display: none; }
  }
  @media (forced-colors: active) {
    .nav::before { background: Canvas; -webkit-backdrop-filter: none; backdrop-filter: none; border-bottom: 1px solid CanvasText; }
    .nav::after { display: none; }
  }
</style>
