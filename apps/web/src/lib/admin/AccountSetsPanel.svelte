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
    <table class="adm-table stack">
      <thead><tr><th>Set</th><th>Kind</th><th class="num">Slots</th><th>State</th></tr></thead>
      <tbody>
        {#each all ? rows : rows.slice(0, LIMIT) as s (s.id)}
          <tr>
            <td class="pk ell"><a href="/sets/{encodeURIComponent(s.id)}"><bdi>{clean(s.name)}</bdi></a></td>
            <td data-l="Kind">{s.kind === 'PERSONAL' ? 'Personal' : 'Channel'}</td>
            <td data-l="Slots" class="num">{count(s.slotsUsed)}/{count(s.capacity)}</td>
            <td data-l="State">{#if s.active}<span class="adm-flags"><span class="ok">Active</span></span>{/if}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <More total={rows.length} limit={LIMIT} bind:expanded={all} />
  {/if}
</Box>

<style>
  td a { display: block; min-height: 22px; line-height: 22px; font-weight: 600; font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .adm-table { border-top: 0; }
</style>
