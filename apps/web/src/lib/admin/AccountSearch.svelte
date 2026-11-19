<script lang="ts">
  // Admin: accounts found by name or login (searchAccounts, the same public search the site uses,
  // so blocked accounts are not listed: look one up by its exact login to see or lift a block).
  // Each row opens the account's admin page.
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import State from '$lib/components/State.svelte';
  import { clean, count, iso } from '$lib/format';
  import { SEARCH_ACCOUNTS } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';

  const SHOWN = 50;
  /** With `onpick` the rows pick the account instead of opening it, and the list is compact (a filter's lookup). */
  let { query, onpick }: { query: string; onpick?: (account: { id: string; displayName: string }) => void } = $props();

  const L = loader(async ({ fetch }) => {
    const text = query.trim();
    const r = await gql(SEARCH_ACCOUNTS, { query: text, page: 1, perPage: SHOWN }, fetch);
    return { text, ...r.searchAccounts };
  });

  const where = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');
  const open = (h: { account: { id: string } }) => `/admin/users/${encodeURIComponent(h.account.id)}`;
</script>

<LoadGate source={L} label="Searching…">
  {#snippet children(r)}
    {#if r.items.length === 0}
      <State title={`No one matches “${iso(r.text)}”.`}>Only people who have signed in are listed, and blocked accounts are not: find one by its exact login.</State>
    {:else}
      <p class="ui-hint total" class:compact={!!onpick} aria-live="polite">
        {count(r.total)} {r.total === 1 ? 'account' : 'accounts'}{#if r.total > r.items.length}, the first {count(r.items.length)} shown. Narrow the name to see the rest.{/if}
      </p>
      <ul class="ui-panel ui-list result" class:compact={!!onpick} aria-label="Accounts">
        {#each r.items as h (h.account.id)}
          <li>
            <Avatar src={h.account.avatar} name={h.account.displayName} size={40} tone="tool" />
            <span class="what">
              {#if onpick}<span class="name"><bdi>{clean(h.account.displayName)}</bdi></span>{:else}<a class="name" href={open(h)}><bdi>{clean(h.account.displayName)}</bdi></a>{/if}
              <span class="ui-hint logins">
                {#each h.identities as i (i.platform + i.login)}<span class="login"><bdi>{clean(i.login)}</bdi> on {where(i.platform)}</span>{/each}
              </span>
            </span>
            {#if onpick}
              <button class="ui-btn secondary small" type="button" onclick={() => onpick(h.account)}>Pick</button>
            {:else}
              <a class="ui-btn secondary small" href={open(h)}>Open account</a>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/snippet}
</LoadGate>

<style>
  .total { margin: 0 0 8px; }
  .result { max-width: var(--measure-form); }
  .result.compact { max-height: 260px; overflow-y: auto; }
  .what { display: grid; flex: 1; min-width: 0; }
  .logins { display: flex; flex-wrap: wrap; gap: 0 12px; }
  .login { overflow-wrap: anywhere; }
  .name { font-weight: 600; color: var(--text); overflow-wrap: anywhere; }
</style>
