<script lang="ts">
  // The latest audited changes that target an account (auditLog by target), with a way on to the
  // whole log, and to what the account did itself.
  import { gql } from '$lib/api';
  import { dateTime } from '$lib/format';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { AUDIT } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import AccountLink from './AccountLink.svelte';
  import { actionText } from './audit';

  let { accountId }: { accountId: string } = $props();
  let rows = $state<AuditQuery['auditLog'] | null>(null);
  let problem = $state('');
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = '';
    try {
      const r = (await gql(AUDIT, { targetKind: 'account', targetId: accountId, limit: 8 })).auditLog;
      if (mine === seq) rows = r;
    } catch (e) {
      if (mine === seq) problem = e instanceof Error ? e.message : 'That didn’t load.';
    }
  }
  $effect(() => {
    void accountId;
    rows = null;
    load();
  });
  const q = $derived(encodeURIComponent(accountId));
</script>

<section class="ui-panel" aria-labelledby="aa-h">
  <header><h2 id="aa-h">Audit log</h2></header>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading the log…" />
  {:else}
    {#if rows.length}
      <ul class="ui-list">
        {#each rows as e (e.id)}
          <li>
            <span class="what">
              <strong>{actionText(e.action)}</strong>
              <span class="ui-hint ui-num">{dateTime(e.createdAt)}{' · by '}<AccountLink id={e.actorId} /></span>
            </span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="ui-hint pad">Nothing has been done to this account.</p>
    {/if}
    <p class="more">
      <a href="/admin/audit?kind=account&id={q}">Everything done to them</a>
      <a href="/admin/audit?actor={q}">Everything they did</a>
    </p>
  {/if}
</section>

<style>
  .what { display: grid; flex: 1; min-width: 0; }
  .what strong { font-weight: 600; }
  .pad { margin: 0; padding: 10px 14px; }
  .more { display: flex; flex-wrap: wrap; gap: 4px 20px; margin: 0; padding: 10px 14px; border-top: 1px solid var(--border); font-size: var(--t-small); }
</style>
