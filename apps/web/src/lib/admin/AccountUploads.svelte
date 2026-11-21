<script lang="ts">
  // An account's emotes in every state, newest first (accountEmotes), 20 at a time.
  import { gql } from '$lib/api';
  import { clean, count, dateTime } from '$lib/format';
  import type { AccountEmotesQuery } from '$lib/gql/graphql';
  import { ACCOUNT_EMOTES } from '$lib/queries';
  import State from '$lib/components/State.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import Box from './ui/Box.svelte';
  import EmoteFlagsPanel, { canEditFlags } from './EmoteFlagsPanel.svelte';

  type Row = AccountEmotesQuery['accountEmotes']['items'][number];
  let { accountId }: { accountId: string } = $props();
  const PER = 20;
  // The upload whose flags are open (staff with emotes.flags, ADR 0024), one at a time.
  let open = $state<string | null>(null);
  const flagsOk = $derived(canEditFlags());

  let rows = $state<Row[]>([]);
  let total = $state(0);
  let loading = $state(true);
  let problem = $state('');
  let seq = 0;

  async function load(more: boolean) {
    const mine = ++seq;
    loading = true;
    problem = '';
    try {
      const page = more ? Math.floor(rows.length / PER) + 1 : 1;
      const r = (await gql(ACCOUNT_EMOTES, { accountId, page, perPage: PER })).accountEmotes;
      if (mine !== seq) return;
      rows = more ? [...rows, ...r.items] : r.items;
      total = r.total;
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
    <ul class="adm-sheet" style="--cell: 150px" aria-label="Uploads">
      {#each rows as e (e.id)}
        <li>
          <div class="adm-cell up">
            <span class="pic"><Thumb emote={e} name={clean(e.name)} box={56} /></span>
            <a class="nm" href="/me/emotes/{encodeURIComponent(e.id)}"><bdi>{clean(e.name)}</bdi></a>
            <span class="sub"><span class="st {e.state.toLowerCase()}">{STATE_LABEL[e.state]}</span>{#if e.channelCount}<span class="adm-num">{count(e.channelCount)}</span>{/if}</span>
            <span class="sub adm-num">{dateTime(e.createdAt)}</span>
            {#if flagsOk}
              <button class="ui-btn ghost small fl" type="button" aria-expanded={open === e.id} aria-controls="up-flags-{e.id}" onclick={() => (open = open === e.id ? null : e.id)}>Flags</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if flagsOk && open}
      {@const e = rows.find((r) => r.id === open)}
      {#if e}
        <div class="flags" id="up-flags-{e.id}">
          <EmoteFlagsPanel bare emote={e} id="up-f-{e.id}" onsaved={(next) => (rows = rows.map((r) => (r.id === next.id ? { ...r, ...next } : r)))} />
        </div>
      {/if}
    {/if}
    {#if rows.length < total}
      <div class="more"><span class="ui-hint ui-num">{count(rows.length)} of {count(total)}</span><button class="ui-btn ghost small" type="button" disabled={loading} onclick={() => load(true)}>{loading ? 'Loading…' : 'Show more'}</button></div>
    {/if}
  {/if}
</Box>

<style>
  .up { cursor: default; }
  .up .nm { font-weight: 600; min-height: 22px; color: var(--text); text-decoration: none; }
  .up .pic { height: 72px; }
  .st { color: var(--dim); font-weight: 500; }
  .st.approved { color: var(--green); }
  .st.rejected, .st.removed { color: var(--red); }
  .fl { justify-self: start; margin: 4px 0 -4px -8px; }
  .flags { display: block; padding: 0; margin-top: 8px; border-top: 1px solid var(--border); background: var(--bg); }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 44px; padding: 0; border-top: 1px solid var(--border); }
</style>
