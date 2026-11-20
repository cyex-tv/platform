<script lang="ts">
  // Admin: redeem code batches (codes.manage). A batch is a name, a prefix, how many codes, how many
  // times each can be used (once per account) and what they give: a shop item's cosmetics and/or
  // badges and paints picked here. Making one shows the plain codes ONCE, with Copy all and
  // Download .txt: the server keeps only hashes, so they can't be shown again. Disable turns off
  // every code of a batch.
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, date, iso } from '$lib/format';
  import type { CodeBatchFieldsFragment } from '$lib/gql/graphql';
  import { CODE_BATCHES, CREATE_CODE_BATCH, DISABLE_CODE_BATCH, SHOP_ITEMS } from '$lib/queries';
  import CosmeticsPick from '$lib/shop/CosmeticsPick.svelte';
  import CodesDialog from '$lib/shop/CodesDialog.svelte';
  import Gate from '$lib/shop/Gate.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import { fromLocalInput, parseCount, staffMessage } from '$lib/shop/shop';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { appendUnique, pageToken } from '$lib/tool/paging';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';

  const PER = 25;
  let more = $state<CodeBatchFieldsFragment[]>([]);
  let page = $state(1);
  let loadingMore = $state(false);
  let busy = $state(false);

  const token = pageToken();
  const L = loader(async ({ fetch }) => {
    token.reset();
    more = [];
    page = 1;
    loadingMore = false;
    const [batches, items, catalog] = await Promise.all([gql(CODE_BATCHES, { page: 1, perPage: PER }, fetch), canItems() ? gql(SHOP_ITEMS, { all: true }, fetch) : null, loadCatalog()]);
    return { ...batches.codeBatches, shopItems: items?.shopItems ?? [], catalog };
  });
  // Item choices need shop.manage; without it the batch just gives picked cosmetics.
  const canItems = () => can('shop.manage');

  async function next() {
    if (loadingMore) return;
    const live = token.begin();
    loadingMore = true;
    try {
      const r = (await gql(CODE_BATCHES, { page: page + 1, perPage: PER })).codeBatches;
      if (!live()) return; // the list was reloaded while this loaded
      page += 1;
      more = appendUnique(more, r.items);
    } catch (e) {
      if (live()) fail(e);
    } finally {
      if (live()) loadingMore = false;
    }
  }

  // The make-a-batch form.
  let dialog: HTMLDialogElement;
  let catalog = $state<Catalog | null>(null);
  let itemChoices = $state<{ id: string; name: string }[]>([]);
  let name = $state('');
  let prefix = $state('');
  let amount = $state('10');
  let uses = $state('1');
  let expires = $state('');
  let itemId = $state('');
  let badgeIds = $state<string[]>([]);
  let paintIds = $state<string[]>([]);
  let tried = $state(false);
  let saving = $state(false);
  let problem = $state<string | null>(null);

  // "1000" or the site's own "1.000".
  const num = parseCount;
  const errors = $derived({
    name: tried && (!name.trim() || name.trim().length > 64) ? 'Give it a name of 1 to 64 characters. Only staff see it.' : null,
    prefix: tried && !/^[A-Za-z0-9]{1,12}$/.test(prefix.trim()) ? 'Use 1 to 12 letters or digits.' : null,
    amount: tried && !(num(amount) >= 1 && num(amount) <= 1000) ? 'Make 1 to 1.000 codes.' : null,
    uses: tried && !(num(uses) >= 1 && num(uses) <= 10000) ? 'Use 1 to 10.000. Each account can use a code once.' : null,
    expires: tried && expires && Date.parse(expires) <= Date.now() ? 'Pick a time in the future, or leave it empty.' : null,
    gives: tried && !itemId && badgeIds.length + paintIds.length === 0 ? 'Pick what the codes give: an item, or badges and paints.' : null,
  });
  const invalid = $derived(Object.values(errors).some(Boolean));

  function make(d: { shopItems: { id: string; name: string }[]; catalog: Catalog }) {
    catalog = d.catalog;
    itemChoices = d.shopItems;
    name = '';
    prefix = '';
    amount = '10';
    uses = '1';
    expires = '';
    itemId = '';
    badgeIds = [];
    paintIds = [];
    tried = false;
    problem = null;
    dialog.showModal();
  }

  // The one view of the plain codes.
  let codesDialog: CodesDialog;

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    if (invalid) {
      // The errors render after `tried` flips: wait for them before looking for the first one.
      await tick();
      dialog.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
      return;
    }
    saving = true;
    problem = null;
    try {
      const r = (
        await gql(CREATE_CODE_BATCH, {
          input: {
            name: name.trim(),
            prefix: prefix.trim().toUpperCase(),
            count: num(amount),
            usesPerCode: num(uses),
            expiresAt: fromLocalInput(expires),
            itemId: itemId || null,
            badgeIds,
            paintIds,
          },
        })
      ).createCodeBatch;
      dialog.close();
      codesDialog.show(r.batch.name, r.codes);
      await L.reload();
      loadShopCounts();
    } catch (err) {
      problem = staffMessage(err);
    } finally {
      saving = false;
    }
  }

  async function disable(b: CodeBatchFieldsFragment) {
    busy = true;
    try {
      const r = (await gql(DISABLE_CODE_BATCH, { id: b.id })).disableCodeBatch;
      L.set(L.data ? { ...L.data, items: L.data.items.map((x) => (x.id === r.id ? r : x)) } : null);
      more = more.map((x) => (x.id === r.id ? r : x));
      notify(`${iso(b.name)} turned off.`);
    } catch (e) {
      notify(staffMessage(e), true);
    } finally {
      busy = false;
    }
  }

  const status = (b: CodeBatchFieldsFragment) => (b.disabledAt ? 'Turned off' : b.expiresAt && Date.parse(b.expiresAt) <= Date.now() ? 'Expired' : 'Active');
  const meta = (b: CodeBatchFieldsFragment) =>
    [
      `${b.prefix}-…`,
      gives(b),
      `${count(b.codeCount)} ${b.codeCount === 1 ? 'code' : 'codes'}, ${count(b.usesPerCode)} ${b.usesPerCode === 1 ? 'use' : 'uses'} each`,
      b.expiresAt ? `expires ${date(b.expiresAt)}` : '',
    ]
      .filter(Boolean)
      .join(' · ');
  const gives = (b: CodeBatchFieldsFragment) =>
    [b.itemName ? clean(b.itemName) : '', b.badgeIds.length ? `${count(b.badgeIds.length)} ${b.badgeIds.length === 1 ? 'badge' : 'badges'}` : '', b.paintIds.length ? `${count(b.paintIds.length)} ${b.paintIds.length === 1 ? 'paint' : 'paints'}` : '']
      .filter(Boolean)
      .join(' + ') || 'Nothing';
