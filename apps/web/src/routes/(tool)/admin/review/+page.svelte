<script lang="ts">
  import { gql } from '$lib/api';
  import { ready, src } from '$lib/emote';
  import { count, date } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { APPROVE, REJECT, REVIEW_QUEUE } from '$lib/queries';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const PER_PAGE = 50;
  let items = $state<EmoteFieldsFragment[] | null>(null);
  let total = $state(0);
  let busy = $state<string | null>(null);
  let rejecting = $state<EmoteFieldsFragment | null>(null);
  let dialog: ReturnType<typeof ReasonDialog>;

  async function load() {
    try {
      const r = await gql(REVIEW_QUEUE, { page: 1, perPage: PER_PAGE });
      items = r.reviewQueue.items;
      total = r.reviewQueue.total;
    } catch (e) {
      fail(e);
    }
  }

  $effect(() => {
    load();
  });

  function done(id: string) {
    items = items?.filter((e) => e.id !== id) ?? null;
    total = Math.max(0, total - 1);
    if (items && items.length === 0 && total > 0) load();
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

<div class="ui-head"><div><h1>Review queue</h1><p><span class="ui-mono">{count(total)}</span> pending, oldest first</p></div></div>

{#if !items}
  <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span></div>
{:else if items.length === 0}
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
  {#if total > items.length}<p class="ui-hint more">Showing the oldest {items.length}. The rest load as you work through these.</p>{/if}
{/if}

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
  .stage { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: center; gap: 16px; min-height: 150px; padding: 16px;
    border-bottom: 1px solid var(--border); background: var(--bg); }
  .stage img { max-width: 100%; height: auto; }
  .info { display: grid; gap: 4px; padding: 10px 14px; min-width: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tags { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 2px; }
  .tags .ui-chip { text-transform: none; letter-spacing: 0; font-size: .7rem; font-weight: 600; }
  .do { padding: 0 14px 12px; }
  .more { margin-top: 16px; text-align: center; }
</style>
