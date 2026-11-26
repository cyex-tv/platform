<script lang="ts">
  import { media, now, PHONE } from '$lib/media.svelte';
  // The one header, identical on every route (public, signed in, admin, docs, errors, sign in):
  // 56px, sticky, flat on the ground with a hairline, its content in the full-width wrapper. The
  // Inspector layout (redesign, 2026/11/21): on the left the mark + cyex.tv and the main links as
  // plain words (Emotes; signed in also Shop, Subscription for subscribers and Admin for staff:
  // nav.ts; the current one bold, no bar or fill); everything else sits together on the right:
  // the search field, which is the search itself (HeaderSearch: results drop down under it, no
  // popup), Upload (signed in) or Sign in, the theme toggle, the notification bell and the
  // AccountMenu (avatar and painted name). "/" outside a field
  // focuses the search (a page's own field marked data-page-search takes it instead); no hint.
  // 720px and narrower: the mark, a search button that turns the bar into the field (no
  // modal), the bell, and a menu button that opens a sheet with the account menu's items plus
  // the main links and Developers.
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Mark from '$lib/components/Mark.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { clean } from '$lib/format';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import NotificationBell from '$lib/account/NotificationBell.svelte';
  import NotificationToasts from '$lib/account/NotificationToasts.svelte';
  import { watchInbox } from '$lib/account/inbox.svelte';
  import AccountLinks from './AccountLinks.svelte';
  import AccountMenu from './AccountMenu.svelte';
  import HeaderSearch from './HeaderSearch.svelte';
  import { headerLinks } from './nav';
  import { typing } from './search';
  import { headerSearch } from './search.svelte';
  import { whatsNew } from '$lib/changelog/seen.svelte';
  import { openUpload } from '$lib/upload/upload.svelte';

  let { account = $bindable(false) }: { account?: boolean } = $props();

  let menu = $state(false);
  // Once the session has ended the header stops showing the account (the page keeps it).
  const me = $derived(session.ended ? null : session.me);
  const who = $derived(me?.identities[0]);
  const path = $derived(page.url.pathname);
  const links = $derived(headerLinks(me, isAdmin(me)));
  // Back to where you were after signing in (the sign-in page itself sends you to /me).
  const signin = $derived(path === '/signin' ? '/signin' : `/signin?back=${encodeURIComponent(path)}`);

  // Keep the unread count fresh while signed in (the bell and the toasts read it).
  $effect(() => (me ? watchInbox() : undefined));

  // Phones: the search button turns the bar into the field; it closes again when the field
  // empties and loses focus, on Cancel, on Escape, or when a result is opened.
  async function openPhoneSearch() {
    menu = false;
    headerSearch.phoneOpen = true;
    await tick();
    headerSearch.focusTick += 1;
  }
  function closePhoneSearch() {
    headerSearch.phoneOpen = false;
    headerSearch.query = '';
  }
  // Escape closes the phone search wherever focus is in it (the field handles its own first: it
  // closes its list, then clears its text, and says so by preventing the default).
  function phoneEscape(e: KeyboardEvent) {
    if (e.key === 'Escape' && !e.defaultPrevented && headerSearch.phoneOpen) closePhoneSearch();
  }
  // Phone or not: only one search field is mounted (so only one asks the API), and wider than a
  // phone again the bar is back to normal.
  const narrow = $derived(media(PHONE).matches);
  let wasNarrow = false;
  $effect(() => {
    // Back to a wider window: the phone's open search field closes.
    if (narrow) wasNarrow = true;
    else if (wasNarrow) {
      wasNarrow = false;
      headerSearch.phoneOpen = false;
    }
  });

  function onwindowkey(e: KeyboardEvent) {
    if (e.key !== '/' || e.ctrlKey || e.metaKey || e.altKey || typing(e.target)) return;
    e.preventDefault();
    const own = document.querySelector<HTMLInputElement>('input[data-page-search]');
    if (own) own.focus();
    else if (now(PHONE)) void openPhoneSearch();
    else headerSearch.focusTick += 1;
  }
