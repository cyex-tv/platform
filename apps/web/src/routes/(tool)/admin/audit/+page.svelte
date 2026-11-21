<script lang="ts">
  // The audit log: a dense table (when, actor, action, target, details) with filters (actor, target
  // kind and ID, action, date range) kept in the address. A row's details open in place: what
  // changed, or the raw JSON (capped, with Show all). Pages of 50 with Newer and Older and the
  // range shown; on a phone each row is a small card.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import ActorField from '$lib/admin/ActorField.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { lookUp } from '$lib/admin/names.svelte';
  import { actionText, changes, dayAfter, dayStart, pageRange, rangeProblem, summarize, targetHref } from '$lib/admin/audit';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { count, dateTimeSeconds } from '$lib/format';
  import { AUDIT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';

  type Entry = AuditQuery['auditLog'][number];
  const LIMIT = 50;
  /** Changes and JSON lines a row shows before "Show all". */
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

  function toggle(id: string) {
    open = open === id ? null : id;
    full = false;
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

<PageHead title="Audit log" line="Every audited change, newest first." />

<form class="adm-toolbar filters" onsubmit={filter}>
  <div class="f actor"><ActorField value={actor} onchange={(id) => apply({ actor: id })} /></div>
  <label class="f kind">
    <span class="ui-label">Kind</span>
    <select class="ui-input" value={kind} onchange={(e) => apply({ kind: e.currentTarget.value })} aria-label="Target kind">
      <option value="">Any kind</option>
      <option value="account">Account</option>
      <option value="emote">Emote</option>
      <option value="set">Set</option>
      <option value="role">Role</option>
    </select>
  </label>
  <label class="f tid">
    <span class="ui-label">Target</span>
    <input class="ui-input" bind:value={target} placeholder="Target ID" aria-label="Target ID" spellcheck="false" autocomplete="off" />
  </label>
  <label class="f act">
    <span class="ui-label">Action</span>
    <input class="ui-input" bind:value={action} placeholder="role.updated" aria-label="Action" list="audit-actions" spellcheck="false" autocomplete="off" />
  </label>
  <datalist id="audit-actions">{#each ACTIONS as a}<option value={a}></option>{/each}</datalist>
  <div class="f date"><DateField label="From" value={from} onchange={(d) => apply({ from: d })} /></div>
  <div class="f date"><DateField label="To" value={to} onchange={(d) => apply({ to: d })} /></div>
  <div class="f btns">
    <button class="ui-btn secondary" type="submit">Filter</button>
    {#if page.url.search}<a class="ui-btn ghost" href="/admin/audit">Clear</a>{/if}
  </div>
</form>
{#if problem}<p class="ui-error range" role="alert">{problem}</p>{/if}

<LoadGate source={L} label="Loading the log…">
{#snippet children()}
{#if !entries || entries.length === 0}
  <State title="No entries{page.url.search ? ' for this filter' : ''}." />
{:else}
  <table class="adm-table log">
    <thead>
      <tr><th class="when">When</th><th>Actor</th><th>Action</th><th>Target</th><th>Details</th><th class="chev"><span class="sr-only">Open</span></th></tr>
    </thead>
    <tbody>
      {#each entries as e (e.id)}
        {@const href = targetHref(e.targetKind, e.targetId)}
        {@const list = changes(e.before, e.after)}
        {@const text = summarize(list)}
        <tr class="entry" class:on={open === e.id}>
          <td class="when mono dim">{dateTimeSeconds(e.createdAt)}</td>
          <td class="by"><AccountLink id={e.actorId} /></td>
          <td class="act">{actionText(e.action)}</td>
          <td class="tgt">
            <span class="ui-chip">{e.targetKind}</span>
            {#if href}<a class="mono id" {href} title={e.targetId}>{e.targetId}</a>{:else}<span class="mono id">{e.targetId}</span>{/if}
          </td>
          <td class="det ell dim" title={text}>{text}</td>
          <td class="chev">
            <button type="button" aria-expanded={open === e.id} aria-label="Show what changed" onclick={() => toggle(e.id)}><Icon name="chevron-down" size={14} /></button>
          </td>
        </tr>
        {#if open === e.id}
          <tr class="more">
            <td colspan="6">
              <div class="diff">
                <Segmented label="Show" size="s" options={[{ value: 'changes', label: 'Changes' }, { value: 'json', label: 'JSON' }]} value={raw ? 'json' : 'changes'} onchange={(v) => (raw = v === 'json')} />
                {#if raw}
                  {@const b = pretty(e.before)}
                  {@const a = pretty(e.after)}
                  <div class="json">
                    <div><span class="ui-label">Before</span><pre>{b.text || '—'}</pre></div>
                    <div><span class="ui-label">After</span><pre>{a.text || '—'}</pre></div>
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
              </div>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
  <div class="adm-pager">
    <button class="ui-btn secondary small" type="button" disabled={ix === 0 || L.loading} onclick={() => turn(-1)}>Newer</button>
    <span>{pageRange(ix, LIMIT, entries.length)}</span>
    <button class="ui-btn secondary small" type="button" disabled={!hasOlder || L.loading} onclick={() => turn(1)}>Older</button>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .range { margin: -8px 0 12px; }
  .filters { align-items: flex-end; }
  .f { display: grid; gap: 4px; min-width: 0; }
  .actor { width: 164px; }
  .kind { width: 118px; }
  .tid { width: 150px; }
  .act { flex: 1 1 140px; }
  .date { width: 124px; }
  @media (pointer: fine) { .filters :is(input, select) { font-size: var(--t-small); } }
  .btns { display: flex; align-items: flex-end; gap: 8px; }

  .log { table-layout: auto; }
  .log td { overflow-wrap: anywhere; }
  .when { width: 1%; white-space: nowrap; }
  .by { max-width: 180px; }
  .act { font-weight: 600; white-space: nowrap; }
  .tgt { display: flex; align-items: center; gap: 8px; max-width: 280px; }
  .tgt .ui-chip { flex: none; }
  .id { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  a.id { color: var(--accent-text); display: inline-flex; align-items: center; min-height: 24px; }
  .det { width: 30%; }
  .chev { width: 1%; padding: 0 4px; text-align: center; }
  .chev button { display: grid; place-items: center; width: 36px; height: 36px; padding: 0; border: 0; border-radius: var(--radius); background: none; color: var(--dim); cursor: pointer;
    transition: rotate var(--fast), background-color var(--fast); }
  .chev button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 0; }
  @media (hover: hover) and (pointer: fine) { .chev button:hover { background: var(--surface-hover); color: var(--text); } }
  .chev button[aria-expanded='true'] { rotate: 180deg; color: var(--accent-text); }
  .entry.on td { background: var(--bg); }
  .more td { height: auto; padding: 0; background: var(--bg); }
  .diff { display: grid; gap: 10px; justify-items: start; padding: 12px 16px 16px; }
  .json { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; width: 100%; }
  pre { margin: 6px 0 0; padding: 10px 12px; background: var(--elevated-2); border: 1px solid var(--border); border-radius: var(--radius); font: .78rem/1.65 var(--font-mono);
    white-space: pre-wrap; overflow-wrap: anywhere; }
  .changes { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; width: 100%; }
  .changes li { display: grid; grid-template-columns: minmax(80px, 200px) minmax(0, 1fr); gap: 12px; padding: 6px 10px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }
  .path { font-size: .78rem; color: var(--muted); overflow-wrap: anywhere; }
  .vals { display: flex; flex-wrap: wrap; gap: 4px 8px; font-size: .8rem; overflow-wrap: anywhere; }
  .was { color: var(--red); text-decoration: line-through; text-decoration-thickness: 1px; }
  .now { color: var(--green); }
  .arrow { color: var(--dim); }
  .link { min-height: 32px; padding: 0; border: 0; background: none; font: inherit; font-size: var(--t-small); color: var(--accent-text); cursor: pointer; }
  .link:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 2px; }

  /* Phones: each entry is a small card, the action and target on top, then who and when, then the details. */
  @media (max-width: 760px) {
    .filters .f { width: calc(50% - 4px); flex: 1 1 calc(50% - 4px); }
    .filters .tid, .filters .act, .filters .actor { width: 100%; flex-basis: 100%; }
    .filters .btns { flex-basis: 100%; }
    .log, .log tbody { display: block; }
    .log thead { display: none; }
    .log tr.entry { display: grid; grid-template-columns: minmax(0, 1fr) auto 44px; grid-template-areas: 'act act chev' 'tgt tgt chev' 'by when chev' 'det det chev';
      gap: 2px 12px; align-items: center; padding: 8px 4px 8px 12px; border-bottom: 1px solid var(--border); }
    .log tr.entry td { display: block; height: auto; padding: 0; border: 0; min-width: 0; background: none; }
    .log td.act { grid-area: act; }
    .log td.tgt { grid-area: tgt; max-width: none; }
    .log td.by { grid-area: by; max-width: none; }
    .log td.when { grid-area: when; width: auto; justify-self: end; }
    .log td.act { font-size: var(--t-ui); }
    .log td.det { grid-area: det; width: auto; max-width: none; }
    .log td.det:empty { display: none; }
    .log td.chev { grid-area: chev; align-self: start; padding: 0; width: auto; }
    .log td.chev button { width: 44px; height: 44px; }
    .log tr.more { display: block; border-bottom: 1px solid var(--border); }
    .log tr.more td { display: block; }
    .log tbody > :last-child { border-bottom: 0; }
    .json { grid-template-columns: minmax(0, 1fr); }
    .changes li { grid-template-columns: minmax(0, 1fr); gap: 2px; }
  }
</style>
