<script lang="ts">
  // The one error pattern (see problem.ts): a headline by cause, Try again where asking again can
  // help, and exactly one link out. After a Try again that worked, focus moves to the page's h1.
  import { onDestroy, tick } from 'svelte';
  import State from './State.svelte';
  import { btnClass, btnFamily } from './btn';
  import { COPY, focusPage, resolveCause, type Cause } from './problem';

  type Props = {
    cause: Cause;
    /** Asks again. Return when done; the problem goes away by itself when it worked. */
    onretry?: () => unknown;
    /** The one link out: the front page, or the list this page belongs to (null when the surface already has one, like a menu's footer). */
    home?: { href: string; label: string } | null;
    /** Makes the one way out a button that calls this, for a page that is already where the link would go. */
    onhome?: () => void;
    /** 1: the headline is the page's h1. */
    level?: 1 | 2;
    bare?: boolean;
    /** Own headline and sentence, for a not-found that can name what's missing ("No emote here"). */
    title?: string;
    body?: string;
  };
  let { cause, onretry, home = { href: '/', label: 'Back to the front page' }, onhome, level = 2, bare = false, title, body }: Props = $props();

  const family = btnFamily();
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine);
  const shown = $derived(resolveCause(cause, online));
  const copy = $derived(COPY[shown]);
  const missing = $derived(shown === 'missing');
  // Own wording only for the causes it was written for: a headline and sentence for not-found and
  // forbidden, the reason's sentence for a payment.
  const custom = $derived(shown === 'missing' || shown === 'forbidden');
  const ownBody = $derived(custom || shown === 'payment');

  let retrying = $state(false);
  let alive = true;
  onDestroy(() => (alive = false));
  async function retry() {
    if (retrying || !onretry) return;
    retrying = true;
    try {
      await onretry();
    } catch {
      /* the page shows its own error again */
    } finally {
      if (alive) retrying = false;
    }
    await tick();
    if (!alive) focusPage();
  }
</script>

<svelte:window ononline={() => (online = true)} onoffline={() => (online = false)} />

<State kind={missing ? 'empty' : 'error'} {level} {bare} role={missing ? 'status' : 'alert'} title={custom && title ? title : copy.title}>
  {ownBody && body ? body : copy.body}
  {#snippet actions()}
    {#if copy.retry && onretry}
      <button class={btnClass(family, 'primary')} type="button" onclick={retry} aria-disabled={retrying}>{retrying ? 'Trying…' : 'Try again'}</button>
    {/if}
    {#if home && onhome}<button class={btnClass(family, copy.retry && onretry ? 'secondary' : 'primary')} type="button" onclick={onhome}>{home.label}</button>
    {:else if home}<a class={btnClass(family, copy.retry && onretry ? 'secondary' : 'primary')} href={home.href}>{home.label}</a>{/if}
  {/snippet}
</State>

<style>
  /* Not `disabled`: a disabled button drops keyboard focus mid-retry. */
  button[aria-disabled='true'] { opacity: .5; cursor: default; }
</style>
