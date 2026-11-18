<script lang="ts">
  // Admin: every shop item, hidden and scheduled ones too (shop.manage). Each row says what state it
  // is in (on sale, hidden, scheduled, ended, sold out), its price, stock and window. New item and
  // Edit share one dialog; Delete is soft (orders stay) and asks twice. "View" opens the item as
  // the shop shows it, which also works for a hidden one.
  import { gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { clean, count, iso } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { CREATE_SHOP_ITEM, DELETE_SHOP_ITEM, SHOP_ITEMS, UPDATE_SHOP_ITEM } from '$lib/queries';
  import CosmeticsPick from '$lib/shop/CosmeticsPick.svelte';
  import { loadShopCounts } from '$lib/shop/counts.svelte';
  import {
    centsFrom, centsText, fromLocalInput, ITEM_STATE_TEXT, itemState, price, saleEndText, staffMessage, toLocalInput,
  } from '$lib/shop/shop';
  import Gate from '$lib/shop/Gate.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { date } from '$lib/format';

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

  const meta = (i: ShopItemFieldsFragment) =>
    [
      contents(i),
      i.stockLimit != null ? `${count(i.stockLeft ?? 0)} of ${count(i.stockLimit)} left` : '',
      i.activeFrom && Date.parse(i.activeFrom) > Date.now() ? `from ${date(i.activeFrom)}` : '',
      saleEndText(i)?.toLowerCase() ?? '',
    ]
      .filter(Boolean)
      .join(' · ');
  const contents = (i: ShopItemFieldsFragment) =>
    [i.badges.length ? `${count(i.badges.length)} ${i.badges.length === 1 ? 'badge' : 'badges'}` : '', i.paints.length ? `${count(i.paints.length)} ${i.paints.length === 1 ? 'paint' : 'paints'}` : '']
      .filter(Boolean)
      .join(' and ');
</script>

<svelte:head><title>Shop items - cyex.tv</title></svelte:head>

<Gate permission="shop.manage">
<PageHead title="Shop items" line="What is for sale: a bundle of badges and paints with a price. Hidden and scheduled items show here too.">
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
      <ul class="list">
        {#each d.items as i (i.id)}
          {@const st = itemState(i)}
          <li class="item">
            <div class="row">
              <span class="what">
                <strong><bdi>{clean(i.name)}</bdi></strong>
                <span class="ui-hint">{meta(i)}</span>
              </span>
              <span class="state" class:live={st === 'on_sale'}>{ITEM_STATE_TEXT[st]}</span>
              <span class="pr ui-num">{price(i.priceCents, i.currency)}</span>
              <span class="acts">
                <a class="ui-btn ghost small" href="/shop?item={encodeURIComponent(i.id)}">View</a>
                {#if st !== 'deleted'}
                  <button class="ui-btn secondary small" type="button" onclick={() => edit(i, d.catalog)}><Icon name="pencil" size={14} />Edit</button>
                  <Confirm small label="Delete" confirm="Click again to delete it" {busy} onconfirm={() => remove(i)} />
                {/if}
              </span>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}
</LoadGate>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="item-title">
  <form onsubmit={save} novalidate>
    <div class="body">
      <h2 id="item-title">{editing ? `Edit ${iso(editing.name)}` : 'New shop item'}</h2>
      <label class="ui-field">
        <span class="ui-label">Name</span>
        <input class="ui-input" bind:value={name} maxlength="64" aria-invalid={errors.name ? 'true' : undefined} />
        {#if errors.name}<span class="ui-error">{errors.name}</span>{/if}
      </label>
      <label class="ui-field">
        <span class="ui-label">Description</span>
        <textarea class="ui-input" rows="3" maxlength="500" bind:value={description}></textarea>
        <span class="ui-hint">Optional, up to 500 characters.</span>
      </label>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Price</span>
          <input class="ui-input" inputmode="decimal" bind:value={priceText} placeholder="4,99" aria-invalid={errors.price ? 'true' : undefined} />
          {#if errors.price}<span class="ui-error">{errors.price}</span>{:else}<span class="ui-hint">What the shop shows. Paddle charges the price below.</span>{/if}
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
      {#if catalog}
        <CosmeticsPick badges={[...catalog.badges.values()]} paints={[...catalog.paints.values()]} bind:badgeIds bind:paintIds error={errors.cosmetics} />
      {/if}
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">On sale from</span>
          <input class="ui-input" type="datetime-local" bind:value={from} />
          <span class="ui-hint">Empty: already.</span>
        </label>
        <label class="ui-field">
          <span class="ui-label">On sale until</span>
          <input class="ui-input" type="datetime-local" bind:value={to} aria-invalid={errors.window ? 'true' : undefined} />
          {#if errors.window}<span class="ui-error">{errors.window}</span>{:else}<span class="ui-hint">Empty: no end.</span>{/if}
        </label>
      </div>
      <div class="two">
        <label class="ui-field">
          <span class="ui-label">Copies</span>
          <input class="ui-input" inputmode="numeric" bind:value={stock} placeholder="No limit" aria-invalid={errors.stock ? 'true' : undefined} />
          {#if errors.stock}<span class="ui-error">{errors.stock}</span>{:else}<span class="ui-hint">The most that can be sold.</span>{/if}
        </label>
        <label class="ui-field">
          <span class="ui-label">Sort order</span>
          <input class="ui-input" inputmode="numeric" bind:value={sort} aria-invalid={errors.sort ? 'true' : undefined} />
          {#if errors.sort}<span class="ui-error">{errors.sort}</span>{:else}<span class="ui-hint">Lower comes first.</span>{/if}
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
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .item { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .row { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; padding: 10px 14px; }
  .what { display: grid; flex: 1 1 220px; min-width: 0; }
  .what strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .state { flex: none; min-width: 72px; font-size: var(--t-small); color: var(--muted); }
  .state.live { color: var(--text); font-weight: 700; }
  .pr { flex: none; min-width: 72px; text-align: right; font-weight: 600; }
  .acts { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; }
  .two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; align-items: start; }
  .mono { font-family: var(--font-mono); }
  textarea { resize: vertical; min-height: 72px; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  @media (max-width: 520px) { .two { grid-template-columns: minmax(0, 1fr); } .pr { text-align: left; } }
</style>
