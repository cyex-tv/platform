<script lang="ts">
  // Where an account is signed in (accountSessions, users.sessions): device, kind, last use and
  // expiry. Never tokens. Blocking the account (Restrictions) ends them all.
  import { gql } from '$lib/api';
  import type { AccountSessionsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_SESSIONS } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import { date, dateTime } from '$lib/format';

  type Row = AccountSessionsQuery['accountSessions'][number];
  let { accountId, refresh = '' }: { accountId: string; refresh?: string } = $props();
  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
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
    load();
  });
</script>

<section class="ui-panel" aria-labelledby="ss-h">
  <header><h2 id="ss-h">Sessions</h2>{#if rows}<span class="ui-hint ui-num">{rows.length}</span>{/if}</header>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading sessions…" />
  {:else if rows.length === 0}
    <p class="ui-hint empty">Not signed in anywhere.</p>
  {:else}
    <ul class="ui-list">
      {#each rows as s (s.id)}
        <li>
          <span class="what"><strong title={s.device}>{s.device}</strong><span class="ui-hint">{s.kind === 'SITE' ? 'Website' : 'App'} · signed in <span class="ui-num">{dateTime(s.createdAt)}</span></span></span>
          <span class="when ui-hint">Last used <span class="ui-num">{dateTime(s.lastSeenAt)}</span><br />Ends <span class="ui-num">{date(s.expiresAt)}</span></span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-size: var(--text-sub); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { text-align: right; }
  .empty { margin: 0; padding: 10px 14px 12px; }
</style>
