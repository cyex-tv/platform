<script lang="ts">
  // One notification: an icon, one sentence and when. Unread ones are bold with an accent icon
  // (the cyex rule: no fills, no bars). A link when the notification leads somewhere; the
  // sentence is printed as text, never as HTML. Used by the bell's popover and /me/notifications.
  import Icon from '$lib/components/Icon.svelte';
  import { dateTime, isoParts } from '$lib/format';
  import { relative } from '$lib/profile/activity';
  import { describe } from './notifications';
  import type { Note } from './inbox.svelte';

  type Props = { note: Note; onpick?: (n: Note) => void };
  let { note, onpick }: Props = $props();
  const line = $derived(describe(note.kind, note.payload));
</script>

{#snippet inner()}
  <Icon name={line.icon} />
  <span class="what">
    <span class="sentence">{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</span>
    <time class="when" datetime={note.createdAt} title={dateTime(note.createdAt)}>{relative(note.createdAt)}</time>
  </span>
{/snippet}

{#if line.href}
  <a class="note" class:unread={!note.read} href={line.href} onclick={() => onpick?.(note)}>{@render inner()}</a>
{:else}
  <button class="note" class:unread={!note.read} type="button" onclick={() => onpick?.(note)}>{@render inner()}</button>
{/if}

<style>
  .note { display: flex; align-items: flex-start; gap: 12px; width: 100%; min-height: 48px; padding: 10px 12px; border: 0;
    border-radius: var(--radius); background: none; color: var(--text); font: inherit; font-size: var(--t-nav); text-align: left;
    text-decoration: none; cursor: pointer; transition: background-color var(--fast); }
  .note :global(svg) { margin-top: 2px; color: var(--muted); }
  .what { display: grid; gap: 2px; min-width: 0; flex: 1; }
  .sentence { overflow-wrap: anywhere; line-height: 1.4; }
  .when { font-size: var(--t-fine); color: var(--dim); }
  .unread .sentence { font-weight: 600; }
  .unread :global(svg) { color: var(--accent-text); }
  .note:focus-visible { outline-offset: -2px; }
  .note:active { opacity: .6; transition: none; }
  @media (hover: hover) and (pointer: fine) { .note:hover { background: var(--row-hover); } }
</style>
