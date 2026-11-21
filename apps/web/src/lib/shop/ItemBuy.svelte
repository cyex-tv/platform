<script lang="ts">
  // One item in full, and the choice of who gets it: every badge and paint on your own name in both
  // themes and in a chat line, what it holds, what it costs, then "For me" or "Gift to someone" (a
  // Twitch or Kick login). Signed out, the button is a sign-in link that comes back to this item.
  // The shop shows it in two places: the right panel on wide windows (variant "panel") and the
  // item dialog on narrow ones (ItemDialog). Starting the purchase calls `onstarted`: both close,
  // because Paddle's checkout opens above the page and the shop shows the progress (CheckoutStatus).
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { clean } from '$lib/format';
  import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
  import { can } from '$lib/tool/session.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { buy } from './checkout.svelte';
  import ItemPreview from './ItemPreview.svelte';
  import { contentsText, isBundle, ITEM_STATE_TEXT, itemState, price, saleEndText, stockText } from './shop';

  type Props = {
    item: ShopItemFieldsFragment;
    /** Start on "Gift to someone" (the card's Gift button, or an owned item). */
    gift?: boolean;
    name: string;
    signedIn: boolean;
    variant?: 'dialog' | 'panel';
    /** The id of the item's heading (the dialog names itself by it). */
    headingId?: string;
    onclose: () => void;
    /** The payment window is opening: the caller closes the item. */
    onstarted: () => void;
    /** Whether the item is still on screen when a start answers late (the error goes to a toast if not). */
    showing: () => boolean;
  };
  let { item, gift = false, name, signedIn, variant = 'dialog', headingId = 'shop-item-title', onclose, onstarted, showing }: Props = $props();

  const uid = $props.id();
  // svelte-ignore state_referenced_locally
  let mode = $state<'me' | 'gift'>(gift || item.owned ? 'gift' : 'me');
  let platform = $state<'TWITCH' | 'KICK'>('TWITCH');
  let login = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let tried = $state(false);
  let loginInput = $state<HTMLInputElement>();
  let root = $state<HTMLFormElement>();

  const cleanLogin = $derived(login.trim().replace(/^@/, ''));
  const loginError = $derived(mode === 'gift' && tried && !cleanLogin ? 'Enter the Twitch or Kick login of who gets it.' : null);
  const st = $derived(itemState(item));
  const staff = $derived(can('shop.manage'));
  const holds = $derived(contentsText(item));
  const stock = $derived(stockText(item));
  const ends = $derived(saleEndText(item));

  /** Puts focus where the item starts: a gift on the login, a purchase for yourself on who gets it, signed out on the heading. */
  export function focusStart() {
    if (mode === 'gift' && signedIn && item.available) loginInput?.focus();
    else if (signedIn && item.available) root?.querySelector<HTMLElement>('[role=radio][aria-checked=true]')?.focus();
    else root?.querySelector<HTMLElement>('h2')?.focus();
  }

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
    if (busy) return;
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
    if (error) {
      // The item may have closed meanwhile (Escape, or another item picked): say it where they are.
      if (showing()) problem = error;
      else notify(error, true);
    } else if (showing()) onstarted();
  }

  const back = $derived(`/signin?back=${encodeURIComponent(`/shop?item=${item.id}`)}`);
  const label = $derived(mode === 'gift' ? `Give for ${price(item.priceCents, item.currency)}` : `Buy for ${price(item.priceCents, item.currency)}`);
</script>

