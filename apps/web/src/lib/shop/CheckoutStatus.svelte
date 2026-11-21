<script lang="ts">
  // Where a purchase stands, once the payment window is open: a calm progress state while Paddle
  // and the webhook do their part, then "it's yours" with a link to equip it, or what went wrong.
  // One flat section (PAYMENT, hairlines) holding the site's one state pattern, announced to screen readers.
  import { onMount } from 'svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import { session } from '$lib/tool/session.svelte';
  import { again, checkAgain, dismiss, flow, resume, retry } from './checkout.svelte';

  onMount(() => {
    resume();
  });

  const item = $derived(clean(flow.purchase?.itemName) || 'your item');
  const to = $derived(clean(flow.purchase?.to ?? flow.purchase?.gift?.login));
  const gift = $derived(!!flow.purchase?.gift || !!flow.purchase?.to || !!flow.purchase?.resumedGift);
</script>

{#if flow.phase === 'paying'}
  <div class="status"><h2 class="label">Payment</h2><State bare kind="loading" title="Waiting for your payment">
    Pay in the window that opened. This page updates as soon as it’s done.
    {#snippet actions()}<button class="btn btn-s" type="button" onclick={dismiss}>Stop waiting</button>{/snippet}
  </State></div>
{:else if flow.phase === 'confirming'}
  <div class="status"><h2 class="label">Payment</h2><State bare kind="loading" title="Confirming your payment">
    This usually takes a few seconds. You don’t need to do anything.
  </State></div>
{:else if flow.phase === 'slow'}
  <div class="status"><h2 class="label">Payment</h2><State bare kind="loading" title="Still confirming">
    Confirming is taking longer than usual. You can leave this page: <bdi>{item}</bdi> shows up in {gift ? 'their' : 'your'} Cosmetics and in your orders as soon as the payment is confirmed.
    {#snippet actions()}<button class="btn btn-s" type="button" onclick={dismiss}>Hide this</button>{/snippet}
  </State></div>
{:else if flow.phase === 'stalled'}
  <div class="status"><h2 class="label">Payment</h2><State bare title="We stopped checking">
    We haven’t heard back about this payment. If you paid, <bdi>{item}</bdi> shows up in {gift ? 'their' : 'your'} Cosmetics and in your orders as soon as it’s confirmed. Nothing is lost.
    {#snippet actions()}
      <button class="btn btn-s btn-primary" type="button" onclick={checkAgain}>Check again</button>
      <button class="btn btn-s" type="button" onclick={dismiss}>Hide this</button>
    {/snippet}
  </State></div>
{:else if flow.phase === 'cancelled'}
  <div class="status"><h2 class="label">Payment</h2><State bare title="The payment window was closed">
    Nothing was charged. Your item is held for you for a little while.
    {#snippet actions()}
      <button class="btn btn-s btn-primary" type="button" onclick={again}>Open it again</button>
      <button class="btn btn-s" type="button" onclick={dismiss}>Not now</button>
    {/snippet}
  </State></div>
{:else if flow.phase === 'paid'}
  <div class="status"><h2 class="label">Payment</h2><State bare title={gift ? 'Your gift is on its way' : 'It’s yours'}>
    {#if gift}
      <bdi>{item}</bdi> now belongs to <bdi>{to || 'them'}</bdi>. They can equip it from their Cosmetics.
    {:else}
      <bdi>{item}</bdi> is in your Cosmetics now. Pick it there to wear it in chat.
    {/if}
    {#snippet actions()}
      {#if !gift && session.me}<a class="btn btn-s btn-primary" href={sectionHref(session.me.id, 'cosmetics')}>Equip it in Cosmetics</a>{/if}
      <button class="btn btn-s" type="button" onclick={dismiss}>Keep shopping</button>
    {/snippet}
  </State></div>
{:else if flow.phase === 'failed'}
  <div class="status"><h2 class="label">Payment</h2>
    <Problem cause="payment" body={flow.error ?? undefined} onretry={flow.purchase ? retry : undefined} home={{ href: '/shop', label: 'Back to the shop' }} onhome={dismiss} />
  </div>
{/if}

<style>
  /* Flat, like every section now: an uppercase label, hairlines above and below, no box. */
  .status { display: grid; margin-bottom: 24px; border-block: 1px solid var(--border); }
  .label { display: flex; align-items: center; min-height: 40px; margin: 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); border-bottom: 1px solid var(--border); }
  .status :global(.state.bare) { justify-items: start; text-align: left; padding: 20px 0; }
  .status :global(.state.bare .actions) { justify-content: flex-start; }
</style>
