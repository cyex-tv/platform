<script lang="ts">
  // Admin: every shop item, hidden and scheduled ones too (shop.manage). A dense table: contents,
  // state (on sale, hidden, scheduled, ended, sold out), price, stock and sale window. The picked
  // item (?item=<id>) opens in the inspector with its facts and Edit, View, Delete. New item and
  // Edit share one dialog; Delete is soft (orders stay) and asks twice. "View" opens the item as
  // the shop shows it, which also works for a hidden one. Arrows move over the rows, Enter picks,
  // Escape closes the inspector.
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import IconButton from '$lib/components/IconButton.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, date, dateTime, iso } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { CREATE_SHOP_ITEM, DELETE_SHOP_ITEM, SHOP_ITEMS, UPDATE_SHOP_ITEM } from '$lib/queries';
  import CosmeticsPick from '$lib/shop/CosmeticsPick.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import {
    centsFrom, centsText, fromLocalInput, ITEM_STATE_TEXT, itemState, price, staffMessage, toLocalInput,
  } from '$lib/shop/shop';
  import Gate from '$lib/shop/Gate.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const L = loader(async ({ fetch }) => {
    const [items, catalog] = await Promise.all([gql(SHOP_ITEMS, { all: true }, fetch), loadCatalog()]);
    return { items: items.shopItems, catalog };
  });

  let busy = $state(false);

  // The form: new (editing null) or edit.
  let dialog: HTMLDialogElement;
  let editing = $state<ShopItemFieldsFragment | null>(null);
  let catalog = $state<Catalog | null>(null);
  let name = $state('');
  let description = $state('');
  let priceText = $state('');
  let currency = $state('EUR');
  let priceId = $state('');
  let badgeIds = $state<string[]>([]);
  let paintIds = $state<string[]>([]);
  let from = $state('');
  let to = $state('');
  let stock = $state('');
  let visible = $state(true);
  let sort = $state('0');
  let tried = $state(false);
  let saving = $state(false);
  let problem = $state<string | null>(null);

  const cents = $derived(centsFrom(priceText));
  const stockN = $derived(stock.trim() === '' ? null : /^\d{1,9}$/.test(stock.trim()) ? Number(stock.trim()) : NaN);
  const errors = $derived({
    name: tried && (!name.trim() || name.trim().length > 64) ? 'Give it a name of 1 to 64 characters.' : null,
    price: tried && (cents === null || cents > 100000) ? 'Enter a price from 0 to 1.000,00, like 4,99.' : null,
    currency: tried && !/^[A-Za-z]{3}$/.test(currency.trim()) ? 'Use a three-letter currency code like EUR.' : null,
    priceId: tried && !/^pri_[a-z0-9]{1,64}$/.test(priceId.trim()) ? 'Use the Paddle price ID, like pri_01abc. Without one nobody can buy it.' : null,
    cosmetics: tried && (badgeIds.length + paintIds.length < 1 || badgeIds.length + paintIds.length > 10) ? 'Pick 1 to 10 badges and paints.' : null,
    window: tried && from && to && Date.parse(to) <= Date.parse(from) ? 'The sale has to end after it starts.' : null,
    stock: tried && stockN !== null && (!Number.isInteger(stockN) || stockN < 1) ? 'Use 1 or more, or leave it empty for no limit.' : null,
    sort: tried && !/^-?\d{1,9}$/.test(sort.trim()) ? 'Use a whole number. Lower comes first.' : null,
  });
  const invalid = $derived(Object.values(errors).some(Boolean));

  function edit(item: ShopItemFieldsFragment | null, cat: Catalog) {
    editing = item;
    catalog = cat;
    name = item?.name ?? '';
    description = item?.description ?? '';
    priceText = item ? centsText(item.priceCents) : '';
    currency = item?.currency ?? 'EUR';
    priceId = item?.paddlePriceId ?? '';
    badgeIds = item?.badges.map((b) => b.id) ?? [];
    paintIds = item?.paints.map((p) => p.id) ?? [];
    from = toLocalInput(item?.activeFrom);
    to = toLocalInput(item?.activeTo);
    stock = item?.stockLimit != null ? String(item.stockLimit) : '';
    visible = item?.visible ?? true;
    sort = String(item?.sortOrder ?? 0);
    tried = false;
    problem = null;
    dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    problem = null;
    if (cents === null || invalid) {
      dialog.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
      return;
    }
    const input = {
      name: name.trim(),
      description: description.trim() || null,
      priceCents: cents,
      currency: currency.trim().toUpperCase(),
      paddlePriceId: priceId.trim(),
      badgeIds,
      paintIds,
      activeFrom: fromLocalInput(from),
      activeTo: fromLocalInput(to),
      stockLimit: stockN,
      visible,
      sortOrder: Number(sort.trim()),
    };
    saving = true;
    try {
      if (editing) await gql(UPDATE_SHOP_ITEM, { id: editing.id, input });
      else await gql(CREATE_SHOP_ITEM, { input });
      notify(`${iso(input.name)} ${editing ? 'saved' : 'made'}.`);
      dialog.close();
      await L.reload();
      loadShopCounts();
    } catch (err) {
      problem = staffMessage(err);
    } finally {
      saving = false;
    }
  }

  async function remove(item: ShopItemFieldsFragment) {
    busy = true;
    try {
      await gql(DELETE_SHOP_ITEM, { id: item.id });
      notify(`${iso(item.name)} deleted. Its orders stay.`);
      await L.reload();
      loadShopCounts();
    } catch (e) {
      fail(e);
    } finally {
      busy = false;
    }
  }

  const pickedId = $derived(page.url.searchParams.get('item'));
  const picked = $derived(L.data?.items.find((i) => i.id === pickedId) ?? null);
  let body = $state<HTMLElement>();
  function pick(id: string | null) {
    void pickInUrl('item', id);
  }
  // Closing the inspector puts focus back on the item's row.
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

  const plural = (n: number, one: string, many: string) => `${count(n)} ${n === 1 ? one : many}`;
  const contents = (i: ShopItemFieldsFragment) =>
    [i.badges.length ? plural(i.badges.length, 'badge', 'badges') : '', i.paints.length ? plural(i.paints.length, 'paint', 'paints') : '']
      .filter(Boolean)
      .join(' and ');
  const stockOf = (i: ShopItemFieldsFragment) => (i.stockLimit != null ? `${count(i.stockLeft ?? 0)} of ${count(i.stockLimit)}` : 'No limit');
  const sale = (i: ShopItemFieldsFragment) => (i.activeFrom || i.activeTo ? `${i.activeFrom ? date(i.activeFrom) : 'Now'} to ${i.activeTo ? date(i.activeTo) : 'no end'}` : 'Always');
