<script lang="ts">
  // Every badge as a tile (picture, name, how many own it) with an inspector on the right for the
  // picked one (?badge=<id>): its sizes, tooltip and id, its owners (Give, Revoke) and Edit,
  // Make larger sizes, Delete. New badge / Edit share one dialog (name, tooltip, picture upload
  // that waits until its files are made). A badge made before the 128, 256 and 512 px sizes
  // offers Make larger sizes (its files made again from the stored original). Keyboard: arrows
  // move over the tiles, Enter opens one in the inspector, Escape closes it.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import IconButton from '$lib/components/IconButton.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import { forgetCatalog, loadCatalog, type Badge } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import { COSMETIC_IMAGE, CREATE_BADGE, DELETE_BADGE, ENTITLEMENTS, REMAKE_COSMETIC_IMAGE, UPDATE_BADGE } from '$lib/queries';
  import { lacksLadder } from '$lib/cosmetics/badgeFile';
  import { waitReady } from '$lib/cosmetics/upload';
  import GrantDialog from '$lib/admin/GrantDialog.svelte';
  import ImageUpload from '$lib/admin/ImageUpload.svelte';
  import Owners from '$lib/admin/Owners.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';

  const manage = $derived(can('cosmetics.manage'));
  const grant = $derived(can('cosmetics.grant'));

  const L = loader(async () => {
    forgetCatalog();
    return [...(await loadCatalog()).badges.values()];
  });

  let owners = $state<Record<string, number>>({});
  $effect(() => {
    const badges = L.data;
    if (!badges || !grant) return;
    for (const b of badges)
      gql(ENTITLEMENTS, { kind: 'BADGE', cosmeticId: b.id, page: 1, perPage: 1 })
        .then((r) => (owners[b.id] = r.cosmeticEntitlements.total))
        .catch(() => {});
  });

  // The picked badge lives in the address; the filter narrows the tiles by name or tooltip.
  const pickedId = $derived(page.url.searchParams.get('badge'));
  const picked = $derived(L.data?.find((b) => b.id === pickedId) ?? null);
  let query = $state('');
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const all = L.data ?? [];
    return q ? all.filter((b) => b.name.toLowerCase().includes(q) || b.tooltip.toLowerCase().includes(q)) : all;
  });
  let grid = $state<HTMLElement>();
  function pick(id: string | null) {
    void pickInUrl('badge', id);
  }
  // Closing the inspector puts focus back on the badge's tile.
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
  let giving = $state<Badge | null>(null);
  let grantDlg = $state<GrantDialog>();
  let ownerList = $state<Owners>();
  let busy = $state(false);

  // The badge dialog: new (editing null) or edit.
  let dialog: HTMLDialogElement;
  let editing = $state<Badge | null>(null);
  let name = $state('');
  let tooltip = $state('');
  let imageId = $state('');
  // The picture is uploading or its files are being made: saving waits for it.
  let picturing = $state(false);
  // A fresh picture field each time the dialog opens (no picture left over from the last badge).
  let opened = $state(0);
  let tried = $state(false);
  let saving = $state(false);
  // A save the API refused, shown in the dialog itself.
  let problem = $state('');
  const nameError = $derived(tried && !name.trim() ? 'Give it a name (1 to 32 characters).' : null);
  const tipError = $derived(tried && !tooltip.trim() ? 'Give it a tooltip (1 to 64 characters).' : null);
  const imageError = $derived(tried && !editing && !imageId && !picturing ? 'Add a picture.' : null);

  function edit(b: Badge | null) {
    editing = b;
    name = b?.name ?? '';
    tooltip = b?.tooltip ?? '';
    imageId = '';
    picturing = false;
    opened += 1;
    tried = false;
    problem = '';
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (nameError || tipError || imageError) return;
    saving = true;
    problem = '';
    try {
      // Saved while the picture is still on its way: wait for it, then save with it.
      // A picture that then fails says why in its field; nothing is saved without it.
      const waited = picturing;
      while (picturing) await new Promise((r) => setTimeout(r, 200));
      if (!dialog.open || ((waited || !editing) && !imageId)) return;
      if (editing) {
        await gql(UPDATE_BADGE, { id: editing.id, input: { name: name.trim(), tooltip: tooltip.trim(), imageId: imageId || null } });
        notify(`${iso(name.trim())} saved.`);
      } else {
        await gql(CREATE_BADGE, { input: { name: name.trim(), tooltip: tooltip.trim(), imageId } });
        notify(`${iso(name.trim())} made.`);
      }
      dialog.close();
      await L.reload();
    } catch (err) {
      problem = err instanceof Error ? err.message : 'That didn’t work.';
    } finally {
      saving = false;
    }
  }

  // Badges whose files are being made again.
  let remaking = $state<Record<string, boolean>>({});
  const lacks = (b: Badge) => lacksLadder(b.image.files.map((f) => f.name));

  async function remake(b: Badge) {
    remaking[b.id] = true;
    try {
      await gql(REMAKE_COSMETIC_IMAGE, { id: b.image.id });
      // Done when the picture has every size; its old files stay in use meanwhile.
      await waitReady(async () => {
        const img = (await gql(COSMETIC_IMAGE, { id: b.image.id })).cosmeticImage;
        return img && { ...img, ready: img.ready && !lacksLadder(img.files.map((f) => f.name)) };
      });
      notify(`${iso(b.name)} has every size now.`);
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      remaking[b.id] = false;
    }
  }

  async function remove(b: Badge) {
    busy = true;
    try {
      await gql(DELETE_BADGE, { id: b.id });
      notify(`${iso(b.name)} deleted.`);
      if (pickedId === b.id) pick(null);
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{picked ? `${clean(picked.name)} - Badges` : 'Badges'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Split label="Badge" open={!!picked} onclose={() => pick(null)}>
  <Bar title="Badges" count={L.data ? `${count(L.data.length)} ${L.data.length === 1 ? 'badge' : 'badges'}` : null}>
    {#snippet actions()}
      {#if manage}<button class="ui-btn primary small" type="button" onclick={() => edit(null)}><Icon name="plus" size={15} />New badge</button>{/if}
    {/snippet}
    {#snippet tools()}
      <label class="adm-find grow">
        <Icon name="search" size={15} />
        <span class="sr-only">Filter badges</span>
        <input class="ui-input" type="search" bind:value={query} placeholder="Filter by name or tooltip" autocomplete="off" spellcheck="false" maxlength="64" />
      </label>
      {#if L.data && query.trim()}<span class="ui-hint adm-num">{count(shown.length)} of {count(L.data.length)}</span>{/if}
    {/snippet}
  </Bar>

  <LoadGate source={L} label="Loading badges…">
    {#snippet children(badges)}
      {#if !badges.length}
        <State title="No badges yet.">
          Make the first one: a name, a tooltip and a square picture.
          {#snippet actions()}{#if manage}<button class="ui-btn primary" type="button" onclick={() => edit(null)}>New badge</button>{/if}{/snippet}
        </State>
      {:else}
        {#if !shown.length}
          <State title="Nothing matches.">No badge has that name or tooltip.</State>
        {:else}
          <ul class="adm-sheet tiles" style="--cell: 150px" bind:this={grid} use:gridKeys aria-label="Badges">
            {#each shown as b, i (b.id)}
              <li>
                <button class="adm-cell tile" type="button" data-item={b.id} aria-current={b.id === pickedId ? 'true' : undefined}
                  tabindex={(pickedId && shown.some((x) => x.id === pickedId) ? b.id === pickedId : i === 0) ? 0 : -1} onclick={() => pick(b.id)}>
                  <span class="tick"><Icon name="check" size={14} /></span>
                  <span class="pic"><BadgeIcon badge={b} size={36} /></span>
                  <span class="nm"><bdi>{clean(b.name)}</bdi></span>
                  <span class="sub">
                    {#if grant && owners[b.id] !== undefined}<span class="adm-num">{count(owners[b.id])}</span>{owners[b.id] === 1 ? 'owner' : 'owners'}{/if}
                    {#if manage && lacks(b)}<span class="old" title="Made before the larger sizes">Old sizes</span>{/if}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Badge</span></div>
    <p class="none">Pick a badge to see its sizes and owners, give it, or change it.</p>
  {/snippet}

  {#snippet panel()}
    {#if picked}
      {@const b = picked}
      <div class="adm-ip-head">
        <span class="adm-label">Badge</span>
        <IconButton label="Close" icon="x" size="s" onclick={unpick} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <div class="ladder" aria-label="Sizes">
            {#each [18, 36, 72] as px (px)}<span class="step"><BadgeIcon badge={b} size={px} /><span class="ui-hint ui-num">{px}</span></span>{/each}
          </div>
          <h2 class="adm-name"><bdi>{clean(b.name)}</bdi></h2>
          <p class="tip">“<bdi>{clean(b.tooltip)}</bdi>”</p>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            {#if grant}<dt>Owners</dt><dd>{owners[b.id] === undefined ? '…' : count(owners[b.id])}</dd>{/if}
            <dt>Sizes</dt><dd class="txt">{lacks(b) ? 'Up to 72 px' : 'Up to 512 px'}</dd>
            <dt>ID</dt><dd class="mono">{b.id}</dd>
          </dl>
        </section>
        {#if manage}
          <section class="adm-sec">
            <div class="adm-acts row">
              <button class="ui-btn secondary small" type="button" onclick={() => edit(b)}><Icon name="pencil" size={14} />Edit</button>
              {#if lacks(b)}
                <button class="ui-btn secondary small" type="button" disabled={remaking[b.id]} title="Made before the 128, 256 and 512 px sizes: make them from the stored original" onclick={() => remake(b)}>
                  {#if remaking[b.id]}<span class="ui-spinner" aria-hidden="true"></span>Making sizes…{:else}<Icon name="refresh" size={14} />Make larger sizes{/if}
                </button>
              {/if}
              <Confirm small label="Delete" confirm={owners[b.id] ? `Click again: ${count(owners[b.id])} ${owners[b.id] === 1 ? 'owner' : 'owners'}` : 'Click again to delete it for everyone'} {busy} onconfirm={() => remove(b)} />
            </div>
          </section>
        {/if}
        {#if grant}
          <section class="adm-sec">
            <div class="sec-head">
              <h3 class="adm-label">Owners</h3>
              <button class="ui-btn secondary small" type="button" onclick={() => { giving = b; grantDlg?.open({ id: b.id, name: b.name }); }}><Icon name="user-plus" size={14} />Give</button>
            </div>
            {#key b.id}
              <div class="owners"><Owners bind:this={ownerList} kind="BADGE" cosmeticId={b.id} name={clean(b.name)} onchange={(n) => (owners[b.id] = n)} /></div>
            {/key}
          </section>
        {/if}
      </div>
    {/if}
  {/snippet}
</Split>

<GrantDialog bind:this={grantDlg} kind="BADGE" ongranted={() => {
  if (giving) owners[giving.id] = (owners[giving.id] ?? 0) + 1;
  if (pickedId === giving?.id) ownerList?.load(1);
}} />

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="badge-title">
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="badge-title">{editing ? `Edit ${iso(editing.name)}` : 'New badge'}</h2>
      <label class="ui-field">
        <span class="ui-label">Name</span>
        <input class="ui-input" bind:value={name} maxlength="32" aria-invalid={nameError ? 'true' : undefined} aria-describedby={nameError ? 'b-name-err' : undefined} />
        {#if nameError}<span class="ui-error" id="b-name-err">{nameError}</span>{/if}
      </label>
      <label class="ui-field">
        <span class="ui-label">Tooltip</span>
        <input class="ui-input" bind:value={tooltip} maxlength="64" placeholder="What chat sees on hover" aria-invalid={tipError ? 'true' : undefined} aria-describedby={tipError ? 'b-tip-err' : undefined} />
        {#if tipError}<span class="ui-error" id="b-tip-err">{tipError}</span>{/if}
      </label>
      {#key opened}
        <ImageUpload kind="badge" label="Picture" bind:imageId bind:busy={picturing} current={editing?.image ?? null} />
      {/key}
      {#if imageError}<span class="ui-error" role="alert">{imageError}</span>{/if}
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? (picturing ? 'Waiting for the picture…' : 'Saving…') : editing ? 'Save' : 'Make badge'}</button>
    </footer>
  </form>
</dialog>

<style>
  /* The contact sheet (admin.css .adm-sheet): the badge on its own ground, the name, how many own it. */
  .tile { height: 132px; justify-items: center; text-align: center; }
  .tile .sub { justify-content: center; min-height: 18px; }
  .old { color: var(--yellow); }
  .tile:active { opacity: .6; transition: none; }

  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .ladder { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); align-items: end; border: 1px solid var(--border); }
  .ladder .step { min-height: 112px; padding: 14px 8px 8px; align-content: end; box-shadow: -1px 0 0 var(--border); }
  .step { display: grid; justify-items: center; gap: 6px; }
  .tip { margin: 0; font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .sec-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .owners { margin: 0 -16px -12px; border-top: 1px solid var(--border); }
</style>
