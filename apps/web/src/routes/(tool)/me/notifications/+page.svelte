<script lang="ts">
  // Every notification (the API keeps the latest 100), newest first, in pages of 20 here, with the
  // same rows as the bell. Mark all read in the header; a row marks itself read when opened.
  // Chips above the list narrow it to one group of kinds (uploads, hand-overs, editors,
  // cosmetics; the address keeps it, ?f=<group>); Settings → Notifications chooses which kinds come
  // at all. In your profile's frame (MeFrame); the panel counts them and holds the actions.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { count } from '$lib/format';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import { inbox, loadInbox, markRead, type Note } from '$lib/account/inbox.svelte';
  import { filterChips, groupOf } from '$lib/account/prefs';
  import Icon from '$lib/components/Icon.svelte';
  import NotificationRow from '$lib/account/NotificationRow.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  const PAGE = 20;
  let shown = $state(PAGE);
  const all = $derived(inbox.items ?? []);
  const filter = $derived(page.url.searchParams.get('f') ?? 'all');
  const chips = $derived(filterChips(all.map((n) => n.kind)));
  // A filter whose last notification went away falls back to All.
  const active = $derived(chips.some((c) => c.id === filter) ? filter : 'all');
  const items = $derived(active === 'all' ? all : all.filter((n) => groupOf(n.kind) === active));
  function pickFilter(id: string) {
    const url = new URL(page.url);
    if (id === 'all') url.searchParams.delete('f');
    else url.searchParams.set('f', id);
    shown = PAGE;
    void goto(`${url.pathname}${url.search}`, { replaceState: true, noScroll: true, keepFocus: true });
  }

  onMount(() => void loadInbox());

  function pick(n: Note) {
    if (!n.read) void markRead([n.id]);
  }
</script>

<svelte:head><title>Notifications - cyex.tv</title></svelte:head>

{#snippet side()}
  <PanelBlock title="In numbers">
    <dl class="kv">
      <dt>Unread</dt><dd>{inbox.items ? count(inbox.count) : '–'}</dd>
      <dt>Kept</dt><dd>{inbox.items ? count(all.length) : '–'}</dd>
    </dl>
    <p class="pb-hint">cyex.tv keeps your latest 100 notifications.</p>
  </PanelBlock>
  <PanelBlock title="What comes">
    <p class="pb-hint">Pick which kinds of notification you get, here and as toasts.</p>
    <div class="pb-actions">
      <a class="ui-btn secondary" href="/me/settings/notifications"><Icon name="settings" size={15} />Notification settings</a>
    </div>
  </PanelBlock>
{/snippet}

<MeFrame title="Notifications" label="About your notifications" panel={side}>
<div class="list-col">
  <PageHead title="Notifications">
    {#snippet line()}{inbox.count ? `${inbox.count} unread` : 'You’re all caught up.'}{/snippet}
    {#snippet actions()}
      {#if inbox.count}<button class="ui-btn secondary" type="button" disabled={inbox.marking} onclick={() => markRead()}>Mark all read</button>{/if}
    {/snippet}
  </PageHead>

  {#if inbox.items === null && inbox.error}
    <Problem cause="server" onretry={() => loadInbox()} />
  {:else if inbox.items === null}
    <State kind="loading" title="Loading notifications…" />
  {:else if all.length === 0}
    <State title="Nothing yet">Decisions on your emotes, editor changes and cosmetics show up here.</State>
  {:else}
    {#if chips.length > 2}
      <div class="chips" role="group" aria-label="Show">
        {#each chips as c (c.id)}
          <button class="chip" type="button" aria-pressed={active === c.id} onclick={() => pickFilter(c.id)}>
            {#if active === c.id}<span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>{/if}<span class="lb" data-t={c.label}>{c.label}</span><span class="n ui-num">{c.count}</span>
          </button>
        {/each}
      </div>
    {/if}
    <section class="ui-panel" aria-label="Notifications">
      <ul class="list">
        {#each items.slice(0, shown) as n (n.id)}<li><NotificationRow note={n} onpick={pick} /></li>{/each}
      </ul>
    </section>
    <div class="foot"><LoadMore hasMore={shown < items.length} total={items.length} onmore={() => (shown += PAGE)} /></div>
  {/if}
</div>
</MeFrame>

<style>
  .list-col { max-width: 760px; }
  .list { list-style: none; margin: 0; padding: 6px; display: grid; gap: 2px; }
  .foot { margin-top: 16px; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px 6px; margin-bottom: 12px; }
  /* Selected: bold words and an accent tick, never a fill or a coloured border (owner rule). */
  .chip { position: relative; display: inline-flex; align-items: center; gap: 6px; height: 32px; padding: 0 10px; border: 1px solid var(--border);
    border-radius: var(--radius); background: var(--bg); color: var(--muted); font: inherit; font-size: 13px; font-weight: 500; cursor: pointer;
    transition: color var(--fast), border-color var(--fast); }
  .chip::after { content: ''; position: absolute; inset: -4px 0; }
  .chip[aria-pressed='true'] { color: var(--text); font-weight: 700; }
  .tick { display: inline-flex; color: var(--accent-text); }
  .lb { display: inline-grid; }
  .lb::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  .n { font-size: 12px; font-weight: 400; color: var(--dim); }
  .chip:active { opacity: .6; transition: none; }
  .chip:focus-visible { outline-offset: 1px; }
  @media (hover: hover) and (pointer: fine) { .chip:hover { color: var(--text); border-color: var(--border-strong); } }
  @media (pointer: coarse) { .chip { height: 36px; } }
  @media (forced-colors: active) { .chip[aria-pressed='true'] { outline: 1px solid Highlight; outline-offset: -2px; } }
</style>
