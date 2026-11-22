<script lang="ts">
  // Admin: redeem code batches (codes.manage), a dense table; the picked batch (?batch=<id>) opens
  // in the inspector with its facts, uses (used, left) and Turn off. A batch is a name, a prefix, how many codes, how many
  // times each can be used (once per account) and what they give: a shop item's cosmetics and/or
  // badges and paints picked here. Making one shows the plain codes ONCE, with Copy all and
  // Download .txt: the server keeps only hashes, so they can't be shown again. Disable turns off
  // every code of a batch.
  // Arrows move over the rows, Enter picks, Escape closes the inspector.
  import { tick } from 'svelte';
  import { page as route } from '$app/state';
  import { gql } from '$lib/api';
  import { batchState, batchUsage } from '$lib/admin/shop-codes';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import IconButton from '$lib/components/IconButton.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, date, dateTime, iso } from '$lib/format';
  import type { CodeBatchFieldsFragment } from '$lib/gql/graphql';
  import { CODE_BATCHES, CREATE_CODE_BATCH, DISABLE_CODE_BATCH, SHOP_ITEMS } from '$lib/queries';
  import CosmeticsPick from '$lib/shop/CosmeticsPick.svelte';
  import CodesDialog from '$lib/shop/CodesDialog.svelte';
  import Gate from '$lib/shop/Gate.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import { fromLocalInput, parseCount, staffMessage } from '$lib/shop/shop';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
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

  const pickedBatch = $derived(L.data ? appendUnique(L.data.items, more).find((b) => b.id === pickedId) ?? null : null);

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

  const pickedId = $derived(route.url.searchParams.get('batch'));
  let body = $state<HTMLElement>();
  function pick(id: string | null) {
    void pickInUrl('batch', id);
  }
  // Closing the inspector puts focus back on the batch's row.
  function unpick() {
    const id = pickedId;
    pick(null);
    void tick().then(() => body?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id ?? '')}"]`)?.focus());
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !pickedId || e.defaultPrevented || document.querySelector('dialog[open]')) return;
    if ((e.target as HTMLElement | null)?.closest('input, textarea, select')) return;
    e.preventDefault();
    unpick();
  }
  function rowKey(e: KeyboardEvent, id: string) {
    if ((e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget) {
      e.preventDefault();
      pick(id);
    }
  }
  const gives = (b: CodeBatchFieldsFragment) =>
    [b.itemName ? clean(b.itemName) : '', b.badgeIds.length ? `${count(b.badgeIds.length)} ${b.badgeIds.length === 1 ? 'badge' : 'badges'}` : '', b.paintIds.length ? `${count(b.paintIds.length)} ${b.paintIds.length === 1 ? 'paint' : 'paints'}` : '']
      .filter(Boolean)
      .join(' + ') || 'Nothing';
</script>

<svelte:head><title>{pickedBatch ? `${clean(pickedBatch.name)} - Codes` : 'Codes'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Gate permission="codes.manage">
  <Split label="Code batch" open={!!pickedBatch} onclose={() => pick(null)}>
    <Bar title="Codes" count={L.data ? `${count(L.data.total)} ${L.data.total === 1 ? 'batch' : 'batches'}` : null}>
      {#snippet actions()}
        <button class="ui-btn primary small" type="button" disabled={!L.data} onclick={() => L.data && make(L.data)}><Icon name="plus" size={15} />New batch</button>
      {/snippet}
    </Bar>

    <LoadGate source={L} label="Loading batches…">
      {#snippet children(d)}
        {@const rows = appendUnique(d.items, more)}
        {#if !rows.length}
          <State title="No batches yet.">
            Make one for a giveaway: pick what it gives and how many codes you need.
            {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => make(d)}>New batch</button>{/snippet}
          </State>
        {:else}
          <table class="adm-table stack">
            <thead>
              <tr><th>Batch</th><th class="num" style="width:64px">Codes</th><th class="num" style="width:100px">Used</th><th style="width:92px">State</th><th style="width:100px">Expires</th></tr>
            </thead>
            <tbody bind:this={body} use:gridKeys={'tr[data-item]'}>
              {#each rows as b, n (b.id)}
                {@const st = batchState(b)}
                {@const u = batchUsage(b)}
                <tr data-row data-item={b.id} aria-selected={b.id === pickedId} tabindex={(pickedBatch ? b.id === pickedId : n === 0) ? 0 : -1}
                  onclick={() => pick(b.id)} onkeydown={(e) => rowKey(e, b.id)}>
                  <td class="ell pk"><span class="txt"><span class="nm"><bdi>{clean(b.name)}</bdi><span class="pre">{b.prefix}-…</span></span><span class="sub"><bdi>{gives(b)}</bdi></span></span></td>
                  <td class="num" data-l="Codes">{count(b.codeCount)}</td>
                  <td class="num" data-l="Used">{count(u.used)} of {count(u.total)}</td>
                  <td data-l="State"><span class="adm-flags"><span class:ok={st === 'Active'}>{st}</span></span></td>
                  <td class="dim mono" data-l="Expires">{b.expiresAt ? date(b.expiresAt) : 'Never'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if rows.length < d.total}
            <div class="adm-pager">
              <button class="ui-btn secondary small" type="button" disabled={loadingMore} onclick={next}>{loadingMore ? 'Loading…' : 'Show more'}</button>
              <span>{count(rows.length)} of {count(d.total)}</span>
            </div>
          {/if}
        {/if}
      {/snippet}
    </LoadGate>

    {#snippet empty()}
      <div class="adm-ip-head"><span class="adm-label">Batch</span></div>
      <p class="none">Pick a batch to see how much of it was used, or turn it off.</p>
    {/snippet}

    {#snippet panel()}
      {#if pickedBatch}
        {@const b = pickedBatch}
        {@const st = batchState(b)}
        {@const u = batchUsage(b)}
        <div class="adm-ip-head">
          <span class="adm-label">Batch</span>
          <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={unpick} />
        </div>
        <div class="adm-ip-body">
          <section class="adm-sec">
            <h2 class="adm-name"><bdi>{clean(b.name)}</bdi></h2>
            <p class="adm-flags"><span class:ok={st === 'Active'}>{st}</span><span class="adm-num">{b.prefix}-…</span></p>
          </section>
          <section class="adm-sec">
            <h3 class="adm-label">Uses</h3>
            <dl class="adm-kv">
              <dt>Codes</dt><dd>{count(b.codeCount)}</dd>
              <dt>Uses per code</dt><dd>{count(b.usesPerCode)}</dd>
              <dt>Used</dt><dd>{count(u.used)} of {count(u.total)}</dd>
              <dt>Unused</dt><dd>{count(u.left)}</dd>
            </dl>
          </section>
          <section class="adm-sec">
            <h3 class="adm-label">Facts</h3>
            <dl class="adm-kv">
              <dt>Gives</dt><dd class="txt">{gives(b)}</dd>
              <dt>Expires</dt><dd>{b.expiresAt ? dateTime(b.expiresAt) : 'Never'}</dd>
              {#if b.disabledAt}<dt>Turned off</dt><dd>{dateTime(b.disabledAt)}</dd>{/if}
              <dt>Made</dt><dd>{dateTime(b.createdAt)}</dd>
              {#if b.createdBy}<dt>Made by</dt><dd class="txt"><bdi>{clean(b.createdBy.displayName)}</bdi></dd>{/if}
              <dt>ID</dt><dd class="mono">{b.id}</dd>
            </dl>
          </section>
          {#if st === 'Active'}
            <section class="adm-sec">
              <div class="adm-acts row">
                <Confirm small label="Turn off" confirm="Click again to turn off every code" {busy} onconfirm={() => disable(b)} />
              </div>
            </section>
          {/if}
        </div>
      {/if}
    {/snippet}
  </Split>

  <dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="batch-title">
    <form onsubmit={save} novalidate>
      <div class="body">
        <h2 id="batch-title">New code batch</h2>
        <h3 class="adm-label">Batch</h3>
        <div>
          <label class="adm-lv">
            <span class="l">Name<small>For staff</small></span>
            <span class="v"><input class="ui-input" bind:value={name} maxlength="64" aria-invalid={errors.name ? 'true' : undefined} />{#if errors.name}<span class="ui-error" role="alert">{errors.name}</span>{/if}</span>
          </label>
          <label class="adm-lv">
            <span class="l">Prefix<small>PREFIX-XXXX-XXXX-XXXX-XXXX</small></span>
            <span class="v"><input class="ui-input mono" bind:value={prefix} maxlength="12" autocapitalize="characters" autocomplete="off" aria-invalid={errors.prefix ? 'true' : undefined} />{#if errors.prefix}<span class="ui-error" role="alert">{errors.prefix}</span>{/if}</span>
          </label>
          <label class="adm-lv">
            <span class="l">How many codes<small>1 to 1.000</small></span>
            <span class="v"><input class="ui-input adm-num" inputmode="numeric" bind:value={amount} aria-invalid={errors.amount ? 'true' : undefined} />{#if errors.amount}<span class="ui-error" role="alert">{errors.amount}</span>{/if}</span>
          </label>
        </div>
        <h3 class="adm-label sect">Limits</h3>
        <div>
          <label class="adm-lv">
            <span class="l">Uses per code<small>Once per account</small></span>
            <span class="v"><input class="ui-input adm-num" inputmode="numeric" bind:value={uses} aria-invalid={errors.uses ? 'true' : undefined} />{#if errors.uses}<span class="ui-error" role="alert">{errors.uses}</span>{/if}</span>
          </label>
          <label class="adm-lv">
            <span class="l">Expires<small>Empty: never</small></span>
            <span class="v"><input class="ui-input" type="datetime-local" bind:value={expires} aria-invalid={errors.expires ? 'true' : undefined} />{#if errors.expires}<span class="ui-error" role="alert">{errors.expires}</span>{/if}</span>
          </label>
        </div>
        <h3 class="adm-label sect">Gives</h3>
        {#if itemChoices.length}
          <label class="adm-lv">
            <span class="l">Cosmetics of</span>
            <span class="v">
              <select class="ui-input" bind:value={itemId}>
                <option value="">No shop item</option>
                {#each itemChoices as i (i.id)}<option value={i.id}>{clean(i.name)}</option>{/each}
              </select>
            </span>
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
  .stack { table-layout: fixed; }
  tr[aria-selected='true'] .pk { font-weight: 700; }
  .txt { display: grid; line-height: 1.25; }
  .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); font-weight: 400; color: var(--dim); }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .pre { margin-left: 6px; font-family: var(--font-mono); font-size: var(--t-fine); color: var(--dim); }
  .pk { font-weight: 600; }
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; overflow-wrap: anywhere; }
  .sect { padding-top: 12px; border-top: 1px solid var(--border); }
  .mono { font-family: var(--font-mono); }
  .adm-lv .v .ui-input { flex: 1 1 180px; min-width: 0; }
  .adm-lv .v .ui-error { flex-basis: 100%; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
</style>
