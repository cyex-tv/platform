<script lang="ts">
  // The latest audited changes that target an account (auditLog by target), with a way on to the
  // whole log, and to what the account did itself.
  import { gql } from '$lib/api';
  import { dateTime } from '$lib/format';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { AUDIT } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import AccountLink from './AccountLink.svelte';
  import Box from './ui/Box.svelte';
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

<Box title="Audit log">
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading the log…" />
  {:else}
    {#if rows.length}
      <table class="adm-table stack">
        <thead><tr><th>Change</th><th>When</th><th>By</th></tr></thead>
        <tbody>
          {#each rows as e (e.id)}
            <tr>
              <td class="pk"><strong>{actionText(e.action)}</strong></td>
              <td data-l="When" class="mono">{dateTime(e.createdAt)}</td>
              <td data-l="By"><AccountLink id={e.actorId} /></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="box-none">Nothing has been done to this account.</p>
    {/if}
    <p class="more">
      <a href="/admin/audit?kind=account&id={q}">Everything done to them</a>
      <a href="/admin/audit?actor={q}">Everything they did</a>
    </p>
  {/if}
</Box>

<style>
  td strong { font-weight: 600; font-size: var(--t-small); }
  .adm-table { border-top: 0; }
  .more a { display: inline-flex; align-items: center; min-height: 28px; }
  .more { display: flex; flex-wrap: wrap; gap: 4px 20px; margin: 0; padding: 8px 0; border-top: 1px solid var(--border); font-size: var(--t-small); }
</style>
