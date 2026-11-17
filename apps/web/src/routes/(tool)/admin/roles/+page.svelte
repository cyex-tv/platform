<script lang="ts">
  import { gql } from '$lib/api';
  import type { RoleFieldsFragment } from '$lib/gql/graphql';
  import { CREATE_ROLE, DELETE_ROLE, ROLES, UPDATE_ROLE } from '$lib/queries';
  import Confirm from '$lib/tool/Confirm.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  let roles = $state<RoleFieldsFragment[] | null>(null);
  let permissions = $state<string[]>([]);
  let dialog: HTMLDialogElement;
  let editing = $state<RoleFieldsFragment | null>(null);
  let name = $state('');
  let description = $state('');
  let position = $state(10);
  let picked = $state<string[]>([]);
  let saving = $state(false);

  const mine = $derived(new Set(session.me?.permissions ?? []));
  const sorted = $derived([...(roles ?? [])].sort((a, b) => b.position - a.position));

  async function load() {
    try {
      const r = await gql(ROLES, {});
      roles = r.roles;
      permissions = r.permissions;
    } catch (e) {
      fail(e);
    }
  }

  $effect(() => {
    load();
  });

  function open(role: RoleFieldsFragment | null) {
    editing = role;
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
    if (!name.trim()) return;
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
      notify(`${role.name} deleted.`);
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

{#if !roles}
  <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span></div>
{:else if roles.length === 0}
  <div class="ui-empty"><p>No roles yet.</p></div>
{:else}
  <section class="ui-panel">
    <ul class="ui-list">
      {#each sorted as r (r.id)}
        <li>
          <span class="pos ui-mono" title="Position">{r.position}</span>
          <span class="what">
            <strong>{r.name}</strong>
            {#if r.description}<span class="ui-muted">{r.description}</span>{/if}
            <span class="perms">{#each r.permissions as p}<span class="ui-chip">{p}</span>{:else}<span class="ui-hint">No permissions</span>{/each}</span>
          </span>
          <div class="ui-actions">
            <button class="ui-btn secondary small" type="button" onclick={() => open(r)}>Edit</button>
            <Confirm small label="Delete" confirm="Click again" onconfirm={() => remove(r)} />
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="role-title">
  <form onsubmit={save}>
    <div class="body">
      <h2 id="role-title">{editing ? `Edit ${editing.name}` : 'New role'}</h2>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Name</span>
          <input class="ui-input" bind:value={name} maxlength="64" required />
        </label>
        <label class="ui-field">
          <span class="ui-label">Position</span>
          <input class="ui-input ui-mono num" type="number" bind:value={position} min="1" max="9999" required />
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
      <button class="ui-btn primary" type="submit" disabled={saving || !name.trim()}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  </form>
</dialog>

<style>
  .ui-list > li { align-items: flex-start; padding-block: 12px; flex-wrap: wrap; }
  .pos { width: 40px; flex: none; color: var(--dim); padding-top: 1px; }
  .what { display: grid; gap: 4px; flex: 1; min-width: 200px; }
  .perms { display: flex; flex-wrap: wrap; gap: 4px; }
  .perms .ui-chip { text-transform: none; letter-spacing: 0; font-family: var(--font-mono); font-weight: 600; }
  .wide { width: min(560px, calc(100vw - 24px)); }
  .two { display: grid; grid-template-columns: minmax(0, 1fr) 110px; gap: 12px; }
  .num { text-align: right; }
  fieldset { margin: 0; padding: 0; border: 0; }
  legend { padding: 0; margin-bottom: 7px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 8px 12px; }
  .grid .ui-mono { font-size: .8rem; }
</style>
