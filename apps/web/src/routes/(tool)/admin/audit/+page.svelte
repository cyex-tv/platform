<script lang="ts">
  // The audit log with filters (actor, target kind and ID, action, date range) kept in the
  // address, readable rows that link to the target, and a per-row diff: what changed, or the raw
  // JSON. Pages back with "Older entries".
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import AccountLink from '$lib/admin/AccountLink.svelte';
  import ActorField from '$lib/admin/ActorField.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { lookUp } from '$lib/admin/names.svelte';
  import { actionText, changes, dayAfter, dayStart, targetHref } from '$lib/admin/audit';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { AUDIT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail } from '$lib/tool/notices.svelte';

  type Entry = AuditQuery['auditLog'][number];
  const LIMIT = 50;
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

  let older = $state<Entry[]>([]);
  let loadingOlder = $state(false);
  let olderSeq = 0;
  let olderMore = $state<boolean | null>(null);
  let open = $state<string | null>(null);
  let raw = $state(false);

  function vars(before: string | null) {
    return {
      targetKind: q('kind') || null,
      targetId: q('id') || null,
      actorId: q('actor') || null,
      action: q('action') || null,
      since: dayStart(q('from')),
      until: dayAfter(q('to')),
      before,
      limit: LIMIT,
    };
  }

  const L = loader(async ({ fetch }) => {
    void page.url.search;
    olderSeq++;
    older = [];
    olderMore = null;
    loadingOlder = false;
    open = null;
    const rows = (await gql(AUDIT, vars(null), fetch)).auditLog;
    lookUp(rows.map((r) => r.actorId));
    return rows;
  });
  const first = $derived(L.data);
  const entries = $derived(first ? [...first, ...older] : null);
  const more = $derived(olderMore ?? first?.length === LIMIT);

  async function fetchOlder() {
    const before = entries?.at(-1)?.id ?? null;
    if (!before || loadingOlder) return;
    const mine = ++olderSeq;
    loadingOlder = true;
    try {
      const r = (await gql(AUDIT, vars(before))).auditLog;
      if (mine !== olderSeq) return;
      lookUp(r.map((x) => x.actorId));
      older = [...older, ...r];
      olderMore = r.length === LIMIT;
    } catch (e) {
      if (mine === olderSeq) fail(e);
    } finally {
      if (mine === olderSeq) loadingOlder = false;
    }
  }

  function filter(e: SubmitEvent) {
    e.preventDefault();
    const p = new URLSearchParams();
    for (const [k, v] of Object.entries({ kind, id: target, actor, action, from, to })) if (v.trim()) p.set(k, v.trim());
    goto(`?${p}`, { keepFocus: true });
  }

  /** `2026-11-16T20:01:02.345Z` as `2026/11/16 20:01:02`, in the viewer's time zone. */
  function when(iso: string): string {
    const d = new Date(iso);
    const p = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`;
  }

  function pretty(json: string | null | undefined): string {
    if (!json) return '';
    try {
      return JSON.stringify(JSON.parse(json), null, 2);
    } catch {
      return json;
    }
  }
</script>

<svelte:head><title>Audit log - Admin - cyex.tv</title></svelte:head>

<PageHead title="Audit log" line="Every audited change, newest first." />

<form class="filters" onsubmit={filter}>
  <ActorField value={actor} onchange={(id) => (actor = id)} />
  <select class="ui-input kind" bind:value={kind} aria-label="Target kind">
    <option value="">Any kind</option>
    <option value="account">Account</option>
    <option value="emote">Emote</option>
    <option value="set">Set</option>
    <option value="role">Role</option>
  </select>
  <input class="ui-input" bind:value={target} placeholder="Target ID" aria-label="Target ID" spellcheck="false" autocomplete="off" />
  <input class="ui-input" bind:value={action} placeholder="Action, such as role.updated" aria-label="Action" list="audit-actions" spellcheck="false" autocomplete="off" />
  <datalist id="audit-actions">{#each ACTIONS as a}<option value={a}></option>{/each}</datalist>
  <DateField label="From" value={from} onchange={(d) => (from = d)} />
  <DateField label="To" value={to} onchange={(d) => (to = d)} />
  <div class="ui-actions">
    <button class="ui-btn secondary" type="submit">Filter</button>
    {#if page.url.search}<a class="ui-btn ghost" href="/admin/audit">Clear</a>{/if}
  </div>
</form>

<LoadGate source={L} label="Loading the log…">
{#snippet children()}
{#if !entries || entries.length === 0}
  <State title="No entries{page.url.search ? ' for this filter' : ''}." />
{:else}
  <section class="ui-panel log">
    <div class="row head" aria-hidden="true">
      <span class="ui-label">When</span><span class="ui-label">What</span><span class="ui-label">Target</span><span class="ui-label">By</span><span></span>
    </div>
    <ul class="rows">
      {#each entries as e, i (e.id)}
        {@const href = targetHref(e.targetKind, e.targetId)}
        <li class:alt={i % 2 === 1}>
          <div class="row">
            <span class="ui-mono t">{when(e.createdAt)}</span>
            <span class="action">{actionText(e.action)}</span>
            <span class="target">
              <span class="ui-chip">{e.targetKind}</span>
              {#if href}<a class="ui-mono id" {href} title={e.targetId}>{e.targetId}</a>{:else}<span class="ui-mono id">{e.targetId}</span>{/if}
            </span>
            <span class="by"><AccountLink id={e.actorId} /></span>
            <button class="chev" type="button" aria-expanded={open === e.id} aria-label="Show what changed" onclick={() => (open = open === e.id ? null : e.id)}>
              <Icon name="chevron-down" size={14} />
            </button>
          </div>
          {#if open === e.id}
            <div class="diff">
              <Segmented label="Show" size="s" options={[{ value: 'changes', label: 'Changes' }, { value: 'json', label: 'JSON' }]} value={raw ? 'json' : 'changes'} onchange={(v) => (raw = v === 'json')} />
              {#if raw}
                <div class="json">
                  <div><span class="ui-label">Before</span><pre>{pretty(e.before) || '—'}</pre></div>
                  <div><span class="ui-label">After</span><pre>{pretty(e.after) || '—'}</pre></div>
                </div>
              {:else}
                {@const list = changes(e.before, e.after)}
                {#if list.length}
                  <ul class="changes">
                    {#each list as c (c.path)}
                      <li>
                        <span class="ui-mono path">{c.path}</span>
                        <span class="vals ui-mono">
                          {#if c.kind !== 'added'}<span class="was">{c.before}</span>{/if}
                          {#if c.kind === 'changed'}<span class="arrow" aria-label="became">→</span>{/if}
                          {#if c.kind !== 'removed'}<span class="now">{c.after}</span>{/if}
                          {#if c.kind === 'removed'}<span class="ui-hint">removed</span>{/if}
                        </span>
                      </li>
                    {/each}
                  </ul>
                {:else}
                  <p class="ui-hint">No field changes recorded for this entry.</p>
                {/if}
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
  {#if more}
    <div class="ui-actions next">
      <button class="ui-btn secondary" type="button" disabled={loadingOlder} onclick={fetchOlder}>
        {loadingOlder ? 'Loading…' : 'Older entries'}
      </button>
    </div>
  {/if}
{/if}
{/snippet}
</LoadGate>

<style>
  .filters { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(200px, 100%), 1fr)); gap: 8px; align-items: end; margin-bottom: 16px; max-width: 1000px; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .rows li.alt { background: var(--row-alt); }
  .row { display: grid; grid-template-columns: 150px minmax(140px, 1fr) minmax(0, 2fr) minmax(0, 1.2fr) 40px; gap: 12px; align-items: center; min-height: 40px;
    padding: 4px 4px 4px 14px; border-bottom: 1px solid var(--border); }
  .head { background: var(--row-alt); }
  .chev { display: grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 0; border-radius: var(--radius); background: none; color: var(--dim); cursor: pointer;
    transition: rotate var(--fast), background-color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .chev:hover { background: var(--surface-hover); color: var(--text); } }
  .chev[aria-expanded='true'] { rotate: 180deg; }
  .target { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .target .ui-chip { flex: none; }
  .id { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .t { font-size: 12px; font-weight: 600; color: var(--dim); }
  .action { font-weight: 600; }
  .id { font-size: .78rem; color: var(--muted); }
  a.id { display: inline-block; max-width: 100%; line-height: 24px; color: var(--accent-text); }
  .by { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--text-sub); }
  .diff { display: grid; gap: 10px; justify-items: start; padding: 10px 14px 14px; border-bottom: 1px solid var(--border); background: var(--bg); }
  .json { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; width: 100%; }
  pre { margin: 6px 0 0; padding: 10px 12px; background: var(--elevated-2); border: 1px solid var(--border); font: .78rem/1.65 var(--font-mono);
    white-space: pre-wrap; overflow-wrap: anywhere; }
  .changes { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; width: 100%; }
  .changes li { display: grid; grid-template-columns: minmax(80px, 200px) minmax(0, 1fr); gap: 12px; padding: 6px 10px; border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); }
  .path { font-size: .78rem; color: var(--muted); overflow-wrap: anywhere; }
  .vals { display: flex; flex-wrap: wrap; gap: 4px 8px; font-size: .8rem; overflow-wrap: anywhere; }
  .was { color: var(--red); text-decoration: line-through; text-decoration-thickness: 1px; }
  .now { color: var(--green); }
  .arrow { color: var(--dim); }
  .next { justify-content: center; margin-top: 16px; }
  @media (max-width: 760px) {
    .head { display: none; }
    .row { grid-template-columns: minmax(0, 1fr) 40px; gap: 2px 8px; }
    .row > :not(.chev) { grid-column: 1; }
    .chev { grid-column: 2; grid-row: 1 / span 4; align-self: start; }
    .json { grid-template-columns: minmax(0, 1fr); }
    .changes li { grid-template-columns: minmax(0, 1fr); gap: 2px; }
  }
  /* a touch target of at least 24px (WCAG 2.5.8) */
  .rows a { display: inline-block; vertical-align: middle; min-height: 24px; line-height: 24px; max-width: 100%; }
</style>
