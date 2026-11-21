<script lang="ts">
  // The audit log: a flush table (when, actor, action, target, details) under one toolbar of
  // filters (actor, target kind and ID, action, date range) kept in the address. Picking a row
  // opens an inspector with the entry's facts and what changed (or the raw JSON before and after,
  // capped, with Show all). Arrows move over the rows, Escape closes the inspector. Pages of 50
  // with Newer and Older and the range shown; on a phone the rows stack.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import ActorField from '$lib/admin/ActorField.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { lookUp } from '$lib/admin/names.svelte';
  import { actionText, changes, dayAfter, dayStart, pageRange, rangeProblem, summarize, targetHref } from '$lib/admin/audit';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { count, dateTimeSeconds } from '$lib/format';
  import { AUDIT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';

  type Entry = AuditQuery['auditLog'][number];
  const LIMIT = 50;
  /** Changes and JSON lines the inspector shows before "Show all". */
  const CAP = 12;
  const JSON_LINES = 40;
  /// Actions that exist today, for the action field's suggestions (any exact name still works).
  const ACTIONS = [
    'emote.approved', 'emote.rejected', 'emote.taken_down', 'report.dismissed', 'role.created', 'role.updated', 'role.deleted',
    'role.assigned', 'role.unassigned', 'account.restricted', 'cosmetic.granted', 'cosmetic.revoked', 'badge.created',
    'paint.created', 'set.created', 'set.deleted',
  ];

  const q = (k: string) => page.url.searchParams.get(k) ?? '';
  let kind = $state(q('kind'));
  let target = $state(q('id'));
  let actor = $state(q('actor'));
  let action = $state(q('action'));
  let from = $state(q('from'));
  let to = $state(q('to'));

  let open = $state<string | null>(null);
  let raw = $state(false);
  let full = $state(false);
  let body = $state<HTMLElement>();

  // Paging: the cursor ("before") of each page seen, so Newer goes back without asking again.
  // A new filter starts over on the first page.
  let pg = $state<{ search: string; ix: number; cursors: (string | null)[] }>({ search: '', ix: 0, cursors: [null] });
  const same = $derived(pg.search === page.url.search);
  const ix = $derived(same ? pg.ix : 0);
  const cursors = $derived(same ? pg.cursors : [null]);

  function vars(before: string | null) {
    return {
      targetKind: q('kind') || null,
      targetId: q('id') || null,
      actorId: q('actor') || null,
      action: q('action') || null,
      since: dayStart(q('from')),
      until: dayAfter(q('to')),
      before,
      // One more than a page, to know whether an older page exists.
      limit: LIMIT + 1,
    };
  }

  const L = loader(async ({ fetch }) => {
    const rows = (await gql(AUDIT, vars(cursors[ix] ?? null), fetch)).auditLog;
    lookUp(rows.map((r) => r.actorId));
    open = null;
    return rows;
  });
  const entries = $derived(L.data ? L.data.slice(0, LIMIT) : null);
  const hasOlder = $derived((L.data?.length ?? 0) > LIMIT);
  const picked = $derived<Entry | null>(entries?.find((e) => e.id === open) ?? null);
  const pickedList = $derived(picked ? changes(picked.before, picked.after) : []);

  function turn(by: 1 | -1) {
    if (by === 1) {
      const last = entries?.at(-1)?.id;
      if (!last) return;
      pg = { search: page.url.search, ix: ix + 1, cursors: [...cursors.slice(0, ix + 1), last] };
    } else if (ix > 0) {
      pg = { search: page.url.search, ix: ix - 1, cursors };
    }
    window.scrollTo({ top: 0 });
  }

  function pick(id: string | null) {
    open = id;
    full = false;
  }
  // Closing the inspector puts focus back on the entry's row.
  function close() {
    const id = open;
    pick(null);
    void tick().then(() => body?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id ?? '')}"]`)?.focus());
  }
  function onkeydown(e: KeyboardEvent) {
    if (document.querySelector('dialog[open]') || e.defaultPrevented) return;
    const t = e.target as HTMLElement | null;
    if (t?.closest('input, textarea, select')) return;
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      close();
    } else if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && t?.matches('[data-item]')) {
      const rows = [...(body?.querySelectorAll<HTMLElement>('[data-item]') ?? [])];
      const to = rows[rows.indexOf(t) + (e.key === 'ArrowDown' ? 1 : -1)];
      if (to) {
        e.preventDefault();
        for (const r of rows) r.tabIndex = -1;
        to.tabIndex = 0;
        to.focus();
      }
    }
  }

  const problem = $derived(rangeProblem(from, to));

  function apply(next: Partial<{ kind: string; actor: string; from: string; to: string }> = {}) {
    kind = next.kind ?? kind;
    actor = next.actor ?? actor;
    from = next.from ?? from;
    to = next.to ?? to;
    if (rangeProblem(from, to)) return;
    const p = new URLSearchParams();
    for (const [k, v] of Object.entries({ kind, id: target, actor, action, from, to })) if (v.trim()) p.set(k, v.trim());
    void goto(p.size ? `?${p}` : '/admin/audit', { keepFocus: true });
  }
  function filter(e: SubmitEvent) {
    e.preventDefault();
    apply();
  }

  /** A JSON side, pretty-printed and cut to JSON_LINES unless `full`. */
  function pretty(json: string | null | undefined): { text: string; cut: boolean } {
    if (!json) return { text: '', cut: false };
    let text = json;
    try {
      text = JSON.stringify(JSON.parse(json), null, 2);
    } catch {
      /* not JSON: shown as it is */
    }
    const lines = text.split('\n');
    return full || lines.length <= JSON_LINES ? { text, cut: false } : { text: lines.slice(0, JSON_LINES).join('\n') + '\n…', cut: true };
  }
