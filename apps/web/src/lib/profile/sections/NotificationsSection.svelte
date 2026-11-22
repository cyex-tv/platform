<script lang="ts">
  // Your profile → Notifications (?s=notifications, owner only; it was /me/notifications until
  // 2026/11/22, which now opens this tab). Every notification (the API keeps the latest 100),
  // newest first, 20 at a time. A slim toolbar: All, Unread and one filter per group of kinds (the
  // address keeps it, ?f=), the unread count and Mark all read. Dense hairline rows; a row marks
  // itself read when picked and the address holds the pick (?n=<id>): the profile's panel shows it
  // (NotificationsPanel). Settings → Notifications chooses which kinds come at all.
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { count } from '$lib/format';
  import Segmented from '$lib/components/Segmented.svelte';
  import Toolbar from '$lib/components/Toolbar.svelte';
  import { inbox, loadInbox, markRead, type Note } from '$lib/account/inbox.svelte';
  import NotificationRow from '$lib/account/NotificationRow.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { notesView } from '../notes-view';

  const PAGE = 20;
  let shown = $state(PAGE);
  const all = $derived(inbox.items ?? []);
  const view = $derived(notesView(all, page.url.searchParams));
  const pickedId = $derived(page.url.searchParams.get('n'));

  function go(mut: (p: URLSearchParams) => void) {
    const url = new URL(page.url);
    mut(url.searchParams);
    void goto(`${url.pathname}${url.search}`, { replaceState: true, noScroll: true, keepFocus: true });
  }
  function pickFilter(id: string) {
    shown = PAGE;
    go((p) => (id === 'all' ? p.delete('f') : p.set('f', id)));
  }
  function pick(n: Note) {
    go((p) => p.set('n', n.id));
    if (!n.read) void markRead([n.id]);
  }

  onMount(() => void loadInbox());
</script>

<PageHead title="Notifications">
  {#snippet line()}{inbox.count ? `${count(inbox.count)} unread` : 'You’re all caught up.'}{/snippet}
</PageHead>

{#if inbox.items === null && inbox.error}
  <Problem cause="server" onretry={() => loadInbox()} />
{:else if inbox.items === null}
  <State kind="loading" title="Loading notifications…" />
{:else if all.length === 0}
  <State title="Nothing yet">Decisions on your emotes, editor changes and cosmetics show up here.</State>
{:else}
  <div class="tb">
    <Toolbar label="Notifications">
      <Segmented label="Show" size="s" options={view.options} value={view.active} onchange={pickFilter} />
      {#snippet end()}
        <span class="tb-count">{inbox.count ? `${count(inbox.count)} unread` : 'all read'}</span>
        <button class="ui-btn secondary small" type="button" disabled={inbox.marking || !inbox.count} onclick={() => markRead()}>Mark all read</button>
      {/snippet}
    </Toolbar>
  </div>
  <section class="list" aria-label="Notifications">
    {#if view.items.length}
      <ul>
        {#each view.items.slice(0, shown) as n (n.id)}<li><NotificationRow note={n} variant="dense" picked={n.id === pickedId} onpick={pick} /></li>{/each}
      </ul>
    {:else}
      <p class="ui-hint none">Nothing here.</p>
    {/if}
  </section>
  <div class="foot"><LoadMore hasMore={shown < view.items.length} total={view.items.length} onmore={() => (shown += PAGE)} /></div>
{/if}

<style>
  .tb { margin: 0 0 12px; }
  .list ul { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  .none { padding: 16px 0; margin: 0; }
  .foot { margin-top: 16px; }
</style>
