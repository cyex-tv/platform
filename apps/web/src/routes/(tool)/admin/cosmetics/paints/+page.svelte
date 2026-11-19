<script lang="ts">
  // Every paint: a preview on a sample name, its name and kind, how many own it, and Edit (the
  // paint studio), Give, Owners and Delete. "New paint" opens the studio.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { forgetCatalog, loadCatalog, type Paint } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import { DELETE_PAINT, ENTITLEMENTS } from '$lib/queries';
  import GrantDialog from '$lib/admin/GrantDialog.svelte';
  import Owners from '$lib/admin/Owners.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can, session } from '$lib/tool/session.svelte';

  const manage = $derived(can('cosmetics.manage'));
  const grant = $derived(can('cosmetics.grant'));
  const sample = $derived(session.me?.identities[0]?.displayName ?? 'cyex.tv');

  const L = loader(async () => {
    forgetCatalog();
    return [...(await loadCatalog()).paints.values()];
  });

  // How many own each paint: one small request per paint, together.
  let owners = $state<Record<string, number>>({});
  $effect(() => {
    const paints = L.data;
    if (!paints || !grant) return;
    for (const p of paints)
      gql(ENTITLEMENTS, { kind: 'PAINT', cosmeticId: p.id, page: 1, perPage: 1 })
        .then((r) => (owners[p.id] = r.cosmeticEntitlements.total))
        .catch(() => {});
  });

  let open = $state<string | null>(null);
  let giving = $state<Paint | null>(null);
  let dlg = $state<GrantDialog>();
  let ownerList = $state<Owners>();
  let busy = $state(false);
  const kindLabel = (p: Paint) => (p.kind === 'IMAGE' ? 'Picture' : p.kind === 'RADIAL_GRADIENT' ? 'Radial' : 'Linear');

  async function remove(p: Paint) {
    busy = true;
    try {
      await gql(DELETE_PAINT, { id: p.id });
      notify(`${iso(p.name)} deleted.`);
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Paints - cyex.tv</title></svelte:head>

<PageHead title="Paints" line="Name paints people show in chat. Deleting one takes it from everyone.">
  {#snippet actions()}
    {#if manage}<a class="ui-btn primary" href="/admin/cosmetics/paints/studio"><Icon name="plus" size={15} />New paint</a>{/if}
  {/snippet}
</PageHead>

<LoadGate source={L} label="Loading paints…">
  {#snippet children(paints)}
    {#if !paints.length}
      <State title="No paints yet.">
        Make the first one in the paint studio.
        {#snippet actions()}{#if manage}<a class="ui-btn primary" href="/admin/cosmetics/paints/studio">New paint</a>{/if}{/snippet}
      </State>
    {:else}
      <ul class="list">
        {#each paints as p (p.id)}
          <li class="item">
            <div class="row">
              <span class="prev"><PaintedName name={sample} paint={p} /></span>
              <span class="what">
                <strong><PaintedName name={p.name} paint={p} /></strong>
                <span class="ui-hint">{kindLabel(p)}{p.kind === 'LINEAR_GRADIENT' ? ` · ${p.angle}°` : ''}{p.repeat ? ' · repeats' : ''}{p.shadows.length ? ` · ${p.shadows.length} shadow${p.shadows.length > 1 ? 's' : ''}` : ''}</span>
              </span>
              {#if grant}
                <button class="ui-btn ghost small own" type="button" aria-expanded={open === p.id} onclick={() => (open = open === p.id ? null : p.id)}>
                  <Icon name="users" size={14} />{owners[p.id] === undefined ? 'Owners' : `${count(owners[p.id])} ${owners[p.id] === 1 ? 'owner' : 'owners'}`}
                </button>
                <button class="ui-btn secondary small" type="button" onclick={() => { giving = p; dlg?.open({ id: p.id, name: p.name }); }}>Give</button>
              {/if}
              {#if manage}
                <a class="ui-btn secondary small" href="/admin/cosmetics/paints/{encodeURIComponent(p.id)}/studio"><Icon name="pencil" size={14} />Edit</a>
                <Confirm small label="Delete" confirm="Click again to delete it for everyone" {busy} onconfirm={() => remove(p)} />
              {/if}
            </div>
            {#if open === p.id}
              <div class="owners"><Owners bind:this={ownerList} kind="PAINT" cosmeticId={p.id} name={clean(p.name)} onchange={(n) => (owners[p.id] = n)} /></div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}
</LoadGate>

<GrantDialog bind:this={dlg} kind="PAINT" ongranted={() => {
  if (giving) owners[giving.id] = (owners[giving.id] ?? 0) + 1;
  if (open === giving?.id) ownerList?.load(1);
}} />

<style>
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 12px; padding: 10px 14px; }
  .prev { display: flex; align-items: center; justify-content: center; flex: none; width: 168px; height: 44px; padding: 0 10px;
    border-radius: var(--radius); background: var(--bg); font-size: 17px; font-weight: 700; overflow: hidden; }
  .what { display: grid; flex: 1 1 180px; min-width: 0; }
  .what strong { font-weight: 600; display: flex; min-width: 0; }
  .owners { border-top: 1px solid var(--border); }
  .own { font-variant-numeric: tabular-nums; }
  @media (max-width: 520px) { .prev { width: 100%; } }
</style>
