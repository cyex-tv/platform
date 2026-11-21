<script lang="ts">
  // /redeem: one field. Case, spaces and dashes don't matter, so a code can be typed, pasted from a
  // chat or read off a card. The answer is what the code gave that you didn't own yet, with a link
  // to wear it. A few wrong codes lock the field for a while; the sentence says for how long.
  import { onDestroy } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadOwned } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import { keyKeeper } from '$lib/idempotency';
  import type { RedeemCodeMutation } from '$lib/gql/graphql';
  import { REDEEM_CODE } from '$lib/queries';
  import { sectionHref } from '$lib/profile/rows';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { lockoutMinutes, normaliseCode, plausibleCode, redeemMessage, tidyCode } from '$lib/shop/shop';
  import { session } from '$lib/tool/session.svelte';

  let code = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let result = $state<RedeemCodeMutation['redeemCode'] | null>(null);
  let lockedUntil = $state(0);
  let now = $state(Date.now());
  let tick: ReturnType<typeof setInterval> | undefined;
  let input = $state<HTMLInputElement>();
  const keys = keyKeeper();

  const me = $derived(session.me);
  const known = $derived(session.known);
  const locked = $derived(lockedUntil > now);
  const normalised = $derived(normaliseCode(code));
  const ready = $derived(plausibleCode(normalised));

  function lock(minutes: number) {
    lockedUntil = Date.now() + minutes * 60_000;
    now = Date.now();
    clearInterval(tick);
    tick = setInterval(() => {
      now = Date.now();
      if (now >= lockedUntil) {
        clearInterval(tick);
        problem = null;
      }
    }, 1000);
  }
  onDestroy(() => clearInterval(tick));

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy || locked) return;
    problem = null;
    result = null;
    if (!ready) {
      problem = normalised ? 'That’s too short to be a code. Check it for typos.' : 'Enter your code first.';
      input?.focus();
      return;
    }
    busy = true;
    try {
      // The same code tried again (after a timeout, say) sends the same key: redeemed once.
      result = (await gql(REDEEM_CODE, { code: normalised, idempotencyKey: keys.key(normalised) })).redeemCode;
      keys.done();
      code = '';
      if (me) loadOwned(me.id, true, true).catch(() => {});
    } catch (err) {
      problem = redeemMessage(err);
      if (err instanceof ApiError && err.code === 'locked_out') lock(lockoutMinutes(err.message) ?? 5);
      input?.focus();
    } finally {
      busy = false;
    }
  }

  const gave = $derived(result ? result.badges.length + result.paints.length : 0);
</script>

<svelte:head>
  <title>Redeem a code - cyex.tv</title>
  <meta name="description" content="Redeem a cyex.tv code for chat badges and name paints." />
</svelte:head>

