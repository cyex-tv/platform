<script lang="ts">
  // What an account can actually do, and which role decided each permission (accountPermissions):
  // the roles from the highest position down with the User role last, the first explicit Allow
  // or Deny wins. Read-only; the roles themselves are changed on the Roles page.
  import { gql } from '$lib/api';
  import { ACCOUNT_PERMISSIONS } from '$lib/queries';
  import type { AccountPermissionsQuery } from '$lib/gql/graphql';
  import Icon from '$lib/components/Icon.svelte';
  import Name from '$lib/components/Name.svelte';
  import State from '$lib/components/State.svelte';
  import { count } from '$lib/format';
  import Box from './ui/Box.svelte';
  import { groupsFor } from './permissions';
  import { decisionGroups, decisionState } from './role-effects';

  type Row = AccountPermissionsQuery['accountPermissions'][number];
  let { accountId, refresh = '' }: { accountId: string; refresh?: string } = $props();
  let rows = $state<Row[] | null>(null);
  let problem = $state(false);
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = false;
    try {
      const r = (await gql(ACCOUNT_PERMISSIONS, { accountId })).accountPermissions;
      if (mine === seq) rows = r;
    } catch {
      if (mine === seq) problem = true;
    }
  }
  $effect(() => {
    void accountId;
    void refresh;
    rows = null;
    load();
  });

  const groups = $derived(rows ? decisionGroups(rows, groupsFor(rows.map((r) => r.permission))) : []);
  const ICON = { allowed: 'check', denied: 'ban', none: 'minus' } as const;
  const granted = $derived(rows ? rows.filter((r) => r.granted).length : 0);
</script>

<Box title="Effective permissions" count={rows ? `${count(granted)} of ${count(rows.length)} granted` : null}>
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !rows}
    <State bare kind="loading" title="Loading permissions…" />
  {:else if rows.length === 0}
    <p class="box-none">No permissions.</p>
  {:else}
    <div class="eff">
      {#each groups as g (g.id)}
        <section aria-label={g.label}>
          <h3 class="box-sub">{g.label}</h3>
          <ul>
            {#each g.items as d (d.permission)}
              {@const st = decisionState(d)}
              <li class={st}>
                <span class="p">{d.label}</span>
                <span class="d"><Icon name={ICON[st]} size={13} />
                  {#if d.owner}Owner
                  {:else if st === 'allowed'}Allowed by <Name value={d.roleName ?? ''} />
                  {:else if st === 'denied'}Denied by <Name value={d.roleName ?? ''} />
                  {:else}Not granted{/if}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</Box>

<style>
  .eff { display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: 0 28px; }
  section { min-width: 0; }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; gap: 10px; min-height: 32px; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .p { flex: 1; min-width: 0; }
  .d { display: inline-flex; align-items: center; gap: 6px; flex: none; max-width: 60%; font-size: var(--t-fine); color: var(--muted); text-align: right; }
  .d :global(svg) { flex: none; color: var(--dim); }
  .allowed .d { color: var(--text); font-weight: 600; }
  .allowed .d :global(svg) { color: var(--accent-text); }
  .denied .d, .denied .d :global(svg) { color: var(--red); font-weight: 700; }
  @media (pointer: coarse) { li { min-height: 44px; } }
</style>
