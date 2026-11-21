<script lang="ts">
  // Every paint as a tile (a sample name painted on a well, its name, kind, how many own it) with
  // an inspector on the right for the picked one (?paint=<id>): a big preview, facts, Edit (the
  // paint studio), Make larger sizes, Delete and its owners (Give). "New paint" opens the studio.
  // A picture paint made before the 512 px size offers Make larger sizes (its files made again
  // from the stored original). Keyboard: arrows move over the tiles, Enter opens one in the
  // inspector, Escape closes it.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import IconButton from '$lib/components/IconButton.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { forgetCatalog, loadCatalog, type Paint } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import { COSMETIC_IMAGE, DELETE_PAINT, ENTITLEMENTS, REMAKE_COSMETIC_IMAGE } from '$lib/queries';
  import { lacksPaintSizes } from '$lib/cosmetics/paintFile';
  import { waitReady } from '$lib/cosmetics/upload';
  import GrantDialog from '$lib/admin/GrantDialog.svelte';
  import Owners from '$lib/admin/Owners.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
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

  // The picked paint lives in the address; the filter narrows the tiles by name.
  const pickedId = $derived(page.url.searchParams.get('paint'));
  const picked = $derived(L.data?.find((p) => p.id === pickedId) ?? null);
  let query = $state('');
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const all = L.data ?? [];
    return q ? all.filter((p) => p.name.toLowerCase().includes(q)) : all;
  });
  let grid = $state<HTMLElement>();
  function pick(id: string | null) {
    void pickInUrl('paint', id);
  }
  // Closing the inspector puts focus back on the paint's tile.
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
  let giving = $state<Paint | null>(null);
  let dlg = $state<GrantDialog>();
  let ownerList = $state<Owners>();
  let busy = $state(false);
  const kindLabel = (p: Paint) => (p.kind === 'IMAGE' ? 'Picture' : p.kind === 'RADIAL_GRADIENT' ? 'Radial gradient' : 'Linear gradient');
  const shadowsLabel = (p: Paint) => (p.shadows.length ? `${p.shadows.length} shadow${p.shadows.length > 1 ? 's' : ''}` : 'None');

  // Paints whose picture files are being made again.
  let remaking = $state<Record<string, boolean>>({});
  const lacks = (p: Paint) => p.kind === 'IMAGE' && !!p.image && lacksPaintSizes(p.image.files.map((f) => f.name));

  async function remake(p: Paint) {
    if (!p.image) return;
    const id = p.image.id;
    remaking[p.id] = true;
    try {
      await gql(REMAKE_COSMETIC_IMAGE, { id });
      // Done when the picture has every size; its old files stay in use meanwhile.
      await waitReady(async () => {
        const img = (await gql(COSMETIC_IMAGE, { id })).cosmeticImage;
        return img && { ...img, ready: img.ready && !lacksPaintSizes(img.files.map((f) => f.name)) };
      });
      notify(`${iso(p.name)} has every size now.`);
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      remaking[p.id] = false;
    }
  }

  async function remove(p: Paint) {
    busy = true;
    try {
      await gql(DELETE_PAINT, { id: p.id });
      notify(`${iso(p.name)} deleted.`);
      if (pickedId === p.id) pick(null);
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{picked ? `${clean(picked.name)} - Paints` : 'Paints'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Split label="Paint" open={!!picked} onclose={() => pick(null)}>
  <Bar title="Paints" count={L.data ? `${count(L.data.length)} ${L.data.length === 1 ? 'paint' : 'paints'}` : null}>
    {#snippet actions()}
      {#if manage}<a class="ui-btn primary small" href="/admin/cosmetics/paints/studio"><Icon name="plus" size={15} />New paint</a>{/if}
    {/snippet}
    {#snippet tools()}
      <label class="adm-find grow">
        <Icon name="search" size={15} />
        <span class="sr-only">Filter paints</span>
        <input class="ui-input" type="search" bind:value={query} placeholder="Filter by name" autocomplete="off" spellcheck="false" maxlength="64" />
      </label>
      {#if L.data && query.trim()}<span class="ui-hint adm-num">{count(shown.length)} of {count(L.data.length)}</span>{/if}
    {/snippet}
  </Bar>

  <LoadGate source={L} label="Loading paints…">
    {#snippet children(paints)}
      {#if !paints.length}
        <State title="No paints yet.">
          Make the first one in the paint studio.
          {#snippet actions()}{#if manage}<a class="ui-btn primary" href="/admin/cosmetics/paints/studio">New paint</a>{/if}{/snippet}
        </State>
      {:else if !shown.length}
        <State title="Nothing matches.">No paint has that name.</State>
      {:else}
        <ul class="adm-sheet tiles" style="--cell: 200px" bind:this={grid} use:gridKeys aria-label="Paints">
          {#each shown as p, i (p.id)}
            <li>
              <button class="adm-cell tile" type="button" data-item={p.id} aria-current={p.id === pickedId ? 'true' : undefined}
                tabindex={(pickedId && shown.some((x) => x.id === pickedId) ? p.id === pickedId : i === 0) ? 0 : -1} onclick={() => pick(p.id)}>
                <span class="tick"><Icon name="check" size={14} /></span>
                <span class="pic"><PaintedName name={sample} paint={p} /></span>
                <span class="nm"><bdi>{clean(p.name)}</bdi></span>
                <span class="sub">
                  <span>{kindLabel(p)}</span>
                  {#if grant && owners[p.id] !== undefined}<span class="adm-num">{count(owners[p.id])}</span>{/if}
                  {#if manage && lacks(p)}<span class="old" title="Made before the 512 px size">Old sizes</span>{/if}
                </span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Paint</span></div>
    <p class="none">Pick a paint to see it large, give it, or change it.</p>
  {/snippet}

  {#snippet panel()}
    {#if picked}
      {@const p = picked}
      <div class="adm-ip-head">
        <span class="adm-label">Paint</span>
        <IconButton label="Close" icon="x" size="s" onclick={unpick} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <div class="strip" aria-label="Preview">
            {#each [14, 20, 30] as px (px)}
              <span class="step"><span class="smp" style="font-size: {px}px"><PaintedName name={sample} paint={p} /></span><span class="ui-hint adm-num">{px}</span></span>
            {/each}
          </div>
          <h2 class="adm-name"><bdi>{clean(p.name)}</bdi></h2>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            <dt>Kind</dt><dd class="txt">{kindLabel(p)}</dd>
            {#if p.kind === 'LINEAR_GRADIENT'}<dt>Angle</dt><dd>{p.angle}°</dd>{/if}
            <dt>Repeat</dt><dd class="txt">{p.repeat ? 'Yes' : 'No'}</dd>
            <dt>Shadows</dt><dd class="txt">{shadowsLabel(p)}</dd>
            {#if grant}<dt>Owners</dt><dd>{owners[p.id] === undefined ? '…' : count(owners[p.id])}</dd>{/if}
            <dt>ID</dt><dd class="mono">{p.id}</dd>
          </dl>
        </section>
        {#if manage}
          <section class="adm-sec">
            <div class="adm-acts row">
              <a class="ui-btn secondary small" href="/admin/cosmetics/paints/{encodeURIComponent(p.id)}/studio"><Icon name="pencil" size={14} />Edit</a>
              {#if lacks(p)}
                <button class="ui-btn secondary small" type="button" disabled={remaking[p.id]} title="Made before the 512 px size: make it from the stored original" onclick={() => remake(p)}>
                  {#if remaking[p.id]}<span class="ui-spinner" aria-hidden="true"></span>Making sizes…{:else}<Icon name="refresh" size={14} />Make larger sizes{/if}
                </button>
              {/if}
              <Confirm small label="Delete" confirm={owners[p.id] ? `Click again: ${count(owners[p.id])} ${owners[p.id] === 1 ? 'owner' : 'owners'}` : 'Click again to delete it for everyone'} {busy} onconfirm={() => remove(p)} />
            </div>
          </section>
        {/if}
        {#if grant}
          <section class="adm-sec">
            <div class="sec-head">
              <h3 class="adm-label">Owners</h3>
              <button class="ui-btn secondary small" type="button" onclick={() => { giving = p; dlg?.open({ id: p.id, name: p.name }); }}><Icon name="user-plus" size={14} />Give</button>
            </div>
            {#key p.id}
              <div class="owners"><Owners bind:this={ownerList} kind="PAINT" cosmeticId={p.id} name={clean(p.name)} onchange={(n) => (owners[p.id] = n)} /></div>
            {/key}
          </section>
        {/if}
      </div>
    {/if}
  {/snippet}
</Split>

<GrantDialog bind:this={dlg} kind="PAINT" ongranted={() => {
  if (giving) owners[giving.id] = (owners[giving.id] ?? 0) + 1;
  if (pickedId === giving?.id) ownerList?.load(1);
}} />

<style>
  /* The contact sheet (admin.css .adm-sheet): the sample name painted on its own ground, the name, kind and owners. */
  .tile { height: 120px; }
  .tile .pic { width: 100%; height: 56px; margin-bottom: 4px; overflow: hidden; font-size: 20px; font-weight: 700; }
  .tile .sub { min-height: 18px; }
  .tile .sub > span + span::before { content: '·'; margin-right: 6px; color: var(--dim); }
  .old { color: var(--yellow); }
  .tile:active { opacity: .6; transition: none; }

  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .strip { display: grid; border: 1px solid var(--border); }
  .strip .step { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 52px; padding: 8px 12px; overflow: hidden; font-weight: 700; }
  .strip .step + .step { border-top: 1px solid var(--border); }
  .smp { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .sec-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .owners { margin: 0 -16px -12px; border-top: 1px solid var(--border); }
</style>
