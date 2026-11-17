<script lang="ts">
  import { gql } from '$lib/api';
  import { ready, src } from '$lib/emote';
  import { count, date } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { APPROVE, REJECT, REVIEW_QUEUE } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const PER_PAGE = 50;
  const L = loader(async ({ fetch }) => (await gql(REVIEW_QUEUE, { page: 1, perPage: PER_PAGE }, fetch)).reviewQueue);
  const items = $derived(L.data?.items ?? null);
  const total = $derived(L.data?.total ?? 0);
  let busy = $state<string | null>(null);
  let rejecting = $state<EmoteFieldsFragment | null>(null);
  let dialog: ReturnType<typeof ReasonDialog>;

  function done(id: string) {
    const q = L.data;
    if (!q) return;
    const left = q.items.filter((e) => e.id !== id);
    L.set({ ...q, items: left, total: Math.max(0, q.total - 1) });
    if (left.length === 0 && q.total - 1 > 0) L.reload();
  }

  async function approve(e: EmoteFieldsFragment) {
    busy = e.id;
    try {
      await gql(APPROVE, { id: e.id });
      notify(`${e.name} approved.`);
      done(e.id);
    } catch (err) {
      fail(err);
    } finally {
      busy = null;
    }
  }

  function askReject(e: EmoteFieldsFragment) {
    rejecting = e;
    dialog.open();
  }

  async function reject(reason: string) {
    const e = rejecting;
    if (!e) return;
    try {
      await gql(REJECT, { id: e.id, reason });
      notify(`${e.name} rejected.`);
      done(e.id);
    } catch (err) {
      fail(err);
      throw err;
    }
  }
</script>

<svelte:head><title>Review - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1>Review queue</h1><p>{#if items}<span class="ui-mono">{count(total)}</span> pending, oldest first{:else}Oldest first{/if}</p></div></div>

<LoadGate source={L} label="Loading the queue…">
{#snippet children(queue)}
{@const items = queue.items}
{#if items.length === 0}
  <div class="ui-empty">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 6 9 17l-5-5" /></svg>
    <p>Nothing waiting. New uploads show up here.</p>
  </div>
{:else}
  <ul class="queue">
    {#each items as e (e.id)}
      <li class="ui-panel">
        <div class="stage">
          {#if ready(e)}
            {#each [1, 2, 4] as const as s}<img src={src(e, s)} alt={s === 4 ? e.name : ''} loading="lazy" />{/each}
          {:else}
            <span class="ui-spinner" aria-hidden="true"></span><span class="ui-hint">Files not made yet</span>
          {/if}
        </div>
        <div class="info">
          <strong class="name">{e.name}</strong>
          <span class="ui-hint">
            {e.owner?.displayName ?? 'Unknown'} · <span class="ui-mono">{date(e.createdAt)}</span>{e.animated ? ' · animated' : ''}
          </span>
          {#if e.tags.length}<span class="tags">{#each e.tags as t}<span class="ui-chip">{t}</span>{/each}</span>{/if}
        </div>
        <div class="ui-actions do">
          <button class="ui-btn primary small" type="button" disabled={busy === e.id || !ready(e)} onclick={() => approve(e)}>Approve</button>
          <button class="ui-btn danger small" type="button" disabled={busy === e.id} onclick={() => askReject(e)}>Reject</button>
        </div>
      </li>
    {/each}
  </ul>
  {#if queue.total > items.length}<p class="ui-hint more">Showing the oldest {items.length}. The rest load as you work through these.</p>{/if}
{/if}
{/snippet}
</LoadGate>

<ReasonDialog
  bind:this={dialog}
  title={`Reject ${rejecting?.name ?? ''}`}
  hint="The uploader sees this. Be short and concrete."
  action="Reject"
  danger
  onsubmit={reject}
/>

<style>
  .queue { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr)); gap: 16px; }
  .queue > li { display: grid; grid-template-rows: auto 1fr auto; min-width: 0; }
  .stage { display: flex; align-items: flex-end; justify-content: center; gap: 16px; height: 150px; padding: 16px; overflow: hidden;
    border-bottom: 1px solid var(--border); background: var(--bg); }
  .stage img { max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; }
  .info { display: grid; align-content: start; gap: 4px; padding: 10px 14px; min-width: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 2px; }
  .tags .ui-chip { text-transform: none; letter-spacing: 0; font-size: .7rem; font-weight: 600; }
  .do { padding: 0 14px 12px; }
  .more { margin-top: 16px; text-align: center; }
</style>
