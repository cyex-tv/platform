<script lang="ts">
  // The profile's panel on the Notifications tab (owner only): the picked notification (?n=; its
  // full text, time, actions and link) or, with nothing picked, the numbers and the way to the
  // settings that choose which kinds come.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { count, date } from '$lib/format';
  import FactRows from '$lib/components/FactRows.svelte';
  import LinkRows from '$lib/components/LinkRows.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import { inbox } from '$lib/account/inbox.svelte';
  import NotificationRow from '$lib/account/NotificationRow.svelte';
  import { notesView } from './notes-view';

  const all = $derived(inbox.items ?? []);
  const picked = $derived(notesView(all, page.url.searchParams).picked);
  const oldest = $derived(all.length ? all[all.length - 1].createdAt : null);

  function close() {
    const url = new URL(page.url);
    url.searchParams.delete('n');
    void goto(`${url.pathname}${url.search}`, { replaceState: true, noScroll: true, keepFocus: true });
  }
</script>

{#if picked}
  <div class="pick"><NotificationRow note={picked} variant="detail" /></div>
  <div class="back"><button class="ui-btn ghost small" type="button" onclick={close}>Close</button></div>
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

<style>
  .pick { padding: 4px 0 8px; }
  .back { padding: 8px 0 0; }
</style>
