<script lang="ts">
  // The one header, identical on every route (public, signed in, admin, docs, errors, sign in):
  // 56px, sticky, flat on the ground with a hairline, its content in the 1520 wrapper. Left the
  // mark + cyex.tv and Emotes; centre a search field that opens the command palette; right
  // Upload (signed in) or Sign in, the theme toggle and the AccountMenu. Nothing in it depends
  // on the route. 720px and narrower: the mark, a search button and a menu button that opens a
  // sheet with the account menu's items plus Emotes and Developers.
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Mark from '$lib/components/Mark.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { openPalette } from '$lib/components/palette.svelte';
  import { clean } from '$lib/format';
  import { session } from '$lib/tool/session.svelte';
  import AccountLinks from './AccountLinks.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import { EMOTES } from './account';

  let { account = $bindable(false) }: { account?: boolean } = $props();

  let menu = $state(false);
  const me = $derived(session.me);
  const who = $derived(me?.identities[0]);
  const path = $derived(page.url.pathname);
  // Back to where you were after signing in (the sign-in page itself sends you to /me).
  const signin = $derived(path === '/signin' ? '/signin' : `/signin?back=${encodeURIComponent(path)}`);

  function search() {
    menu = false;
    openPalette();
  }
</script>

<header class="app-header">
  <div class="wrap bar">
    <div class="start">
      <a class="brand" href="/" aria-label="cyex.tv home"><Mark /><span>cyex.tv</span></a>
      <nav class="links" aria-label="Main">
        <a href={EMOTES.href} aria-current={EMOTES.match(path) ? 'page' : undefined}>{EMOTES.label}</a>
      </nav>
    </div>

    <button class="search" type="button" onclick={search} aria-label="Search emotes, channels and pages" aria-haspopup="dialog">
      <Icon name="search" />
      <span class="ph">Search emotes</span>
      <kbd class="key" aria-hidden="true">/</kbd>
    </button>

    <div class="end">
      {#if session.known}
        {#if me}
          <a class="btn btn-primary hb" href="/me/upload"><Icon name="upload" />Upload</a>
        {:else}
          <a class="btn btn-primary hb" href={signin}>Sign in</a>
        {/if}
      {/if}
      <ThemeToggle />
      <AccountMenu bind:open={account} />
    </div>

    <div class="phone">
      <IconButton icon="search" label="Search" tip="none" onclick={search} />
      <button class="menu-btn" type="button" aria-label="Menu" aria-expanded={menu} aria-haspopup="dialog" onclick={() => (menu = true)}>
        {#if me}<Avatar src={who?.avatar} name={clean(who?.displayName)} size={28} />{:else}<Icon name="menu" size={18} />{/if}
      </button>
    </div>
  </div>
</header>

<BottomSheet bind:open={menu} label="Menu">
  <AccountLinks emotes touch {signin} onpick={() => (menu = false)} />
</BottomSheet>

<style>
  .app-header { position: sticky; top: 0; z-index: var(--z-nav); flex: none; background: var(--bg);
    border-bottom: 1px solid var(--border); padding-top: env(safe-area-inset-top, 0px); }
  .bar { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 420px) minmax(0, 1fr); align-items: center; gap: 16px;
    height: var(--nav-h); }
  .start { display: flex; align-items: center; gap: 16px; min-width: 0; height: 100%; }
  .brand { flex: none; display: flex; align-items: center; gap: 9px; min-height: 40px;
    font-size: 16px; font-weight: 600; letter-spacing: -.01em; color: var(--text); }
  .links { display: flex; height: 100%; }
  .links a { position: relative; display: flex; align-items: center; padding: 0 10px; font-size: var(--t-nav); font-weight: 500;
    color: var(--muted); transition: color var(--fast); }
  .links a::after { content: ''; position: absolute; left: 10px; right: 10px; bottom: -1px; height: 2px; background: var(--accent);
    scale: 0 1; transition: scale var(--base) var(--ease); }
  .links a[aria-current='page'] { color: var(--text); }
  .links a[aria-current='page']::after { scale: 1 1; }
  .links a:active { opacity: .6; transition: none; }
  .links a:focus-visible { outline-offset: -2px; }

  .search { display: flex; align-items: center; gap: 10px; width: 100%; min-width: 0; height: 36px; padding: 0 8px 0 12px;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: var(--surface); color: var(--muted);
    font: inherit; font-size: var(--t-nav); cursor: text; text-align: left;
    transition: border-color var(--fast), color var(--fast); }
  .ph { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .key { flex: none; display: inline-grid; place-items: center; min-width: 20px; height: 20px; padding: 0 5px;
    border: 1px solid var(--border); border-radius: var(--radius); font-size: 12px; color: var(--dim); }

  .end { display: flex; align-items: center; justify-content: flex-end; gap: 8px; min-width: 0; }
  .hb { height: 36px; padding: 0 14px; gap: 8px; font-size: var(--t-nav); box-shadow: none; } /* flat in the bar: no glow over the page */
  .hb:not(:has(svg)) { padding: 0 16px; }

  .phone { display: none; align-items: center; justify-content: flex-end; gap: 4px; }
  .menu-btn { display: inline-grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); cursor: pointer; transition: transform var(--dur-press) ease-out; }
  .menu-btn:active { transform: scale(var(--press)); }
  @media (pointer: coarse) { .menu-btn { width: 44px; height: 44px; } }

  @media (hover: hover) and (pointer: fine) {
    .links a:hover { color: var(--text); }
    .search:hover { border-color: var(--border-strong); color: var(--text); }
    .hb:hover { box-shadow: none; }
    .menu-btn:hover { color: var(--text); }
  }
  @media (max-width: 1000px) {
    .bar { grid-template-columns: minmax(0, 1fr) minmax(0, 260px) auto; }
    .key { display: none; }
  }
  @media (max-width: 720px) {
    .bar { grid-template-columns: minmax(0, 1fr) auto; }
    .links, .search, .end { display: none; }
    .phone { display: flex; }
  }
  @media (prefers-reduced-motion: reduce) { .links a::after { transition: none; } }
  @media (forced-colors: active) { .app-header { border-bottom-color: CanvasText; } .links a[aria-current='page']::after { background: Highlight; } }
</style>
