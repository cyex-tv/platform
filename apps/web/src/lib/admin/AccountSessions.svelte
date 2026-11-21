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
    <table class="adm-table stack">
      <thead><tr><th>Device</th><th>Kind</th><th>Signed in</th><th>Last used</th><th>Ends</th></tr></thead>
      <tbody>
        {#each all ? rows : rows.slice(0, LIMIT) as s (s.id)}
          <tr>
            <td class="pk lead" title={s.device}><strong>{s.device}</strong></td>
            <td data-l="Kind">{s.kind === 'SITE' ? 'Website' : 'App'}</td>
            <td data-l="Signed in" class="mono">{dateTime(s.createdAt)}</td>
            <td data-l="Last used" class="mono">{dateTime(s.lastSeenAt)}</td>
            <td data-l="Ends" class="mono">{date(s.expiresAt)}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    <More total={rows.length} limit={LIMIT} bind:expanded={all} />
  {/if}
</Box>

<style>
  td strong { font-size: var(--t-small); }
  .adm-table { border-top: 0; }
</style>
