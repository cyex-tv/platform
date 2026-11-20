<script lang="ts">
  // Every badge: picture, name and tooltip, how many own it; New badge / Edit in one dialog
  // (name, tooltip, picture upload that waits until its files are made); Give, Owners, Delete.
  // A badge made before the 128, 256 and 512 px sizes offers Make larger sizes (its files made
  // again from the stored original).
  import { gql } from '$lib/api';
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
  import PageHead from '$lib/tool/PageHead.svelte';
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

  let open = $state<string | null>(null);
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
      await L.reload();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Badges - Admin - cyex.tv</title></svelte:head>

<PageHead title="Badges" line="Chat badges shown before a name. Deleting one takes it from everyone.">
  {#snippet actions()}
    {#if manage}<button class="ui-btn primary" type="button" onclick={() => edit(null)}><Icon name="plus" size={15} />New badge</button>{/if}
  {/snippet}
</PageHead>

<LoadGate source={L} label="Loading badges…">
  {#snippet children(badges)}
    {#if !badges.length}
      <State title="No badges yet.">
        Make the first one: a name, a tooltip and a square picture.
        {#snippet actions()}{#if manage}<button class="ui-btn primary" type="button" onclick={() => edit(null)}>New badge</button>{/if}{/snippet}
      </State>
    {:else}
      <ul class="list">
        {#each badges as b (b.id)}
          <li class="item">
            <div class="row">
              <span class="pic"><BadgeIcon badge={b} size={36} /></span>
              <span class="what">
                <strong><bdi>{clean(b.name)}</bdi></strong>
                <span class="ui-hint"><bdi>{clean(b.tooltip)}</bdi></span>
              </span>
              {#if grant}
                <button class="ui-btn ghost small own" type="button" aria-expanded={open === b.id} onclick={() => (open = open === b.id ? null : b.id)}>
                  <Icon name="users" size={14} />{owners[b.id] === undefined ? 'Owners' : `${count(owners[b.id])} ${owners[b.id] === 1 ? 'owner' : 'owners'}`}
                </button>
                <button class="ui-btn secondary small" type="button" onclick={() => { giving = b; grantDlg?.open({ id: b.id, name: b.name }); }}>Give</button>
              {/if}
              {#if manage}
                {#if lacks(b)}
                  <button class="ui-btn secondary small" type="button" disabled={remaking[b.id]} title="Made before the 128, 256 and 512 px sizes: make them from the stored original" onclick={() => remake(b)}>
                    {#if remaking[b.id]}<span class="ui-spinner" aria-hidden="true"></span>Making sizes…{:else}<Icon name="refresh" size={14} />Make larger sizes{/if}
                  </button>
                {/if}
                <button class="ui-btn secondary small" type="button" onclick={() => edit(b)}><Icon name="pencil" size={14} />Edit</button>
                <Confirm small label="Delete" confirm={owners[b.id] ? `Click again: ${count(owners[b.id])} ${owners[b.id] === 1 ? 'owner' : 'owners'}` : 'Click again to delete it for everyone'} {busy} onconfirm={() => remove(b)} />
              {/if}
            </div>
            {#if open === b.id}
              <div class="owners"><Owners bind:this={ownerList} kind="BADGE" cosmeticId={b.id} name={clean(b.name)} onchange={(n) => (owners[b.id] = n)} /></div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}
</LoadGate>

<GrantDialog bind:this={grantDlg} kind="BADGE" ongranted={() => {
  if (giving) owners[giving.id] = (owners[giving.id] ?? 0) + 1;
  if (open === giving?.id) ownerList?.load(1);
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
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 12px; padding: 10px 14px; }
  .pic { display: grid; place-items: center; flex: none; width: 48px; height: 48px; border-radius: var(--radius); background: var(--bg); }
  .what { display: grid; flex: 1 1 180px; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .owners { border-top: 1px solid var(--border); }
  .own { font-variant-numeric: tabular-nums; }
</style>
