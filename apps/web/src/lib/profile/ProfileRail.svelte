<script lang="ts">
  // The profile's rail (7TV's profile side bar, cyex styling), the same on /users/<id> and on the
  // /me pages that belong to it, so managing your emotes never feels like another area. Head:
  // avatar ringed in the paint, the name in the paint with the badge, role chips; then the
  // collapsible Connections (yours) and Editors (yours, or with EDITORS); then the sections;
  // "Admin ›" pinned at the bottom for staff on their own profile. Under 900px the head becomes a
  // card above the content and the rows a tab strip (SideRail).
  import type { Snippet } from 'svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { paintRing } from '$lib/cosmetics/paint';
  import { loadOwned, looks, NO_LOOK } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import { platformName } from '$lib/shell/account';
  import SideRail from '$lib/shell/SideRail.svelte';
  import { isAdmin, session } from '$lib/tool/session.svelte';
  import { editorsOf, rolesOf, type Editor, type Role } from './data.svelte';
  import { canEditors, canProfile, profileRail, type RailKey, type Relation } from './rows';

  type Account = { id: string; displayName: string; avatar?: string | null };
  type Props = { account: Account; relation: Relation; current: RailKey; activity?: boolean; actions?: Snippet };
  let { account, relation, current, activity = false, actions }: Props = $props();

  const name = $derived(clean(account.displayName));
  const groups = $derived(profileRail(account.id, relation, current, activity));
  const look = $derived(looks[account.id] ?? NO_LOOK);
  const staff = $derived(relation.mine && isAdmin(session.me));
  const connections = $derived(relation.mine ? (session.me?.identities ?? []) : []);

  let roles = $state<Role[]>([]);
  let editors = $state<Editor[] | null>(null);

  // Roles: yours, or anyone's for staff (everyone else gets none back).
  $effect(() => {
    const id = account.id;
    roles = [];
    if (!session.me) return;
    rolesOf(id).then((r) => {
      if (id === account.id) roles = r;
    });
  });
  $effect(() => {
    const id = account.id;
    const r = relation;
    editors = null;
    if (!canEditors(r)) return;
    editorsOf(id, r.mine)
      .then((e) => {
        if (id === account.id) editors = e;
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
    <p class="name"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={20} /></p>
    {#if roles.length}
      <ul class="roles" aria-label="Roles">
        {#each roles as r (r.id)}<li class="role"><bdi>{clean(r.name)}</bdi></li>{/each}
      </ul>
    {/if}
    {#if actions}<div class="acts">{@render actions()}</div>{/if}
  </div>
{/snippet}

{#snippet more()}
  {#if connections.length}
    <details class="dis" open={current === 'connections'}>
      <summary class="drow"><Icon name="link" /><span class="lbl">Connections</span><span class="chev"><Icon name="chevron-down" size={14} /></span></summary>
      <ul class="inner">
        {#each connections as c (c.platform)}
          <li class="item"><Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} /><span class="sr-only">{platformName(c.platform)}: </span><bdi class="txt">{clean(c.login)}</bdi></li>
        {/each}
        <li><a class="item manage" href="/me/account" aria-current={current === 'connections' ? 'page' : undefined}><Icon name="settings" />Manage connections</a></li>
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
{/snippet}

<div class="phone">
  <div class="pcard">{@render who(56)}</div>
  {#if connections.length || editors}<div class="pmore">{@render more()}</div>{/if}
</div>

<SideRail label="{name}'s profile" {groups} collapse="tabs">
  {#snippet head()}
    <div class="head">{@render who(72)}</div>
    {#if connections.length || editors}<div class="more">{@render more()}</div>{/if}
  {/snippet}
  {#snippet foot()}
    {#if staff}<a class="rail-foot" href="/admin"><Icon name="wrench" /><span class="lbl">Admin</span><Icon name="chevron-right" size={14} /></a>{/if}
  {/snippet}
</SideRail>

<style>
  .head { display: grid; justify-items: center; gap: 10px; padding: 8px 0 4px; text-align: center; }
  .head .id { justify-items: center; }
  .id { display: grid; gap: 8px; min-width: 0; max-width: 100%; }
  .name { margin: 0; min-width: 0; font-size: 18px; font-weight: 600; letter-spacing: -.01em; line-height: 1.3; }
  .ring { display: inline-grid; flex: none; padding: 2px; border-radius: 50%; background: var(--border); }
  .gap { display: inline-grid; padding: 2px; border-radius: 50%; background: var(--surface); }
  .phone .gap { background: var(--bg); }
  .roles { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; }
  .role { display: inline-flex; align-items: center; height: 22px; padding: 0 7px; border-radius: var(--radius);
    background: var(--neutral-chip); color: var(--muted); font-size: 12px; font-weight: 600; white-space: nowrap; }

  .acts { display: flex; flex-wrap: wrap; gap: 8px; }
  .head .acts { justify-content: center; }
  .acts :global(.btn) { height: 32px; padding: 0 10px; gap: 6px; font-size: var(--t-fine); font-weight: 500; color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .acts :global(.btn:hover) { color: var(--text); } }
  .more { margin-top: 10px; display: grid; gap: 1px; }
  .dis summary { list-style: none; }
  .dis summary::-webkit-details-marker { display: none; }
  .drow { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer; transition: background-color var(--fast), color var(--fast); }
  .drow :global(svg) { color: var(--dim); }
  .lbl { flex: 1; min-width: 0; text-align: left; }
  .n { font-size: 12px; color: var(--dim); font-variant-numeric: tabular-nums; }
  .chev { display: inline-grid; transition: rotate var(--base) var(--ease); }
  .dis[open] .chev { rotate: 180deg; }
  .dis[open] .drow { color: var(--text); }
  .inner { list-style: none; margin: 0 0 6px; padding: 0; display: grid; gap: 1px; }
  .item { display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 0 10px 0 22px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--text); min-width: 0; }
  .item > :global(svg) { flex: none; color: var(--muted); }
  .txt { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
    .phone { display: block; padding: 16px 0 8px; }
    .pcard { display: flex; align-items: center; gap: 14px; min-width: 0; }
    .pcard .roles { justify-content: flex-start; }
    .pmore { margin-top: 10px; display: grid; gap: 1px; }
    .pmore .drow { padding-left: 0; }
    .pmore .item { padding-left: 12px; }
  }
  @media (prefers-reduced-motion: reduce) { .chev { transition: none; } }
</style>
