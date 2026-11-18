<script lang="ts">
  // The profile's rail (7TV's profile side bar, cyex styling), the same on /users/<id> and on the
  // /me pages that belong to it, so managing your emotes never feels like another area. Head:
  // avatar ringed in the paint, the name in the paint with the badge, public role chips; then the
  // collapsible Connections (everyone's, each linking to the platform), Active on (channel and its set), Editors (yours, or with EDITORS) and Member since; then the sections;
  // "Admin ›" pinned at the bottom for staff on their own profile. Under 900px the head becomes one
  // compact row (avatar, painted name, a disclosure that opens roles, actions, connections and the
  // rest of this same content) and the rows a tab strip (SideRail), so the page starts right below.
  import type { Snippet } from 'svelte';
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { paintRing } from '$lib/cosmetics/paint';
  import { loadOwned, looks, NO_LOOK, seedLook, wantLooks, type Look } from '$lib/cosmetics/store.svelte';
  import { clean, date } from '$lib/format';
  import { platformName } from '$lib/shell/account';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import { editorsOf, profileOf, type Editor, type PublicProfile } from './data.svelte';
  import { connectionUrl } from './public';
  import { canEditors, canProfile, profileRail, stripExtra, stripTabs, type RailKey, type Relation } from './rows';

  type Account = { id: string; displayName: string; avatar?: string | null };
  type Props = {
    account: Account;
    relation: Relation;
    current: RailKey;
    activity?: boolean;
    /** The name is the page's h1 (the profile page; the /me pages have their own). */
    heading?: boolean;
    actions?: Snippet;
    /** The public profile and look when the page loaded them; otherwise the rail asks (the /me pages). */
    profile?: PublicProfile | null;
    seed?: Look | null;
  };
  let { account, relation, current, activity = false, heading = false, actions, profile: given, seed = null }: Props = $props();

  const name = $derived(clean(account.displayName));
  const groups = $derived(profileRail(account.id, relation, current, activity));
  const look = $derived(looks[account.id] ?? seed ?? NO_LOOK);
  const staff = $derived(relation.mine && isAdmin(session.me));
  // The /me pages without a row of their own (Upload, Notifications, Account) add their tab to the phone strip.
  const tabs = $derived(relation.mine ? stripTabs(groups, stripExtra(page.url.pathname)) : undefined);
  let more_open = $state(false);
  const uid = $props.id();
  const panel = `${uid}-id`;

  let fetched = $state<{ id: string; profile: PublicProfile | null } | null>(null);
  const profile = $derived(given ?? (fetched?.id === account.id ? fetched.profile : null));
  const roles = $derived(profile?.roles ?? []);
  const connections = $derived(profile?.identities ?? []);
  const active = $derived(profile?.activeSets ?? []);
  let editors = $state<Editor[] | null>(null);

  // One cache for every name: this profile's look and its editors' are shared with the rest of the site.
  $effect(() => {
    if (seed) seedLook(account.id, seed);
    else if (!relation.mine) wantLooks([account.id]);
  });
  $effect(() => {
    const id = account.id;
    if (given) return;
    profileOf(id)
      .then((p) => {
        if (id === account.id) fetched = { id, profile: p };
      })
      .catch(() => {});
  });
  $effect(() => {
    const id = account.id;
    const r = relation;
    editors = null;
    if (!canEditors(r)) return;
    editorsOf(id, r.mine)
      .then((e) => {
        if (id !== account.id) return;
        editors = e;
        wantLooks(e.map((x) => x.account.id));
      })
      .catch(() => {});
  });
  $effect(() => {
    const r = relation;
    if (canProfile(r)) loadOwned(account.id, r.mine).catch(() => {});
  });
</script>

