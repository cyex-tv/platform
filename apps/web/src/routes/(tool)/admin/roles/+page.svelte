<script lang="ts">
  import Name from '$lib/components/Name.svelte';
  import { iso } from '$lib/format';
  import { gql } from '$lib/api';
  import type { RoleFieldsFragment } from '$lib/gql/graphql';
  import { CREATE_ROLE, DELETE_ROLE, ROLES, UPDATE_ROLE } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const L = loader(({ fetch }) => gql(ROLES, {}, fetch));
  const roles = $derived(L.data?.roles ?? null);
  const permissions = $derived(L.data?.permissions ?? []);
  let dialog: HTMLDialogElement;
  let editing = $state<RoleFieldsFragment | null>(null);
  let name = $state('');
  let description = $state('');
  let position = $state(10);
  let picked = $state<string[]>([]);
  let saving = $state(false);
  let tried = $state(false);
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : null);
  const posError = $derived(tried && !(Number.isInteger(position) && position >= 1 && position <= 9999) ? 'Position 1 to 9999.' : null);

  const mine = $derived(new Set(session.me?.permissions ?? []));
  const sorted = $derived([...(roles ?? [])].sort((a, b) => b.position - a.position));

  const load = () => L.reload();

  function open(role: RoleFieldsFragment | null) {
    editing = role;
    tried = false;
    name = role?.name ?? '';
    description = role?.description ?? '';
    position = role?.position ?? 10;
    picked = [...(role?.permissions ?? [])];
    dialog.showModal();
  }

  function toggle(p: string, on: boolean) {
    picked = on ? [...picked, p] : picked.filter((x) => x !== p);
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (!name.trim() || !(Number.isInteger(position) && position >= 1 && position <= 9999)) return;
    saving = true;
    try {
      if (editing) {
        await gql(UPDATE_ROLE, {
          id: editing.id,
          input: { name: name.trim(), description: description.trim(), position, permissions: picked },
        });
        notify(`${name.trim()} saved.`);
      } else {
        await gql(CREATE_ROLE, {
          input: { name: name.trim(), description: description.trim() || null, position, permissions: picked },
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
</script>

<svelte:head><title>Roles - cyex.tv</title></svelte:head>

<div class="ui-head">
  <div><h1>Roles</h1><p>Higher positions are more senior. You can only change roles below your own highest one, and only grant what you hold.</p></div>
  <div class="ui-actions"><button class="ui-btn primary" type="button" onclick={() => open(null)}>New role</button></div>
</div>

<LoadGate source={L} label="Loading roles…">
{#snippet children()}
{#if !roles || roles.length === 0}
  <div class="ui-empty"><p>No roles yet.</p></div>
{:else}
  <section class="ui-panel">
    <div class="row head" aria-hidden="true">
      <span class="ui-label">Pos</span><span class="ui-label">Role</span><span class="ui-label">Permissions</span><span class="ui-label actions-h">Action</span>
    </div>
    <ul class="ui-list">
      {#each sorted as r (r.id)}
        <li class="row">
          <span class="pos ui-mono" title="Position">{r.position}</span>
          <span class="what">
            <strong><Name value={r.name} /></strong>
            {#if r.description}<span class="ui-hint"><Name value={r.description} /></span>{/if}
          </span>
          <span class="perms">{#each r.permissions as p}<span class="ui-chip">{p}</span>{:else}<span class="ui-hint">No permissions</span>{/each}</span>
          <div class="ui-actions">
            <button class="ui-btn secondary small" type="button" onclick={() => open(r)}>Edit</button>
            <Confirm small label="Delete" confirm="Click again" onconfirm={() => remove(r)} />
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
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Name</span>
          <input class="ui-input" bind:value={name} maxlength="64" aria-invalid={nameError ? 'true' : undefined} aria-describedby={nameError ? 'role-name-err' : undefined} />
          {#if nameError}<span class="ui-error" id="role-name-err">{nameError}</span>{/if}
        </label>
        <label class="ui-field">
          <span class="ui-label">Position</span>
          <input class="ui-input ui-mono num" type="number" bind:value={position} min="1" max="9999" aria-invalid={posError ? 'true' : undefined} aria-describedby={posError ? 'role-pos-err' : undefined} />
          {#if posError}<span class="ui-error" id="role-pos-err">{posError}</span>{/if}
        </label>
      </div>
      <label class="ui-field">
        <span class="ui-label">Description</span>
        <input class="ui-input" bind:value={description} maxlength="200" />
      </label>
      <fieldset class="ui-field">
        <legend class="ui-label">Permissions</legend>
        <div class="grid">
          {#each permissions as p}
            <label class="ui-check" title={mine.has(p) ? undefined : 'You can only grant what you hold.'}>
              <input type="checkbox" checked={picked.includes(p)} disabled={!mine.has(p) && !picked.includes(p)}
                onchange={(e) => toggle(p, e.currentTarget.checked)} />
              <span class="ui-mono">{p}</span>
            </label>
          {/each}
        </div>
      </fieldset>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  /* One grid for the header and every row, so the columns line up. */
  .row { display: grid; grid-template-columns: 44px minmax(140px, 1fr) minmax(0, 2.2fr) 180px; gap: 14px; align-items: start; }
  .ui-list > li.row { padding-block: 12px; }
  .head { align-items: center; padding: 8px 14px; border-bottom: 1px solid var(--border); background: var(--row-alt); }
  .actions-h { text-align: right; }
  .pos { color: var(--dim); padding-top: 1px; }
  .what { display: grid; gap: 2px; min-width: 0; overflow-wrap: anywhere; }
  .perms { display: flex; flex-wrap: wrap; gap: 4px; }
  .perms .ui-chip { text-transform: none; letter-spacing: 0; font-family: var(--font-mono); font-weight: 600; }
  .row .ui-actions { justify-content: flex-end; flex-wrap: nowrap; }
  @media (max-width: 860px) {
    .head { display: none; }
    .row { grid-template-columns: 36px minmax(0, 1fr); gap: 8px 12px; }
    .perms, .row .ui-actions { grid-column: 2; }
    .row .ui-actions { justify-content: flex-start; }
  }
  .wide { width: min(560px, calc(100vw - 24px)); }
  .two { display: grid; grid-template-columns: minmax(0, 1fr) 110px; gap: 12px; }
  .num { text-align: right; }
  fieldset { margin: 0; padding: 0; border: 0; }
  legend { padding: 0; margin-bottom: 7px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 8px 12px; }
  .grid .ui-mono { font-size: .8rem; }
</style>
