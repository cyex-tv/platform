<script lang="ts">
  // On an emote's public page, for people who are signed in: add it to one of their sets, or
  // report it. Signed out, it offers sign-in instead. The browser asks (only it has the cookie).
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import type { MySetNamesQuery } from '$lib/gql/graphql';
  import { ADD_SET_EMOTE, ME, MY_SET_NAMES, REPORT_EMOTE } from '$lib/queries';

  let { emoteId, name }: { emoteId: string; name: string } = $props();

  type Sets = MySetNamesQuery['mySets'];
  let who = $state<'loading' | 'out' | 'in' | 'none'>('loading');
  let sets = $state<Sets>([]);
  let target = $state('');
  let added = $state<string | null>(null);
  let addError = $state<string | null>(null);
  let reporting = $state(false);
  let reason = $state('');
  let reported = $state(false);
  let reportError = $state<string | null>(null);
  let busy = $state(false);

  const open = $derived(sets.filter((s) => s.kind !== 'GLOBAL' && !s.emotes.some((m) => m.emote.id === emoteId)));

  onMount(async () => {
    try {
      await gql(ME, {});
      who = 'in';
      sets = (await gql(MY_SET_NAMES, {})).mySets;
      target = open[0]?.id ?? '';
    } catch (e) {
      // Signed out: offer sign-in. cyex.tv unreachable: offer nothing.
      who = e instanceof ApiError && e.code === 'signed_out' ? 'out' : 'none';
    }
  });

  async function add() {
    if (!target) return;
    busy = true;
    addError = null;
    try {
      await gql(ADD_SET_EMOTE, { setId: target, emoteId });
      const set = sets.find((s) => s.id === target);
      added = set?.name ?? 'your set';
      if (set) set.emotes.push({ emote: { id: emoteId } });
      target = open[0]?.id ?? '';
    } catch (e) {
      addError = e instanceof Error ? e.message : 'That didn’t work.';
    } finally {
      busy = false;
    }
  }

  async function report(e: SubmitEvent) {
    e.preventDefault();
    if (!reason.trim()) return;
    busy = true;
    reportError = null;
    try {
      await gql(REPORT_EMOTE, { id: emoteId, reason: reason.trim() });
      reported = true;
      reporting = false;
    } catch (err) {
      reportError = err instanceof Error ? err.message : 'That didn’t work.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="actions-block" aria-live="polite">
  {#if who === 'out'}
    <a class="btn btn-s" href="/signin?back={encodeURIComponent(page.url.pathname)}">Sign in to add {name} to your set</a>
  {:else if who === 'in'}
    {#if open.length}
      <div class="add">
        <select class="input select" bind:value={target} aria-label="Your set">
          {#each open as s}<option value={s.id}>{s.name}</option>{/each}
        </select>
        <button class="btn btn-primary btn-s" type="button" disabled={busy} onclick={add}>Add to set</button>
      </div>
    {:else if sets.length}
      <p class="fine">It’s in all your sets.</p>
    {:else}
      <p class="fine">You have no sets yet. <a class="link" href="/me/sets">Make one</a></p>
    {/if}
    {#if added}<p class="fine ok">Added to {added}.</p>{/if}
    {#if addError}<p class="fine err" role="alert">{addError}</p>{/if}

    {#if reported}
      <p class="fine">Thanks. A moderator will look at it.</p>
    {:else if reporting}
      <form class="report" onsubmit={report}>
        <label class="fine" for="report-reason">What’s wrong with it?</label>
        <textarea id="report-reason" class="input area" bind:value={reason} maxlength="500" rows="3" placeholder="It copies someone else’s art, it’s hateful, …"></textarea>
        <div class="row">
          <button class="btn btn-s" type="submit" disabled={busy || !reason.trim()}>Send report</button>
          <button class="btn btn-s ghost" type="button" onclick={() => (reporting = false)}>Cancel</button>
        </div>
        {#if reportError}<p class="fine err" role="alert">{reportError}</p>{/if}
      </form>
    {:else}
      <button class="report-link fine" type="button" onclick={() => (reporting = true)}>Report this emote</button>
    {/if}
  {/if}
</div>

<style>
  .actions-block { display: grid; gap: 12px; margin-top: 28px; min-height: 38px; }
  .add { display: flex; flex-wrap: wrap; gap: 10px; }
  .select { flex: 0 1 240px; height: 38px; padding: 0 12px; }
  .area { height: auto; padding: 12px 14px; resize: vertical; }
  .report { display: grid; gap: 8px; max-width: 480px; }
  .row { display: flex; gap: 10px; }
  .ghost { border-color: transparent; background: none; }
  .ok { color: var(--text); }
  .err { color: var(--text); }
  .report-link { justify-self: start; padding: 0; border: 0; background: none; text-decoration: underline; text-underline-offset: 3px; cursor: pointer; }
  .report-link:hover { color: var(--text); }
</style>
