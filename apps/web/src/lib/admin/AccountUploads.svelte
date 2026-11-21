<script lang="ts">
  // An account's emotes in every state, newest first (accountEmotes), 20 at a time.
  import { gql } from '$lib/api';
  import { clean, count, dateTime } from '$lib/format';
  import type { AccountEmotesQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import Box from './ui/Box.svelte';
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

<Box title="Uploads" count={!problem && !(loading && !rows.length) ? count(total) : null}>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={() => load(false)}>Try again</button>{/snippet}
    </State>
  {:else if loading && !rows.length}
    <State bare kind="loading" title="Loading uploads…" />
  {:else if !rows.length}
    <p class="box-none">They haven’t uploaded an emote.</p>
  {:else}
    <ul class="ui-list">
      {#each rows as e (e.id)}
        <li>
          <Thumb emote={e} name={clean(e.name)} box={32} />
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
      <div class="more"><span class="ui-hint ui-num">{count(rows.length)} of {count(total)}</span><button class="ui-btn ghost small" type="button" disabled={loading} onclick={() => load(true)}>{loading ? 'Loading…' : 'Show more'}</button></div>
    {/if}
  {/if}
</Box>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { font-weight: 600; font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .flags { display: block; padding: 0; background: var(--bg); }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 44px; padding: 0 8px 0 16px; border-top: 1px solid var(--border); }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  a { display: inline-flex; align-items: center; min-height: 24px; }
</style>
