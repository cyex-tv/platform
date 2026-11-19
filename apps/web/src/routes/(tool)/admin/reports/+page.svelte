<script lang="ts">
  // Open reports grouped by emote: one row per reported emote with how many reports it has, the
  // reasons, and who reported. Take down (one note, closes every report on the image) or Dismiss
  // (one note, closes this emote's reports). Names link to the account's admin page.
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import { clean, count, dateTime, iso } from '$lib/format';
  import type { ReportQueueQuery } from '$lib/gql/graphql';
  import { REPORT_QUEUE, RESOLVE_REPORT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import State from '$lib/components/State.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { STATE_LABEL } from '$lib/tool/state';
  import { can } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Report = ReportQueueQuery['reportQueue']['items'][number];
  type Group = { key: string; emote: Report['emote']; reports: Report[] };
  const PER = 100;
  const L = loader(async ({ fetch }) => (await gql(REPORT_QUEUE, { page: 1, perPage: PER }, fetch)).reportQueue);
  const total = $derived(L.data?.total ?? 0);
  const takedown = can('emotes.takedown');

  /** One group per emote (a deleted emote's reports each stand alone), the one with most reports first. */
  function group(items: Report[]): Group[] {
    const by = new Map<string, Group>();
    for (const r of items) {
      const key = r.emote?.id ?? `gone-${r.id}`;
      const g = by.get(key) ?? { key, emote: r.emote, reports: [] };
      g.reports.push(r);
      by.set(key, g);
    }
    return [...by.values()].sort((a, b) => b.reports.length - a.reports.length);
  }

  let current = $state<Group | null>(null);
  let mode = $state<'DISMISS' | 'TAKE_DOWN'>('DISMISS');
  let dialog: ReturnType<typeof ReasonDialog>;
  let open = $state<string | null>(null);

  function ask(g: Group, action: 'DISMISS' | 'TAKE_DOWN') {
    current = g;
    mode = action;
    dialog.open();
  }

  async function resolve(note: string) {
    const g = current;
    if (!g) return;
    try {
      if (mode === 'TAKE_DOWN') {
        // A takedown closes every open report on the same image, so one call does it.
        await gql(RESOLVE_REPORT, { id: g.reports[0].id, action: mode, note: note || null });
        notify(`${g.emote ? iso(g.emote.name) : 'The emote'} taken down everywhere.`);
      } else {
        for (const r of g.reports) await gql(RESOLVE_REPORT, { id: r.id, action: mode, note: note || null });
        notify(g.reports.length === 1 ? 'Report dismissed.' : `${count(g.reports.length)} reports dismissed.`);
      }
      await L.reload();
    } catch (err) {
      fail(err);
      await L.reload();
      throw err;
    }
  }

  const name = (g: Group | null) => clean(g?.emote?.name) || 'this emote';
</script>

<svelte:head><title>Reports - Admin - cyex.tv</title></svelte:head>

<PageHead title="Reports" line={`${count(total)} open, grouped by emote`} />

<LoadGate source={L} label="Loading reports…">
{#snippet children(queue)}
{@const groups = group(queue.items)}
{#if groups.length === 0}
  <State title="No open reports.">Reports from viewers show up here.</State>
{:else}
  <section class="ui-panel">
    <ul class="ui-list">
      {#each groups as g (g.key)}
        {@const owner = g.emote?.owner}
        <li class="row">
          <span class="frame">
            {#if g.emote}<Thumb emote={g.emote} name={clean(g.emote.name)} box={48} />{:else}<span class="gone" aria-hidden="true"></span>{/if}
          </span>
          <div class="what">
            <strong>
              {#if g.emote}<a href="/me/emotes/{encodeURIComponent(g.emote.id)}"><bdi title={clean(g.emote.name)}>{clean(g.emote.name)}</bdi></a>{:else}Deleted emote{/if}
              <span class="ui-chip count" title="Open reports">{count(g.reports.length)} {g.reports.length === 1 ? 'report' : 'reports'}</span>
              {#if g.emote && g.emote.state !== 'APPROVED'}<span class="ui-chip {g.emote.state.toLowerCase()}">{STATE_LABEL[g.emote.state]}</span>{/if}
            </strong>
            {#if owner}<span class="ui-hint">by <AccountLink id={owner.id} name={owner.displayName} /></span>{/if}
            <ul class="reasons">
              {#each open === g.key ? g.reports : g.reports.slice(0, 2) as r (r.id)}
                <li>
                  <span class="reason">“<bdi>{clean(r.reason)}</bdi>”</span>
                  <span class="ui-hint ui-num">{dateTime(r.createdAt)}{#if r.reporterId}{' · '}<AccountLink id={r.reporterId} />{/if}</span>
                </li>
              {/each}
            </ul>
            {#if g.reports.length > 2}
              <button class="more" type="button" aria-expanded={open === g.key} onclick={() => (open = open === g.key ? null : g.key)}>
                {open === g.key ? 'Show fewer' : `Show all ${count(g.reports.length)}`}
              </button>
            {/if}
          </div>
          <div class="ui-actions">
            <button class="ui-btn secondary small" type="button" onclick={() => ask(g, 'DISMISS')}>{g.reports.length > 1 ? 'Dismiss all' : 'Dismiss'}</button>
            {#if takedown && g.emote}
              <button class="ui-btn danger small" type="button" onclick={() => ask(g, 'TAKE_DOWN')}>Take down</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
  {#if queue.total > queue.items.length}<p class="ui-hint tail">Showing the oldest {count(queue.items.length)} reports. The rest load as you work through these.</p>{/if}
{/if}
{/snippet}
</LoadGate>

<ReasonDialog
  bind:this={dialog}
  title={mode === 'DISMISS' ? `Dismiss the reports on ${name(current)}` : `Take down ${name(current)}`}
  hint={mode === 'DISMISS'
    ? 'Kept with the reports for the audit log.'
    : 'It leaves every set, its image is blocked and deleted, and its owners are told this note. This can’t be undone.'}
  action={mode === 'DISMISS' ? 'Dismiss' : 'Take down'}
  danger={mode === 'TAKE_DOWN'}
  optional={mode === 'DISMISS'}
  onsubmit={resolve}
/>

<style>
  .row { display: grid; grid-template-columns: 56px minmax(0, 1fr) auto; gap: 14px; align-items: start; padding-block: 12px; }
  .frame { display: grid; place-items: center; width: 56px; height: 56px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--bg); }
  .what { display: grid; gap: 3px; min-width: 0; }
  .what strong { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .what strong a { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }
  .what strong .ui-chip { flex: none; }
  .count { background: color-mix(in srgb, var(--yellow) 18%, transparent); color: var(--yellow); }
  .reasons { list-style: none; display: grid; gap: 4px; margin: 4px 0 0; padding: 0; }
  .reasons li { display: grid; gap: 0; padding: 0; border: 0; overflow-wrap: anywhere; }
  .reason { color: var(--text); }
  .more { justify-self: start; min-height: 32px; padding: 0; border: 0; background: none; font: inherit; font-size: var(--t-small); color: var(--accent-text); cursor: pointer; }
  .gone { width: 40px; height: 40px; border: 1px dashed var(--border-strong); border-radius: var(--radius); }
  .tail { margin-top: 16px; text-align: center; }
  .row .ui-actions { justify-content: flex-end; }
  @media (max-width: 760px) {
    .row { grid-template-columns: 56px minmax(0, 1fr); gap: 8px 12px; }
    .row .ui-actions { grid-column: 1 / -1; justify-content: flex-start; }
  }
</style>
