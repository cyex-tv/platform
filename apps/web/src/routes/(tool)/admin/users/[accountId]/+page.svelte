<script lang="ts">
  // One account as staff see it, the Inspector shape of the public profile: the person in the
  // panel on the right (above the content under 1100px), the groups as tabs (?s=): moderation (restrictions, staff notes,
  // sessions), roles and cosmetics, billing (orders, subscriptions),
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
  import AccountEditorsPanel from '$lib/admin/AccountEditorsPanel.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import Box from '$lib/admin/ui/Box.svelte';
  import { withParam } from '$lib/admin/ui/pick';
  import { arrowNav } from '$lib/shell/rail';
  import { permissionLabel } from '$lib/admin/permissions';
  import Avatar from '$lib/components/Avatar.svelte';
  import Name from '$lib/components/Name.svelte';
  import State from '$lib/components/State.svelte';
  import { count, date, iso } from '$lib/format';
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
  const canUploads = ['users.view', 'emotes.review', 'emotes.takedown', 'emotes.edit_any', 'emotes.delete_any', 'emotes.flags'].some(can);
  const canEditors = can('editors.manage_any');
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

  // The groups this viewer has something in: the tabs (?s=), the first one when none is asked for.
  const groups = $derived(
    [
      { id: 'moderation', label: 'Moderation', show: !!status || canNotes || canSessions },
      { id: 'roles', label: 'Roles and cosmetics', show: canRoles || can('cosmetics.grant') },
      { id: 'billing', label: 'Billing', show: canBilling },
      { id: 'content', label: 'Content', show: canUploads || canReports || canEditors },
      { id: 'activity', label: 'Activity', show: can('audit.view') },
    ].filter((g) => g.show),
  );
  const section = $derived(groups.find((g) => g.id === page.url.searchParams.get('s'))?.id ?? groups[0]?.id);
  const tabHref = (id: string) => withParam(page.url, 's', id === groups[0]?.id ? null : id);
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
  <Split label="Account" open={true} stack="top" onclose={() => {}}>
    {#snippet panel()}
      <div class="adm-ip-head"><span class="adm-label">Account</span></div>
      <div class="adm-ip-body">
        <section class="adm-sec who">
          <Avatar src={account.avatar} name={account.displayName} size={64} tone="tool" />
          <h2 class="adm-name"><PaintedName name={account.displayName} paint={look.paint} badge={look.badge} badgeSize={20} /></h2>
          <p class="chips">
            {#if status?.blocked}<span class="ui-chip rejected">Blocked</span>{:else}
              {#if status?.uploadsBlocked}<span class="ui-chip pending">Uploads stopped</span>{/if}
              {#if status?.ticketsBlocked}<span class="ui-chip pending">Reports stopped</span>{/if}
              {#if status && !status.uploadsBlocked && !status.ticketsBlocked}<span class="ui-chip approved">Active</span>{/if}
            {/if}
            {#each roles as r (r.id)}<span class="ui-chip"><Name value={r.name} /></span>{/each}
          </p>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            {#if account.createdAt}<dt>Joined</dt><dd>{date(account.createdAt)}</dd>{/if}
            {#each account.identities as i (i.platform + i.platformUserId)}
              <dt>{i.platform === 'TWITCH' ? 'Twitch' : 'Kick'}</dt>
              <dd class="ident">
                <a href="/channels/{i.platform.toLowerCase()}/{encodeURIComponent(i.login)}"><bdi>{i.displayName}</bdi></a>
                <span class="mid">{i.platformUserId}</span>
              </dd>
            {/each}
            <dt>Account ID</dt><dd class="mono">{account.id}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Links</h3>
          <div class="adm-acts">
            <a class="ui-btn secondary small" href="/users/{encodeURIComponent(account.id)}"><Icon name="circle-user" size={14} />Public profile</a>
            {#if canGraph}<a class="ui-btn secondary small" href="/admin/entitlements?account={encodeURIComponent(account.id)}"><Icon name="network" size={14} />Entitlement graph</a>{/if}
            {#if can('audit.view')}<a class="ui-btn secondary small" href="/admin/audit?kind=account&id={encodeURIComponent(account.id)}"><Icon name="scroll" size={14} />Audit log</a>{/if}
          </div>
        </section>
      </div>
    {/snippet}

    <PageHead title={account.displayName} name back={{ href: '/admin/users', label: 'Users' }} />

    {#if groups.length > 1}
      <nav class="tabs" aria-label="Sections" use:arrowNav={'horizontal'}>
        {#each groups as g (g.id)}
          <a href={tabHref(g.id)} aria-current={g.id === section ? 'page' : undefined} data-sveltekit-noscroll data-sveltekit-keepfocus><span class="tl" data-t={g.label}>{g.label}</span></a>
        {/each}
      </nav>
    {/if}

    <div class="stack">
      {#if section === 'moderation'}
        {#if status}<Restrictions accountId={account.id} bind:status {self} />{/if}
        <div class="cols">
          {#if canNotes}<StaffNotes accountId={account.id} />{/if}
          {#if canSessions}<AccountSessions accountId={account.id} refresh={sessionKey} />{/if}
        </div>
      {:else if section === 'roles'}
        {#if canRoles}
          <Box title="Roles" count={count(roles.length)}>
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
              <p class="box-none">No roles.</p>
            {/if}
            {#if manageRoles && assignable.length}
              <div class="give">
                <select class="ui-input" bind:value={pick} aria-label="Role to give">
                  {#each assignable as r}<option value={r.id}>{iso(r.name)}</option>{/each}
                </select>
                <button class="ui-btn secondary" type="button" onclick={assign}>Give role</button>
              </div>
            {/if}
          </Box>
        {/if}
        {#if can('cosmetics.grant')}<UserCosmetics account={account} />{/if}
      {:else if section === 'billing'}
        <AccountBilling accountId={account.id} />
      {:else if section === 'content'}
        <div class="cols">
          {#if canUploads}<AccountUploads accountId={account.id} />{/if}
          <AccountSetsPanel accountId={account.id} />
        </div>
        {#if canEditors}<AccountEditorsPanel accountId={account.id} name={account.displayName} />{/if}
        {#if canReports}<AccountReports accountId={account.id} />{/if}
      {:else if section === 'activity'}
        <AccountActivity accountId={account.id} />
      {/if}
    </div>
  </Split>
{/if}
{/snippet}
</LoadGate>

<style>
  .stack { display: grid; gap: 16px; min-width: 0; }
  .cols { display: grid; gap: 16px; align-items: start; min-width: 0; }
  @media (min-width: 2200px) { .cols { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); } }
  .cols:empty { display: none; }

  /* The person, in the panel. */
  .who { justify-items: start; }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; margin: 0; }
  .ident { display: grid; justify-items: end; }
  .ident a { display: inline-flex; align-items: center; min-height: 22px; }
  .mid { font-family: var(--font-mono); font-size: var(--t-fine); font-weight: 500; color: var(--muted); word-break: break-all; }

  /* The tabs: words on a hairline; the current one bold (no bar or fill), as the profile's. */
  .tabs { display: flex; gap: 4px; overflow-x: auto; scrollbar-width: none; margin: 0 -10px 16px; border-bottom: 1px solid var(--border); }
  .tabs a { flex: none; display: inline-flex; align-items: center; height: 44px; padding: 0 10px; color: var(--muted);
    font-size: var(--t-ui); font-weight: 500; white-space: nowrap; transition: color var(--fast); }
  .tabs a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .tabs a:focus-visible { outline-offset: -2px; }
  .tl { display: inline-grid; }
  .tl::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  @media (hover: hover) and (pointer: fine) { .tabs a:hover { color: var(--text); } }

  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-size: var(--t-small); }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .give { display: flex; gap: 8px; padding: 10px 16px; border-top: 1px solid var(--border); }
  .give select { flex: 1; background: var(--bg); }
</style>
