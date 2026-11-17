<script lang="ts">
  import { gql } from '$lib/api';
  import { ready, size, src, srcsetW } from '$lib/emote';
  import { tick } from 'svelte';
  import { clean, count, date, iso } from '$lib/format';
  import type { EmoteFieldsFragment } from '$lib/gql/graphql';
  import { APPROVE, REJECT, REVIEW_QUEUE } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  const PER_PAGE = 50;
  /** The art box, as in EmoteTile at its comfortable density. */
  const ART = 112;
  const L = loader(async ({ fetch }) => (await gql(REVIEW_QUEUE, { page: 1, perPage: PER_PAGE }, fetch)).reviewQueue);
  const items = $derived(L.data?.items ?? null);
  const total = $derived(L.data?.total ?? 0);
  let busy = $state<string | null>(null);
  let rejecting = $state<EmoteFieldsFragment | null>(null);
  let dialog: ReturnType<typeof ReasonDialog>;

  function done(id: string) {
    const q = L.data;
    if (!q) return;
    const at = q.items.findIndex((e) => e.id === id);
    const nextId = (q.items[at + 1] ?? q.items[at - 1])?.id;
    tick().then(() => {
      const next = nextId ? document.querySelector<HTMLElement>(`[data-card="${CSS.escape(nextId)}"] .approve`) : null;
      (next ?? document.querySelector<HTMLElement>('h1'))?.focus();
    });
    const left = q.items.filter((e) => e.id !== id);
    L.set({ ...q, items: left, total: Math.max(0, q.total - 1) });
    if (left.length === 0 && q.total - 1 > 0) L.reload();
  }

  async function approve(e: EmoteFieldsFragment) {
    busy = e.id;
    try {
      await gql(APPROVE, { id: e.id });
      notify(`${iso(e.name)} approved.`);
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
      notify(`${iso(e.name)} rejected.`);
      done(e.id);
    } catch (err) {
      fail(err);
      throw err;
    }
  }
</script>

<svelte:head><title>Review - cyex.tv</title></svelte:head>

<div class="ui-head"><div><h1 tabindex="-1">Review queue</h1><p>{#if items}<span class="ui-mono">{count(total)}</span> pending, oldest first{:else}Oldest first{/if}</p></div></div>

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
      {@const dims = size(e)}
      <li class="tile" data-card={e.id}>
        <span class="status"><span class="ui-chip pending">Pending</span></span>
        <span class="art">
          {#if ready(e)}
            <img src={src(e, 1)} srcset={srcsetW(e)} sizes="{Math.round((ART * 0.8 * dims.width) / dims.height)}px" width={dims.width} height={dims.height} alt={clean(e.name)} loading="lazy" decoding="async" />
          {:else}
            <span class="ui-spinner" aria-hidden="true"></span><span class="ui-hint">Files not made yet</span>
          {/if}
        </span>
        <span class="name"><bdi>{clean(e.name)}</bdi></span>
        <span class="by"><bdi>{clean(e.owner?.displayName) || 'Unknown'}</bdi></span>
        <span class="meta ui-mono">{date(e.createdAt)}</span>
        {#if e.animated || e.tags.length}<span class="tags">{#if e.animated}<span class="ui-chip">animated</span>{/if}{#each e.tags.slice(0, 3) as t}<span class="ui-chip">{t}</span>{/each}</span>{/if}
        <div class="do">
          <button class="ui-btn primary small approve" type="button" disabled={busy === e.id || !ready(e)} onclick={() => approve(e)}>Approve</button>
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
  title={`Reject ${iso(rejecting?.name)}`}
  hint="The uploader sees this. Be short and concrete."
  action="Reject"
  danger
  onsubmit={reject}
/>

<style>
  h1:focus { outline: none; }
  /* The anatomy of EmoteTile (surface card, hairline, art box, name, creator), with the verdict
     buttons under it. Not an EmoteTile itself: that is one link, and a pending emote has no public page. */
  .queue { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(min(var(--emote-tile, 150px), 100%), 1fr)); gap: 12px; }
  .tile { position: relative; display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 0; padding: 14px 10px 10px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); text-align: center;
    transition: border-color var(--fast); }
  @media (hover: hover) and (pointer: fine) { .tile:hover { border-color: var(--border-strong); } }
  .status { position: absolute; top: 6px; left: 6px; z-index: 1; }
  .art { display: grid; place-items: center; align-content: center; gap: 6px; width: 100%; height: 112px; margin-bottom: 8px; }
  .art img { display: block; height: calc(112px * .8); width: auto; max-width: 100%; object-fit: contain; }
  .name, .by { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .name { font-size: var(--t-nav); font-weight: 500; line-height: 1.35; }
  .by { font-size: 12px; line-height: 1.4; color: var(--muted); }
  .meta { font-size: 11px; color: var(--dim); }
  .tags { display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; margin-top: 4px; }
  .tags .ui-chip { text-transform: none; letter-spacing: 0; font-size: .7rem; font-weight: 600; max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
  .do { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; width: 100%; margin-top: auto; padding-top: 10px; }
  .do .ui-btn { padding-inline: 6px; }
  .more { margin-top: 16px; text-align: center; }
</style>
