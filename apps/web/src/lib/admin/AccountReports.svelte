<script lang="ts">
  // Reports about an account's emotes and the reports it filed (accountReports), newest first.
  import { gql } from '$lib/api';
  import { clean, dateTime } from '$lib/format';
  import type { AccountReportsQuery } from '$lib/gql/graphql';
  import { ACCOUNT_REPORTS } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import AccountLink from './AccountLink.svelte';
  import Box from './ui/Box.svelte';
  import More from './ui/More.svelte';

  type Row = AccountReportsQuery['accountReports']['about'][number];
  let { accountId }: { accountId: string } = $props();

  let data = $state<AccountReportsQuery['accountReports'] | null>(null);
  let problem = $state('');
  const LIMIT = 5;
  const open = $state({ about: false, by: false });
  let seq = 0;

  async function load() {
    const mine = ++seq;
    problem = '';
    try {
      const r = (await gql(ACCOUNT_REPORTS, { accountId })).accountReports;
      if (mine === seq) data = r;
    } catch (e) {
      if (mine === seq) problem = e instanceof Error ? e.message : 'That didn’t load.';
    }
  }
  $effect(() => {
    void accountId;
    data = null;
    load();
  });

  const STATE = { OPEN: 'Open', DISMISSED: 'Dismissed', ACTIONED: 'Taken down' } as const;
</script>

{#snippet list(rows: Row[], who: 'owner' | 'reporter', empty: string, which: 'about' | 'by')}
  {#if rows.length}
    <ul class="ui-list">
      {#each open[which] ? rows : rows.slice(0, LIMIT) as r (r.id)}
        <li>
          {#if r.emote}<Thumb emote={r.emote} name={clean(r.emote.name)} box={36} />{/if}
          <span class="what">
            <strong>
              {#if r.emote}<a href="/me/emotes/{encodeURIComponent(r.emote.id)}"><bdi>{clean(r.emote.name)}</bdi></a>{:else}Deleted emote{/if}
              <span class="ui-chip {r.state === 'OPEN' ? 'pending' : r.state === 'ACTIONED' ? 'rejected' : ''}">{STATE[r.state]}</span>
            </strong>
            <span class="reason">“<bdi>{clean(r.reason)}</bdi>”</span>
            <span class="ui-hint ui-num">
              {dateTime(r.createdAt)}
              {#if who === 'reporter' && r.reporterId}{' · by '}<AccountLink id={r.reporterId} />{/if}
              {#if who === 'owner' && r.emote?.owner}{' · emote by '}<AccountLink id={r.emote.owner.id} name={r.emote.owner.displayName} />{/if}
              {#if r.resolution}{' · '}<bdi>{clean(r.resolution)}</bdi>{/if}
            </span>
          </span>
        </li>
      {/each}
    </ul>
    <More total={rows.length} limit={LIMIT} bind:expanded={open[which]} />
  {:else}
    <p class="box-none">{empty}</p>
  {/if}
{/snippet}

<Box title="Reports">
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={load}>Try again</button>{/snippet}
    </State>
  {:else if !data}
    <State bare kind="loading" title="Loading reports…" />
  {:else}
    <h3 class="box-sub">About their emotes</h3>
    {@render list(data.about, 'reporter', 'No one has reported their emotes.', 'about')}
    <h3 class="box-sub">Filed by them</h3>
    {@render list(data.by, 'owner', 'They haven’t reported anything.', 'by')}
  {/if}
</Box>

<style>
  .what { display: grid; gap: 1px; flex: 1; min-width: 0; }
  .what strong { display: flex; align-items: center; gap: 8px; min-width: 0; font-weight: 600; font-size: var(--t-small); }
  .what strong a { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .what strong .ui-chip { flex: none; }
  .reason { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-small); color: var(--muted); }
  .what .ui-hint { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  a { display: inline-flex; align-items: center; min-height: 24px; }
</style>
