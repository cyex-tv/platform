<script lang="ts">
  // An account's emote sets as the public sees them (accountSets): name, kind, how full, and
  // which one a channel shows now.
  import { gql } from '$lib/api';
  import type { AccountSetsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_SETS } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';
  import Box from './ui/Box.svelte';
  import More from './ui/More.svelte';

  type Row = AccountSetsQuery['accountSets'][number];
  let { accountId }: { accountId: string } = $props();
  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
  const LIMIT = 6;
  let all = $state(false);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(ACCOUNT_SETS, { accountId })).accountSets;
      if (mine === seq) rows = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    rows = null;
    all = false;
    load();
  });
</script>

<Box title="Emote sets" count={rows ? count(rows.length) : null}>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading sets…" />
  {:else if rows.length === 0}
    <p class="box-none">No sets.</p>
  {:else}
    <ul class="ui-list">
      {#each all ? rows : rows.slice(0, LIMIT) as s (s.id)}
        <li>
          <span class="what"><a href="/sets/{encodeURIComponent(s.id)}"><bdi>{clean(s.name)}</bdi></a>
            <span class="ui-hint">{s.kind === 'PERSONAL' ? 'Personal' : 'Channel'} · <span class="ui-num">{count(s.slotsUsed)}/{count(s.capacity)}</span></span></span>
          {#if s.active}<span class="ui-chip approved">Active</span>{/if}
        </li>
      {/each}
    </ul>
    <More total={rows.length} limit={LIMIT} bind:expanded={all} />
  {/if}
</Box>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { display: block; min-height: 22px; line-height: 22px; font-weight: 600; font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
