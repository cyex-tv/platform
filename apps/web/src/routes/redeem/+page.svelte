<script lang="ts">
  // /redeem: one field. Case, spaces and dashes don't matter, so a code can be typed, pasted from a
  // chat or read off a card. The answer is what the code gave that you didn't own yet, with a link
  // to wear it. A few wrong codes lock the field for a while; the sentence says for how long.
  import { onDestroy } from 'svelte';
  import { ApiError, gql } from '$lib/api';
  import State from '$lib/components/State.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadOwned } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
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
      result = (await gql(REDEEM_CODE, { code: normalised })).redeemCode;
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
    <h1 class="page-title">Redeem a code</h1>
    <p class="lead">Got a code from a stream, a giveaway or a friend? Enter it here. Capital letters, spaces and dashes don’t matter.</p>
  </header>

  {#if !known}
    <State kind="loading" title="Loading…" />
  {:else if !me}
    <State title="Sign in to redeem a code.">
      Codes are added to your account.
      {#snippet actions()}<a class="ui-btn primary" href="/signin?back={encodeURIComponent('/redeem')}">Sign in</a>{/snippet}
    </State>
  {:else}
    <form class="form" onsubmit={submit} novalidate>
      <label class="ui-field">
        <span class="ui-label">Code</span>
        <input class="ui-input mono" bind:this={input} value={code} oninput={(e) => (code = tidyCode(e.currentTarget.value))} autocomplete="off" autocapitalize="characters"
          spellcheck="false" maxlength="80" placeholder="ABCD-1234-EFGH" disabled={locked} aria-invalid={problem ? 'true' : undefined} aria-describedby={problem ? 'redeem-problem' : undefined} />
      </label>
      {#if problem}<p class="problem" id="redeem-problem" role="alert">{problem}</p>{/if}
      <div class="ui-actions">
        <button class="ui-btn primary" type="submit" disabled={busy || locked}>{busy ? 'Checking…' : 'Redeem'}</button>
        <a class="ui-btn ghost" href="/shop">Back to the shop</a>
      </div>
    </form>

    {#if result}
      <section class="result" aria-labelledby="gave-h" role="status">
        {#if gave}
          <h2 id="gave-h">Added to your account</h2>
          <p class="sub"><bdi>{clean(result.name)}</bdi> gave you {gave === 1 ? 'this' : 'these'}:</p>
          <ul>
            {#each result.badges as b (b.id)}
              <li><BadgeIcon badge={b} size={28} label /><span class="nm"><bdi>{clean(b.name)}</bdi><span class="kind">Badge</span></span></li>
            {/each}
            {#each result.paints as p (p.id)}
              <li><span class="pt"><PaintedName name={p.name} paint={p} /></span><span class="kind">Paint</span></li>
            {/each}
          </ul>
          <div class="ui-actions"><a class="ui-btn primary" href={sectionHref(me.id, 'cosmetics')}>Equip {gave === 1 ? 'it' : 'them'} in Cosmetics</a></div>
        {:else}
          <h2 id="gave-h">You have all of it already</h2>
          <p class="sub">That code worked, but <bdi>{clean(result.name)}</bdi> only holds things you already own.</p>
        {/if}
      </section>
    {/if}
  {/if}
</ColumnLayout>

<style>
  .head { padding-bottom: 24px; }
  .lead { margin: 12px 0 0; max-width: 56ch; font-size: var(--t-body, 16px); line-height: 1.6; color: var(--muted); }
  .form { display: grid; gap: 14px; }
  .mono { font-family: var(--font-mono); letter-spacing: .04em; }
  .problem { margin: 0; font-size: var(--t-small); color: var(--red); }
  .result { display: grid; gap: 10px; margin-top: 28px; padding: 18px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  h2 { margin: 0; font-size: var(--t-ui); font-weight: 600; }
  .sub { margin: 0; font-size: var(--t-small); color: var(--muted); }
  ul { list-style: none; margin: 4px 0 6px; padding: 0; display: grid; gap: 8px; }
  li { display: flex; align-items: center; gap: 12px; min-height: 40px; min-width: 0; }
  .nm { display: grid; min-width: 0; font-weight: 600; }
  .kind { font-size: var(--t-fine); font-weight: 400; color: var(--dim); }
  .pt { font-size: var(--t-ui); font-weight: 700; min-width: 0; }
  li > .kind { margin-left: auto; }
</style>
