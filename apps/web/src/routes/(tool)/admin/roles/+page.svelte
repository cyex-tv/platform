<script lang="ts">
  // Roles, most senior first. Built-in ones are marked: User (everyone holds it), Admin, and the
  // owners' top role (only its holders see it). The API says per role whether you may edit it
  // (below your highest, not one you hold) or give it; the page follows that and says why not.
  import Name from '$lib/components/Name.svelte';
  import { count, iso } from '$lib/format';
  import { gql } from '$lib/api';
  import type { RoleFieldsFragment } from '$lib/gql/graphql';
  import { ACCOUNT_ROLES, CREATE_ROLE, DELETE_ROLE, ROLES, UPDATE_ROLE } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import State from '$lib/components/State.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PermissionPicker from '$lib/admin/PermissionPicker.svelte';
  import { groupsFor } from '$lib/admin/permissions';

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
  let picked = $state<string[]>([]);
  let shown = $state(false);
  // Permission groups whose names are listed in the row (the chip's tooltip, for touch and keyboard).
  let listed = $state<Record<string, boolean>>({});
  let saving = $state(false);
  let tried = $state(false);
  const MAX_POS = 999_999;
  const userRole = $derived(editing?.builtIn === 'USER');
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : null);
  const posOk = $derived(userRole || (Number.isInteger(position) && position >= 1 && position <= MAX_POS));
  const posError = $derived(tried && !posOk ? `Position 1 to ${count(MAX_POS)}.` : null);

  const mine = $derived(new Set(session.me?.permissions ?? []));
  const sorted = $derived([...(roles ?? [])].sort((a, b) => b.position - a.position));
  // What you may turn on: only what you hold (the owner holds everything). What the role already
  // has may always be turned off (and back on before saving).
  const before = $derived(new Set(editing?.permissions ?? []));
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
    picked = [...(role?.permissions ?? [])];
    shown = role?.public ?? false;
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (!name.trim() || !posOk) return;
    saving = true;
    try {
      if (editing) {
        await gql(UPDATE_ROLE, {
          id: editing.id,
          input: { name: name.trim(), description: description.trim(), position: userRole ? null : position, permissions: picked, public: shown },
        });
        notify(`${name.trim()} saved.`);
      } else {
        await gql(CREATE_ROLE, {
          input: { name: name.trim(), description: description.trim() || null, position, permissions: picked, public: shown },
        });
        notify(`${name.trim()} created.`);
      }
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
  function summary(r: RoleFieldsFragment) {
    const have = new Set(r.permissions);
    return groupsFor(permissions)
      .map((g) => ({ label: g.label, n: g.items.filter((i) => have.has(i.key)).length, of: g.items.length,
        names: g.items.filter((i) => have.has(i.key)).map((i) => i.label).join(', ') }))
      .filter((g) => g.n > 0);
  }

  const BUILT_IN: Record<string, string> = { ROOT: 'Top role', ADMIN: 'Built in', USER: 'Everyone' };
</script>

<svelte:head><title>Roles - Admin - cyex.tv</title></svelte:head>

<div class="ui-head">
  <div><h1>Roles</h1><p>Higher positions are more senior. You can change roles below your own highest one that you don’t hold, and only grant what you hold.</p></div>
  {#if manager}<div class="ui-actions"><button class="ui-btn primary" type="button" onclick={() => open(null)}>New role</button></div>{/if}
</div>

<LoadGate source={L} label="Loading roles…">
{#snippet children()}
{#if !roles || roles.length === 0}
  <State title="No roles yet." />
{:else}
  <section class="ui-panel">
    <div class="row head" aria-hidden="true">
      <span class="ui-label">Pos</span><span class="ui-label">Role</span><span class="ui-label">Permissions</span><span class="ui-label actions-h">Action</span>
    </div>
    <ul class="ui-list">
      {#each sorted as r (r.id)}
        {@const groups = summary(r)}
        <li class="row">
          <span class="pos ui-num" title="Position">{count(r.position)}</span>
          <span class="what">
            <strong class="title"><Name value={r.name} />
              {#if r.builtIn}<span class="ui-chip">{BUILT_IN[r.builtIn]}</span>{/if}
              {#if held.has(r.id) || r.builtIn === 'ROOT'}<span class="ui-chip">Yours</span>{/if}
              {#if r.public}<span class="ui-chip" title="Shown on the profiles of the people who hold it">On profiles</span>{/if}
            </strong>
            {#if r.description}<span class="ui-hint"><Name value={r.description} /></span>{/if}
          </span>
          <span class="perms">
            {#if r.builtIn === 'ROOT'}
              <span class="ui-hint">Every permission, including ones added later.</span>
            {:else}
              {#each groups as g (g.label)}{@const key = r.id + g.label}<button class="grp" type="button" aria-expanded={!!listed[key]} title={g.names} onclick={() => (listed[key] = !listed[key])}><span>{g.label}</span> <span class="ui-num">{g.n}/{g.of}</span></button>{#if listed[key]}<span class="ui-hint names">{g.names}</span>{/if}{:else}<span class="ui-hint">No permissions</span>{/each}
            {/if}
          </span>
          <div class="ui-actions">
            {#if r.editable}
              <button class="ui-btn secondary small" type="button" onclick={() => open(r)}>Edit</button>
              {#if !r.builtIn}<Confirm small label="Delete" confirm="Click again" onconfirm={() => remove(r)} />{/if}
            {:else}
              <span class="ui-hint locked" title={why(r)}>{r.builtIn === 'ROOT' ? 'Fixed' : held.has(r.id) ? 'You hold it' : 'Above you'}<span class="sr-only">: {why(r)}</span></span>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
{/snippet}
</LoadGate>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="role-title">
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="role-title">{editing ? `Edit ${iso(editing.name)}` : 'New role'}</h2>
      {#if userRole}<p class="ui-hint">Every account has this role, so what you give it, everyone gets. It always sits at position 0.</p>{/if}
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
        <label class="ui-check">
          <input type="checkbox" bind:checked={shown} />
          <span>Shown on profiles <span class="ui-hint">Everyone sees this role on the profile of anyone who holds it.</span></span>
        </label>
      {/if}
      <div class="ui-field">
        <span class="ui-label" id="perm-label">Permissions <span class="ui-num count">{picked.length} of {permissions.length}</span></span>
        <div role="group" aria-labelledby="perm-label">
          <PermissionPicker available={permissions} bind:picked canChange={canChange} />
        </div>
        <span class="ui-hint">Hover or focus a permission to see what it allows. A group’s box picks or clears the whole group.</span>
      </div>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  /* One grid for the header and every row, so the columns line up. */
  .row { display: grid; grid-template-columns: 76px minmax(160px, 1fr) minmax(0, 2fr) 170px; gap: 14px; align-items: start; }
  .ui-list > li.row { padding-block: 12px; }
  .head { align-items: center; padding: 8px 14px; border-bottom: 1px solid var(--border); background: var(--row-alt); }
  .actions-h { text-align: right; }
  .title { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .pos { color: var(--dim); padding-top: 1px; font-size: var(--text-sub); }
  .what { display: grid; gap: 2px; min-width: 0; overflow-wrap: anywhere; }
  .perms { display: flex; flex-wrap: wrap; gap: 4px 12px; font-size: var(--text-sub); color: var(--muted); padding-top: 1px; }
  .grp { display: inline-flex; align-items: center; min-height: 24px; padding: 0; border: 0; background: none; color: inherit; font: inherit; cursor: pointer; text-align: left; }
  .names { flex-basis: 100%; overflow-wrap: anywhere; }
  .grp .ui-num { color: var(--dim); font-size: 12px; }
  @media (pointer: coarse) { .grp { min-height: 44px; } }
  .grp:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: var(--radius); }
  .row .ui-actions { justify-content: flex-end; flex-wrap: nowrap; align-items: center; }
  .locked { white-space: nowrap; cursor: help; }
  @media (max-width: 860px) {
    .head { display: none; }
    .row { grid-template-columns: 56px minmax(0, 1fr); gap: 8px 12px; }
    .perms, .row .ui-actions { grid-column: 2; }
    .row .ui-actions { justify-content: flex-start; }
  }
  .wide { width: min(820px, calc(100vw - 24px)); }
  .two { display: grid; grid-template-columns: minmax(0, 1fr) 130px; gap: 12px; }
  .num { text-align: right; }
  .count { margin-left: 6px; font-weight: 500; letter-spacing: 0; text-transform: none; }
</style>
