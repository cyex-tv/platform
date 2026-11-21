<script lang="ts">
  // Every notification (the API keeps the latest 100), newest first, in pages of 20 here. A slim
  // toolbar: All, Unread and one filter per group of kinds (the address keeps it, ?f=), the unread
  // count and Mark all read. Dense hairline rows; a row marks itself read when picked and the
  // address holds the pick (?n=<id>). The panel shows the picked one (its full text, time,
  // actions and link) or, with nothing picked, the numbers. Settings → Notifications chooses which
  // kinds come at all. In your profile's frame (MeFrame).
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { count, date } from '$lib/format';
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import { inbox, loadInbox, markRead, type Note } from '$lib/account/inbox.svelte';
  import { filterChips, groupOf } from '$lib/account/prefs';
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
  const active = $derived(filter === 'unread' || chips.some((c) => c.id === filter) ? filter : 'all');
  const items = $derived(active === 'all' ? all : active === 'unread' ? all.filter((n) => !n.read) : all.filter((n) => groupOf(n.kind) === active));
  const options = $derived([
    { value: 'all', label: 'All' },
    { value: 'unread', label: 'Unread' },
    ...chips.filter((c) => c.id !== 'all').map((c) => ({ value: c.id, label: c.label })),
  ]);
  const pickedId = $derived(page.url.searchParams.get('n'));
  const picked = $derived(pickedId ? all.find((n) => n.id === pickedId) : undefined);
  const oldest = $derived(all.length ? all[all.length - 1].createdAt : null);

  function go(mut: (p: URLSearchParams) => void) {
    const url = new URL(page.url);
    mut(url.searchParams);
    void goto(`${url.pathname}${url.search}`, { replaceState: true, noScroll: true, keepFocus: true });
  }
  function pickFilter(id: string) {
    shown = PAGE;
    go((p) => (id === 'all' ? p.delete('f') : p.set('f', id)));
  }

  onMount(() => void loadInbox());

  function pick(n: Note) {
    go((p) => p.set('n', n.id));
    if (!n.read) void markRead([n.id]);
  }
</script>

<svelte:head><title>Notifications - cyex.tv</title></svelte:head>

{#snippet side()}
  {#if picked}
    <div class="pick"><NotificationRow note={picked} variant="detail" /></div>
    <div class="back"><button class="ui-btn ghost small" type="button" onclick={() => go((p) => p.delete('n'))}>Close</button></div>
  {:else}
    <PanelBlock title="In numbers">
      <FactRows items={[
        { label: 'Unread', value: inbox.items ? count(inbox.count) : '–', mono: true },
        { label: 'Kept', value: inbox.items ? count(all.length) : '–', mono: true },
        { label: 'Oldest', value: oldest ? date(oldest) : '–', mono: true },
      ]} />
      <p class="pb-hint">cyex.tv keeps your latest 100 notifications. Pick one to read it in full.</p>
    </PanelBlock>
    <PanelBlock title="What comes">
      <LinkRows items={[{ href: '/me/settings/notifications', label: 'Notification settings', hint: 'Pick which kinds you get, here and as toasts', icon: 'settings' }]} />
    </PanelBlock>
  {/if}
{/snippet}

{#snippet toolbar()}
  {#if inbox.items && all.length}
    <Toolbar label="Notifications">
      <Segmented label="Show" size="s" {options} value={active} onchange={pickFilter} />
      {#snippet end()}
        <span class="tb-count">{inbox.count ? `${count(inbox.count)} unread` : 'all read'}</span>
        <button class="ui-btn secondary small" type="button" disabled={inbox.marking || !inbox.count} onclick={() => markRead()}>Mark all read</button>
      {/snippet}
    </Toolbar>
  {/if}
{/snippet}

{#snippet head()}
  <PageHead title="Notifications">
    {#snippet line()}{inbox.count ? `${inbox.count} unread` : 'You’re all caught up.'}{/snippet}
  </PageHead>
{/snippet}

<MeFrame title="Notification" label="About your notifications" panel={side} {head} {toolbar}>

  {#if inbox.items === null && inbox.error}
    <Problem cause="server" onretry={() => loadInbox()} />
  {:else if inbox.items === null}
    <State kind="loading" title="Loading notifications…" />
  {:else if all.length === 0}
    <State title="Nothing yet">Decisions on your emotes, editor changes and cosmetics show up here.</State>
  {:else}
    <section class="list" aria-label="Notifications">
      {#if items.length}
        <ul>
          {#each items.slice(0, shown) as n (n.id)}<li><NotificationRow note={n} variant="dense" picked={n.id === pickedId} onpick={pick} /></li>{/each}
        </ul>
      {:else}
        <p class="ui-hint none">Nothing here.</p>
      {/if}
    </section>
    <div class="foot"><LoadMore hasMore={shown < items.length} total={items.length} onmore={() => (shown += PAGE)} /></div>
  {/if}
</MeFrame>

<style>
  .list ul { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  .none { padding: 16px 0; margin: 0; }
  .foot { margin-top: 16px; }
  .pick { padding: 4px 0 8px; }
  .back { padding: 8px 0 0; }
</style>