<ColumnLayout width="form">
  <header class="page-head head">
    <div>
      <h1 class="page-title">Redeem a code</h1>
      <p class="line">A code from a stream, a giveaway or a friend. Capital letters, spaces and dashes don’t matter.</p>
    </div>
    <a class="btn btn-s" href="/shop"><Icon name="shopping-bag" size={15} />Shop</a>
  </header>

  {#if !known}
    <div class="panel skeleton" aria-busy="true" aria-label="Loading…"><span class="bar s"></span><span class="bar"></span><span class="bar b"></span></div>
  {:else if !me}
    <State title="Sign in to redeem a code.">
      Codes are added to your account.
      {#snippet actions()}<a class="btn btn-s btn-primary" href="/signin?back={encodeURIComponent('/redeem')}">Sign in</a>{/snippet}
    </State>
  {:else}
    <form class="panel form" onsubmit={submit} novalidate>
      <label class="ui-field">
        <span class="label">Code</span>
        <input class="ui-input mono" bind:this={input} value={code} oninput={(e) => (code = tidyCode(e.currentTarget.value))} autocomplete="off" autocapitalize="characters"
          spellcheck="false" maxlength="80" placeholder="ABCD-1234-EFGH" disabled={locked} aria-invalid={problem ? 'true' : undefined} aria-describedby={problem ? 'redeem-problem' : 'redeem-hint'} />
      </label>
      {#if problem}<p class="problem" id="redeem-problem" role="alert">{problem}</p>{:else}<p class="hint" id="redeem-hint">What it holds is added to {clean(me.identities[0]?.displayName) || 'your account'} at once.</p>{/if}
      <div class="actions">
        <button class="btn btn-s btn-primary" type="submit" disabled={busy || locked}>{busy ? 'Checking…' : 'Redeem'}</button>
      </div>
    </form>

    {#if result}
      <section class="panel result" aria-labelledby="gave-h" role="status">
        {#if gave}
          <h2 class="label" id="gave-h"><Icon name="check" size={13} stroke={2.4} />Added to your account</h2>
          <p class="sub"><bdi>{clean(result.name)}</bdi> gave you {gave === 1 ? 'this' : 'these'}:</p>
          <ul class="rows">
            {#each result.badges as b (b.id)}
              <li><span class="px"><BadgeIcon badge={b} size={18} /></span><span class="px"><BadgeIcon badge={b} size={36} label /></span><span class="nm"><bdi>{clean(b.name)}</bdi></span><span class="kind">Badge</span></li>
            {/each}
            {#each result.paints as p (p.id)}
              <li><span class="pt"><PaintedName name={clean(me.identities[0]?.displayName) || p.name} paint={p} /></span><span class="nm sub-nm"><bdi>{clean(p.name)}</bdi></span><span class="kind">Paint</span></li>
            {/each}
          </ul>
          <div class="actions"><a class="btn btn-s btn-primary" href={sectionHref(me.id, 'cosmetics')}>Equip {gave === 1 ? 'it' : 'them'} in Cosmetics</a></div>
        {:else}
          <h2 class="label" id="gave-h">You have all of it already</h2>
          <p class="sub">That code worked, but <bdi>{clean(result.name)}</bdi> only holds things you already own.</p>
        {/if}
      </section>
    {/if}
  {/if}
</ColumnLayout>

<style>
  p, h2 { margin: 0; }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .head > div { min-width: 0; }
  .head .btn { flex: none; }
  @media (max-width: 560px) { .head { flex-direction: column; gap: 12px; } }
  .line { margin-top: 6px; max-width: 56ch; font-size: var(--t-ui); line-height: 1.55; color: var(--muted); }
  .head .btn :global(svg) { color: var(--muted); }
  .label { display: flex; align-items: center; gap: 6px; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .label :global(svg) { color: var(--accent-text); }
  .panel { display: grid; gap: 12px; padding: 16px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .mono { font-family: var(--font-mono); letter-spacing: .04em; }
  .hint { font-size: var(--t-small); color: var(--muted); }
  .problem { font-size: var(--t-small); color: var(--red); }
  .actions { display: flex; gap: 8px; }
  .result { margin-top: 16px; }
  .sub { font-size: var(--t-small); color: var(--muted); }
  .rows { list-style: none; margin: 0; padding: 0; display: grid; border-top: 1px solid var(--border); }
  .rows li { display: flex; align-items: center; gap: 12px; min-height: 52px; min-width: 0; border-bottom: 1px solid var(--border); }
  .px { display: grid; place-items: center; flex: none; min-width: 36px; height: 40px; }
  .nm { flex: 1; min-width: 0; font-size: var(--t-small); font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub-nm { font-weight: 500; color: var(--muted); }
  .pt { flex: none; max-width: 55%; min-width: 0; font-size: var(--t-ui); font-weight: 700; overflow: hidden; }
  .kind { flex: none; font-size: var(--t-fine); color: var(--dim); }
  /* Loading: the form's shape, so nothing moves when it arrives. */
  .skeleton { min-height: 154px; align-content: start; }
  .bar { display: block; height: 40px; border-radius: var(--radius); background: var(--surface-2); }
  .bar.s { width: 48px; height: 12px; }
  .bar.b { width: 96px; }
</style>
