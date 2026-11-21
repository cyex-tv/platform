<script lang="ts">
  // One notification: an icon, one sentence and when (date and time, how long ago on hover). Unread ones are bold with an accent icon
  // (the cyex rule: no fills, no bars). A link when the notification leads somewhere; the
  // sentence is printed as text, never as HTML. Used by the bell's popover and /me/notifications.
  import Icon from '$lib/components/Icon.svelte';
  import { isoParts } from '$lib/format';
  import ActivityWhen from '$lib/profile/ActivityWhen.svelte';
  import { gql } from '$lib/api';
  import { ACCEPT_EMOTE_TRANSFER, DECLINE_EMOTE_TRANSFER } from '$lib/queries';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { closedRequest, describe, requestGone } from './notifications';
  import { answered, markRead, type Note } from './inbox.svelte';

  // `variant`: 'card' is the bell's two-line row; 'dense' a fixed-height hairline row (icon, one line
  // that ellipsizes, mono time, an accent dot when unread) for lists; 'detail' the whole notification
  // (full sentence, time, its actions and link) for an inspector panel.
  type Props = { note: Note; onpick?: (n: Note) => void; variant?: 'card' | 'dense' | 'detail'; picked?: boolean };
  let { note, onpick, variant = 'card', picked = false }: Props = $props();
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
      // Answered is seen: the row stops being unread and the count follows.
      if (!note.read) void markRead([note.id]);
    } catch (err) {
      fail(err);
      // Closed already (expired, withdrawn or answered): say so once, then leave it. Any other
      // failure (offline, a timeout, a rate limit) leaves the request open and the buttons here.
      if (requestGone(err)) {
        answered[req] = 'This request is closed.';
        if (!note.read) void markRead([note.id]);
      }
    } finally {
      busy = false;
    }
  }
</script>

{#snippet inner()}
  <Icon name={line.icon} />
  <span class="what">
    <span class="sentence">{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</span>
    <span class="when"><ActivityWhen at={note.createdAt} /></span>
  </span>
{/snippet}

{#snippet dense()}
  <Icon name={line.icon} size={15} />
  <span class="sentence one">{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</span>
  <span class="when ui-mono"><ActivityWhen at={note.createdAt} /></span>
  <span class="dot" aria-hidden="true"></span>
{/snippet}

{#snippet answers()}
  {#if line.request}
    <div class="answer" class:flush={variant === 'detail'}>
      {#if answered[line.request] ?? closed}<span class="ui-hint" role="status">{answered[line.request] ?? closed}</span>
      {:else}
        <button class="ui-btn primary small" type="button" disabled={busy} onclick={() => answer(true)}>Accept</button>
        <button class="ui-btn secondary small" type="button" disabled={busy} onclick={() => answer(false)}>Decline</button>
      {/if}
    </div>
  {/if}
{/snippet}

{#if variant === 'detail'}
  <div class="detail">
    <p class="full" class:unread={!note.read}>{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</p>
    <p class="when ui-mono"><Icon name={line.icon} size={14} /><ActivityWhen at={note.createdAt} /></p>
    {@render answers()}
    {#if line.href}<a class="ui-btn secondary small open" href={line.href}>Open</a>{/if}
  </div>
{:else if variant === 'dense'}
  {#if line.href && !onpick}
    <a class="drow" class:unread={!note.read} href={line.href}>{@render dense()}</a>
  {:else}
    <button class="drow" class:unread={!note.read} class:picked type="button" aria-current={picked ? 'true' : undefined} onclick={() => onpick?.(note)}>{@render dense()}</button>
  {/if}
{:else}
  {#if line.href}
    <a class="note" class:unread={!note.read} href={line.href} onclick={() => onpick?.(note)}>{@render inner()}</a>
  {:else}
    <button class="note" class:unread={!note.read} type="button" onclick={() => onpick?.(note)}>{@render inner()}</button>
  {/if}
  {@render answers()}
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
  .drow { display: grid; grid-template-columns: 16px minmax(0, 1fr) auto 8px; align-items: center; gap: 12px; width: 100%; height: 44px; padding: 0 4px;
    border: 0; border-bottom: 1px solid var(--border); border-radius: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-small);
    text-align: left; text-decoration: none; cursor: pointer; transition: background-color var(--fast); }
  .drow :global(svg) { color: var(--dim); }
  .drow .one { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; line-height: 1.2; }
  .drow .when { font-size: 12px; color: var(--muted); }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: transparent; }
  .drow.unread :global(svg) { color: var(--accent-text); }
  .drow.unread .dot { background: var(--accent-text); }
  .drow.picked { background: var(--row-hover); box-shadow: inset 2px 0 0 var(--accent-text); }
  .drow:focus-visible { outline-offset: -2px; }
  .drow:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) { .drow:hover { background: var(--row-hover); } }
  @media (pointer: coarse) { .drow { height: 48px; } }
  .detail { display: grid; justify-items: start; gap: 10px; }
  .full { margin: 0; font-size: var(--t-ui); line-height: 1.5; overflow-wrap: anywhere; }
  .full.unread { font-weight: 600; }
  .detail .when { display: inline-flex; align-items: center; gap: 6px; margin: 0; font-size: 12px; color: var(--muted); }
  .answer.flush { padding: 0; }
  .answer { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 0 12px 10px 44px; }
</style>