</script>

<svelte:head><title>{picked ? `${clean(picked.name)} - Shop items` : 'Shop items'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Gate permission="shop.manage">
<Split label="Shop item" open={!!picked} onclose={() => pick(null)}>
  <PageHead title="Shop items" line="What is for sale: a bundle of badges and paints with a price.">
    {#snippet actions()}
      <button class="ui-btn primary" type="button" disabled={!L.data} onclick={() => L.data && edit(null, L.data.catalog)}><Icon name="plus" size={15} />New item</button>
    {/snippet}
  </PageHead>

  <LoadGate source={L} label="Loading items…">
    {#snippet children(d)}
      {#if !d.items.length}
        <State title="Nothing for sale yet.">
          Make the first item: pick its badges and paints, set a price and paste the Paddle price ID.
          {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => edit(null, d.catalog)}>New item</button>{/snippet}
        </State>
      {:else}
        <table class="adm-table stack">
          <thead>
            <tr><th>Item</th><th style="width:80px">State</th><th class="num" style="width:76px">Price</th><th class="num" style="width:88px">Stock</th><th style="width:166px">Sale</th></tr>
          </thead>
          <tbody bind:this={body} use:gridKeys={'tr[data-item]'}>
            {#each d.items as i, n (i.id)}
              {@const st = itemState(i)}
              <tr data-row data-item={i.id} aria-selected={i.id === pickedId} tabindex={(picked ? i.id === pickedId : n === 0) ? 0 : -1}
                onclick={() => pick(i.id)} onkeydown={(e) => rowKey(e, i.id)}>
                <td class="ell pk">
                  <span class="lead-in">
                    <span class="pics" aria-hidden="true">{#each i.badges.slice(0, 3) as b (b.id)}<BadgeIcon badge={b} size={20} />{/each}</span>
                    <span class="txt">
                      <span class="nm"><bdi>{clean(i.name)}</bdi></span>
                      <span class="sub">{contents(i)}</span>
                    </span>
                  </span>
                </td>
                <td data-l="State"><span class:live={st === 'on_sale'} class="state">{ITEM_STATE_TEXT[st]}</span></td>
                <td class="num" data-l="Price">{price(i.priceCents, i.currency)}</td>
                <td class="num dim" data-l="Stock">{stockOf(i)}</td>
                <td class="ell dim" data-l="Sale">{sale(i)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Item</span></div>
    <p class="none">Pick an item to see its price, sale window and contents.</p>
  {/snippet}

  {#snippet panel()}
    {#if picked && L.data}
      {@const i = picked}
      {@const st = itemState(i)}
      <div class="adm-ip-head">
        <span class="adm-label">Item</span>
        <IconButton label="Close" icon="x" size="s" onclick={unpick} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi>{clean(i.name)}</bdi></h2>
          {#if i.description}<p class="desc"><bdi>{clean(i.description)}</bdi></p>{/if}
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            <dt>State</dt><dd>{ITEM_STATE_TEXT[st]}</dd>
            <dt>Price</dt><dd>{price(i.priceCents, i.currency)}</dd>
            <dt>Stock</dt><dd>{stockOf(i)}{i.stockLimit != null ? ' left' : ''}</dd>
            <dt>On sale from</dt><dd>{i.activeFrom ? dateTime(i.activeFrom) : 'Already'}</dd>
            <dt>On sale until</dt><dd>{i.activeTo ? dateTime(i.activeTo) : 'No end'}</dd>
            <dt>Listed</dt><dd>{i.visible ? 'Yes' : 'No'}</dd>
            <dt>Sort order</dt><dd>{i.sortOrder}</dd>
            <dt>Paddle price</dt><dd class="mono">{i.paddlePriceId}</dd>
            <dt>ID</dt><dd class="mono">{i.id}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Contents <span class="n">{contents(i) || 'Empty'}</span></h3>
          <ul class="bundle">
            {#each i.badges as b (b.id)}<li><BadgeIcon badge={b} size={20} /><span><bdi>{clean(b.name)}</bdi></span><span class="kind">Badge</span></li>{/each}
            {#each i.paints as p (p.id)}<li><Icon name="palette" size={20} /><span><bdi>{clean(p.name)}</bdi></span><span class="kind">Paint</span></li>{/each}
          </ul>
        </section>
        <section class="adm-sec">
          <div class="adm-acts row">
            {#if st !== 'deleted'}
              <button class="ui-btn secondary small" type="button" onclick={() => edit(i, L.data!.catalog)}><Icon name="pencil" size={14} />Edit</button>
            {/if}
            <a class="ui-btn ghost small" href="/shop?item={encodeURIComponent(i.id)}">View in shop</a>
            {#if st !== 'deleted'}
              <Confirm small label="Delete" confirm="Click again to delete it" {busy} onconfirm={() => remove(i)} />
            {/if}
          </div>
        </section>
      </div>
    {/if}
  {/snippet}
</Split>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="item-title">
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="item-title">{editing ? `Edit ${iso(editing.name)}` : 'New shop item'}</h2>
      <h3 class="adm-label">Item</h3>
      <label class="ui-field">
        <span class="ui-label">Name</span>
        <input class="ui-input" bind:value={name} maxlength="64" aria-invalid={errors.name ? 'true' : undefined} />
        {#if errors.name}<span class="ui-error">{errors.name}</span>{/if}
      </label>
      <label class="ui-field">
        <span class="ui-label">Description</span>
        <textarea class="ui-input" rows="3" maxlength="500" bind:value={description}></textarea>
      </label>
      <h3 class="adm-label sect">Price</h3>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Price</span>
          <input class="ui-input" inputmode="decimal" bind:value={priceText} placeholder="4,99" aria-invalid={errors.price ? 'true' : undefined} />
          {#if errors.price}<span class="ui-error">{errors.price}</span>{/if}
        </label>
        <label class="ui-field">
          <span class="ui-label">Currency</span>
          <input class="ui-input" bind:value={currency} maxlength="3" autocapitalize="characters" aria-invalid={errors.currency ? 'true' : undefined} />
          {#if errors.currency}<span class="ui-error">{errors.currency}</span>{/if}
        </label>
      </div>
      <label class="ui-field">
        <span class="ui-label">Paddle price ID</span>
        <input class="ui-input mono" bind:value={priceId} placeholder="pri_01abc…" autocomplete="off" spellcheck="false" aria-invalid={errors.priceId ? 'true' : undefined} />
        {#if errors.priceId}<span class="ui-error">{errors.priceId}</span>{/if}
      </label>
      <h3 class="adm-label sect">Contents</h3>
      {#if catalog}
        <CosmeticsPick badges={[...catalog.badges.values()]} paints={[...catalog.paints.values()]} bind:badgeIds bind:paintIds error={errors.cosmetics} />
      {/if}
      <h3 class="adm-label sect">Availability</h3>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">On sale from (empty: now)</span>
          <input class="ui-input" type="datetime-local" bind:value={from} />
        </label>
        <label class="ui-field">
          <span class="ui-label">On sale until (empty: no end)</span>
          <input class="ui-input" type="datetime-local" bind:value={to} aria-invalid={errors.window ? 'true' : undefined} />
          {#if errors.window}<span class="ui-error">{errors.window}</span>{/if}
        </label>
      </div>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Copies (empty: no limit)</span>
          <input class="ui-input" inputmode="numeric" bind:value={stock} placeholder="No limit" aria-invalid={errors.stock ? 'true' : undefined} />
          {#if errors.stock}<span class="ui-error">{errors.stock}</span>{/if}
        </label>
        <label class="ui-field">
          <span class="ui-label">Sort order (lower first)</span>
          <input class="ui-input" inputmode="numeric" bind:value={sort} aria-invalid={errors.sort ? 'true' : undefined} />
          {#if errors.sort}<span class="ui-error">{errors.sort}</span>{/if}
        </label>
      </div>
      <label class="ui-check"><input type="checkbox" bind:checked={visible} />Listed in the shop</label>
      {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : editing ? 'Save' : 'Make item'}</button>
    </footer>
  </form>
</dialog>
</Gate>

<style>
  .stack { table-layout: fixed; }
  tr[aria-selected='true'] .pk { font-weight: 700; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .lead-in { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .pics { display: flex; flex: none; gap: 2px; min-width: 20px; }
  tr[aria-selected='true'] .nm { font-weight: 700; }
  .txt { display: grid; min-width: 0; line-height: 1.25; }
  .nm { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); color: var(--dim); font-weight: 400; }
  .state { color: var(--muted); }
  .state.live { color: var(--text); font-weight: 700; }
  .desc { margin: 0; font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .bundle { list-style: none; margin: 0; padding: 0; display: grid; }
  .bundle li { display: flex; align-items: center; gap: 10px; min-height: 36px; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .bundle li:last-child { border-bottom: 0; }
  .bundle li > span:first-of-type { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bundle :global(svg) { flex: none; color: var(--dim); }
  .kind { font-size: var(--t-fine); color: var(--dim); }
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; }
  .two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; align-items: start; }
  .sect { padding-top: 12px; border-top: 1px solid var(--border); }
  .mono { font-family: var(--font-mono); }
  textarea { resize: vertical; min-height: 72px; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  @media (max-width: 520px) { .two { grid-template-columns: minmax(0, 1fr); } }
  /* Phones: each row stacks, its cells labelled. */
  @media (max-width: 719.98px) {
    .stack thead { display: none; }
    .stack, .stack tbody { display: block; }
    .stack tr { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 2px 16px; padding: 10px 12px; min-height: 44px; border-bottom: 1px solid var(--border); }
    .stack tbody tr:last-child { border-bottom: 0; }
    .stack td { display: flex; justify-content: space-between; gap: 8px; height: auto; padding: 2px 0; border: 0; text-align: left; }
    .stack td.pk { grid-column: 1 / -1; max-width: none; }
    .stack td.ell { overflow: visible; white-space: normal; text-overflow: clip; }
    .stack td[data-l]::before { content: attr(data-l); color: var(--dim); font-size: var(--t-fine); }
  }
</style>
