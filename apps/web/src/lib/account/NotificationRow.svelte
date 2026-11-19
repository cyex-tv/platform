<script lang="ts">
  // One notification: an icon, one sentence and when (date and time, how long ago on hover). Unread ones are bold with an accent icon
  // (the cyex rule: no fills, no bars). A link when the notification leads somewhere; the
  // sentence is printed as text, never as HTML. Used by the bell's popover and /me/notifications.
  import Icon from '$lib/components/Icon.svelte';
  import { dateTime, isoParts } from '$lib/format';
  import { relative } from '$lib/profile/activity';
  import { gql } from '$lib/api';
  import { ACCEPT_EMOTE_TRANSFER, DECLINE_EMOTE_TRANSFER } from '$lib/queries';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { closedRequest, describe } from './notifications';
  import { answered, type Note } from './inbox.svelte';

  type Props = { note: Note; onpick?: (n: Note) => void };
  let { note, onpick }: Props = $props();
  const line = $derived(describe(note.kind, note.payload));

  // What the request is now (answered elsewhere, withdrawn or run out): the buttons go, the state stays.
  const closed = $derived(closedRequest(note.transferState));
  // A transfer request is answered here too (ADR 0014); the emote's edit page lists it as well.
  let busy = $state(false);
  async function answer(accept: boolean) {
    const req = line.request;
    if (!req || busy) return;
    busy = true;
    try {
      if (accept) await gql(ACCEPT_EMOTE_TRANSFER, { id: req });
      else await gql(DECLINE_EMOTE_TRANSFER, { id: req });
      answered[req] = accept ? 'Accepted: it moved.' : 'Declined: it stays yours.';
      notify(answered[req]);
    } catch (err) {
      // Closed already (expired, withdrawn or answered): say so once, then leave it.
      fail(err);
      answered[req] = 'This request is closed.';
    } finally {
      busy = false;
    }
  }
</script>

{#snippet inner()}
  <Icon name={line.icon} />
  <span class="what">
    <span class="sentence">{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</span>
    <time class="when" datetime={note.createdAt} title={relative(note.createdAt)}>{dateTime(note.createdAt)}</time>
  </span>
{/snippet}

{#if line.href}
  <a class="note" class:unread={!note.read} href={line.href} onclick={() => onpick?.(note)}>{@render inner()}</a>
{:else}
  <button class="note" class:unread={!note.read} type="button" onclick={() => onpick?.(note)}>{@render inner()}</button>
{/if}
{#if line.request}
  <div class="answer">
    {#if answered[line.request] ?? closed}<span class="ui-hint" role="status">{answered[line.request] ?? closed}</span>
    {:else}
      <button class="ui-btn primary small" type="button" disabled={busy} onclick={() => answer(true)}>Accept</button>
      <button class="ui-btn secondary small" type="button" disabled={busy} onclick={() => answer(false)}>Decline</button>
    {/if}
  </div>
{/if}

<style>
  .note { display: flex; align-items: flex-start; gap: 12px; width: 100%; min-height: 48px; padding: 10px 12px; border: 0;
    border-radius: var(--radius); background: none; color: var(--text); font: inherit; font-size: var(--t-nav); text-align: left;
    text-decoration: none; cursor: pointer; transition: background-color var(--fast); }
  .note :global(svg) { margin-top: 2px; color: var(--muted); }
  .what { display: grid; gap: 2px; min-width: 0; flex: 1; }
  .sentence { overflow-wrap: anywhere; line-height: 1.4; }
  .when { font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .unread .sentence { font-weight: 600; }
  .unread :global(svg) { color: var(--accent-text); }
  .note:focus-visible { outline-offset: -2px; }
  .note:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) { .note:hover { background: var(--row-hover); } }
  .answer { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 0 12px 10px 44px; }
</style>