</script>

<svelte:window onkeydown={onwindowkey} />

<header class="app-header">
  <div class="wrap bar" class:searching={headerSearch.phoneOpen}>
    <div class="start">
      <a class="brand" href="/" aria-label="cyex.tv home"><Mark size={24} /><span>cyex.tv</span></a>
      <nav class="links" class:many={links.length > 1} aria-label="Main">
        {#each links as l (l.href)}
          <a href={l.href} aria-current={l.match(path) ? 'page' : undefined}><span class="lb" data-t={l.label}>{l.label}</span></a>
        {/each}
      </nav>
    </div>

    <div class="search">{#if !narrow}<HeaderSearch />{/if}</div>

    <div class="end">
      {#if me}
        <!-- A button, not a link: there is no upload page, only the dialog (owner, 2026/11/22). -->
        <button type="button" class="btn hb up" title="Upload" aria-label="Upload" aria-haspopup="dialog" data-testid="header-upload" onclick={() => openUpload()}><Icon name="upload" /><span class="hl">Upload</span></button>
      {:else if session.known}
        <!-- Signed out, or the API didn't answer (session.failed): either way a way in, never an empty box. -->
        <a class="btn btn-primary hb" href={signin}>Sign in</a>
      {:else}
        <!-- Not known yet: a neutral slot while the first answer is on its way. -->
        <span class="hb-hold" aria-hidden="true"></span>
      {/if}
      <ThemeToggle />
      {#if me}<NotificationBell />{/if}
      <AccountMenu bind:open={account} />
    </div>

    <div class="phone">
      <IconButton icon="search" label="Search" tip="none" onclick={openPhoneSearch} />
      {#if me}<NotificationBell phone />{:else if !session.known}<span class="bell-hold" aria-hidden="true"></span>{/if}
      <button class="menu-btn" type="button" aria-label="Menu" aria-expanded={menu} aria-haspopup="dialog" onclick={() => (menu = true)}>
        {#if me}<Avatar src={who?.avatar} name={clean(who?.displayName)} size={28} />{#if whatsNew.unseen}<span class="dot" aria-hidden="true"></span><span class="sr-only">, something new in What’s new</span>{/if}{:else}<Icon name="menu" size={18} />{/if}
      </button>
    </div>

    {#if narrow && headerSearch.phoneOpen}
      <div class="phone-search">
        <HeaderSearch phone onclose={closePhoneSearch} />
        <button class="cancel" type="button" onclick={closePhoneSearch}>Cancel</button>
      </div>
    {/if}
  </div>
</header>

{#if me}<NotificationToasts />{/if}

<BottomSheet bind:open={menu} label="Menu">
  <AccountLinks emotes touch {signin} onpick={() => (menu = false)} />
</BottomSheet>

<svelte:document onkeydown={phoneEscape} />

<style>
  .app-header { position: sticky; top: 0; z-index: var(--z-nav); flex: none; background: var(--bg);
    border-bottom: 1px solid var(--border); padding-top: env(safe-area-inset-top, 0px); }
  /* The two sides take their content width (never squeezed under the search); the search is what
     gives: 420px at most, 160px at least, centred while the sides are the same width. */
  /* Brand and words on the left; the search and the tools together on the right. The search is
     what gives: 420px at most, 180px at least. */
  .bar { display: flex; align-items: center; gap: 16px; height: var(--nav-h); }
  .start { display: flex; align-items: center; gap: 12px; height: 100%; flex: none; }
  .brand { flex: none; display: flex; align-items: center; gap: 9px; min-height: 40px;
    font-size: 16px; font-weight: 600; letter-spacing: -.01em; color: var(--text); }
  .links { display: flex; height: 100%; gap: 2px; }
  .links a { white-space: nowrap; }
  /* The current link is bold in the text colour: no underline, bar or fill (owner rule). */
  .links a { position: relative; display: flex; align-items: center; padding: 0 10px; font-size: var(--t-nav); font-weight: 500;
    color: var(--muted); transition: color var(--fast); }
  .lb { display: inline-grid; }
  .lb::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; } /* bold width reserved: no shift */
  .links a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .links a:active { opacity: .6; transition: none; }
  .links a:focus-visible { outline-offset: -2px; }

  .search { display: flex; flex: 1 1 420px; min-width: 180px; max-width: 420px; margin-left: auto; }
  .phone-search { display: none; }

  .end { display: flex; flex: none; align-items: center; justify-content: flex-end; gap: 6px; }
  /* Flat in the bar, by design: the primary button's glow (app.css, cyex-web allows it on a page's
     one main action) would spill over the header's hairline and the page under it. */
  .hb { height: 40px; padding: 0 14px; gap: 8px; font-size: var(--t-nav); box-shadow: none; }
  .up :global(svg) { color: var(--muted); }
  /* The theme toggle and the bell sit flat in the bar (a fill on hover), like the account. */
  .end :global(.theme), .end :global(.bell) { border-color: transparent; }
  .hb:not(:has(svg)) { padding: 0 16px; }
  .hb-hold { width: 84px; height: 40px; border: 1px solid var(--border); border-radius: var(--radius-btn); }
  /* Signed in last time on this device (app.html's hint): the hold is Upload's size and keeps the
     bell's place too, so the bar doesn't move when the session arrives. */
  :global(html[data-signed]) .hb-hold { width: 100px; margin-right: 46px; }
  .bell-hold { display: none; width: 40px; height: 40px; }
  :global(html[data-signed]) .bell-hold { display: block; }

  .phone { display: none; align-items: center; justify-content: flex-end; gap: 4px; }
  .menu-btn { display: inline-grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); cursor: pointer; transition: transform var(--dur-press) ease-out; }
  .menu-btn { position: relative; }
  .menu-btn .dot { position: absolute; top: 4px; right: 4px; width: 9px; height: 9px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 2px var(--bg); }
  @media (forced-colors: active) { .menu-btn .dot { background: Highlight; } }
  .menu-btn:active { transform: scale(var(--press)); }
  @media (pointer: coarse) {
    .menu-btn, .bell-hold { width: 44px; height: 44px; }
    .brand { min-height: 44px; }
    .hb { min-height: 0; } /* 40px to see, 44px to hit (the ::after below) */
    .hb { position: relative; }
    .hb::after { content: ''; position: absolute; inset: -2px 0; }
    .up::after { inset: -2px; } /* icon-only Upload: 44 by 44 */
  }

  @media (hover: hover) and (pointer: fine) {
    .links a:hover { color: var(--text); }
    .hb:hover { box-shadow: none; }
    .menu-btn:hover { color: var(--text); }
  }
  /* Not enough room for the label: Upload becomes its icon (name and tooltip stay), before the search or anything else gives up. */
  @media (max-width: 899.98px) {
    .up { width: 40px; padding: 0; }
    :global(html[data-signed]) .hb-hold { width: 40px; }
    .hl { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }
  }
  @media (max-width: 719.98px) {
    .bar { position: relative; justify-content: space-between; }
    .links, .search, .end { display: none; }
    .phone { display: flex; }
    /* The search button turned the bar into the field: it covers the row, nothing else moves. */
    .phone-search { position: absolute; inset: 0 var(--page-pad); display: flex; align-items: center; gap: 8px; background: var(--bg); }
    .phone-search :global(.hs) { flex: 1; }
    .cancel { flex: none; height: 44px; padding: 0 4px; border: 0; background: none; color: var(--accent-text); font: inherit;
      font-size: var(--t-ui); font-weight: 500; cursor: pointer; }
    .cancel:active { opacity: .6; }
    .searching > .start, .searching > .phone { visibility: hidden; }
  }
  @media (forced-colors: active) {
    .app-header { border-bottom-color: CanvasText; }
    /* Weight alone doesn't show in forced colours: the current link gets an outline. */
    .links a[aria-current='page'] { outline: 1px solid Highlight; outline-offset: -2px; }
  }
</style>
