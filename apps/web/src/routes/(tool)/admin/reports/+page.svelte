<script lang="ts">
  // Open reports grouped by emote: a flush list (tick, picture, name, uploader, newest reason, how
  // many reports, age) and a large inspector for the picked emote (?emote=<key>): the emote at its
  // sizes on one baseline, uploader, flags, facts, every report as a row, and Dismiss (one note,
  // closes this emote's reports) or Take down (one note, closes every report on the image).
  // Arrows move over the rows, Enter opens one, Escape closes it; after a resolve the next row is
  // picked.
  // Many at once: tick rows (X, or Shift for a range) and Dismiss closes every report on them with
  // one note, in one call per 100 reports (dismissReports, one audit row each). Takedowns stay one by one.
  import { page } from '$app/state';
  import { tick, untrack } from 'svelte';
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import BulkBar from '$lib/admin/BulkBar.svelte';
  import PickBox from '$lib/admin/PickBox.svelte';
  import { BATCH_MAX, batchSummary } from '$lib/admin/bulk';
  import { age, flagWords, groupReports, newestReport, nextKey, sizesOf, type ReportGroup } from '$lib/admin/reports';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import { gridKeys, pickInUrl } from '$lib/admin/ui/pick';
  import Avatar from '$lib/components/Avatar.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import State from '$lib/components/State.svelte';
  import { all as pickAll, EMPTY, range as pickRange, toggle as pickToggle, type Selection, type Step } from '$lib/components/browse/selection';
  import { ready, src } from '$lib/emote';
  import { clean, count, date, dateTime, iso } from '$lib/format';
  import type { ReportQueueQuery } from '$lib/gql/graphql';
  import { DISMISS_REPORTS, REPORT_QUEUE, RESOLVE_REPORT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { STATE_LABEL } from '$lib/tool/state';

  type Report = ReportQueueQuery['reportQueue']['items'][number];
  type Group = ReportGroup<Report>;
  const PER = 100;
  /** Reports an inspector lists before "Show more". */
  const SHOWN = 5;
  const L = loader(async ({ fetch }) => (await gql(REPORT_QUEUE, { page: 1, perPage: PER }, fetch)).reportQueue);
  const total = $derived(L.data?.total ?? 0);
  const takedown = can('emotes.takedown');
  const now = Date.now();

  const groups = $derived<Group[]>(L.data ? groupReports(L.data.items) : []);
  const pickedKey = $derived(page.url.searchParams.get('emote'));
  const current = $derived(groups.find((g) => g.key === pickedKey) ?? null);
  const tabStop = $derived(current ? current.key : (groups[0]?.key ?? null));
  let list = $state<HTMLElement>();
  let more = $state(false);

  function open(key: string | null) {
    more = false;
    void pickInUrl('emote', key);
  }
  // Closing the inspector puts focus back on the emote's row.
  function close() {
    const key = pickedKey;
    open(null);
    void tick().then(() => list?.querySelector<HTMLElement>(`[data-item="${CSS.escape(key ?? '')}"]`)?.focus());
  }

  let mode = $state<'DISMISS' | 'TAKE_DOWN'>('DISMISS');
  let target = $state<Group | null>(null);
  let dialog: ReturnType<typeof ReasonDialog>;
  let many = $state(false);

  function ask(g: Group, action: 'DISMISS' | 'TAKE_DOWN') {
    many = false;
    target = g;
    mode = action;
    dialog.open();
  }

  async function resolve(note: string) {
    if (many) return dismissMany(note);
    const g = target;
    if (!g) return;
    const at = groups.findIndex((x) => x.key === g.key);
    const wasPicked = pickedKey === g.key;
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
      // The next row takes its place in the inspector.
      if (wasPicked) open(nextKey(groups.map((x) => x.key), at));
    } catch (err) {
      fail(err);
      await L.reload();
      throw err;
    }
  }

  const name = (g: Group | null) => clean(g?.emote?.name) || 'this emote';

  // Ticked rows (by group key) for one dismissal of all their reports.
  let sel = $state<Selection>(EMPTY);
  let selNote = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  const ticked = $derived(new Set(sel.ids));
  const order = () => groups.map((g) => g.key);
  function tickStep(step: Step) {
    sel = step.sel;
    if (step.refused) {
      clearTimeout(noteTimer);
      selNote = `That’s the most at once: ${BATCH_MAX}.`;
      noteTimer = setTimeout(() => (selNote = ''), 3000);
    }
  }
  const tickRow = (key: string, shift: boolean) => tickStep(shift ? pickRange(sel, order(), key) : pickToggle(sel, key));
  $effect(() => {
    const shown = new Set(groups.map((g) => g.key));
    untrack(() => {
      if (sel.ids.some((k) => !shown.has(k))) sel = { ids: sel.ids.filter((k) => shown.has(k)), anchor: sel.anchor && shown.has(sel.anchor) ? sel.anchor : null };
    });
  });
  const tickedReports = $derived(groups.filter((g) => ticked.has(g.key)).flatMap((g) => g.reports));
  function askMany() {
    if (!tickedReports.length) return;
    target = null;
    mode = 'DISMISS';
    many = true;
    dialog.open();
  }
  async function dismissMany(note: string) {
    const ids = tickedReports.map((r) => r.id);
    const out = { done: [] as string[], failed: [] as { id: string; code: string; message: string }[] };
    try {
      for (let i = 0; i < ids.length; i += BATCH_MAX) {
        const r = (await gql(DISMISS_REPORTS, { ids: ids.slice(i, i + BATCH_MAX), note: note || null })).dismissReports;
        out.done.push(...r.done);
        out.failed.push(...r.failed);
      }
      notify(batchSummary(out, 'dismissed', ['report', 'reports']), out.failed.length > 0);
      sel = EMPTY;
    } catch (err) {
      fail(err);
      throw err;
    } finally {
      await L.reload();
    }
  }

  function onkeydown(ev: KeyboardEvent) {
    if (document.querySelector('dialog[open]') || ev.defaultPrevented) return;
    const t = ev.target as HTMLElement | null;
    if (t?.closest('input, textarea, select')) return;
    if (ev.key === 'Escape') {
      if (pickedKey) {
        ev.preventDefault();
        close();
      } else if (sel.ids.length) {
        ev.preventDefault();
        sel = EMPTY;
      }
    } else if ((ev.key === 'x' || ev.key === 'X') && !ev.ctrlKey && !ev.metaKey && !ev.altKey && t?.matches('[data-item]')) {
      // X ticks the row the focus is on; with Shift the range from the last tick takes the state
      // this row ends up with (untick a ticked row and the whole range is unticked).
      ev.preventDefault();
      tickRow(t.dataset.item!, ev.shiftKey);
    }
  }
