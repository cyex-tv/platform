<script lang="ts">
  // The bell in the header (desktop; on phones the same list is a row in the menu sheet): a
  // 40px button with the unread count, opening a popover with the newest notifications, Mark all
  // read and a link to the full page. Enter or Space opens it, Escape closes it with focus back
  // on the bell. The count is kept fresh by watchInbox (the AppHeader starts it).
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import State from '$lib/components/State.svelte';
  import { badge } from './notifications';
  import { inbox, loadInbox, markRead, type Note } from './inbox.svelte';
  import NotificationRow from './NotificationRow.svelte';

  let open = $state(false);
  const shown = $derived(badge(inbox.count));
  const recent = $derived((inbox.items ?? []).slice(0, 8));

  $effect(() => {
    if (open) void loadInbox();
  });

  function pick(n: Note) {
    if (!n.read) void markRead([n.id]);
    open = false;
  }
</script>

<Popover label="Notifications" align="end" width="380px" bind:open>
  {#snippet trigger(props)}
    <button
      class="bell"
      type="button"
      {...props}
      aria-label={inbox.count ? `Notifications, ${shown === '99+' ? 'more than 99' : shown} unread` : 'Notifications'}
    >
      <Icon name="bell" size={18} />
      {#if shown}<span class="badge" aria-hidden="true">{shown}</span>{/if}
    </button>
  {/snippet}
  <div class="head">
    <h2>Notifications</h2>
    {#if inbox.count}<button class="ui-btn ghost small" type="button" onclick={() => markRead()}>Mark all read</button>{/if}
  </div>
  {#if inbox.items === null && inbox.error}
    <State bare kind="error" title="That didn’t load">
      {inbox.error}
      {#snippet actions()}<button class="ui-btn secondary small" type="button" onclick={() => loadInbox()}>Try again</button>{/snippet}
    </State>
  {:else if inbox.items === null}
    <State bare kind="loading" title="Loading…" />
  {:else if recent.length === 0}
    <State bare title="Nothing yet">Moderation decisions, editor changes and cosmetics show up here.</State>
  {:else}
    <ul class="list">
      {#each recent as n (n.id)}<li><NotificationRow note={n} onpick={pick} /></li>{/each}
    </ul>
  {/if}
  {#snippet footer()}
    <a class="all" href="/me/notifications" onclick={() => (open = false)}>See all notifications<Icon name="chevron-right" size={14} /></a>
  {/snippet}
</Popover>

<style>
  .bell { position: relative; width: 40px; height: 40px; flex: none; display: inline-grid; place-items: center; padding: 0;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: transparent; color: var(--muted); cursor: pointer;
    transition: color var(--fast), border-color var(--fast), background-color var(--fast), transform var(--dur-press) ease-out; }
  .bell:active { transform: scale(var(--press)); }
  .bell[aria-expanded='true'] { color: var(--text); background: var(--surface-hover); border-color: var(--border-strong); }
  .badge { position: absolute; top: -6px; right: -6px; min-width: 18px; height: 18px; padding: 0 5px; border-radius: var(--radius-pill);
    background: var(--accent); color: var(--accent-ink); font-size: 11px; font-weight: 600; line-height: 18px; text-align: center;
    font-variant-numeric: tabular-nums; box-shadow: 0 0 0 2px var(--bg); }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 40px; padding: 0 4px 0 12px; }
  .head h2 { margin: 0; font-size: var(--t-ui); font-weight: 600; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .all { display: flex; align-items: center; justify-content: space-between; width: 100%; min-height: 28px; margin: -2px 0; padding: 0 8px;
    color: var(--accent-text); font-size: var(--t-nav); font-weight: 500; text-decoration: none; }
  .all:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .bell:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-hover); } }
  @media (forced-colors: active) { .badge { border: 1px solid CanvasText; } .bell[aria-expanded='true'] { outline: 2px solid Highlight; } }
</style>
