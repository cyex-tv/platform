<script lang="ts">
  // Admin: accounts found by name or login (searchAccounts, the same public search the site uses,
  // so blocked accounts are not listed: look one up by its exact login to see or lift a block).
  // Each row opens the account's admin page.
  import { gql } from '$lib/api';
  import State from '$lib/components/State.svelte';
  import { count, iso } from '$lib/format';
  import { SEARCH_ACCOUNTS } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import AccountList from './AccountList.svelte';

  const SHOWN = 50;
  /** With `onpick` the rows pick the account instead of opening it, and the list is compact (a filter's lookup). */
  let { query, onpick }: { query: string; onpick?: (account: { id: string; displayName: string }) => void } = $props();

  const L = loader(async ({ fetch }) => {
    const text = query.trim();
    const r = await gql(SEARCH_ACCOUNTS, { query: text, page: 1, perPage: SHOWN }, fetch);
    return { text, ...r.searchAccounts };
  });
</script>

<LoadGate source={L} label="Searching…">
  {#snippet children(r)}
    {#if r.items.length === 0}
      <State title={`No one matches “${iso(r.text)}”.`}>Only people who have signed in are listed, and blocked accounts are not: find one by its exact login.</State>
    {:else}
      <p class="ui-hint total" aria-live="polite">
        {count(r.total)} {r.total === 1 ? 'account' : 'accounts'}{#if r.total > r.items.length}, the first {count(r.items.length)} shown. Narrow the name to see the rest.{/if}
      </p>
      <AccountList items={r.items} {onpick} compact={!!onpick} />
    {/if}
  {/snippet}
</LoadGate>

<style>
  .total { margin: 0 0 8px; }
</style>
