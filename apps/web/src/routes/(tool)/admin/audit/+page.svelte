<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import type { AuditQuery } from '$lib/gql/graphql';
  import { AUDIT } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { fail } from '$lib/tool/notices.svelte';

  type Entry = AuditQuery['auditLog'][number];
  const LIMIT = 50;
  let older = $state<Entry[]>([]);
  let loadingOlder = $state(false);
  let olderSeq = 0;
  let olderMore = $state<boolean | null>(null);
  let open = $state<string | null>(null);
  let kind = $state(page.url.searchParams.get('kind') ?? '');
  let target = $state(page.url.searchParams.get('id') ?? '');

  const L = loader(async ({ fetch }) => {
    const kindQ = page.url.searchParams.get('kind') || null;
    const idQ = page.url.searchParams.get('id') || null;
    olderSeq++;
    older = [];
    olderMore = null;
    loadingOlder = false;
    return (await gql(AUDIT, { targetKind: kindQ, targetId: idQ, before: null, limit: LIMIT }, fetch)).auditLog;
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
      const r = await gql(AUDIT, {
        targetKind: page.url.searchParams.get('kind') || null,
        targetId: page.url.searchParams.get('id') || null,
        before,
        limit: LIMIT,
      });
      if (mine !== olderSeq) return;
      older = [...older, ...r.auditLog];
      olderMore = r.auditLog.length === LIMIT;
    } catch (e) {
      if (mine === olderSeq) fail(e);
    } finally {
      if (mine === olderSeq) loadingOlder = false;
    }
  }

  function filter(e: SubmitEvent) {
    e.preventDefault();
    const q = new URLSearchParams();
    if (kind.trim()) q.set('kind', kind.trim());
    if (target.trim()) q.set('id', target.trim());
    goto(`?${q}`, { keepFocus: true });
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

<svelte:head><title>Audit log - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Audit log</h1><p>Every audited change, newest first.</p></div></div>

<form class="filters" onsubmit={filter}>
  <select class="ui-input kind" bind:value={kind} aria-label="Target kind">
    <option value="">Any kind</option>
    <option value="account">Account</option>
    <option value="emote">Emote</option>
    <option value="set">Set</option>
    <option value="role">Role</option>
  </select>
  <input class="ui-input" bind:value={target} placeholder="Target ID" aria-label="Target ID" spellcheck="false" />
  <button class="ui-btn secondary" type="submit">Filter</button>
  {#if page.url.search}<a class="ui-btn ghost" href="/admin/audit">Clear</a>{/if}
</form>

<LoadGate source={L} label="Loading the log…">
{#snippet children()}
{#if !entries || entries.length === 0}
  <div class="ui-empty"><p>No entries{page.url.search ? ' for this filter' : ''}.</p></div>
{:else}
  <section class="ui-panel log">
    <div class="row head" aria-hidden="true">
      <span class="ui-label">When</span><span class="ui-label">Action</span><span class="ui-label">Target</span><span class="ui-label">By</span>
    </div>
    <ul class="rows">
      {#each entries as e, i (e.id)}
        <li class:alt={i % 2 === 1}>
          <button class="row" type="button" aria-expanded={open === e.id} onclick={() => (open = open === e.id ? null : e.id)}>
            <span class="ui-mono t">{when(e.createdAt)}</span>
            <span class="action">{e.action}</span>
            <span class="ui-mono target">{e.targetKind} {e.targetId}</span>
            <span class="ui-mono by">{e.actorId ?? 'system'}</span>
          </button>
          {#if open === e.id}
            <div class="diff">
              <div><span class="ui-label">Before</span><pre>{pretty(e.before) || '—'}</pre></div>
              <div><span class="ui-label">After</span><pre>{pretty(e.after) || '—'}</pre></div>
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
  .filters { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; max-width: var(--measure); }
  .filters .ui-input { flex: 1; min-width: min(180px, 100%); }
  .filters .kind { flex: 0 1 160px; }
  .rows { list-style: none; margin: 0; padding: 0; }
  .rows li.alt { background: var(--row-alt); }
  .row { display: grid; grid-template-columns: 150px minmax(140px, 1fr) minmax(0, 2fr) minmax(0, 1.2fr); gap: 12px; align-items: baseline;
    width: 100%; padding: 7px 14px; border: 0; border-bottom: 1px solid var(--border); background: none; color: var(--text);
    font: inherit; text-align: left; cursor: pointer; }
  @media (hover: hover) and (pointer: fine) { button.row:hover { background: var(--row-hover); box-shadow: inset 2px 0 0 var(--border-strong); } }
  button.row:focus-visible { outline-offset: -2px; }
  .head { cursor: default; }
  .t { font-size: .74rem; font-weight: 600; color: var(--dim); }
  .action { font-weight: 600; }
  .target, .by { font-size: .78rem; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .diff { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; padding: 10px 14px 14px; border-bottom: 1px solid var(--border); background: var(--bg); }
  pre { margin: 6px 0 0; padding: 10px 12px; background: var(--elevated-2); border: 1px solid var(--border); font: .78rem/1.65 var(--font-mono);
    white-space: pre-wrap; overflow-wrap: anywhere; }
  .next { justify-content: center; margin-top: 16px; }
  @media (max-width: 760px) {
    .head { display: none; }
    .row { grid-template-columns: minmax(0, 1fr); gap: 2px; }
    .diff { grid-template-columns: minmax(0, 1fr); }
  }
</style>