{#snippet who(size: number)}
  <span class="ring" style={paintRing(look.paint)}>
    <span class="gap"><Avatar src={account.avatar} {name} {size} /></span>
  </span>
  <div class="id">
    <svelte:element this={heading ? 'h1' : 'p'} class="name"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={20} /></svelte:element>
    {@render extras()}
  </div>
{/snippet}

{#snippet extras()}
  {#if roles.length}
    <ul class="roles" aria-label="Roles">
      {#each roles as r (r.id)}<li class="role"><bdi>{clean(r.name)}</bdi></li>{/each}
    </ul>
  {/if}
  {#if actions}<div class="acts">{@render actions()}</div>{/if}
{/snippet}

{#snippet more()}
  {#if connections.length || relation.mine}
    <details class="dis" open>
      <summary class="drow"><Icon name="link" /><span class="lbl">Connections</span><span class="chev"><Icon name="chevron-down" size={14} /></span></summary>
      <ul class="inner">
        {#each connections as c (c.platform + c.login)}
          {@const url = connectionUrl(c.platform, c.login)}
          <li>
            {#if url}
              <a class="item" href={url} target="_blank" rel="noopener">
                <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} />
                <span class="sr-only">{platformName(c.platform)}: </span><bdi class="txt">{clean(c.login)}</bdi>
                <span class="sr-only"> (opens in a new tab)</span>
              </a>
            {:else}
              <span class="item"><Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} /><span class="sr-only">{platformName(c.platform)}: </span><bdi class="txt">{clean(c.login)}</bdi></span>
            {/if}
          </li>
        {:else}
          {#if relation.mine}<li class="item none">No connections yet.</li>{/if}
        {/each}
        {#if relation.mine}
          <li><a class="item manage" href="/me/account" aria-current={current === 'connections' ? 'page' : undefined}><Icon name="settings" />Manage connections</a></li>
        {/if}
      </ul>
    </details>
  {/if}
  {#if active.length}
    <details class="dis" open>
      <summary class="drow"><Icon name="zap" /><span class="lbl">Active on</span><span class="chev"><Icon name="chevron-down" size={14} /></span></summary>
      <ul class="inner">
        {#each active as a (a.platform + a.login)}
          <li>
            <a class="item two" href="/sets/{encodeURIComponent(a.setId)}">
              <Icon name={a.platform === 'KICK' ? 'kick' : 'twitch'} />
              <span class="stack">
                <bdi class="txt top">{clean(a.setName)}</bdi>
                <span class="sub"><span class="sr-only">on {platformName(a.platform)} channel </span><bdi>{clean(a.login)}</bdi></span>
              </span>
            </a>
          </li>
        {/each}
      </ul>
    </details>
  {/if}
  {#if editors}
    <details class="dis" open={current === 'editors'}>
      <summary class="drow"><Icon name="circle-user" /><span class="lbl">Editors</span>{#if editors.length}<span class="n">{editors.length}</span>{/if}<span class="chev"><Icon name="chevron-down" size={14} /></span></summary>
      <ul class="inner">
        {#each editors as e (e.account.id)}
          <li>
            <a class="item" href="/users/{encodeURIComponent(e.account.id)}">
              <Avatar src={e.account.avatar} name={e.account.displayName} size={22} />
              <PaintedName name={e.account.displayName} paint={looks[e.account.id]?.paint} badge={looks[e.account.id]?.badge} />
            </a>
          </li>
        {:else}
          <li class="item none">No editors yet.</li>
        {/each}
        <li><a class="item manage" href="/me/editors" aria-current={current === 'editors' ? 'page' : undefined}><Icon name="settings" />Manage editors</a></li>
      </ul>
    </details>
  {/if}
  {#if profile?.createdAt}<p class="since">Member since <time datetime={profile.createdAt}>{date(profile.createdAt)}</time></p>{/if}
{/snippet}

<div class="phone">
  <svelte:element this={heading ? 'h1' : 'div'} class="ph">
  <button class="pcard" type="button" aria-expanded={more_open} aria-controls={panel} onclick={() => (more_open = !more_open)}>
    <span class="ring" style={paintRing(look.paint)}>
      <span class="gap"><Avatar src={account.avatar} {name} size={32} /></span>
    </span>
    <span class="name"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={18} /></span>
    <span class="chev"><Icon name="chevron-down" size={16} /></span>
  </button>
  </svelte:element>
  <div class="pmore" id={panel} hidden={!more_open}>
    {@render extras()}
    {#if profile || editors}{@render more()}{/if}
  </div>
</div>

<SideRail label="{name}'s profile" {groups} {tabs} collapse="tabs">
  {#snippet head()}
    <div class="head">{@render who(72)}</div>
    {#if profile || editors}<div class="more">{@render more()}</div>{/if}
  {/snippet}
  {#snippet foot()}
    {#if staff}<a class="rail-foot" href="/admin"><Icon name="wrench" /><span class="lbl">Admin</span><Icon name="chevron-right" size={14} /></a>{/if}
  {/snippet}
</SideRail>

<style>
  .head { display: grid; grid-template-columns: minmax(0, 1fr); justify-items: center; gap: 10px; padding: 8px 0 4px; text-align: center; }
  .head .id { justify-items: center; }
  .head .name { text-align: center; }
  .id { display: grid; grid-template-columns: minmax(0, 1fr); gap: 8px; min-width: 0; max-width: 100%; }
  .name { margin: 0; min-width: 0; max-width: 100%; font-size: 18px; font-weight: 600; letter-spacing: -.01em; line-height: 1.3; }
  .ring { display: inline-grid; flex: none; padding: 2px; border-radius: 50%; background: var(--border); }
  .gap { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--surface); }
  .phone .gap { background: var(--bg); }
  .ph { margin: 0; font: inherit; }
  .pcard .ring { padding: 2px; }
  .roles { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; }
  .role { display: inline-flex; align-items: center; height: 22px; padding: 0 7px; border-radius: var(--radius);
    background: var(--neutral-chip); color: var(--muted); font-size: 12px; font-weight: 600; white-space: nowrap; }

  .acts { display: flex; flex-wrap: wrap; gap: 8px; }
  .head .acts { justify-content: center; }
  .acts :global(.btn), .acts :global(.ui-btn) { height: 32px; padding: 0 10px; gap: 6px; font-size: var(--t-fine); font-weight: 500; color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .acts :global(.btn:hover) { color: var(--text); } }
  .more { margin-top: 10px; display: grid; grid-template-columns: minmax(0, 1fr); gap: 1px; }
  .dis summary { list-style: none; }
  .dis summary::-webkit-details-marker { display: none; }
  .drow { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer; transition: background-color var(--fast), color var(--fast); }
  .drow :global(svg) { color: var(--dim); }
  .lbl { flex: 1; min-width: 0; text-align: left; }
  .n { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .chev { display: inline-grid; transition: rotate var(--base) var(--ease); }
  .dis[open] .chev, [aria-expanded='true'] > .chev { rotate: 180deg; }
  .dis[open] .drow { color: var(--text); }
  .dis { min-width: 0; }
  .inner { list-style: none; margin: 0 0 6px; padding: 0; display: grid; grid-template-columns: minmax(0, 1fr); gap: 1px; }
  .item { display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 0 10px 0 22px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--text); min-width: 0; }
  .item > :global(svg) { flex: none; color: var(--muted); }
  .txt { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item.two { min-height: 44px; padding-block: 4px; }
  .stack { display: grid; grid-template-columns: minmax(0, 1fr); min-width: 0; line-height: 1.25; }
  .stack .top { font-weight: 500; }
  .sub { font-size: 12px; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .since { margin: 6px 0 0; padding: 0 10px; font-size: 12px; color: var(--dim); }
  .item.none { color: var(--dim); font-weight: 400; }
  .manage { color: var(--muted); }
  .manage[aria-current='page'] { color: var(--text); font-weight: 600; }
  .manage[aria-current='page'] :global(svg) { color: var(--accent-text); }
  a.item:active, .drow:active { opacity: .6; transition: none; }
  .drow:focus-visible, a.item:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) {
    .drow:hover, a.item:hover { background: var(--row-hover); color: var(--text); }
  }
  @media (pointer: coarse) { .drow, .item { min-height: 44px; } }

  .phone { display: none; }
  @media (max-width: 899.98px) {
    .phone { display: block; padding: 4px 0; }
    /* One compact row: the page's own content starts right below the header and the strip. */
    .pcard { display: flex; align-items: center; gap: 12px; width: 100%; min-width: 0; min-height: 48px; padding: 4px 0; border: 0; background: none;
      color: var(--text); font: inherit; text-align: left; cursor: pointer; }
    .pcard .name { flex: 1; font-size: var(--t-ui); }
    .pcard .chev { flex: none; color: var(--muted); }
    .pcard:focus-visible { outline-offset: 2px; }
    .pmore { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1px; padding-bottom: 8px; }
    .pmore[hidden] { display: none; }
    .pmore .roles, .pmore .acts { justify-content: flex-start; margin-bottom: 8px; }
    .pmore .drow, .pmore .since { padding-left: 0; }
    .pmore .item { padding-left: 12px; }
  }
  @media (prefers-reduced-motion: reduce) { .chev { transition: none; } }
</style>
