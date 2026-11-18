<script lang="ts">
  // One item in full: every badge and paint on your own name in both themes and in a chat line, what
  // it costs, and the choice between buying it for yourself or giving it to a Twitch or Kick login.
  // Signed out, the button is a sign-in link that comes back to this item. Starting the purchase
  // closes the dialog (Paddle's checkout opens above the page, and a modal would hide it); the shop
  // page then shows the progress (CheckoutStatus).
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { clean } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { can } from '$lib/tool/session.svelte';
  import { buy } from './checkout.svelte';
  import ItemPreview from './ItemPreview.svelte';
  import { ITEM_STATE_TEXT, itemState, price, saleEndText, stockText } from './shop';

  type Props = {
    item: ShopItemFieldsFragment | null;
    /** Open on "Give to" (the card's Gift button, or an owned item). */
    gift?: boolean;
    name: string;
    signedIn: boolean;
    onclose: () => void;
  };
  let { item, gift = false, name, signedIn, onclose }: Props = $props();

  let dialog: HTMLDialogElement;
  let mode = $state<'me' | 'gift'>('me');
  let platform = $state<'TWITCH' | 'KICK'>('TWITCH');
  let login = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let tried = $state(false);
  let loginInput = $state<HTMLInputElement>();

  const cleanLogin = $derived(login.trim().replace(/^@/, ''));
  const loginError = $derived(mode === 'gift' && tried && !cleanLogin ? 'Enter the Twitch or Kick login of who gets it.' : null);
  const st = $derived(item ? itemState(item) : 'on_sale');
  const staff = $derived(can('shop.manage'));

  // Opens and closes with `item`; every opening starts clean.
  $effect(() => {
    if (item && !dialog.open) {
      mode = gift || item.owned ? 'gift' : 'me';
      login = '';
      problem = null;
      tried = false;
      busy = false;
      // The form is drawn for this item on the next tick; opening after it lets the browser
      // focus the first control (who gets it), not the dialog itself.
      tick().then(() => {
        if (!item || dialog.open) return;
        dialog.showModal();
        // A gift needs a login: start there. A purchase for yourself: on the choice of who gets it.
        if (mode === 'gift' && signedIn) loginInput?.focus();
        else dialog.querySelector<HTMLElement>('[role=radio][aria-checked=true]')?.focus();
      });
    } else if (!item && dialog.open) dialog.close();
  });

  async function pick(v: 'me' | 'gift') {
    mode = v;
    problem = null;
    if (v === 'gift') {
      await tick();
      loginInput?.focus();
    }
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!item || busy) return;
    tried = true;
    problem = null;
    if (mode === 'gift' && !cleanLogin) {
      loginInput?.focus();
      return;
    }
    busy = true;
    const to = mode === 'gift' ? { platform, login: cleanLogin } : null;
    const error = await buy({ itemId: item.id, itemName: item.name, gift: to, to: to?.login ?? null });
    busy = false;
    if (error) problem = error;
    else dialog.close();
  }

  const back = $derived(item ? `/signin?back=${encodeURIComponent(`/shop?item=${item.id}`)}` : '/signin');
  const label = $derived(
    !item ? '' : mode === 'gift' ? `Give for ${price(item.priceCents, item.currency)}` : `Buy for ${price(item.priceCents, item.currency)}`,
  );
</script>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="shop-item-title" onclose={onclose}>
  {#if item}
    <form onsubmit={submit} novalidate>
      <div class="body">
        <header class="head">
          <h2 id="shop-item-title"><bdi>{clean(item.name)}</bdi></h2>
          <p class="price">{price(item.priceCents, item.currency)}</p>
        </header>
        {#if item.description}<p class="desc">{clean(item.description)}</p>{/if}

        {#if st !== 'on_sale' || !item.available}
          <p class="note" role="status">
            {#if staff && (st === 'hidden' || st === 'deleted')}{ITEM_STATE_TEXT[st]}: only staff can open this.{:else if st === 'sold_out'}This item is sold out.{:else if st === 'scheduled'}This item isn’t on sale yet.{:else if st === 'ended'}This sale has ended.{:else}This item isn’t on sale right now.{/if}
          </p>
        {/if}

        <ItemPreview badges={item.badges} paints={item.paints} {name} full />

        <p class="facts">
          {#if stockText(item)}<span>{stockText(item)}</span>{/if}
          {#if saleEndText(item)}<span>{saleEndText(item)}</span>{/if}
          {#if item.owned}<span class="own"><Icon name="check" size={14} />You own everything in it</span>{/if}
        </p>

        {#if signedIn && item.available}
          <div class="who">
            <Segmented label="Who gets it" size="s" options={[{ value: 'me', label: 'For me' }, { value: 'gift', label: 'Gift to someone' }]} value={mode} onchange={pick} />
            {#if mode === 'me' && item.owned}
              <p class="ui-hint">You own all of it already. You can still give it to someone.</p>
            {/if}
            {#if mode === 'gift'}
              <div class="gift">
                <label class="ui-field">
                  <span class="ui-label">Platform</span>
                  <select class="ui-input" bind:value={platform}>
                    <option value="TWITCH">Twitch</option>
                    <option value="KICK">Kick</option>
                  </select>
                </label>
                <label class="ui-field">
                  <span class="ui-label">Their login</span>
                  <input class="ui-input" bind:this={loginInput} bind:value={login} maxlength="64" autocomplete="off" autocapitalize="off" spellcheck="false"
                    placeholder="For example quietoak" aria-invalid={loginError ? 'true' : undefined} aria-describedby={loginError ? 'gift-err' : 'gift-hint'} />
                  {#if loginError}<span class="ui-error" id="gift-err">{loginError}</span>{:else}<span class="ui-hint" id="gift-hint">They need to have signed in to cyex.tv once.</span>{/if}
                </label>
              </div>
            {/if}
          </div>
        {/if}

        {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
      </div>
      <footer>
        <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Close</button>
        {#if !signedIn}
          <a class="ui-btn primary" href={back}>Sign in to buy</a>
        {:else}
          <button class="ui-btn primary" type="submit" disabled={busy || !item.available || (mode === 'me' && item.owned)}>{busy ? 'Starting…' : label}</button>
        {/if}
      </footer>
    </form>
  {/if}
</dialog>

<style>
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  form { max-height: calc(100dvh - 24px); grid-template-rows: minmax(0, 1fr) auto; }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  .head { display: flex; align-items: baseline; justify-content: space-between; gap: 16px; }
  h2 { margin: 0; min-width: 0; font-size: 18px; font-weight: 600; line-height: 1.3; letter-spacing: -.01em; overflow-wrap: anywhere; }
  .price { margin: 0; flex: none; font-size: 18px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .desc { margin: -4px 0 0; font-size: var(--t-small); line-height: 1.55; color: var(--muted); white-space: pre-line; overflow-wrap: anywhere; }
  .note { margin: 0; padding: 8px 12px; border: 1px solid var(--border-strong); border-radius: var(--radius); font-size: var(--t-small); color: var(--text); }
  .facts { display: flex; flex-wrap: wrap; gap: 2px 14px; margin: 0; font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; }
  .own { display: inline-flex; align-items: center; gap: 5px; font-weight: 700; color: var(--text); }
  .own :global(svg) { color: var(--accent-text); }
  .who { display: grid; gap: 12px; justify-items: start; }
  .gift { display: grid; grid-template-columns: 140px minmax(0, 1fr); gap: 12px; width: 100%; align-items: start; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  @media (max-width: 520px) { .gift { grid-template-columns: minmax(0, 1fr); } }
</style>
