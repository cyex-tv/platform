<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import type { Platform, RoleFieldsFragment, UserQuery } from '$lib/gql/graphql';
  import { ALL_ROLES, ASSIGN_ROLE, RESTRICT, UNASSIGN_ROLE, USER, USER_DETAILS } from '$lib/queries';
  import Avatar from '$lib/components/Avatar.svelte';
  import { clean } from '$lib/format';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Found = NonNullable<UserQuery['channelByLogin']>;

  const canStatus = can('users.view') || can('users.restrict');
  const canRestrict = can('users.restrict');
  const canRoles = can('roles.manage') || can('users.view');
  const manageRoles = can('roles.manage');

  let platform = $state<Platform>((page.url.searchParams.get('platform')?.toUpperCase() as Platform) || 'TWITCH');
  let login = $state(page.url.searchParams.get('login') ?? '');
  let found = $state<Found | null>(null);
  let status = $state<{ uploadsBlocked: boolean; blocked: boolean } | null>(null);
  let roles = $state<RoleFieldsFragment[]>([]);
  let allRoles = $state<RoleFieldsFragment[]>([]);
  let pick = $state('');
  let busy = $state(false);

  const assignable = $derived(allRoles.filter((r) => !roles.some((x) => x.id === r.id)));

  /** The account in the address (`?platform=…&login=…`), looked up again when the address changes. */
  const L = loader(async ({ fetch }) => {
    const p = (page.url.searchParams.get('platform')?.toUpperCase() as Platform) || 'TWITCH';
    const name = (page.url.searchParams.get('login') ?? '').trim().toLowerCase();
    const mine = page.url.search;
    found = null;
    status = null;
    roles = [];
    if (!name) return { missing: false };
    const r = await gql(USER, { platform: p, login: name }, fetch);
    if (!r.channelByLogin) return { missing: true };
    const f = r.channelByLogin;
    const d = await gql(USER_DETAILS, { accountId: f.account.id, status: canStatus, roles: canRoles }, fetch);
    if (manageRoles && allRoles.length === 0) allRoles = (await gql(ALL_ROLES, {}, fetch)).roles;
    if (mine === page.url.search) {
      found = f;
      status = d.accountStatus ?? null;
      roles = d.accountRoles ?? [];
      pick = assignable[0]?.id ?? '';
    }
    return { missing: false };
  });

  function search(e: SubmitEvent) {
    e.preventDefault();
    const name = login.trim().replace(/^@/, '').toLowerCase();
    goto(`?platform=${platform.toLowerCase()}&login=${encodeURIComponent(name)}`, { keepFocus: true });
  }

  async function restrict(input: { uploadsBlocked?: boolean; blocked?: boolean }) {
    if (!found || busy) return;
    busy = true;
    try {
      status = (await gql(RESTRICT, { accountId: found.account.id, ...input })).restrictAccount;
      notify('Saved.');
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function assign() {
    if (!found || !pick) return;
    try {
      await gql(ASSIGN_ROLE, { accountId: found.account.id, roleId: pick });
      const role = allRoles.find((r) => r.id === pick);
      if (role) roles = [...roles, role];
      pick = assignable[0]?.id ?? '';
      notify(`${role?.name ?? 'Role'} given.`);
    } catch (e) {
      fail(e);
    }
  }

  async function unassign(role: RoleFieldsFragment) {
    if (!found) return;
    try {
      await gql(UNASSIGN_ROLE, { accountId: found.account.id, roleId: role.id });
      roles = roles.filter((r) => r.id !== role.id);
      pick = assignable[0]?.id ?? '';
      notify(`${role.name} taken away.`);
    } catch (e) {
      fail(e);
    }
  }
</script>

<svelte:head><title>Users - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Users</h1><p>Find someone by the login they signed in with.</p></div></div>

<form class="find" onsubmit={search}>
  <div class="ui-seg" role="group" aria-label="Platform">
    <button type="button" aria-pressed={platform === 'TWITCH'} onclick={() => (platform = 'TWITCH')}>Twitch</button>
    <button type="button" aria-pressed={platform === 'KICK'} onclick={() => (platform = 'KICK')}>Kick</button>
  </div>
  <input class="ui-input" bind:value={login} placeholder="Login" aria-label="Login" autocomplete="off" spellcheck="false" />
  <button class="ui-btn primary" type="submit" disabled={!login.trim()}>Find</button>
</form>

{#if page.url.searchParams.get('login')}
<LoadGate source={L} label="Looking them up…">
{#snippet children(result)}
{#if result.missing}
  <div class="ui-empty"><p>No one with that login has signed in to cyex.tv.</p></div>
{:else if found}
  <div class="layout">
    <section class="ui-card who">
      <Avatar src={found.account.avatar} name={found.account.displayName} size={48} />
      <div>
        <h2>{clean(found.account.displayName)}</h2>
        <p class="ui-hint">
          {found.platform === 'TWITCH' ? 'Twitch' : 'Kick'} <span class="ui-mono">{found.platformChannelId}</span> · account
          <span class="ui-mono">{found.account.id}</span>
        </p>
        <p class="links">
          <a href="/channels/{found.platform.toLowerCase()}/{encodeURIComponent(found.login)}">Channel page</a>
          {#if can('audit.view')}<a href="/admin/audit?kind=account&id={found.account.id}">Audit log</a>{/if}
        </p>
      </div>
    </section>

    {#if status}
      <section class="ui-panel">
        <header><h2>Restrictions</h2></header>
        <div class="pad">
          <label class="ui-row">
            <div><strong>Block uploads</strong><span class="ui-hint">They can still sign in and use sets.</span></div>
            <input class="ui-switch" type="checkbox" checked={status.uploadsBlocked} disabled={!canRestrict || busy || found.account.id === session.me?.id}
              onchange={(e) => restrict({ uploadsBlocked: e.currentTarget.checked })} />
          </label>
          <label class="ui-row">
            <div><strong>Block the account</strong><span class="ui-hint">Signs them out everywhere and refuses sign-in.</span></div>
            {#if status.blocked}
              <input class="ui-switch" type="checkbox" checked disabled={!canRestrict || busy || found.account.id === session.me?.id}
                aria-label="Block the account" onchange={() => restrict({ blocked: false })} />
            {:else}
              <Confirm small label="Block" confirm="Click again to block and sign out" busy={!canRestrict || busy || found.account.id === session.me?.id}
                onconfirm={() => restrict({ blocked: true })} />
            {/if}
          </label>
          {#if !canRestrict}<p class="ui-hint">Changing these needs users.restrict.</p>{/if}
        </div>
      </section>
    {/if}

    {#if canRoles}
      <section class="ui-panel">
        <header><h2>Roles</h2></header>
        {#if roles.length}
          <ul class="ui-list">
            {#each roles as r (r.id)}
              <li>
                <span class="what"><strong>{r.name}</strong><span class="ui-hint">{r.permissions.join(', ') || 'No permissions'}</span></span>
                {#if manageRoles}
                  <button class="ui-btn ghost small" type="button" onclick={() => unassign(r)}>Take away</button>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="ui-hint pad">No roles.</p>
        {/if}
        {#if manageRoles && assignable.length}
          <div class="give">
            <select class="ui-input" bind:value={pick} aria-label="Role to give">
              {#each assignable as r}<option value={r.id}>{r.name}</option>{/each}
            </select>
            <button class="ui-btn secondary" type="button" onclick={assign}>Give role</button>
          </div>
        {/if}
      </section>
    {/if}
  </div>
{/if}
{/snippet}
</LoadGate>
{/if}

<style>
  .find { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; max-width: var(--measure-form); margin-bottom: 20px; }
  .find .ui-input { flex: 1; min-width: 160px; }
  .layout { display: grid; gap: 16px; max-width: var(--measure); }
  .who { display: flex; gap: 16px; align-items: center; }
  .who { min-width: 0; }
  .who > div { min-width: 0; }
  .who h2 { overflow-wrap: anywhere; }
  .who p { margin-top: 2px; overflow-wrap: anywhere; }
  .links { display: flex; gap: 16px; font-size: var(--text-sub); }
  .pad { padding: 2px 14px 10px; }
  .ui-row .ui-hint { display: block; }
  .what { display: grid; flex: 1; min-width: 0; }
  .give { display: flex; gap: 8px; padding: 10px; border-top: 1px solid var(--border); }
  .give select { flex: 1; background: var(--bg); }
</style>
