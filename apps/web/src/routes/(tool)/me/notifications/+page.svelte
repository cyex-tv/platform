<script lang="ts">
  // Every notification (the API keeps the latest 100), newest first, in pages of 20 here, with the
  // same rows as the bell. Mark all read in the header; a row marks itself read when opened.
  import { onMount, setContext } from 'svelte';
  import { inbox, loadInbox, markRead, type Note } from '$lib/account/inbox.svelte';
  import NotificationRow from '$lib/account/NotificationRow.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  // Your profile's name above is the page's h1; this title is a section heading like the profile tabs'.
  setContext('heading-level', 2);

  const PAGE = 20;
  let shown = $state(PAGE);
  const items = $derived(inbox.items ?? []);

  onMount(() => void loadInbox());

  function pick(n: Note) {
    if (!n.read) void markRead([n.id]);
  }
</script>

<svelte:head><title>Notifications - cyex.tv</title></svelte:head>

<div class="narrow">
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
  {:else if items.length === 0}
    <State title="Nothing yet">Decisions on your emotes, editor changes and cosmetics show up here.</State>
  {:else}
    <section class="ui-panel" aria-label="Notifications">
      <ul class="list">
        {#each items.slice(0, shown) as n (n.id)}<li><NotificationRow note={n} onpick={pick} /></li>{/each}
      </ul>
    </section>
    <div class="foot"><LoadMore hasMore={shown < items.length} total={items.length} onmore={() => (shown += PAGE)} /></div>
  {/if}
</div>

<style>
  .list { list-style: none; margin: 0; padding: 6px; display: grid; gap: 2px; }
  .foot { margin-top: 16px; }
</style>