</script>

<svelte:head><title>{current ? `${name(current)} - Reports` : 'Reports'} - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Split label="Report" size="l" open={!!current} onclose={() => open(null)}>
  <Bar title="Reports" count={L.data ? `${count(groups.length)} ${groups.length === 1 ? 'emote' : 'emotes'} · ${count(total)} ${total === 1 ? 'report' : 'reports'}` : null} />

  <LoadGate source={L} label="Loading reports…">
    {#snippet children(queue)}
      {#if queue.items.length === 0}
        <State title="No open reports.">Reports from viewers show up here.</State>
      {:else}
        <ul class="adm-rows sheet" bind:this={list} use:gridKeys aria-label="Reported emotes">
          {#each groups as g (g.key)}
            {@const owner = g.emote?.owner}
            {@const last = newestReport(g.reports)}
            <li class="row" class:ticked={ticked.has(g.key)}>
              <PickBox checked={ticked.has(g.key)} label="Select the reports on {g.emote ? clean(g.emote.name) : 'a deleted emote'}" onpick={(ev) => tickRow(g.key, ev.shiftKey)} />
              <button class="item" type="button" data-item={g.key} aria-current={g.key === pickedKey ? 'true' : undefined} tabindex={g.key === tabStop ? 0 : -1} onclick={() => open(g.key)}>
                <span class="frame">
                  {#if g.emote}<Thumb emote={g.emote} name="" box={40} />{:else}<span class="gone" aria-hidden="true"></span>{/if}
                </span>
                <span class="who">
                  <span class="nm"><bdi>{g.emote ? clean(g.emote.name) : 'Deleted emote'}</bdi></span>
                  <span class="own">{#if owner}<bdi>{clean(owner.displayName)}</bdi>{/if}</span>
                </span>
                <span class="why">{#if last}<bdi>{clean(last.reason)}</bdi>{/if}</span>
                <span class="cnt adm-num" title="Open reports">{count(g.reports.length)}</span>
                <span class="ago adm-num">{last ? age(last.createdAt, now) : ''}</span>
              </button>
            </li>
          {/each}
        </ul>
        {#if queue.total > queue.items.length}<p class="ui-hint adm-num tail">Showing the oldest {count(queue.items.length)} reports. The rest load as you work through these.</p>{/if}
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Report</span></div>
    <p class="none">Pick an emote to read its reports and dismiss them or take it down.</p>
  {/snippet}

  {#snippet panel()}
    {#if current}
      {@const g = current}
      {@const e = g.emote}
      {@const shown = more ? g.reports : g.reports.slice(0, SHOWN)}
      {@const files = e ? sizesOf(e) : []}
      <div class="adm-ip-head">
        <span class="adm-label">Report</span>
        <IconButton label="Close" icon="x" size="s" tip="bottom-end" onclick={close} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          {#if e && ready(e) && files.length}
            <div class="sizes" role="group" aria-label="Sizes">
              {#each files as f, i (f.name)}
                {@const h = Math.min(f.height, 24 * f.scale)}
                <span class="size">
                  <span class="pic"><img src={src(e, f.scale as 1 | 2 | 3 | 4)} alt={i === 0 ? clean(e.name) : ''} style:height="{h}px" style:aspect-ratio="{f.width} / {f.height}" /></span>
                  <span class="x adm-num">{f.scale}x</span>
                  <span class="dm adm-num">{f.width} × {f.height}</span>
                </span>
              {/each}
            </div>
          {:else}
            <div class="sizes one"><span class="size">{#if e}<Thumb emote={e} name={clean(e.name)} box={96} />{:else}<span class="gone big" aria-hidden="true"></span>{/if}</span></div>
          {/if}
          <h2 class="adm-name"><bdi>{e ? clean(e.name) : 'Deleted emote'}</bdi></h2>
          {#if e}
            {@const marks = flagWords(e)}
            <p class="by">
              {#if e.owner}<Avatar src={e.owner.avatar} name={e.owner.displayName} size={20} /><AccountLink id={e.owner.id} name={e.owner.displayName} />{:else}No owner{/if}
            </p>
            {#if marks.length || e.tags.length}
              <ul class="adm-flags">
                {#each marks as m (m)}<li>{m}</li>{/each}
                {#each e.tags as t (t)}<li class="tag"><bdi>{clean(t)}</bdi></li>{/each}
              </ul>
            {/if}
            <dl class="adm-facts">
              <div><dt>State</dt><dd>{STATE_LABEL[e.state]}</dd></div>
              <div><dt>Open reports</dt><dd class="n">{count(g.reports.length)}</dd></div>
              <div><dt>Channels</dt><dd class="n">{count(e.channelCount)}</dd></div>
              <div><dt>Added</dt><dd class="n">{date(e.createdAt)}</dd></div>
              <div><dt>Emote</dt><dd><a href="/me/emotes/{encodeURIComponent(e.id)}">Open</a></dd></div>
            </dl>
          {/if}
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">Reports <span class="n">{count(g.reports.length)}</span></h3>
          <ul class="reports">
            {#each shown as r (r.id)}
              <li>
                <span class="reason"><bdi>{clean(r.reason)}</bdi></span>
                <span class="who2">{#if r.reporterId}<AccountLink id={r.reporterId} />{:else}Anonymous{/if}</span>
                <span class="at adm-num">{dateTime(r.createdAt)}</span>
              </li>
            {/each}
          </ul>
          {#if g.reports.length > SHOWN}
            <button class="more" type="button" aria-expanded={more} onclick={() => (more = !more)}>
              {more ? 'Show fewer' : `Show more (${count(g.reports.length - SHOWN)})`}
            </button>
          {/if}
        </section>
        <section class="adm-sec">
          <div class="adm-acts row">
            <button class="ui-btn secondary small" type="button" onclick={() => ask(g, 'DISMISS')}>{g.reports.length > 1 ? 'Dismiss all' : 'Dismiss'}</button>
            {#if takedown && e}
              <button class="ui-btn danger small" type="button" onclick={() => ask(g, 'TAKE_DOWN')}>Take down</button>
            {/if}
          </div>
        </section>
      </div>
    {/if}
  {/snippet}
</Split>

{#if sel.ids.length}
  <BulkBar n={sel.ids.length} status={selNote} onclear={() => (sel = EMPTY)} onall={sel.ids.length < Math.min(groups.length, BATCH_MAX) ? () => tickStep(pickAll(sel, order())) : undefined}>
    <button class="ui-btn secondary small" type="button" onclick={askMany}>Dismiss {count(tickedReports.length)} {tickedReports.length === 1 ? 'report' : 'reports'}</button>
  </BulkBar>
{/if}

<ReasonDialog
  bind:this={dialog}
  title={many ? `Dismiss ${count(tickedReports.length)} ${tickedReports.length === 1 ? 'report' : 'reports'} on ${count(sel.ids.length)} ${sel.ids.length === 1 ? 'emote' : 'emotes'}` : mode === 'DISMISS' ? `Dismiss the reports on ${name(target)}` : `Take down ${name(target)}`}
  hint={mode === 'DISMISS'
    ? 'Kept with the reports for the audit log.'
    : 'It leaves every set, its image is blocked and deleted, and its owners are told this note. This can’t be undone.'}
  action={mode === 'DISMISS' ? 'Dismiss' : 'Take down'}
  danger={mode === 'TAKE_DOWN'}
  optional={mode === 'DISMISS'}
  onsubmit={resolve}
/>

<style>
  .sheet { border-top: 0; }
  .row { display: grid; grid-template-columns: 40px minmax(0, 1fr); align-items: center; height: 60px; padding-left: 4px; }
  .row.ticked { background: var(--row-hover); }
  .item { display: grid; grid-template-columns: 40px minmax(120px, 220px) minmax(0, 1fr) 36px 44px; align-items: center; gap: 12px; width: 100%; height: 100%;
    padding: 0 12px 0 4px; border: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-small); text-align: left; cursor: pointer; }
  .item:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .item:active { opacity: .6; }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--row-hover); } }
  .frame { display: grid; place-items: center; width: 40px; height: 40px; }
  .gone { display: block; width: 24px; height: 24px; border: 1px dashed var(--border-strong); border-radius: var(--radius); }
  .gone.big { width: 64px; height: 64px; }
  .who { display: grid; min-width: 0; line-height: 1.3; }
  .nm, .own, .why { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm { font-weight: 600; }
  .item[aria-current='true'] .nm { font-weight: 700; }
  .item[aria-current='true'] { box-shadow: inset 0 0 0 1px var(--accent-border); }
  .own { font-size: var(--t-fine); color: var(--dim); }
  .cnt { text-align: right; font-weight: 600; color: var(--text); }
  .why { color: var(--muted); }
  .ago { text-align: right; color: var(--dim); }
  .tail { margin-top: 16px; text-align: center; }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }

  /* The emote at its sizes on one baseline, cells sharing hairlines (as the emote page). */
  .sizes { display: grid; grid-auto-flow: column; grid-auto-columns: minmax(0, 1fr); border: 1px solid var(--border); overflow: hidden; }
  .sizes.one { grid-auto-flow: row; }
  .size { display: grid; justify-items: center; align-content: end; gap: 4px; min-height: 160px; padding: 12px 6px; background: var(--bg); box-shadow: 1px 0 0 var(--border); }
  .size .pic { display: grid; place-items: end center; }
  .size img { display: block; max-width: 100%; width: auto; }
  .x { margin-top: 6px; font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .dm { font-size: var(--t-fine); color: var(--muted); }
  .by { display: flex; align-items: center; gap: 8px; margin: 0; font-size: var(--t-small); color: var(--muted); }
  .tag { color: var(--muted); }

  /* The reports: flush rows, reason, who, when in mono. */
  .reports { list-style: none; margin: 0; padding: 0; display: grid; border-top: 1px solid var(--border); }
  .reports li { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 2px 12px; padding: 9px 0; border-bottom: 1px solid var(--border); overflow-wrap: anywhere; font-size: var(--t-small); }
  .reason { grid-column: 1 / -1; font-weight: 600; }
  .who2 { min-width: 0; color: var(--muted); }
  .at { color: var(--dim); white-space: nowrap; }
  .more { justify-self: start; min-height: 32px; padding: 0; border: 0; background: none; font: inherit; font-size: var(--t-small); color: var(--accent-text); cursor: pointer; }
  .more:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  @media (max-width: 719.98px) {
    .row { height: 64px; }
    .item { grid-template-columns: 40px minmax(0, 1fr) 30px 40px; gap: 10px; }
    .why { display: none; }
    .size { min-height: 120px; }
  }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  .adm-sec a { display: inline-flex; align-items: center; min-height: 24px; }
</style>
