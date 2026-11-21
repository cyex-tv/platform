<script lang="ts">
  // Where an account is signed in (accountSessions, users.sessions): device, kind, last use and
  // expiry. Never tokens. Blocking the account (Restrictions) ends them all.
  import { gql } from '$lib/api';
  import type { AccountSessionsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_SESSIONS } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { count, date, dateTime } from '$lib/format';
  import Box from './ui/Box.svelte';
  import More from './ui/More.svelte';

  type Row = AccountSessionsQuery['accountSessions'][number];
  let { accountId, refresh = '' }: { accountId: string; refresh?: string } = $props();
  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
  const LIMIT = 6;
  let all = $state(false);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(ACCOUNT_SESSIONS, { accountId })).accountSessions;
      if (mine === seq) rows = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    void refresh;
    all = false;
    load();
  });
</script>

<Box title="Sessions" count={rows ? count(rows.length) : null}>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading sessions…" />
  {:else if rows.length === 0}
    <p class="box-none">Not signed in anywhere.</p>
  {:else}
    <ul class="ui-list">
      {#each all ? rows : rows.slice(0, LIMIT) as s (s.id)}
        <li>
          <span class="what"><strong title={s.device}>{s.device}</strong><span class="ui-hint">{s.kind === 'SITE' ? 'Website' : 'App'} · signed in <span class="ui-num">{dateTime(s.createdAt)}</span></span></span>
          <span class="when ui-hint">Last used <span class="ui-num">{dateTime(s.lastSeenAt)}</span><br />Ends <span class="ui-num">{date(s.expiresAt)}</span></span>
        </li>
      {/each}
    </ul>
    <More total={rows.length} limit={LIMIT} bind:expanded={all} />
  {/if}
</Box>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-size: var(--t-small); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { flex: none; text-align: right; font-size: var(--t-fine); line-height: 1.3; }
  @media (max-width: 640px) {
    li { flex-wrap: wrap; row-gap: 2px; }
    .when { flex-basis: 100%; text-align: left; }
  }
</style>
