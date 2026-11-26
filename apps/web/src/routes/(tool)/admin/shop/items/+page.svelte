<script lang="ts">
  import Dialog from '$lib/components/Dialog.svelte';
  // Admin: every shop item, hidden and scheduled ones too (shop.manage), as a contact sheet: the
  // picture, the name, the price and the state as a flag word (on sale, hidden, scheduled, ended,
  // sold out). The picked
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
  import { clean, count, dateTime, iso } from '$lib/format';
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
  import Bar from '$lib/admin/ui/Bar.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const L = loader(async ({ fetch }) => {
    const [items, catalog] = await Promise.all([gql(SHOP_ITEMS, { all: true }, fetch), loadCatalog()]);
    return { items: items.shopItems, catalog };
  });

  let busy = $state(false);
  const plural = (n: number, one: string, many: string) => `${count(n)} ${n === 1 ? one : many}`;
  let query = $state('');
  const shown = (all: ShopItemFieldsFragment[]) => {
    const q = query.trim().toLowerCase();
    return q ? all.filter((i) => i.name.toLowerCase().includes(q)) : all;
  };

  // The form: new (editing null) or edit.
  let dialog: Dialog;
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
    dialog.open();
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (saving) return;
    tried = true;
    problem = null;
    if (cents === null || invalid) {
      dialog.element()?.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
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

  const contents = (i: ShopItemFieldsFragment) =>
    [i.badges.length ? plural(i.badges.length, 'badge', 'badges') : '', i.paints.length ? plural(i.paints.length, 'paint', 'paints') : '']
      .filter(Boolean)
      .join(' and ');
  const stockOf = (i: ShopItemFieldsFragment) => (i.stockLimit != null ? `${count(i.stockLeft ?? 0)} of ${count(i.stockLimit)}` : 'No limit');
  const sold = (i: ShopItemFieldsFragment) => (i.stockLimit != null ? count(i.stockLimit - (i.stockLeft ?? 0)) : 'n/a');
</script>

<svelte:head><title>{picked ? `${clean(picked.name)} - Shop items` : 'Shop items'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Gate permission="shop.manage">
<Split label="Shop item" open={!!picked} onclose={() => pick(null)}>
  <Bar title="Shop items" count={L.data ? plural(L.data.items.length, 'item', 'items') : null}>
    {#snippet actions()}
      <button class="ui-btn primary small" type="button" disabled={!L.data} onclick={() => L.data && edit(null, L.data.catalog)}><Icon name="plus" size={15} />New item</button>
    {/snippet}
    {#snippet tools()}
      <label class="adm-find grow">
        <Icon name="search" size={15} />
        <span class="sr-only">Filter items</span>
        <input class="ui-input" type="search" bind:value={query} placeholder="Filter by name" autocomplete="off" spellcheck="false" maxlength="64" />
      </label>
      {#if L.data && query.trim()}<span class="ui-hint adm-num">{count(shown(L.data.items).length)} of {count(L.data.items.length)}</span>{/if}
    {/snippet}
  </Bar>

  <LoadGate source={L} label="Loading items…">
    {#snippet children(d)}
      {@const rows = shown(d.items)}
      {#if !d.items.length}
        <State title="Nothing for sale yet.">
          Make the first item: pick its badges and paints, set a price and paste the Paddle price ID.
          {#snippet actions()}<button class="ui-btn primary" type="button" onclick={() => edit(null, d.catalog)}>New item</button>{/snippet}
        </State>
      {:else if !rows.length}
        <State title="Nothing matches.">No item has that name.</State>
      {:else}
        <ul class="adm-sheet tiles" style="--cell: 170px" bind:this={body} use:gridKeys aria-label="Shop items">
          {#each rows as i, n (i.id)}
            {@const st = itemState(i)}
            <li>
              <button class="adm-cell tile" type="button" data-item={i.id} aria-current={i.id === pickedId ? 'true' : undefined}
                tabindex={(picked && rows.some((x) => x.id === pickedId) ? i.id === pickedId : n === 0) ? 0 : -1} onclick={() => pick(i.id)}>
                <span class="tick"><Icon name="check" size={14} /></span>
                <span class="pic">
                  {#if i.badges.length}
                    <span class="pics" aria-hidden="true">{#each i.badges.slice(0, 3) as b (b.id)}<BadgeIcon badge={b} size={32} />{/each}</span>
                  {:else}<Icon name="palette" size={28} />{/if}
                </span>
                <span class="nm"><bdi>{clean(i.name)}</bdi></span>
                <span class="sub"><span class="adm-num pr">{price(i.priceCents, i.currency)}</span></span>
                <span class="sub">
                  <span class="adm-flags"><span class:ok={st === 'on_sale'} class:warn={st === 'sold_out' || st === 'deleted'}>{ITEM_STATE_TEXT[st]}</span>{#if i.stockLimit != null}<span class="adm-num">{count(i.stockLeft ?? 0)} left</span>{/if}</span>
                </span>
              </button>
            </li>
          {/each}
        </ul>
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
        <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={unpick} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name"><bdi>{clean(i.name)}</bdi></h2>
          {#if i.description}<p class="desc"><bdi>{clean(i.description)}</bdi></p>{/if}
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Facts</h3>
          <dl class="adm-kv">
            <dt>State</dt><dd class="txt">{ITEM_STATE_TEXT[st]}</dd>
            <dt>Price</dt><dd>{price(i.priceCents, i.currency)}</dd>
            <dt>Window</dt><dd>{i.activeFrom || i.activeTo ? `${i.activeFrom ? dateTime(i.activeFrom) : 'Now'} to ${i.activeTo ? dateTime(i.activeTo) : 'no end'}` : 'Always'}</dd>
            <dt>Stock</dt><dd>{stockOf(i)}{i.stockLimit != null ? ' left' : ''}</dd>
            <dt>Sold</dt><dd>{sold(i)}</dd>
            <dt>Listed</dt><dd class="txt">{i.visible ? 'Yes' : 'No'}</dd>
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

<Dialog bind:this={dialog} wide class="item-dialog" title={editing ? `Edit ${iso(editing.name)}` : 'New shop item'} titleId="item-title" onsubmit={save}>
  <h3 class="adm-label">Item</h3>
  <div>
    <label class="adm-lv">
      <span class="l">Name</span>
      <span class="v"><input class="ui-input" bind:value={name} maxlength="64" aria-invalid={errors.name ? 'true' : undefined} />{#if errors.name}<span class="ui-error">{errors.name}</span>{/if}</span>
    </label>
    <label class="adm-lv top">
      <span class="l">Description</span>
      <span class="v"><textarea class="ui-input" rows="3" maxlength="500" bind:value={description}></textarea></span>
    </label>
  </div>
  <h3 class="adm-label sect">Price</h3>
  <div>
    <label class="adm-lv">
      <span class="l">Price</span>
      <span class="v"><input class="ui-input adm-num" inputmode="decimal" bind:value={priceText} placeholder="4,99" aria-invalid={errors.price ? 'true' : undefined} />{#if errors.price}<span class="ui-error">{errors.price}</span>{/if}</span>
    </label>
    <label class="adm-lv">
      <span class="l">Currency</span>
      <span class="v"><input class="ui-input" bind:value={currency} maxlength="3" autocapitalize="characters" aria-invalid={errors.currency ? 'true' : undefined} />{#if errors.currency}<span class="ui-error">{errors.currency}</span>{/if}</span>
    </label>
    <label class="adm-lv">
      <span class="l">Paddle price ID</span>
      <span class="v"><input class="ui-input mono" bind:value={priceId} placeholder="pri_01abc…" autocomplete="off" spellcheck="false" aria-invalid={errors.priceId ? 'true' : undefined} />{#if errors.priceId}<span class="ui-error">{errors.priceId}</span>{/if}</span>
    </label>
  </div>
  <h3 class="adm-label sect">Contents</h3>
  {#if catalog}
    <CosmeticsPick badges={[...catalog.badges.values()]} paints={[...catalog.paints.values()]} bind:badgeIds bind:paintIds error={errors.cosmetics} />
  {/if}
  <h3 class="adm-label sect">Availability</h3>
  <div>
    <label class="adm-lv">
      <span class="l">On sale from<small>Empty: now</small></span>
      <span class="v"><input class="ui-input" type="datetime-local" bind:value={from} /></span>
    </label>
    <label class="adm-lv">
      <span class="l">On sale until<small>Empty: no end</small></span>
      <span class="v"><input class="ui-input" type="datetime-local" bind:value={to} aria-invalid={errors.window ? 'true' : undefined} />{#if errors.window}<span class="ui-error">{errors.window}</span>{/if}</span>
    </label>
    <label class="adm-lv">
      <span class="l">Copies<small>Empty: no limit</small></span>
      <span class="v"><input class="ui-input adm-num" inputmode="numeric" bind:value={stock} placeholder="No limit" aria-invalid={errors.stock ? 'true' : undefined} />{#if errors.stock}<span class="ui-error">{errors.stock}</span>{/if}</span>
    </label>
    <label class="adm-lv">
      <span class="l">Sort order<small>Lower first</small></span>
      <span class="v"><input class="ui-input adm-num" inputmode="numeric" bind:value={sort} aria-invalid={errors.sort ? 'true' : undefined} />{#if errors.sort}<span class="ui-error">{errors.sort}</span>{/if}</span>
    </label>
    <label class="adm-lv">
      <span class="l">Listed in the shop</span>
      <span class="v"><input type="checkbox" bind:checked={visible} /></span>
    </label>
  </div>
  {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
  {#snippet footer()}
    <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
    <button class="ui-btn primary" type="submit" disabled={saving}>{saving ? 'Saving…' : editing ? 'Save' : 'Make item'}</button>
  {/snippet}
</Dialog>
</Gate>

<style>
  /* The contact sheet (admin.css .adm-sheet): the item's badges, the name, the price, the state. */
  .tile { height: 150px; justify-items: center; text-align: center; }
  .tile .nm, .tile .sub { justify-content: center; max-width: 100%; }
  .tile .pic { color: var(--dim); }
  .pics { display: flex; gap: 4px; align-items: center; }
  .pr { color: var(--text); font-weight: 500; }
  .tile .adm-flags { justify-content: center; }
  .tile:active { opacity: .6; transition: none; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .desc { margin: 0; font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .bundle { list-style: none; margin: 0; padding: 0; display: grid; }
  .bundle li { display: flex; align-items: center; gap: 10px; min-height: 36px; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .bundle li:last-child { border-bottom: 0; }
  .bundle li > span:first-of-type { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bundle :global(svg) { flex: none; color: var(--dim); }
  .kind { font-size: var(--t-fine); color: var(--dim); }
  :global(.ui-dialog.item-dialog form) { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  :global(.ui-dialog.item-dialog .body) { overflow-y: auto; overscroll-behavior: contain; }
  :global(.ui-dialog.item-dialog h2) { margin: 0; font-size: 18px; }
  .sect { padding-top: 12px; border-top: 1px solid var(--border); }
  .mono { font-family: var(--font-mono); }
  .adm-lv .v .ui-input { flex: 1 1 180px; min-width: 0; }
  .adm-lv .v .ui-error { flex-basis: 100%; }
  .adm-lv.top { align-items: start; }
  textarea { resize: vertical; min-height: 72px; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
</style>