<form class="buy {variant}" bind:this={root} onsubmit={submit} novalidate>
  <div class="body">
    <header class="head">
      <div class="title">
        <h2 id={headingId} tabindex="-1"><bdi>{clean(item.name)}</bdi></h2>
        <p class="kind"><span>{isBundle(item) ? 'Bundle' : item.paints.length ? 'Paint' : 'Badge'}</span><span class="sep" aria-hidden="true">·</span><span>{holds}</span></p>
      </div>
      <p class="price">{price(item.priceCents, item.currency)}</p>
    </header>
    {#if item.description}<p class="desc">{clean(item.description)}</p>{/if}

    {#if st !== 'on_sale' || !item.available}
      <p class="note" role="status">
        {#if staff && (st === 'hidden' || st === 'deleted')}{ITEM_STATE_TEXT[st]}: only staff can open this.{:else if st === 'sold_out'}This item is sold out.{:else if st === 'scheduled'}This item isn’t on sale yet.{:else if st === 'ended'}This sale has ended.{:else}This item isn’t on sale right now.{/if}
      </p>
    {/if}

    <section class="block" aria-labelledby="{uid}-look">
      <h3 class="label" id="{uid}-look">On {signedIn ? 'your name' : 'a name'}</h3>
      <ItemPreview badges={item.badges} paints={item.paints} {name} full />
    </section>

    {#if stock || ends || item.owned}
      <dl class="kv">
        {#if stock}<dt>Stock</dt><dd>{stock}</dd>{/if}
        {#if ends}<dt>Sale</dt><dd>{ends}</dd>{/if}
        {#if item.owned}<dt>You</dt><dd class="own"><Icon name="check" size={13} stroke={2.4} />Own everything in it</dd>{/if}
      </dl>
    {/if}

    {#if signedIn && item.available}
      <section class="block who" aria-labelledby="{uid}-who">
        <h3 class="label" id="{uid}-who">Who gets it</h3>
        <Segmented label="Who gets it" size="s" options={[{ value: 'me', label: 'For me' }, { value: 'gift', label: 'Gift to someone' }]} value={mode} onchange={pick} />
        {#if mode === 'me' && item.owned}
          <p class="ui-hint">You own all of it already. You can still give it to someone.</p>
        {/if}
        {#if mode === 'gift'}
          <div class="gift">
            <div class="ui-field">
              <span class="ui-label">Platform</span>
              <Segmented label="Their platform" size="s" options={[{ value: 'TWITCH', label: 'Twitch', icon: 'twitch' }, { value: 'KICK', label: 'Kick', icon: 'kick' }]} value={platform} onchange={(v) => (platform = v)} />
            </div>
            <label class="ui-field">
              <span class="ui-label">Their login</span>
              <input class="ui-input" bind:this={loginInput} bind:value={login} maxlength="64" autocomplete="off" autocapitalize="off" spellcheck="false"
                placeholder="For example quietoak" aria-invalid={loginError ? 'true' : undefined} aria-describedby={loginError ? `${uid}-err` : `${uid}-hint`} />
              {#if loginError}<span class="ui-error" id="{uid}-err">{loginError}</span>{:else}<span class="ui-hint" id="{uid}-hint">They need to have signed in to cyex.tv once.</span>{/if}
            </label>
          </div>
        {/if}
      </section>
    {/if}

    {#if problem}<p class="problem" role="alert">{problem}</p>{/if}
  </div>
  <footer>
    <button class="btn btn-s" type="button" onclick={onclose}>Close</button>
    {#if !signedIn}
      <a class="btn btn-s btn-primary" href={back}>Sign in to buy</a>
    {:else}
      <button class="btn btn-s btn-primary" type="submit" disabled={busy || !item.available || (mode === 'me' && item.owned)}>{busy ? 'Starting…' : label}</button>
    {/if}
  </footer>
</form>

<style>
  p, h2, h3 { margin: 0; }
  .body { display: grid; gap: 16px; min-width: 0; }
  .dialog .body { overflow-y: auto; overscroll-behavior: contain; }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; min-width: 0; }
  .title { display: grid; gap: 2px; min-width: 0; }
  h2 { font-size: 18px; font-weight: 600; line-height: 1.3; letter-spacing: -.01em; overflow-wrap: anywhere; }
  h2:focus { outline: none; }
  h2:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; border-radius: var(--radius); }
  .kind { display: flex; flex-wrap: wrap; gap: 0 6px; font-size: var(--t-small); color: var(--muted); }
  .sep { color: var(--dim); }
  .price { flex: none; font-size: 18px; font-weight: 600; line-height: 1.3; font-variant-numeric: tabular-nums; }
  .desc { margin-top: -6px; font-size: var(--t-small); line-height: 1.55; color: var(--muted); white-space: pre-line; overflow-wrap: anywhere; }
  .note { padding: 8px 12px; border: 1px solid var(--border-strong); border-radius: var(--radius); font-size: var(--t-small); color: var(--text); }
  .block { display: grid; gap: 8px; min-width: 0; }
  .label { font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }

  .kv { display: grid; grid-template-columns: minmax(0, 1fr) auto; margin: 0; border-top: 1px solid var(--border); }
  .kv dt, .kv dd { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .kv dt { color: var(--muted); padding-right: 16px; }
  .kv dd { text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; }
  .own { display: inline-flex; align-items: center; justify-content: flex-end; gap: 5px; }
  .own :global(svg) { color: var(--accent-text); }

  .who { justify-items: start; }
  .gift { display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; width: 100%; align-items: start; }
  .problem { font-size: var(--t-small); color: var(--red); }

  /* The panel: the buttons follow the form (not pinned to the panel's bottom, which can sit below
     the window while the announcement bar is open). */
  .panel .body { padding: 16px 16px 0; align-content: start; }
  .panel footer { display: flex; justify-content: flex-end; gap: 8px; margin: 16px 16px 0; padding: 12px 0 16px; border-top: 1px solid var(--border); }
</style>