</script>

<svelte:head><title>Audit log - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} />

<Split label="Audit entry" open={!!picked} onclose={() => pick(null)}>
  <Bar title="Audit log" count={entries ? `${pageRange(ix, LIMIT, entries.length)} shown` : null}>
    {#snippet tools()}
      <form class="filters" onsubmit={filter}>
        <div class="f actor"><ActorField value={actor} onchange={(id) => apply({ actor: id })} /></div>
        <select class="ui-input f kind" value={kind} onchange={(e) => apply({ kind: e.currentTarget.value })} aria-label="Target kind">
          <option value="">Any kind</option>
          <option value="account">Account</option>
          <option value="emote">Emote</option>
          <option value="set">Set</option>
          <option value="role">Role</option>
        </select>
        <input class="ui-input f tid" bind:value={target} placeholder="Target ID" aria-label="Target ID" spellcheck="false" autocomplete="off" />
        <input class="ui-input f act" bind:value={action} placeholder="Action, e.g. role.updated" aria-label="Action" list="audit-actions" spellcheck="false" autocomplete="off" />
        <datalist id="audit-actions">{#each ACTIONS as a}<option value={a}></option>{/each}</datalist>
        <div class="f date"><DateField label="From" value={from} onchange={(d) => apply({ from: d })} /></div>
        <div class="f date"><DateField label="To" value={to} onchange={(d) => apply({ to: d })} /></div>
        <button class="ui-btn secondary small" type="submit">Filter</button>
        {#if page.url.search}<a class="ui-btn ghost small" href="/admin/audit">Clear</a>{/if}
      </form>
    {/snippet}
  </Bar>
  {#if problem}<p class="ui-error range" role="alert">{problem}</p>{/if}

  <LoadGate source={L} label="Loading the log…">
    {#snippet children()}
      {#if !entries || entries.length === 0}
        <State title="No entries{page.url.search ? ' for this filter' : ''}." />
      {:else}
        <div bind:this={body}>
          <table class="adm-table stack log" aria-label="Audit entries">
            <thead>
              <tr><th class="when">When</th><th>Actor</th><th>Action</th><th>Target</th><th>Details</th></tr>
            </thead>
            <tbody>
              {#each entries as e, i (e.id)}
                {@const text = summarize(changes(e.before, e.after))}
                {@const on = open === e.id}
                <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
                <tr class="entry" class:on data-row onclick={(ev) => { if (!(ev.target as HTMLElement).closest('a, button')) pick(on ? null : e.id); }}>
                  <td class="when pk" data-l="When">
                    <button class="lead adm-num" type="button" data-item={e.id} aria-pressed={on} tabindex={(picked ? on : i === 0) ? 0 : -1} onclick={() => pick(on ? null : e.id)}>{dateTimeSeconds(e.createdAt)}</button>
                  </td>
                  <td class="by" data-l="Actor"><AccountLink id={e.actorId} /></td>
                  <td class="act" data-l="Action">{actionText(e.action)}</td>
                  <td class="tgt" data-l="Target"><span class="tk">{e.targetKind}</span> <span class="mono id">{e.targetId}</span></td>
                  <td class="det ell dim" data-l="Details" title={text}>{text}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="adm-pager">
          <button class="ui-btn secondary small" type="button" disabled={ix === 0 || L.loading} onclick={() => turn(-1)}>Newer</button>
          <span>{pageRange(ix, LIMIT, entries.length)}</span>
          <button class="ui-btn secondary small" type="button" disabled={!hasOlder || L.loading} onclick={() => turn(1)}>Older</button>
        </div>
      {/if}
    {/snippet}
  </LoadGate>

  {#snippet empty()}
    <div class="adm-ip-head"><span class="adm-label">Entry</span></div>
    <p class="none">Pick an entry to see who did what, and what changed.</p>
  {/snippet}

  {#snippet panel()}
    {#if picked}
      {@const e = picked}
      {@const href = targetHref(e.targetKind, e.targetId)}
      {@const list = pickedList}
      <div class="adm-ip-head">
        <span class="adm-label">Entry</span>
        <IconButton label="Close" icon="x" size="s" onclick={close} />
      </div>
      <div class="adm-ip-body">
        <section class="adm-sec">
          <h2 class="adm-name">{actionText(e.action)}</h2>
          <dl class="adm-kv">
            <dt>When</dt><dd>{dateTimeSeconds(e.createdAt)}</dd>
            <dt>Actor</dt><dd class="txt"><AccountLink id={e.actorId} /></dd>
            <dt>Action</dt><dd class="mono">{e.action}</dd>
            <dt>Target kind</dt><dd class="txt">{e.targetKind}</dd>
            <dt>Target</dt><dd class="mono">{#if href}<a {href}>{e.targetId}</a>{:else}{e.targetId}{/if}</dd>
            <dt>Entry</dt><dd class="mono">{e.id}</dd>
          </dl>
        </section>
        <section class="adm-sec">
          <h3 class="adm-label">What changed <span class="n adm-num">{count(list.length)}</span></h3>
          <Segmented label="Show" size="s" options={[{ value: 'changes', label: 'Changes' }, { value: 'json', label: 'JSON' }]} value={raw ? 'json' : 'changes'} onchange={(v) => (raw = v === 'json')} />
          {#if raw}
            {@const b = pretty(e.before)}
            {@const a = pretty(e.after)}
            <div class="json">
              <div><span class="adm-label">Before</span><pre>{b.text || '—'}</pre></div>
              <div><span class="adm-label">After</span><pre>{a.text || '—'}</pre></div>
            </div>
            {#if b.cut || a.cut}<button class="link" type="button" onclick={() => (full = true)}>Show all</button>{/if}
          {:else if list.length}
            <ul class="changes">
              {#each full ? list : list.slice(0, CAP) as c (c.path)}
                <li>
                  <span class="mono path">{c.path}</span>
                  <span class="vals mono">
                    {#if c.kind !== 'added'}<span class="was">{c.before}</span>{/if}
                    {#if c.kind === 'changed'}<span class="arrow" aria-label="became">→</span>{/if}
                    {#if c.kind !== 'removed'}<span class="now">{c.after}</span>{/if}
                    {#if c.kind === 'removed'}<span class="ui-hint">removed</span>{/if}
                  </span>
                </li>
              {/each}
            </ul>
            {#if !full && list.length > CAP}<button class="link" type="button" onclick={() => (full = true)}>Show all {count(list.length)}</button>{/if}
          {:else}
            <p class="ui-hint">No field changes recorded for this entry.</p>
          {/if}
        </section>
      </div>
    {/if}
  {/snippet}
</Split>

<style>
  .range { margin: -4px 0 12px; }
  .filters { display: contents; }
  .filters :global(.ui-label) { display: none; }
  .f { min-width: 0; }
  .actor { width: 150px; }
  .kind { width: 118px; }
  .tid { width: 140px; }
  .act { flex: 1 1 160px; width: auto; max-width: 240px; }
  .date { width: 128px; }
  .f :global(.date), .f :global(.who) { width: 100%; }
  .filters :is(a.ui-btn, button.ui-btn) { height: 34px; }
  .filters select.ui-input { padding-block: 0; }
  .log td { vertical-align: middle; }

  .log td { overflow-wrap: anywhere; }
  .when { width: 1%; white-space: nowrap; }
  .lead { display: block; min-height: 0; height: auto; margin: 0; line-height: 1.4; padding: 0; border: 0; background: none; color: var(--muted); font: inherit; text-align: left; cursor: pointer; }
  .lead:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  .entry.on .lead { color: var(--text); font-weight: 700; }
  .by { max-width: 180px; }
  .act { font-weight: 600; white-space: nowrap; }
  .tgt { max-width: 280px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tk { color: var(--dim); font-size: var(--t-fine); text-transform: uppercase; letter-spacing: .04em; }
  .id { color: var(--muted); }
  .det { width: 28%; }
  .entry.on td { background: var(--row-hover); }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }

  .json { display: grid; gap: 12px; width: 100%; }
  pre { margin: 6px 0 0; padding: 8px 0; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); font: .78rem/1.65 var(--font-mono);
    white-space: pre-wrap; overflow-wrap: anywhere; }
  .changes { list-style: none; margin: 0; padding: 0; display: grid; border-top: 1px solid var(--border); }
  .changes li { display: grid; grid-template-columns: minmax(80px, 160px) minmax(0, 1fr); gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .path { font-size: var(--t-fine); color: var(--muted); overflow-wrap: anywhere; }
  .vals { display: flex; flex-wrap: wrap; gap: 4px 8px; font-size: var(--t-fine); overflow-wrap: anywhere; }
  .was { color: var(--red); text-decoration: line-through; text-decoration-thickness: 1px; }
  .now { color: var(--green); }
  .arrow { color: var(--dim); }
  .link { justify-self: start; min-height: 32px; padding: 0; border: 0; background: none; font: inherit; font-size: var(--t-small); color: var(--accent-text); cursor: pointer; }
  .link:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  @media (max-width: 760px) {
    .filters :is(.actor, .tid, .act) { flex: 1 1 100%; width: 100%; max-width: none; }
    .filters :is(.kind, .date) { flex: 1 1 calc(33% - 8px); width: auto; }
    .log td.det { width: auto; max-width: none; }
    .changes li { grid-template-columns: minmax(0, 1fr); gap: 2px; }
  }
</style>
