<script lang="ts">
  // Where a purchase stands, once the payment window is open: a calm progress state while Paddle
  // and the webhook do their part, then "it's yours" with a link to equip it, or what went wrong.
  // One State panel (the site's one empty / loading / error pattern), announced to screen readers.
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
  <div class="status"><State kind="loading" title="Waiting for your payment">
    Pay in the window that opened. This page updates as soon as it’s done.
    {#snippet actions()}<button class="btn btn-s" type="button" onclick={dismiss}>Stop waiting</button>{/snippet}
  </State></div>
{:else if flow.phase === 'confirming'}
  <div class="status"><State kind="loading" title="Confirming your payment">
    This usually takes a few seconds. You don’t need to do anything.
  </State></div>
{:else if flow.phase === 'slow'}
  <div class="status"><State kind="loading" title="Still confirming">
    Confirming is taking longer than usual. You can leave this page: <bdi>{item}</bdi> shows up in {gift ? 'their' : 'your'} Cosmetics and in your orders as soon as the payment is confirmed.
    {#snippet actions()}<button class="btn btn-s" type="button" onclick={dismiss}>Hide this</button>{/snippet}
  </State></div>
{:else if flow.phase === 'stalled'}
  <div class="status"><State title="We stopped checking">
    We haven’t heard back about this payment. If you paid, <bdi>{item}</bdi> shows up in {gift ? 'their' : 'your'} Cosmetics and in your orders as soon as it’s confirmed. Nothing is lost.
    {#snippet actions()}
      <button class="btn btn-s btn-primary" type="button" onclick={checkAgain}>Check again</button>
      <button class="btn btn-s" type="button" onclick={dismiss}>Hide this</button>
    {/snippet}
  </State></div>
{:else if flow.phase === 'cancelled'}
  <div class="status"><State title="The payment window was closed">
    Nothing was charged. Your item is held for you for a little while.
    {#snippet actions()}
      <button class="btn btn-s btn-primary" type="button" onclick={again}>Open it again</button>
      <button class="btn btn-s" type="button" onclick={dismiss}>Not now</button>
    {/snippet}
  </State></div>
{:else if flow.phase === 'paid'}
  <div class="status"><State title={gift ? 'Your gift is on its way' : 'It’s yours'}>
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
  <div class="status">
    <Problem cause="payment" body={flow.error ?? undefined} onretry={flow.purchase ? retry : undefined} home={{ href: '/shop', label: 'Back to the shop' }} onhome={dismiss} />
  </div>
{/if}

<style>
  .status { margin-bottom: 24px; }
</style>
