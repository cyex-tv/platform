<script lang="ts">
  // Roles, most senior first. Built-in ones are marked: User (everyone holds it), Admin, and the
  // owners' top role (only its holders see it). The API says per role whether you may edit it
  // (below your highest, not one you hold) or give it; the page follows that and says why not.
  // The editor also sets the role's limits (RoleLimits): overrides of the per-account limits for
  // everyone who holds it, saved with setRoleLimits after the role itself.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import Name from '$lib/components/Name.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import { count, iso } from '$lib/format';
  import { gql } from '$lib/api';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import type { RoleFieldsFragment, RoleLimitFieldsFragment } from '$lib/gql/graphql';
  import { ACCOUNT_ROLES, CREATE_ROLE, DELETE_ROLE, ENTITLEMENT_HOLDERS, LIMIT_DEFAULTS, ROLE_LIMITS, ROLES, SET_ROLE_LIMITS, UPDATE_ROLE } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import State from '$lib/components/State.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PermissionPicker from '$lib/admin/PermissionPicker.svelte';
  import { groupsFor } from '$lib/admin/permissions';
  import { summarize } from '$lib/admin/role-effects';
  import RoleLimits from '$lib/admin/RoleLimits.svelte';
  import { amount, changed, info, overrides, toInput, toValues, valueProblem } from '$lib/admin/role-limits';

  const L = loader(async ({ fetch }) => {
    const [r, mine] = await Promise.all([
      gql(ROLES, {}, fetch),
      session.me ? gql(ACCOUNT_ROLES, { accountId: session.me.id }, fetch) : Promise.resolve({ accountRoles: [] }),
    ]);
    return { ...r, held: new Set(mine.accountRoles.map((x) => x.id)) };
  });
  const roles = $derived(L.data?.roles ?? null);
  const permissions = $derived(L.data?.permissions ?? []);
  const held = $derived(L.data?.held ?? new Set<string>());
  const manager = $derived(can('roles.manage'));
  let dialog: HTMLDialogElement;
  let editing = $state<RoleFieldsFragment | null>(null);
  let name = $state('');
  let description = $state('');
  let position = $state(10);
  let allows = $state<string[]>([]);
  let denies = $state<string[]>([]);
  let invisible = $state(false);
  let shown = $state(false);
  let saving = $state(false);
  let tried = $state(false);
  // The role's limits: what the API said, and the boxes as typed. null while loading or when
  // they couldn't be read (the rest of the role still saves).
  let limits = $state<RoleLimitFieldsFragment[] | null>(null);
  let limitsFailed = $state(false);
  let limitValues = $state<Record<string, string>>({});
  let limitsBefore: Record<string, string> = {};
  let limitsFor = 0;
  const MAX_POS = 999_999;
  const userRole = $derived(editing?.builtIn === 'USER');
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : null);
  const posOk = $derived(userRole || (Number.isInteger(position) && position >= 1 && position <= MAX_POS));
  const posError = $derived(tried && !posOk ? `Position 1 to ${count(MAX_POS)}.` : null);
  const limitsOk = $derived(Object.values(limitValues).every((v) => !valueProblem(v)));

  const mine = $derived(new Set(session.me?.permissions ?? []));
  const sorted = $derived([...(roles ?? [])].sort((a, b) => b.position - a.position));
  // What you may change: only what you hold (the owner holds everything). What the role already
  // allows or denies may always be moved off (and back before saving).
  const before = $derived(new Set([...(editing?.permissions ?? []), ...(editing?.denies ?? [])]));
  const canChange = (k: string) => mine.has(k) || before.has(k);

  const load = () => L.reload();

  function why(r: RoleFieldsFragment): string {
    if (r.builtIn === 'ROOT') return 'The top role holds every permission and can’t be changed.';
    if (held.has(r.id)) return 'You hold this role, so someone above you has to change it.';
    if (!manager) return 'Editing roles needs Make and edit roles.';
    return 'Only roles below your own highest one can be changed.';
  }

  function open(role: RoleFieldsFragment | null) {
    editing = role;
    tried = false;
    name = role?.name ?? '';
    description = role?.description ?? '';
    position = role?.position ?? 10;
    allows = [...(role?.permissions ?? [])];
    denies = [...(role?.denies ?? [])];
    invisible = role?.invisible ?? false;
    shown = role?.public ?? false;
    limits = null;
    limitsFailed = false;
    limitValues = {};
    limitsBefore = {};
    if (manager && role?.builtIn !== 'USER') void loadLimits(role);
    dialog.showModal();
  }

  async function loadLimits(role: RoleFieldsFragment | null) {
    const mine = ++limitsFor;
    try {
      const got = role ? (await gql(ROLE_LIMITS, { id: role.id })).roleLimits : (await gql(LIMIT_DEFAULTS, {})).limitDefaults;
      if (mine !== limitsFor) return;
      limits = got;
      limitValues = toValues(got);
      limitsBefore = { ...limitValues };
    } catch {
      if (mine === limitsFor) limitsFailed = true;
    }
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (!name.trim() || !posOk || !limitsOk) return;
    saving = true;
    try {
      let id: string;
      if (editing) {
        id = (await gql(UPDATE_ROLE, {
          id: editing.id,
          input: { name: name.trim(), description: description.trim(), position: userRole ? null : position, permissions: allows, denies, public: invisible ? false : shown, invisible },
        })).updateRole.id;
      } else {
        id = (await gql(CREATE_ROLE, {
          input: { name: name.trim(), description: description.trim() || null, position, permissions: allows, denies, public: invisible ? false : shown, invisible },
        })).createRole.id;
      }
      if (limits && changed(limitsBefore, limitValues)) await gql(SET_ROLE_LIMITS, { id, limits: toInput(limitValues) });
      notify(`${name.trim()} ${editing ? 'saved' : 'created'}.`);
      dialog.close();
      await load();
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
    }
  }

  async function remove(role: RoleFieldsFragment) {
    try {
      await gql(DELETE_ROLE, { id: role.id });
      notify(`${iso(role.name)} deleted.`);
      await load();
    } catch (err) {
      fail(err);
    }
  }

  // "Emotes 3/3 · Users 1/3": the groups a role touches, instead of a wall of names.
  const summary = (keys: string[]) => summarize(keys, groupsFor(permissions));

  const BUILT_IN: Record<string, string> = { ROOT: 'Top role', ADMIN: 'Built in', USER: 'Everyone' };

  // The picked role lives in the address (?role=<id>). Its limits and holders load for the panel.
  const pickedId = $derived(page.url.searchParams.get('role'));
  const cur = $derived(sorted.find((r) => r.id === pickedId) ?? null);
  let grid = $state<HTMLElement>();
  const pick = (id: string | null) => void pickInUrl('role', id);
  // Closing the inspector puts focus back on the role's row.
  function unpick() {
    const id = pickedId;
    pick(null);
    void tick().then(() => grid?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id ?? '')}"]`)?.focus());
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !pickedId || e.defaultPrevented || document.querySelector('dialog[open]')) return;
    if ((e.target as HTMLElement | null)?.closest('input, textarea, select')) return;
    e.preventDefault();
    unpick();
  }

  let panelLimits = $state<RoleLimitFieldsFragment[] | null>(null);
  let panelLimitsFailed = $state(false);
  let holders = $state<number | null>(null);
  const mayHolders = can('entitlements.view') || can('cosmetics.grant');
  let panelFor = 0;
  $effect(() => {
    const r = cur?.id;
    const kind = cur?.builtIn;
    const n = ++panelFor;
    panelLimits = null;
    panelLimitsFailed = false;
    holders = null;
    if (!r || kind === 'USER') return;
    if (manager)
      gql(ROLE_LIMITS, { id: r })
        .then((x) => n === panelFor && (panelLimits = x.roleLimits))
        .catch(() => n === panelFor && (panelLimitsFailed = true));
    if (mayHolders)
      gql(ENTITLEMENT_HOLDERS, { kind: 'ROLE', id: r })
        .then((x) => n === panelFor && (holders = x.entitlementHolders.filter((g) => g.active).length))
        .catch(() => {});
  });
  // Members per role, for the list (who may see holders).
  let members = $state<Record<string, number>>({});
  $effect(() => {
    if (!roles || !mayHolders) return;
    for (const r of roles) {
      if (r.builtIn === 'USER') continue;
      gql(ENTITLEMENT_HOLDERS, { kind: 'ROLE', id: r.id })
        .then((x) => (members[r.id] = x.entitlementHolders.filter((g) => g.active).length))
        .catch(() => {});
    }
  });
  let query = $state('');
  const visible = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? sorted.filter((r) => r.name.toLowerCase().includes(q)) : sorted;
  });
  const allowGroups = $derived(cur ? summary(cur.permissions) : []);
  const denyGroups = $derived(cur ? summary(cur.denies) : []);
  const set = $derived(panelLimits ? overrides(panelLimits) : []);
