<script lang="ts">
  // The account menu's content, shared by the AccountMenu popover (desktop) and the phone menu
  // sheet, so both list the same places in the same order: who you are (the row is the link to
  // your profile dashboard), your library, account, sign out (this session only). Admin, Shop
  // and Subscription are in the header's links (nav.ts), so the sheet lists them with the site's
  // links and the desktop menu doesn't repeat them. The header already has Upload and the theme toggle on
  // desktop, so those two rows only appear in the phone sheet (`touch`), whose header has neither. `emotes` adds the site's own links first (the
  // phone header has no room for them). `touch` makes the rows 48px for fingers.
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import { ACCOUNT, UPLOAD, menuLinks, platformName, profileLink, type ShellLink } from './account';
  import { headerLinks } from './nav';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { rolesOf, type Role } from '$lib/profile/data.svelte';
  import { badge } from '$lib/account/notifications';
  import { inbox } from '$lib/account/inbox.svelte';
  import { leave } from './signout';
  import { theme, toggleTheme } from './theme.svelte';
  import { whatsNew } from '$lib/changelog/seen.svelte';

  /** `menu`: the items are menuitems of the Popover's role="menu" (the desktop account menu); otherwise a nav of links. */
  type Props = { emotes?: boolean; touch?: boolean; menu?: boolean; onpick?: () => void; signin?: string };
  let { emotes = false, touch = false, menu = false, onpick, signin = '/signin' }: Props = $props();
  const item = $derived(menu ? 'menuitem' : undefined);

  const me = $derived(session.ended ? null : session.me);
  const who = $derived(me?.identities[0]);
  const name = $derived(clean(who?.displayName));
  const platform = $derived(platformName(who?.platform));
  const path = $derived(page.url.pathname);
  // Only signed in (never while prerendering, where the query string can't be read).
  const search = $derived(me ? page.url.searchParams : undefined);
  const look = $derived((me && looks[me.id]) || NO_LOOK);
  let role = $state<Role | null>(null);
  // The header of the menu: your name in your paint, your badge and your highest role.
  $effect(() => {
    const id = me?.id;
    role = null;
    if (!id) return;
    loadOwned(id, true).catch(() => {});
    rolesOf(id).then((r) => {
      if (me?.id === id) role = r[0] ?? null;
    });
  });
  const profile = $derived(me ? profileLink(me.id) : null);
  const site = $derived(headerLinks(me, isAdmin(me)));

  async function out() {
    onpick?.();
    await leave();
  }
</script>