</script>

<svelte:head><title>Codes - Admin - cyex.tv</title></svelte:head>

<Gate permission="codes.manage">
  <PageHead title="Codes" line="Batches of redeem codes for giveaways and events. The codes are shown once, when you make the batch.">
    {#snippet actions()}
      <button class="ui-btn primary" type="button" disabled={!L.data} onclick={() => L.data && make(L.data)}><Icon name="plus" size={15} />New batch</button>
    {/snippet}
  </PageHead>

  <LoadGate source={L} label="Loading batches…">
    {#snippet children(d)}
      {@const rows = appendUnique(d.items, more)}
      {#if !rows.length}
        <State title="No batches yet.">
          Make one for a giveaway: pick what it gives and how many codes you need.
          {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => make(d)}>New batch</button>{/snippet}
        </State>
      {:else}
        <ul class="list">
          {#each rows as b (b.id)}
            {@const st = status(b)}
            <li class="item">
              <div class="row">
                <span class="what">
                  <strong><bdi>{clean(b.name)}</bdi></strong>
                  <span class="ui-hint">
                    {meta(b)}
                  </span>
                </span>
                <span class="used ui-num">{count(b.redemptions)} redeemed</span>
                <span class="st" class:live={st === 'Active'}>{st}</span>
                {#if st === 'Active'}
                  <Confirm small label="Turn off" confirm="Click again to turn off every code" {busy} onconfirm={() => disable(b)} />
                {:else}
                  <span class="gap" aria-hidden="true"></span>
                {/if}
              </div>
            </li>
          {/each}
        </ul>
        {#if rows.length < d.total}
          <div class="foot">
            <button class="ui-btn secondary" type="button" disabled={loadingMore} onclick={next}>{loadingMore ? 'Loading…' : 'Show more'}</button>
            <span class="ui-hint">{count(rows.length)} of {count(d.total)}</span>
          </div>
        {/if}
      {/if}
    {/snippet}
  </LoadGate>

  <dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="batch-title">
    <form onsubmit={save} novalidate>
      <div class="body">
        <h2 id="batch-title">New code batch</h2>
        <label class="ui-field">
          <span class="ui-label">Name</span>
          <input class="ui-input" bind:value={name} maxlength="64" aria-invalid={errors.name ? 'true' : undefined} />
          {#if errors.name}<span class="ui-error" role="alert">{errors.name}</span>{:else}<span class="ui-hint">For staff, like “Launch stream giveaway”.</span>{/if}
        </label>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Prefix</span>
            <input class="ui-input mono" bind:value={prefix} maxlength="12" autocapitalize="characters" autocomplete="off" aria-invalid={errors.prefix ? 'true' : undefined} />
            {#if errors.prefix}<span class="ui-error" role="alert">{errors.prefix}</span>{:else}<span class="ui-hint">Codes look like PREFIX-XXXX-XXXX-XXXX-XXXX.</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">How many codes</span>
            <input class="ui-input" inputmode="numeric" bind:value={amount} aria-invalid={errors.amount ? 'true' : undefined} />
            {#if errors.amount}<span class="ui-error" role="alert">{errors.amount}</span>{:else}<span class="ui-hint">1 to 1.000.</span>{/if}
          </label>
        </div>
        <div class="two">
          <label class="ui-field">
            <span class="ui-label">Uses per code</span>
            <input class="ui-input" inputmode="numeric" bind:value={uses} aria-invalid={errors.uses ? 'true' : undefined} />
            {#if errors.uses}<span class="ui-error" role="alert">{errors.uses}</span>{:else}<span class="ui-hint">Each account can use a code once.</span>{/if}
          </label>
          <label class="ui-field">
            <span class="ui-label">Expires</span>
            <input class="ui-input" type="datetime-local" bind:value={expires} aria-invalid={errors.expires ? 'true' : undefined} />
            {#if errors.expires}<span class="ui-error" role="alert">{errors.expires}</span>{:else}<span class="ui-hint">Empty: never.</span>{/if}
          </label>
        </div>
        {#if itemChoices.length}
          <label class="ui-field">
            <span class="ui-label">Gives the cosmetics of</span>
            <select class="ui-input" bind:value={itemId}>
              <option value="">No shop item</option>
              {#each itemChoices as i (i.id)}<option value={i.id}>{clean(i.name)}</option>{/each}
            </select>
          </label>
        {/if}
        {#if catalog}
          <CosmeticsPick badges={[...catalog.badges.values()]} paints={[...catalog.paints.values()]} bind:badgeIds bind:paintIds error={errors.gives} />
        {/if}
        {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      </div>
      <footer>
        <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
        <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Making…' : 'Make codes'}</button>
      </footer>
    </form>
  </dialog>

    <CodesDialog bind:this={codesDialog} />
</Gate>

<style>
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; padding: 10px 14px; }
  .what { display: grid; flex: 1 1 260px; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .used { flex: none; font-size: var(--t-small); color: var(--muted); }
  .st { flex: none; min-width: 72px; font-size: var(--t-small); color: var(--muted); }
  .st.live { color: var(--text); font-weight: 700; }
  .gap { flex: none; width: 84px; }
  .foot { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; overflow-wrap: anywhere; }
  .two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; align-items: start; }
  .mono { font-family: var(--font-mono); }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  @media (max-width: 520px) { .two { grid-template-columns: minmax(0, 1fr); } }
</style>
