<script lang="ts">
  // An account's emote sets as the public sees them (accountSets): name, kind, how full, and
  // which one a channel shows now.
  import { gql } from '$lib/api';
  import type { AccountSetsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_SETS } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { clean, count } from '$lib/format';

  type Row = AccountSetsQuery['accountSets'][number];
  let { accountId }: { accountId: string } = $props();
  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
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
    load();
  });
</script>

<section class="ui-panel" aria-labelledby="st-h">
  <header><h2 id="st-h">Emote sets</h2>{#if rows}<span class="ui-hint ui-num">{rows.length}</span>{/if}</header>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading sets…" />
  {:else if rows.length === 0}
    <p class="ui-hint empty">No sets.</p>
  {:else}
    <ul class="ui-list">
      {#each rows as s (s.id)}
        <li>
          <span class="what"><a href="/sets/{encodeURIComponent(s.id)}"><bdi>{clean(s.name)}</bdi></a>
            <span class="ui-hint">{s.kind === 'PERSONAL' ? 'Personal' : 'Channel'} · <span class="ui-num">{count(s.slotsUsed)}/{count(s.capacity)}</span></span></span>
          {#if s.active}<span class="ui-chip approved">Active</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what a { display: block; min-height: 24px; line-height: 24px; font-weight: 600; font-size: var(--text-sub); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { margin: 0; padding: 10px 14px 12px; }
</style>
