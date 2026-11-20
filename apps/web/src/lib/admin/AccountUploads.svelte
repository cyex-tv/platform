<script lang="ts">
  // An account's emotes in every state, newest first (accountEmotes), 20 at a time.
  import { gql } from '$lib/api';
  import { clean, count, dateTime } from '$lib/format';
  import type { AccountEmotesQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import EmoteFlagsPanel, { canEditFlags } from './EmoteFlagsPanel.svelte';

  type Row = AccountEmotesQuery['accountEmotes']['items'][number];
  let { accountId }: { accountId: string } = $props();
  const PER = 20;
  // The upload whose flags are open (staff with emotes.flags, ADR 0024), one at a time.
  let open = $state<string | null>(null);
  const flagsOk = $derived(canEditFlags());

  let rows = $state<Row[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let problem = $state('');
  let seq = 0;

  async function load(more: boolean) {
    const mine = ++seq;
    loading = true;
    problem = '';
    try {
      const page = more ? Math.floor(rows.length / PER) + 1 : 1;
      const r = (await gql(ACCOUNT_EMOTES, { accountId, page, perPage: PER })).accountEmotes;
      if (mine !== seq) return;
      rows = more ? [...rows, ...r.items] : r.items;
      total = r.total;
    } catch (e) {
      if (mine === seq) problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      if (mine === seq) loading = false;
    }
  }
  $effect(() => {
    void accountId;
    rows = [];
    load(false);
  });
</script>

<section class="ui-panel" aria-labelledby="up-h">
  <header><h2 id="up-h">Uploads</h2>{#if !problem && !(loading && !rows.length)}<span class="ui-hint ui-num">{count(total)}</span>{/if}</header>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={() => load(false)}>Try again</button>{/snippet}
    </State>
  {:else if loading && !rows.length}
    <State bare kind="loading" title="Loading uploads…" />
  {:else if !rows.length}
    <State bare title="No uploads.">They haven’t uploaded an emote.</State>
  {:else}
    <ul class="ui-list">
      {#each rows as e (e.id)}
        <li>
          <Thumb emote={e} name={clean(e.name)} box={40} />
          <span class="what">
            <a href="/me/emotes/{encodeURIComponent(e.id)}"><bdi>{clean(e.name)}</bdi></a>
            <span class="ui-hint ui-num">{dateTime(e.createdAt)}{#if e.channelCount}{' · '}{count(e.channelCount)} channels{/if}</span>
          </span>
          <span class="ui-chip {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>
          {#if flagsOk}
            <button class="ui-btn ghost small" type="button" aria-expanded={open === e.id} aria-controls="up-flags-{e.id}" onclick={() => (open = open === e.id ? null : e.id)}>Flags</button>
          {/if}
        </li>
        {#if flagsOk && open === e.id}
          <li class="flags" id="up-flags-{e.id}">
            <EmoteFlagsPanel bare emote={e} id="up-f-{e.id}" onsaved={(next) => (rows = rows.map((r) => (r.id === next.id ? { ...r, ...next } : r)))} />
          </li>
        {/if}
      {/each}
    </ul>
    {#if rows.length < total}
      <div class="more"><button class="ui-btn secondary small" type="button" disabled={loading} onclick={() => load(true)}>{loading ? 'Loading…' : 'Show more'}</button></div>
    {/if}
  {/if}
</section>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .flags { display: block; padding: 0; background: var(--surface-2, var(--surface)); }
  .more { display: flex; justify-content: center; padding: 10px; border-top: 1px solid var(--border); }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  a { display: inline-flex; align-items: center; min-height: 24px; }
</style>
