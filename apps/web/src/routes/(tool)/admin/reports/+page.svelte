<script lang="ts">
  import { gql } from '$lib/api';
  import { clean, count, date, iso } from '$lib/format';
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
      notify(mode === 'DISMISS' ? 'Report dismissed.' : `${r.emote ? iso(r.emote.name) : 'The emote'} taken down everywhere.`);
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
    <div class="row head" aria-hidden="true">
      <span></span><span class="ui-label">Emote</span><span class="ui-label">Report</span><span class="ui-label actions-h">Action</span>
    </div>
    <ul class="ui-list">
      {#each items as r (r.id)}
        <li class="row">
          <span class="frame">
            {#if r.emote}<Thumb emote={r.emote} name={clean(r.emote.name)} box={48} />{:else}<span class="gone" aria-hidden="true"></span>{/if}
          </span>
          <div class="what">
            <strong>
              {#if clean(r.emote?.name)}<bdi>{clean(r.emote?.name)}</bdi>{:else}Deleted emote{/if}
              {#if r.emote && r.emote.state !== 'APPROVED'}<span class="ui-chip {r.emote.state.toLowerCase()}">{STATE_LABEL[r.emote.state]}</span>{/if}
            </strong>
            {#if r.emote?.owner}<span class="ui-hint"><bdi>{clean(r.emote.owner.displayName)}</bdi></span>{/if}
          </div>
          <div class="why">
            <span class="reason">“<bdi>{clean(r.reason)}</bdi>”</span>
            <span class="ui-hint meta">
              <span class="ui-mono">{date(r.createdAt)}</span>
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
  title={mode === 'DISMISS' ? 'Dismiss the report' : `Take down ${iso(current?.emote?.name)}`}
  hint={mode === 'DISMISS'
    ? 'Kept with the report for the audit log.'
    : 'It leaves every set, its image is blocked and deleted, and its owners are told this note. This can’t be undone.'}
  action={mode === 'DISMISS' ? 'Dismiss' : 'Take down'}
  danger={mode === 'TAKE_DOWN'}
  optional={mode === 'DISMISS'}
  onsubmit={resolve}
/>

<style>
  /* One grid for the header and every row, so the columns line up. */
  .row { display: grid; grid-template-columns: 56px minmax(0, 1fr) minmax(0, 1.6fr) 210px; gap: 14px; align-items: center; }
  .ui-list > li.row { padding-block: 10px; }
  .head { padding: 8px 14px; border-bottom: 1px solid var(--border); background: var(--row-alt); }
  .actions-h { text-align: right; }
  .frame { display: grid; place-items: center; width: 56px; height: 56px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--bg); }
  .what, .why { display: grid; gap: 3px; min-width: 0; }
  .what strong { display: flex; align-items: center; gap: 8px; min-width: 0; overflow-wrap: anywhere; }
  .reason { color: var(--text); overflow-wrap: anywhere; }
  .meta { display: grid; overflow-wrap: anywhere; }
  .gone { width: 40px; height: 40px; border: 1px dashed var(--border-strong); border-radius: var(--radius); }
  .row .ui-actions { justify-content: flex-end; }
  @media (max-width: 760px) {
    .head { display: none; }
    .row { grid-template-columns: 56px minmax(0, 1fr); gap: 8px 12px; }
    .why, .row .ui-actions { grid-column: 1 / -1; }
    .row .ui-actions { justify-content: flex-start; }
  }
</style>
