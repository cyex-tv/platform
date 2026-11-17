<script lang="ts">
  // The account menu's content, shared by the AccountMenu popover (desktop) and the phone menu
  // sheet, so both list the same places in the same order: who you are, your library, account
  // (and Admin for staff), sign out. The header already has Upload and the theme toggle on
  // desktop, so those two rows only appear in the phone sheet (`touch`), whose header has neither. `emotes` adds the site's own links first (the
  // phone header has no room for them). `touch` makes the rows 48px for fingers.
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import { EMOTES, MENU, UPLOAD, platformName, profileLink, settingsLinks, type ShellLink } from './account';
  import { leave } from './signout';
  import { theme, toggleTheme } from './theme.svelte';

  type Props = { emotes?: boolean; touch?: boolean; onpick?: () => void; signin?: string };
  let { emotes = false, touch = false, onpick, signin = '/signin' }: Props = $props();

  const me = $derived(session.me);
  const who = $derived(me?.identities[0]);
  const name = $derived(clean(who?.displayName));
  const platform = $derived(platformName(who?.platform));
  const path = $derived(page.url.pathname);
  const settings = $derived(settingsLinks(isAdmin(me)));

  async function out() {
    onpick?.();
    await leave();
  }
</script>

{#snippet row(l: ShellLink)}
  <a href={l.href} aria-current={l.match(path) ? 'page' : undefined} onclick={() => onpick?.()}>
    <Icon name={l.icon} /><span>{l.label}</span>
  </a>
{/snippet}

<div class="acct" class:touch>
  {#if me}
    <div class="head">
      <Avatar src={who?.avatar} {name} size={36} />
      <span class="id">
        <strong><bdi>{name}</bdi></strong>
        {#if platform}<span>{platform}{#if who?.login && who.login !== name.toLowerCase()} · <bdi>{who.login}</bdi>{/if}</span>{/if}
      </span>
    </div>
  {/if}
  <nav class="list" aria-label={me ? 'Your account' : 'Menu'}>
    {#if emotes}
      {@render row(EMOTES)}
      <a href="/developers" aria-current={path.startsWith('/developers') ? 'page' : undefined} onclick={() => onpick?.()}>
        <Icon name="file" /><span>Developers</span>
      </a>
      <hr />
    {/if}
    {#if me}
      {@render row(profileLink(me.id))}
      {#each MENU as l (l.href)}{@render row(l)}{/each}
      {#if touch}{@render row(UPLOAD)}{/if}
      <hr />
      {#each settings as l (l.href)}{@render row(l)}{/each}
      <hr />
    {/if}
    {#if touch}
    <button type="button" class="theme-row" onclick={toggleTheme} aria-label={theme.light ? 'Theme: light. Switch to dark' : 'Theme: dark. Switch to light'}>
      <Icon name={theme.light ? 'sun' : 'moon'} /><span>Theme</span><span class="value">{theme.light ? 'Light' : 'Dark'}</span>
    </button>
    {/if}
    {#if me}
      {#if touch}<hr />{/if}
      <button type="button" onclick={out}><Icon name="log-out" /><span>Sign out</span></button>
    {:else if session.known && !session.failed}
      <a href={signin} onclick={() => onpick?.()}><Icon name="user" /><span>Sign in</span></a>
    {/if}
  </nav>
</div>

<style>
  .acct { --row: 36px; display: grid; font-size: var(--t-nav); }
  .touch { --row: 48px; font-size: var(--t-ui); }
  .head { display: flex; align-items: center; gap: 12px; padding: 6px 8px 12px; margin-bottom: 4px; border-bottom: 1px solid var(--border); }
  .id { display: grid; min-width: 0; line-height: 1.3; }
  .id strong { font-size: var(--t-ui); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .id span { font-size: var(--t-fine); color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
  hr { width: 100%; margin: 4px 0; border: 0; border-top: 1px solid var(--border); }
  @media (hover: hover) and (pointer: fine) {
    .list a:hover, .list button:hover { background: var(--row-hover); }
    .list a[aria-current='page']:hover { color: var(--text); }
  }
  @media (forced-colors: active) { .list a[aria-current='page'] { outline: 2px solid Highlight; outline-offset: -2px; } }
</style>
