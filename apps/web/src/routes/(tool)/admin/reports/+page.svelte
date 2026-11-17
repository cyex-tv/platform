<script lang="ts">
  import { gql } from '$lib/api';
  import { count, date } from '$lib/format';
  import type { ReportQueueQuery } from '$lib/gql/graphql';
  import { REPORT_QUEUE, RESOLVE_REPORT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import { can } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Report = ReportQueueQuery['reportQueue']['items'][number];
  const L = loader(async ({ fetch }) => (await gql(REPORT_QUEUE, { page: 1, perPage: 50 }, fetch)).reportQueue);
  const total = $derived(L.data?.total ?? 0);
  let current = $state<Report | null>(null);
  let mode = $state<'DISMISS' | 'TAKE_DOWN'>('DISMISS');
  let dialog: ReturnType<typeof ReasonDialog>;
  const takedown = can('emotes.takedown');

  const load = () => L.reload();

  function ask(r: Report, action: 'DISMISS' | 'TAKE_DOWN') {
    current = r;
    mode = action;
    dialog.open();
  }

  async function resolve(note: string) {
    const r = current;
    if (!r) return;
    try {
      await gql(RESOLVE_REPORT, { id: r.id, action: mode, note: note || null });
      notify(mode === 'DISMISS' ? 'Report dismissed.' : `${r.emote?.name ?? 'The emote'} taken down everywhere.`);
      // A takedown closes every open report on the same image, so load again.
      await load();
    } catch (err) {
      fail(err);
      throw err;
    }
  }
</script>

<svelte:head><title>Reports - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Reports</h1><p><span class="ui-mono">{count(total)}</span> open, oldest first</p></div></div>

<LoadGate source={L} label="Loading reports…">
{#snippet children(queue)}
{@const items = queue.items}
{#if items.length === 0}
  <div class="ui-empty">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 6 9 17l-5-5" /></svg>
    <p>No open reports.</p>
  </div>
{:else}
  <section class="ui-panel">
    <ul class="ui-list">
      {#each items as r (r.id)}
        <li>
          {#if r.emote}<Thumb emote={r.emote} name={r.emote.name} box={48} />{:else}<span class="gone" aria-hidden="true"></span>{/if}
          <div class="what">
            <strong>
              {r.emote?.name ?? 'Deleted emote'}
              {#if r.emote && r.emote.state !== 'APPROVED'}<span class="ui-chip {r.emote.state.toLowerCase()}">{STATE_LABEL[r.emote.state]}</span>{/if}
            </strong>
            <span class="reason">“{r.reason}”</span>
            <span class="ui-hint meta">
              <span class="ui-mono">{date(r.createdAt)}</span>
              {#if r.emote?.owner}<span>by {r.emote.owner.displayName}</span>{/if}
              {#if r.reporterId}<span>reporter <span class="ui-mono">{r.reporterId}</span></span>{/if}
            </span>
          </div>
          <div class="ui-actions">
            {#if r.emote}<a class="ui-btn ghost small" href="/me/emotes/{r.emote.id}">Open</a>{/if}
            <button class="ui-btn secondary small" type="button" onclick={() => ask(r, 'DISMISS')}>Dismiss</button>
            {#if takedown && r.emote}
              <button class="ui-btn danger small" type="button" onclick={() => ask(r, 'TAKE_DOWN')}>Take down</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
{/snippet}
</LoadGate>

<ReasonDialog
  bind:this={dialog}
  title={mode === 'DISMISS' ? 'Dismiss the report' : `Take down ${current?.emote?.name ?? ''}`}
  hint={mode === 'DISMISS'
    ? 'Kept with the report for the audit log.'
    : 'It leaves every set, its image is blocked and deleted, and its owners are told this note. This can’t be undone.'}
  action={mode === 'DISMISS' ? 'Dismiss' : 'Take down'}
  danger={mode === 'TAKE_DOWN'}
  optional={mode === 'DISMISS'}
  onsubmit={resolve}
/>

<style>
  .ui-list > li { align-items: flex-start; padding-block: 12px; flex-wrap: wrap; }
  .what { display: grid; gap: 3px; flex: 1; min-width: 200px; }
  .what strong { display: flex; align-items: center; gap: 8px; }
  .reason { color: var(--muted); overflow-wrap: anywhere; }
  .meta { display: flex; flex-wrap: wrap; overflow-wrap: anywhere; }
  .meta > :global(* + *)::before { content: '·'; margin: 0 6px; }
  .gone { width: 48px; height: 48px; border: 1px dashed var(--border-strong); flex: none; }
</style>