</script>

<svelte:head><title>{cur ? `${iso(cur.name)} - Roles` : 'Roles'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Split label="Role" open={!!cur} onclose={() => pick(null)}>
  <Bar title="Roles" count={roles ? `${count(roles.length)} ${roles.length === 1 ? 'role' : 'roles'}` : null}>
    {#snippet actions()}
      {#if manager}<button class="ui-btn primary small" type="button" onclick={() => open(null)}><Icon name="plus" size={15} />New role</button>{/if}
    {/snippet}
    {#snippet tools()}
      <label class="adm-find grow">
        <Icon name="search" size={15} />
        <span class="sr-only">Filter roles</span>
        <input class="ui-input" type="search" bind:value={query} placeholder="Filter by name" autocomplete="off" spellcheck="false" maxlength="50" />
      </label>
      {#if roles && query.trim()}<span class="ui-hint adm-num">{count(visible.length)} of {count(roles.length)}</span>{/if}
    {/snippet}
  </Bar>

  <LoadGate source={L} label="Loading roles…">
    {#snippet children()}
      {#if !roles || roles.length === 0}
        <State title="No roles yet." />
      {:else if !visible.length}
        <State title="Nothing matches.">No role has that name.</State>
      {:else}
        <div class="rhead" aria-hidden="true"><span class="p">Position</span><span>Role</span><span>Flags</span><span class="m">Members</span></div>
        <ul class="adm-rows" bind:this={grid} use:gridKeys aria-label="Roles">
          {#each visible as r, i (r.id)}
            <li>
              <button class="rrow" type="button" data-item={r.id} aria-current={r.id === pickedId ? 'true' : undefined}
                tabindex={(cur && visible.some((x) => x.id === pickedId) ? r.id === pickedId : i === 0) ? 0 : -1} onclick={() => pick(r.id)}>
                <span class="pos adm-num" title="Position">{count(r.position)}</span>
                <span class="nm"><Icon name="check" size={14} /><bdi><Name value={r.name} /></bdi></span>
                <span class="adm-flags">
                  {#if r.builtIn}<span>{BUILT_IN[r.builtIn]}</span>{/if}
                  {#if held.has(r.id) || r.builtIn === 'ROOT'}<span class="on">Yours</span>{/if}
                  {#if r.invisible}<span class="inv" title="Never shown publicly, and ignored where roles rank people"><Icon name="eye-off" size={13} />Invisible</span>
                  {:else if r.public}<span title="Shown on the profiles of the people who hold it">On profiles</span>{/if}
                </span>
                <span class="mem adm-num">{r.builtIn === 'USER' ? 'Everyone' : members[r.id] === undefined ? '' : count(members[r.id])}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Role</span></div>
    <p class="none">Pick a role to see what it allows, its limits and who holds it.</p>
  {/snippet}

  {#snippet panel()}
    {#if cur}
      {@const r = cur}
      <div class="adm-ip-head">
        <span class="adm-label">Role</span>
        <IconButton label="Close" icon="x" size="s" onclick={unpick} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi><Name value={r.name} /></bdi></h2>
          <ul class="adm-flags">
            {#if r.builtIn}<li>{BUILT_IN[r.builtIn]}</li>{/if}
            {#if held.has(r.id) || r.builtIn === 'ROOT'}<li class="on">Yours</li>{/if}
            {#if r.invisible}<li class="inv"><Icon name="eye-off" size={13} />Invisible</li>
            {:else if r.public}<li>On profiles</li>{/if}
          </ul>
          {#if r.description}<p class="desc"><Name value={r.description} /></p>{/if}
          <dl class="adm-kv">
            <dt>Position</dt><dd>{count(r.position)}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Allows<span class="n">{r.builtIn === 'ROOT' ? 'All' : `${count(r.permissions.length)} of ${count(permissions.length)}`}</span></h3>
          {#if r.builtIn === 'ROOT'}
            <p class="desc">Every permission, including ones added later.</p>
          {:else if !allowGroups.length}
            <p class="desc">Nothing allowed.</p>
          {:else}
            <dl class="adm-kv perms">
              {#each allowGroups as g (g.label)}
                <dt>{g.label} <span class="adm-num">{count(g.n)}/{count(g.of)}</span></dt>
                <dd class="txt">{g.names}</dd>
              {/each}
            </dl>
          {/if}
        </section>
        {#if r.builtIn !== 'ROOT'}
          <section class="adm-sec">
            <h3 class="adm-label deny">Denies<span class="n">{count(r.denies.length)}</span></h3>
            {#if !denyGroups.length}
              <p class="desc">Nothing denied.</p>
            {:else}
              <dl class="adm-kv perms deny">
                {#each denyGroups as g (g.label)}
                  <dt>{g.label} <span class="adm-num">{count(g.n)}/{count(g.of)}</span></dt>
                  <dd class="txt">{g.names}</dd>
                {/each}
              </dl>
              <p class="desc">A deny beats an allow from any role under this one.</p>
            {/if}
          </section>
        {/if}
        {#if r.builtIn === 'USER'}
          <section class="adm-sec">
            <h3 class="adm-label">Limits</h3>
            <p class="desc">Everyone gets the default limits.</p>
          </section>
        {:else if manager}
          <section class="adm-sec">
            <h3 class="adm-label">Limits{#if panelLimits && set.length}<span class="n">{count(set.length)} set</span>{/if}</h3>
            {#if panelLimitsFailed}
              <p class="desc">Couldn’t load this role’s limits.</p>
            {:else if !panelLimits}
              <p class="desc">Loading…</p>
            {:else if !set.length}
              <p class="desc">None set: the defaults apply.</p>
            {:else}
              <dl class="adm-kv">
                {#each set as l (l.key)}<dt>{info(l.key).label}</dt><dd>{amount(l.value ?? 0, l.windowSeconds)}</dd>{/each}
              </dl>
            {/if}
          </section>
        {/if}
        <section class="adm-sec">
          <h3 class="adm-label">Members</h3>
          <dl class="adm-kv">
            {#if r.builtIn === 'USER'}
              <dt>Holders</dt><dd class="txt">Everyone</dd>
            {:else if mayHolders}
              <dt>Holders</dt><dd>{holders === null ? '…' : count(holders)}{' '}<a class="txt" href="/admin/entitlements?role={encodeURIComponent(r.id)}">See</a></dd>
            {:else}
              <dt>Holders</dt><dd class="txt">Not shown</dd>
            {/if}
          </dl>
        </section>
        <section class="adm-sec">
          {#if r.editable}
            <div class="adm-acts row">
              <button class="ui-btn secondary small" type="button" onclick={() => open(r)}><Icon name="pencil" size={14} />Edit</button>
              {#if !r.builtIn}<Confirm small label="Delete" confirm="Click again to delete" onconfirm={() => remove(r)} />{/if}
            </div>
          {:else}
            <p class="locked"><Icon name="lock" size={14} />{why(r)}</p>
          {/if}
        </section>
      </div>
    {/if}
  {/snippet}
</Split>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="role-title">
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="role-title">{editing ? `Edit ${iso(editing.name)}` : 'New role'}</h2>
      {#if userRole}<p class="ui-hint">Every account has this role, so what you give it, everyone gets. It always sits at position 0.</p>{/if}
      <section class="grp">
        <h3 class="adm-label">Basics</h3>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Name</span>
            <input class="ui-input" bind:value={name} maxlength="50" aria-invalid={nameError ? 'true' : undefined} aria-describedby={nameError ? 'role-name-err' : undefined} />
            {#if nameError}<span class="ui-error" id="role-name-err">{nameError}</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">Position</span>
            <input class="ui-input ui-mono num" type="number" bind:value={position} min="1" max={MAX_POS} disabled={userRole}
              aria-invalid={posError ? 'true' : undefined} aria-describedby={posError ? 'role-pos-err' : undefined} />
            {#if posError}<span class="ui-error" id="role-pos-err">{posError}</span>{/if}
          </label>
        </div>
        <label class="ui-field">
          <span class="ui-label">Description</span>
          <input class="ui-input" bind:value={description} maxlength="200" />
        </label>
        {#if !userRole}
          <label class="ui-check" class:dim={invisible}>
            <input type="checkbox" bind:checked={shown} disabled={invisible} />
            <span>Shown on profiles <span class="ui-hint">{invisible ? 'Invisible wins: an invisible role is never shown.' : 'Everyone sees this role on the profile of anyone who holds it.'}</span></span>
          </label>
          <label class="sw">
            <input class="ui-switch" type="checkbox" role="switch" bind:checked={invisible} aria-describedby="role-inv-hint" />
            <span class="swt"><strong>Invisible</strong>
              <span class="ui-hint" id="role-inv-hint">Never shown on profiles or user cards, and ignored wherever roles rank people (like the channels on an emote page). Staff and holders still see it.</span></span>
          </label>
        {/if}
      </section>
      <section class="grp">
        <h3 class="adm-label" id="perm-label">Permissions <span class="ui-num count">{count(allows.length)} allowed · {count(denies.length)} denied</span></h3>
        <div role="group" aria-labelledby="perm-label">
          <PermissionPicker available={permissions} bind:allows bind:denies canChange={canChange} noDeny={userRole} />
        </div>
        <span class="ui-hint">Allow or Deny decides it for everyone who holds this role; Inherit leaves it to the roles below. A person’s highest role with an Allow or Deny wins, so a Deny here beats an Allow from a lower role. Hover or focus a permission to see what it does.</span>
        {#if userRole}<span class="ui-hint">Deny isn’t offered here: the User role is last, so a Deny on it would never win.</span>{/if}
      </section>
      {#if manager}
        <section class="grp">
          <h3 class="adm-label" id="limits-label">Limits</h3>
          {#if userRole}
            <span class="ui-hint">Everyone gets the default limits. Give a role its own to raise or lower them for the people who hold it.</span>
          {:else if limitsFailed}
            <span class="ui-hint">Couldn’t load this role’s limits. The rest of the role still saves; try again to change them.</span>
          {:else if !limits}
            <span class="ui-hint">Loading limits…</span>
          {:else}
            <div role="group" aria-labelledby="limits-label">
              <RoleLimits {limits} bind:values={limitValues} {tried} />
            </div>
            <span class="ui-hint">Empty keeps the default, shown in grey. Someone with several roles gets the highest of their roles’ limits.</span>
          {/if}
        </section>
      {/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  /* One flush row per role: position, name, flags, members (mono). Picked = bold name + accent tick. */
  .rhead, .rrow { display: grid; grid-template-columns: 72px minmax(120px, 1.2fr) minmax(0, 1.6fr) 96px; align-items: center; gap: 12px; padding: 0 12px; }
  .rhead { min-height: 32px; font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .rhead .p, .rhead .m, .pos, .mem { text-align: right; }
  .rrow { width: 100%; height: 44px; border: 0; background: none; color: var(--text); font: inherit; text-align: left; cursor: pointer; }
  .pos, .mem { color: var(--muted); }
  .nm { display: flex; align-items: center; gap: 6px; min-width: 0; font-size: var(--t-small); font-weight: 500; }
  .nm :global(svg) { display: none; color: var(--accent-text); flex: none; }
  .nm bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rrow[aria-current='true'] .nm { font-weight: 700; }
  .rrow[aria-current='true'] .nm :global(svg) { display: block; }
  .rrow:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .rrow:hover { background: var(--row-hover); } }
  @media (max-width: 760px) {
    .rhead, .rrow { grid-template-columns: 84px minmax(0, 1fr) auto; }
    .rhead span:nth-child(3), .rrow .adm-flags { display: none; }
    .rrow { height: 48px; }
  }

  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .desc { margin: 0; font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .perms { grid-template-columns: minmax(0, 1fr); }
  .perms dt { padding-bottom: 0; border-bottom: 0; color: var(--text); font-weight: 600; }
  .perms dt .adm-num { margin-left: 6px; font-weight: 500; color: var(--dim); }
  .perms dd { padding-top: 2px; text-align: left; font-weight: 400; font-size: var(--t-fine); line-height: 1.5; color: var(--muted); }
  .locked { display: flex; gap: 8px; align-items: flex-start; margin: 0; font-size: var(--t-small); color: var(--muted); }
  .locked :global(svg) { flex: none; margin-top: 2px; }

  .wide { width: min(860px, calc(100vw - 24px)); }
  .grp { display: grid; gap: 6px; }
  .grp + .grp { padding-top: 14px; border-top: 1px solid var(--border); }
  .two { display: grid; grid-template-columns: minmax(0, 1fr) 130px; gap: 12px; }
  .num { text-align: right; }
  .inv { display: inline-flex; align-items: center; gap: 4px; }
  .inv :global(svg) { color: var(--dim); }
  .deny, .perms.deny dt { color: var(--red); }
  .perms.deny dd { color: var(--muted); }
  .sw { display: flex; align-items: flex-start; gap: 6px; cursor: pointer; }
  .swt { display: grid; gap: 2px; padding-top: 10px; font-size: var(--t-small); }
  .dim { opacity: .6; }
  .count { margin-left: 6px; font-weight: 500; letter-spacing: 0; text-transform: none; }
</style>