{#snippet row(l: ShellLink)}
  <a href={l.href} role={item} aria-current={l.match(path, search) ? 'page' : undefined} onclick={() => onpick?.()}>
    <Icon name={l.icon} /><span>{l.label}</span>
  </a>
{/snippet}

<div class="acct" class:touch role={menu ? 'none' : undefined}>
  {#if me}
    <a class="head" href={profile?.href} role={item} aria-current={profile?.match(path, search) ? 'page' : undefined} onclick={() => onpick?.()}>
      <Avatar src={who?.avatar} {name} size={40} />
      <span class="id">
        <strong><PaintedName {name} paint={look.paint} badge={look.badge} /></strong>
        <span class="sub">
          {#if role}<span class="role"><bdi>{role.name}</bdi></span>{/if}
          {#if platform}<span class="plat">{platform}{#if who?.login && who.login !== name.toLowerCase()} · <bdi>{who.login}</bdi>{/if}</span>{/if}
        </span>
      </span>
      <span class="sr-only">, your profile</span>
      <Icon name="chevron-right" size={14} />
    </a>
  {/if}
  <svelte:element this={menu ? 'div' : 'nav'} class="list" role={menu ? 'none' : undefined} aria-label={menu ? undefined : me ? 'Your account' : 'Menu'}>
    {#if emotes}
      {#each site as l (l.href)}{@render row(l)}{/each}
      <a href="/developers" role={item} aria-current={path.startsWith('/developers') ? 'page' : undefined} onclick={() => onpick?.()}>
        <Icon name="file" /><span>Developers</span>
      </a>
      <hr />
    {/if}
    {#if me}
      {#each menuLinks(me.id) as l (l.href)}{@render row(l)}{/each}
      {#if touch}
        {@render row(UPLOAD)}
        <a href="/me/notifications" role={item} aria-current={path.startsWith('/me/notifications') ? 'page' : undefined} onclick={() => onpick?.()}>
          <Icon name="bell" /><span>Notifications</span>{#if badge(inbox.count)}<span class="value" aria-label="{inbox.count} unread">{badge(inbox.count)}</span>{/if}
        </a>
      {/if}
      <hr />
      {@render row(ACCOUNT)}
    {/if}
    {#if me || touch}
      <!-- What's new: the changelog, with a dot until this device has seen its newest entry. -->
      <a href="/changelog" role={item} aria-current={path.startsWith('/changelog') ? 'page' : undefined} onclick={() => { whatsNew.markSeen(); onpick?.(); }}>
        <Icon name="sparkles" /><span>What’s new</span>{#if whatsNew.unseen && me}<span class="dot" aria-hidden="true"></span><span class="sr-only">, new since your last visit</span>{/if}
      </a>
      {#if me}<hr />{/if}
    {/if}
    {#if touch}
    <button type="button" class="theme-row" role={item} onclick={toggleTheme} aria-label={theme.light ? 'Theme: light. Switch to dark' : 'Theme: dark. Switch to light'}>
      <Icon name={theme.light ? 'sun' : 'moon'} /><span>Theme</span><span class="value">{theme.light ? 'Light' : 'Dark'}</span>
    </button>
    {/if}
    {#if me}
      {#if touch}<hr />{/if}
      <button type="button" role={item} onclick={out}><Icon name="log-out" /><span>Sign out</span></button>
    {:else if session.known && !session.failed}
      <a href={signin} role={item} onclick={() => onpick?.()}><Icon name="user" /><span>Sign in</span></a>
    {/if}
  </svelte:element>
</div>

<style>
  .acct { --row: 36px; display: grid; font-size: var(--t-nav); }
  .touch { --row: 48px; font-size: var(--t-ui); }
  .head { position: relative; display: flex; align-items: center; gap: 12px; padding: 6px 8px 12px; margin-bottom: 4px; border-bottom: 1px solid var(--border);
    border-radius: var(--radius) var(--radius) 0 0; color: var(--text); text-decoration: none; transition: background-color var(--fast); }
  .head > :global(svg) { flex: none; margin-left: auto; color: var(--dim); transition: color var(--fast), transform var(--fast); }
  .head[aria-current='page'] .id strong { font-weight: 700; }
  .head[aria-current='page'] > :global(svg) { color: var(--accent-text); }
  .head:focus-visible { outline-offset: -2px; }
  .head:active { opacity: .6; transition: none; }
  .id { display: grid; min-width: 0; line-height: 1.3; }
  .id strong { font-size: var(--t-ui); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .id strong { display: flex; min-width: 0; }
  .sub { display: flex; align-items: center; gap: 6px; min-width: 0; margin-top: 2px; }
  .plat { font-size: var(--t-fine); color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .role { flex: none; display: inline-flex; align-items: center; height: 20px; padding: 0 6px; border-radius: var(--radius);
    background: var(--neutral-chip); color: var(--muted); font-size: 12px; font-weight: 600; }
  .list { display: grid; }
  .list a, .list button { display: flex; align-items: center; gap: 10px; min-height: var(--row); padding: 0 8px; border: 0;
    border-radius: var(--radius); background: none; color: var(--text); font: inherit; text-align: left; cursor: pointer;
    transition: background-color var(--fast); }
  .list :global(svg) { color: var(--muted); }
  .list a[aria-current='page'] { color: var(--text); font-weight: 600; }
  .list a[aria-current='page'] :global(svg) { color: var(--accent-text); }
  .list a:focus-visible, .list button:focus-visible { outline-offset: -2px; }
  .list a:active, .list button:active { opacity: .6; transition: none; }
  .value { margin-left: auto; color: var(--dim); }
  /* New: a small accent dot at the row's end (the text says it too, for screen readers). */
  .dot { flex: none; width: 8px; height: 8px; margin-left: auto; border-radius: 50%; background: var(--accent); }
  @media (forced-colors: active) { .dot { background: Highlight; } }
  hr { width: 100%; margin: 4px 0; border: 0; border-top: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) {
    .head:hover { background: var(--row-hover); }
    .head:hover > :global(svg) { color: var(--text); transform: translateX(2px); }
    .list a:hover, .list button:hover { background: var(--row-hover); }
    .list a[aria-current='page']:hover { color: var(--text); }
  }
  @media (forced-colors: active) { .list a[aria-current='page'] { outline: 2px solid Highlight; outline-offset: -2px; } }
</style>
