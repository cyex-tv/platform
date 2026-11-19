<script lang="ts">
  // One account as staff see it: who they are, restrictions, roles, cosmetics, uploads, reports
  // about them and by them, and the latest audit entries. Each part shows only what your
  // permissions allow, and a failure in one part never takes the others down.
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { AdminAccountQuery, RoleFieldsFragment } from '$lib/gql/graphql';
  import { ACCOUNT, ADMIN_ACCOUNT, ALL_ROLES, ASSIGN_ROLE, RESTRICT, UNASSIGN_ROLE, USER_DETAILS } from '$lib/queries';
  import Avatar from '$lib/components/Avatar.svelte';
  import Name from '$lib/components/Name.svelte';
  import State from '$lib/components/State.svelte';
  import { date, iso } from '$lib/format';
  import AccountActivity from '$lib/admin/AccountActivity.svelte';
  import AccountReports from '$lib/admin/AccountReports.svelte';
  import AccountUploads from '$lib/admin/AccountUploads.svelte';
  import UserCosmetics from '$lib/admin/UserCosmetics.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { lookFor, NO_LOOK, type Look } from '$lib/cosmetics/store.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can, session } from '$lib/tool/session.svelte';

  type Account = NonNullable<AdminAccountQuery['adminAccount']>;
  type Loaded = { account: Account | null; status: { uploadsBlocked: boolean; blocked: boolean } | null; roles: RoleFieldsFragment[] };

  const canStatus = can('users.view') || can('users.restrict');
  const canRestrict = can('users.restrict');
  const manageRoles = can('roles.manage');
  const canRoles = manageRoles || can('users.view');
  const canUploads = can('users.view') || can('emotes.review') || can('emotes.takedown');
  const canReports = can('emotes.review') || can('emotes.takedown');

  let status = $state<Loaded['status']>(null);
  let roles = $state<RoleFieldsFragment[]>([]);
  let allRoles = $state<RoleFieldsFragment[]>([]);
  let pick = $state('');
  let busy = $state(false);
  let look = $state<Look>(NO_LOOK);

  const assignable = $derived(allRoles.filter((r) => !roles.some((x) => x.id === r.id)));

  const L = loader<Loaded>(async ({ fetch }) => {
    const id = page.params.accountId ?? '';
    status = null;
    roles = [];
    look = NO_LOOK;
    let account: Account | null = null;
    try {
      if (canStatus) {
        account = (await gql(ADMIN_ACCOUNT, { accountId: id }, fetch)).adminAccount;
      } else {
        // Without users.view only the public summary is known.
        const a = (await gql(ACCOUNT, { id }, fetch)).account;
        account = a ? { id: a.id, displayName: a.displayName, avatar: a.avatar, createdAt: '', uploadsBlocked: false, blocked: false, identities: [] } : null;
      }
    } catch (e) {
      if (e instanceof ApiError && e.code === 'not_found') return { account: null, status: null, roles: [] };
      throw e;
    }
    if (!account) return { account: null, status: null, roles: [] };
    const d = await gql(USER_DETAILS, { accountId: id, status: false, roles: canRoles }, fetch);
    if (manageRoles && allRoles.length === 0) allRoles = (await gql(ALL_ROLES, {}, fetch)).roles;
    status = canStatus ? { uploadsBlocked: account.uploadsBlocked, blocked: account.blocked } : null;
    roles = d.accountRoles ?? [];
    pick = assignable[0]?.id ?? '';
    const first = account.identities[0];
    if (first) lookFor(first.platform, first.platformUserId).then((l) => page.params.accountId === id && (look = l));
    return { account, status, roles };
  });

  async function restrict(input: { uploadsBlocked?: boolean; blocked?: boolean }) {
    const id = L.data?.account?.id;
    if (!id || busy) return;
    busy = true;
    try {
      status = (await gql(RESTRICT, { accountId: id, ...input })).restrictAccount;
      notify('Saved.');
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  async function assign() {
    const id = L.data?.account?.id;
    if (!id || !pick) return;
    try {
      await gql(ASSIGN_ROLE, { accountId: id, roleId: pick });
      const role = allRoles.find((r) => r.id === pick);
      if (role) roles = [...roles, role];
      pick = assignable[0]?.id ?? '';
      notify(`${role ? iso(role.name) : 'Role'} given.`);
    } catch (e) {
      fail(e);
    }
  }

  async function unassign(role: RoleFieldsFragment) {
    const id = L.data?.account?.id;
    if (!id) return;
    try {
      await gql(UNASSIGN_ROLE, { accountId: id, roleId: role.id });
      roles = roles.filter((r) => r.id !== role.id);
      pick = assignable[0]?.id ?? '';
      notify(`${iso(role.name)} taken away.`);
    } catch (e) {
      fail(e);
    }
  }

  const me = $derived(session.me?.id);
</script>

<svelte:head><title>{L.data?.account?.displayName ?? 'Account'} - Admin - cyex.tv</title></svelte:head>

<LoadGate source={L} label="Loading the account…">
{#snippet children(loaded)}
{@const account = loaded.account}
{#if !account}
  <State level={1} title="No such account.">It may have been deleted, or the link is wrong.
    {#snippet actions()}<a class="ui-btn secondary" href="/admin/users">Back to users</a>{/snippet}
  </State>
{:else}
  {@const self = account.id === me}
  <PageHead title={account.displayName} name back={{ href: '/admin/users', label: 'Users' }} line="Account details for staff." />

  <div class="layout">
    <section class="ui-card who" aria-label="Profile">
      <Avatar src={account.avatar} name={account.displayName} size={72} tone="tool" />
      <h2><PaintedName name={account.displayName} paint={look.paint} badge={look.badge} badgeSize={20} /></h2>
      <p class="chips">
        {#if status?.blocked}<span class="ui-chip rejected">Blocked</span>{:else if status?.uploadsBlocked}<span class="ui-chip pending">Uploads blocked</span>{:else if status}<span class="ui-chip approved">Active</span>{/if}
        {#each roles as r (r.id)}<span class="ui-chip"><Name value={r.name} /></span>{/each}
      </p>
      <dl class="ids">
        {#if account.createdAt}<div><dt class="ui-label">Joined</dt><dd class="ui-num">{date(account.createdAt)}</dd></div>{/if}
        {#each account.identities as i (i.platform + i.platformUserId)}
          <div>
            <dt class="ui-label">{i.platform === 'TWITCH' ? 'Twitch' : 'Kick'}</dt>
            <dd>
              <a href="/channels/{i.platform.toLowerCase()}/{encodeURIComponent(i.login)}"><bdi>{i.displayName}</bdi></a>
              <span class="ui-hint ui-mono">{i.platformUserId}</span>
            </dd>
          </div>
        {/each}
        <div><dt class="ui-label">Account ID</dt><dd class="ui-mono">{account.id}</dd></div>
      </dl>
      <p class="links"><a href="/users/{encodeURIComponent(account.id)}">Public profile</a></p>
    </section>

    <div class="stack">
      {#if status}
        <section class="ui-panel" aria-labelledby="rs-h">
          <header><h2 id="rs-h">Restrictions</h2></header>
          <div class="pad">
            <label class="ui-row">
              <div><strong>Block uploads</strong><span class="ui-hint">They can still sign in and use sets.</span></div>
              <input class="ui-switch" type="checkbox" checked={status.uploadsBlocked} disabled={!canRestrict || busy || self}
                onchange={(e) => restrict({ uploadsBlocked: e.currentTarget.checked })} />
            </label>
            <label class="ui-row">
              <div><strong>Block the account</strong><span class="ui-hint">Signs them out everywhere and refuses sign-in.</span></div>
              {#if status.blocked}
                <input class="ui-switch" type="checkbox" checked disabled={!canRestrict || busy || self}
                  aria-label="Block the account" onchange={() => restrict({ blocked: false })} />
              {:else}
                <Confirm small label="Block" confirm="Click again to block and sign out" busy={!canRestrict || busy || self}
                  onconfirm={() => restrict({ blocked: true })} />
              {/if}
            </label>
            {#if !canRestrict}<p class="ui-hint">Changing these needs users.restrict.</p>{/if}
          </div>
        </section>
      {/if}

      {#if canRoles}
        <section class="ui-panel" aria-labelledby="ro-h">
          <header><h2 id="ro-h">Roles</h2></header>
          {#if roles.length}
            <ul class="ui-list">
              {#each roles as r (r.id)}
                <li>
                  <span class="what"><strong><Name value={r.name} /></strong><span class="ui-hint">{r.permissions.join(', ') || 'No permissions'}</span></span>
                  {#if manageRoles}<button class="ui-btn ghost small" type="button" onclick={() => unassign(r)}>Take away</button>{/if}
                </li>
              {/each}
            </ul>
          {:else}
            <p class="ui-hint pad">No roles.</p>
          {/if}
          {#if manageRoles && assignable.length}
            <div class="give">
              <select class="ui-input" bind:value={pick} aria-label="Role to give">
                {#each assignable as r}<option value={r.id}>{iso(r.name)}</option>{/each}
              </select>
              <button class="ui-btn secondary" type="button" onclick={assign}>Give role</button>
            </div>
          {/if}
        </section>
      {/if}

      {#if can('cosmetics.grant')}<UserCosmetics account={account} />{/if}
      {#if canUploads}<AccountUploads accountId={account.id} />{/if}
      {#if canReports}<AccountReports accountId={account.id} />{/if}
      {#if can('audit.view')}<AccountActivity accountId={account.id} />{/if}
    </div>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .layout { display: grid; gap: 16px; align-items: start; }
  .stack { display: grid; gap: 16px; min-width: 0; }
  @media (min-width: 1100px) { .layout { grid-template-columns: 300px minmax(0, 1fr); } .who { position: sticky; top: calc(var(--nav-h) + 20px); } }
  .who { display: grid; justify-items: center; gap: 10px; min-width: 0; text-align: center; }
  .who h2 { display: flex; justify-content: center; font-size: 18px; max-width: 100%; min-width: 0; }
  .chips { display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; }
  .ids { display: grid; gap: 10px; width: 100%; margin: 4px 0 0; padding-top: 12px; border-top: 1px solid var(--border); text-align: left; }
  .ids dd { display: grid; margin: 2px 0 0; font-size: var(--text-sub); overflow-wrap: anywhere; }
  .links { display: flex; gap: 16px; font-size: var(--text-sub); }
  .pad { padding: 2px 14px 10px; margin: 0; }
  .ui-row .ui-hint { display: block; }
  .what { display: grid; flex: 1; min-width: 0; }
  .give { display: flex; gap: 8px; padding: 10px; border-top: 1px solid var(--border); }
  .give select { flex: 1; background: var(--bg); }
</style>
