<script lang="ts">
  // One account as staff see it, a dashboard in groups: moderation (restrictions, staff notes,
  // sessions), roles and cosmetics (with the entitlement graph), billing (orders, subscriptions),
  // content (uploads, sets, reports about and by them) and activity (the audit log). Each part
  // shows only what your permissions allow, and a failure in one part never takes the others down.
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { AdminAccountQuery, RoleFieldsFragment } from '$lib/gql/graphql';
  import { ACCOUNT, ADMIN_ACCOUNT, ALL_ROLES, ASSIGN_ROLE, UNASSIGN_ROLE, USER_DETAILS } from '$lib/queries';
  import Restrictions, { type Status } from '$lib/admin/Restrictions.svelte';
  import StaffNotes from '$lib/admin/StaffNotes.svelte';
  import AccountSessions from '$lib/admin/AccountSessions.svelte';
  import AccountBilling from '$lib/admin/AccountBilling.svelte';
  import AccountSetsPanel from '$lib/admin/AccountSetsPanel.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { permissionLabel } from '$lib/admin/permissions';
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
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can, session } from '$lib/tool/session.svelte';

  type Account = NonNullable<AdminAccountQuery['adminAccount']>;
  type Loaded = { account: Account | null; status: Status | null; roles: RoleFieldsFragment[] };

  const canStatus = ['users.view', 'users.restrict', 'users.restrict_uploads', 'users.restrict_tickets', 'users.block'].some(can);
  const manageRoles = can('roles.manage') || can('roles.assign');
  const canRoles = manageRoles || can('users.view');
  const canUploads = can('users.view') || can('emotes.review') || can('emotes.takedown');
  const canReports = can('emotes.review') || can('emotes.takedown');

  let status = $state<Loaded['status']>(null);
  let roles = $state<RoleFieldsFragment[]>([]);
  let allRoles = $state<RoleFieldsFragment[]>([]);
  let pick = $state('');
  let look = $state<Look>(NO_LOOK);

  // Roles you may give that they don't have yet (the API says which you may give).
  const assignable = $derived(allRoles.filter((r) => r.assignable && !roles.some((x) => x.id === r.id)));

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
        account = a ? { id: a.id, displayName: a.displayName, avatar: a.avatar, createdAt: '', uploadsBlocked: false, ticketsBlocked: false, blocked: false, identities: [] } : null;
      }
    } catch (e) {
      if (e instanceof ApiError && e.code === 'not_found') return { account: null, status: null, roles: [] };
      throw e;
    }
    if (!account) return { account: null, status: null, roles: [] };
    const d = await gql(USER_DETAILS, { accountId: id, status: false, roles: canRoles }, fetch);
    if (manageRoles && allRoles.length === 0) allRoles = (await gql(ALL_ROLES, {}, fetch)).roles;
    status = canStatus
      ? { uploadsBlocked: account.uploadsBlocked, ticketsBlocked: account.ticketsBlocked, blocked: account.blocked,
          uploadsBlockedUntil: account.uploadsBlockedUntil, ticketsBlockedUntil: account.ticketsBlockedUntil, blockedUntil: account.blockedUntil }
      : null;
    roles = d.accountRoles ?? [];
    pick = assignable[0]?.id ?? '';
    const first = account.identities[0];
    if (first) lookFor(first.platform, first.platformUserId).then((l) => page.params.accountId === id && (look = l));
    return { account, status, roles };
  });

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
  const canNotes = can('users.notes');
  const canSessions = can('users.sessions');
  const canBilling = can('billing.view') || can('shop.orders');
  const canGraph = can('entitlements.view') || can('cosmetics.grant');
  // Sessions end when the account is blocked: ask again after a change.
  const sessionKey = $derived(status ? `${status.blocked}` : '');

  // The groups this viewer has something in, for the jump links.
  const groups = $derived(
    [
      { id: 'g-moderation', label: 'Moderation', show: !!status || canNotes || canSessions },
      { id: 'g-roles', label: 'Roles and cosmetics', show: canRoles || can('cosmetics.grant') },
      { id: 'g-billing', label: 'Billing', show: canBilling },
      { id: 'g-content', label: 'Content', show: canUploads || canReports },
      { id: 'g-activity', label: 'Activity', show: can('audit.view') },
    ].filter((g) => g.show),
  );
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
  <PageHead title={account.displayName} name back={{ href: '/admin/users', label: 'Users' }} line="Everything staff know about this account, in one place." />

  <div class="layout">
    <section class="ui-card who" aria-label="Profile">
      <Avatar src={account.avatar} name={account.displayName} size={72} tone="tool" />
      <h2><PaintedName name={account.displayName} paint={look.paint} badge={look.badge} badgeSize={20} /></h2>
      <p class="chips">
        {#if status?.blocked}<span class="ui-chip rejected">Blocked</span>{:else}
          {#if status?.uploadsBlocked}<span class="ui-chip pending">Uploads stopped</span>{/if}
          {#if status?.ticketsBlocked}<span class="ui-chip pending">Reports stopped</span>{/if}
          {#if status && !status.uploadsBlocked && !status.ticketsBlocked}<span class="ui-chip approved">Active</span>{/if}
        {/if}
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
      <p class="links">
        <a href="/users/{encodeURIComponent(account.id)}">Public profile</a>
        {#if canGraph}<a href="/admin/entitlements?account={encodeURIComponent(account.id)}">Entitlement graph</a>{/if}
        {#if can('audit.view')}<a href="/admin/audit?targetKind=account&targetId={encodeURIComponent(account.id)}">Audit log</a>{/if}
      </p>
    </section>

    <div class="stack">
      {#if groups.length > 1}
        <nav class="jump" aria-label="Sections">
          {#each groups as g (g.id)}<a href="#{g.id}">{g.label}</a>{/each}
        </nav>
      {/if}

      {#if groups.some((g) => g.id === 'g-moderation')}
      <div class="group" id="g-moderation">
        <h2 class="cap ui-label">Moderation</h2>
        {#if status}<Restrictions accountId={account.id} bind:status {self} />{/if}
        <div class="cols">
          {#if canNotes}<StaffNotes accountId={account.id} />{/if}
          {#if canSessions}<AccountSessions accountId={account.id} refresh={sessionKey} />{/if}
        </div>
      </div>
      {/if}

      {#if groups.some((g) => g.id === 'g-roles')}
      <div class="group" id="g-roles">
        <h2 class="cap ui-label">Roles and cosmetics</h2>
      {#if canRoles}
        <section class="ui-panel" aria-labelledby="ro-h">
          <header><h2 id="ro-h">Roles</h2></header>
          {#if roles.length}
            <ul class="ui-list">
              {#each roles as r (r.id)}
                <li>
                  <span class="what"><strong><Name value={r.name} /></strong><span class="ui-hint">{r.builtIn === 'ROOT' ? 'Every permission' : r.permissions.map(permissionLabel).join(', ') || 'No permissions'}</span></span>
                  {#if manageRoles && r.assignable}<button class="ui-btn ghost small" type="button" onclick={() => unassign(r)}>Take away</button>{/if}
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
      {#if canGraph}
        <a class="graph-link ui-panel" href="/admin/entitlements?account={encodeURIComponent(account.id)}">
          <Icon name="network" /><span><strong>Entitlement graph</strong><span class="ui-hint">How each role, badge and paint was given, by whom and when.</span></span><Icon name="chevron-right" size={14} />
        </a>
      {/if}
      </div>
      {/if}

      {#if canBilling}
      <div class="group" id="g-billing">
        <h2 class="cap ui-label">Billing</h2>
        <AccountBilling accountId={account.id} />
      </div>
      {/if}

      {#if canUploads || canReports}
      <div class="group" id="g-content">
        <h2 class="cap ui-label">Content</h2>
        <div class="cols">
          {#if canUploads}<AccountUploads accountId={account.id} />{/if}
          <AccountSetsPanel accountId={account.id} />
        </div>
        {#if canReports}<AccountReports accountId={account.id} />{/if}
      </div>
      {/if}

      {#if can('audit.view')}
      <div class="group" id="g-activity">
        <h2 class="cap ui-label">Activity</h2>
        <AccountActivity accountId={account.id} />
      </div>
      {/if}
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
  .ids dd.ui-mono { word-break: break-all; }
  .links { display: flex; flex-wrap: wrap; justify-content: center; gap: 6px 16px; font-size: var(--text-sub); }
  .jump { display: flex; flex-wrap: wrap; gap: 4px 18px; padding: 2px 2px 0; font-size: var(--text-sub); }
  .jump a { color: var(--muted); font-weight: 500; }
  @media (hover: hover) and (pointer: fine) { .jump a:hover { color: var(--text); } }
  .group { display: grid; gap: 12px; min-width: 0; scroll-margin-top: calc(var(--nav-h) + 16px); }
  .cap { margin: 8px 0 -2px; font-size: var(--text-label); line-height: 1.6; }
  .cols { display: grid; gap: 12px; align-items: start; min-width: 0; }
  @media (min-width: 1380px) { .cols { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); } }
  .cols:empty { display: none; }
  .graph-link { display: flex; align-items: center; gap: 12px; padding: 12px 14px; color: var(--text); }
  .graph-link > span { display: grid; flex: 1; min-width: 0; font-size: var(--text-sub); }
  .graph-link :global(svg) { color: var(--dim); flex: none; }
  @media (hover: hover) and (pointer: fine) { .graph-link:hover { background: var(--row-hover); } }
  .pad { padding: 2px 14px 10px; margin: 0; }
  .what { display: grid; flex: 1; min-width: 0; }
  .give { display: flex; gap: 8px; padding: 10px; border-top: 1px solid var(--border); }
  .give select { flex: 1; background: var(--bg); }
</style>
