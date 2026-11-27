<script lang="ts">
  // An account's emotes in every state, newest first (accountEmotes), 20 at a time, each with its
  // two approvals (public listing, personal use; ADR 0039). A filter narrows the loaded rows to the
  // ones approved (or waiting, or denied) for personal use; staff who review change the approvals
  // of one upload at a time.
  import { gql } from '$lib/api';
  import { clean, count, dateTime } from '$lib/format';
  import type { AccountEmotesQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import Box from './ui/Box.svelte';
  import { can } from '$lib/tool/session.svelte';
  import ApprovalsPanel from './ApprovalsPanel.svelte';
  import { byPersonal, type Approvals, type PersonalFilter } from './queue';

  type Row = AccountEmotesQuery['accountEmotes']['items'][number];
  let { accountId }: { accountId: string } = $props();
  const PER = 20;
  // The upload whose approvals are open (staff who review), one at a time.
  let open = $state<string | null>(null);
  const approvalsOk = $derived(can('emotes.review') || can('emotes.review_personal'));
  let filter = $state<PersonalFilter>('all');
  const FILTERS: { value: PersonalFilter; label: string }[] = [
    { value: 'all', label: 'All uploads' },
    { value: 'APPROVED', label: 'Approved for personal use' },
    { value: 'PENDING', label: 'Personal use waiting' },
    { value: 'DENIED', label: 'Personal use denied' },
  ];
  const WORD: Record<string, string> = { PENDING: 'waiting', APPROVED: 'approved', DENIED: 'denied' };

  let rows = $state<Row[]>([]);
  const shown = $derived(byPersonal(rows, filter));
  let total = $state(0);
  /** Where the next page starts (cursor paging); null when there is none. */
  let after = $state<string | null>(null);
  let loading = $state(true);
  let problem = $state('');
  let seq = 0;

  async function load(more: boolean) {
    const mine = ++seq;
    loading = true;
    problem = '';
    try {
      const from = more ? after : null;
      const r = (await gql(ACCOUNT_EMOTES, { accountId, perPage: PER, after: from })).accountEmotes;
      if (mine !== seq) return;
      rows = more ? [...rows, ...r.items] : r.items;
      total = r.total;
      after = r.next ?? null;
    } catch (e) {
      if (mine === seq) problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      if (mine === seq) loading = false;
    }
  }
  $effect(() => {
    void accountId;
    rows = [];
    load(false);
  });
</script>

<Box title="Uploads" count={!problem && !(loading && !rows.length) ? count(total) : null}>
  {#snippet actions()}
    {#if rows.length}
      <select class="ui-input pick" aria-label="Filter uploads" bind:value={filter}>
        {#each FILTERS as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
      </select>
    {/if}
  {/snippet}
  {#if problem}
    <State bare kind="error" title="That didn’t load">
      {problem}
      {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={() => load(false)}>Try again</button>{/snippet}
    </State>
  {:else if loading && !rows.length}
    <State bare kind="loading" title="Loading uploads…" />
  {:else if !rows.length}
    <p class="box-none">They haven’t uploaded an emote.</p>
  {:else}
    {#if !shown.length}<p class="box-none">None of the {count(rows.length)} loaded uploads match.{#if rows.length < total} Show more to look further.{/if}</p>{/if}
    <ul class="adm-sheet" style="--cell: 150px" aria-label="Uploads">
      {#each shown as e (e.id)}
        <li>
          <div class="adm-cell up">
            <span class="pic"><Thumb emote={e} name={clean(e.name)} box={56} /></span>
            <a class="nm" href="/me/emotes/{encodeURIComponent(e.id)}"><bdi>{clean(e.name)}</bdi></a>
            <span class="sub"><span class="st {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>{#if e.channelCount}<span class="adm-num">{count(e.channelCount)}</span>{/if}</span>
            <span class="sub adm-num">{dateTime(e.createdAt)}</span>
            {#if e.approvals}
              <span class="sub ap"><span class="a {e.approvals.publicListing.toLowerCase()}" title="Public listing {WORD[e.approvals.publicListing]}">Listing {WORD[e.approvals.publicListing]}</span><span class="a {e.approvals.personalUse.toLowerCase()}" title="Personal use {WORD[e.approvals.personalUse]}">Personal {WORD[e.approvals.personalUse]}</span></span>
            {/if}
            {#if approvalsOk}
              <button class="ui-btn ghost small fl" type="button" aria-expanded={open === e.id} aria-controls="up-ap-{e.id}" onclick={() => (open = open === e.id ? null : e.id)}>Approvals</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if approvalsOk && open}
      {@const e = rows.find((r) => r.id === open)}
      {#if e}
        <div class="flags" id="up-ap-{e.id}">
          <ApprovalsPanel emote={e} id="up-a-{e.id}" onsaved={(a: Approvals) => (rows = rows.map((r) => (r.id === e.id ? { ...r, approvals: a } : r)))} />
        </div>
      {/if}
    {/if}
    {#if rows.length < total && after}
      <div class="more"><span class="ui-hint ui-num">{count(rows.length)} of {count(total)}</span><button class="ui-btn ghost small" type="button" disabled={loading} onclick={() => load(true)}>{loading ? 'Loading…' : 'Show more'}</button></div>
    {/if}
  {/if}
</Box>

<style>
  .up { cursor: default; }
  .up .nm { font-weight: 600; min-height: 22px; color: var(--text); text-decoration: none; }
  .up .pic { height: 72px; }
  .st { color: var(--dim); font-weight: 500; }
  .pick { width: auto; min-height: 28px; padding-left: 10px; }
  .ap { display: flex; flex-wrap: wrap; gap: 0 8px; }
  .a.approved { color: var(--green); }
  .a.denied { color: var(--red); }
  .st.approved { color: var(--green); }
  .st.rejected, .st.removed { color: var(--red); }
  .fl { justify-self: start; margin: 4px 0 -4px -8px; }
  .flags { display: block; padding: 0 12px 12px; margin-top: 8px; border-top: 1px solid var(--border); background: var(--bg); }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 44px; padding: 0; border-top: 1px solid var(--border); }
</style>
