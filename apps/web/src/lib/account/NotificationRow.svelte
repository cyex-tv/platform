<script lang="ts">
  // One notification: an icon, one sentence and when (date and time, how long ago on hover). Unread ones are bold with an accent icon
  // (the cyex rule: no fills, no bars). A link when the notification leads somewhere; the
  // sentence is printed as text, never as HTML. Used by the bell's popover and your profile's Notifications tab.
  import { tick } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { isoParts } from '$lib/format';
  import ActivityWhen from '$lib/profile/ActivityWhen.svelte';
  import { describe } from './notifications';
  import type { Note } from './inbox.svelte';

  // `variant`: 'card' is the bell's two-line row; 'dense' a fixed-height hairline row (icon, one line
  // that ellipsizes, mono time, an accent dot when unread) for lists; 'detail' the whole notification
  // (full sentence, time and link) for an inspector panel. Nothing here is answered: transfer
  // requests are retired (ADR 0014, amendment 2026/11/22).
  // `onread` (card only): an unread notification gets its own "Mark as read" button beside it, a
  // real button in the tab order; once it is read the button is gone and focus goes to the row.
  type Props = { note: Note; onpick?: (n: Note) => void; onread?: (n: Note) => void | Promise<unknown>; variant?: 'card' | 'dense' | 'detail'; picked?: boolean };
  let { note, onpick, onread, variant = 'card', picked = false }: Props = $props();
  const line = $derived(describe(note.kind, note.payload));
  let main = $state<HTMLElement>();
  async function markOne() {
    await onread?.(note);
    await tick();
    main?.focus();
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

{#if variant === 'detail'}
  <div class="detail">
    <p class="full" class:unread={!note.read}>{#each isoParts(line.text) as p}{#if p.name}<bdi>{p.text}</bdi>{:else}{p.text}{/if}{/each}</p>
    <p class="when ui-mono"><Icon name={line.icon} size={14} /><ActivityWhen at={note.createdAt} /></p>
    {#if line.href}<a class="ui-btn secondary small open" href={line.href}>Open</a>{/if}
  </div>
{:else if variant === 'dense'}
  {#if line.href && !onpick}
    <a class="drow" class:unread={!note.read} href={line.href}>{@render dense()}</a>
  {:else}
    <button class="drow" class:unread={!note.read} class:picked type="button" aria-current={picked ? 'true' : undefined} onclick={() => onpick?.(note)}>{@render dense()}</button>
  {/if}
{:else}
  <div class="item" class:withmark={onread && !note.read}>
    {#if line.href}
      <a class="note" class:unread={!note.read} href={line.href} bind:this={main} onclick={() => onpick?.(note)}>{@render inner()}</a>
    {:else}
      <button class="note" class:unread={!note.read} type="button" bind:this={main} onclick={() => onpick?.(note)}>{@render inner()}</button>
    {/if}
    {#if onread && !note.read}
      <button class="mark" type="button" aria-label="Mark as read" title="Mark as read" onclick={markOne}><Icon name="check" size={15} /></button>
    {/if}
  </div>
{/if}

<style>
  .item { position: relative; display: flex; align-items: flex-start; }
  .item .note { flex: 1; min-width: 0; }
  .withmark .note { padding-right: 44px; }
  /* The mark button sits at the row's end, 32px to see and the row's height to hit; it is the
     accent icon on hover and focus, never a fill. */
  .mark { position: absolute; top: 6px; right: 6px; display: inline-grid; place-items: center; width: 32px; height: 32px; padding: 0;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--dim); cursor: pointer;
    transition: color var(--fast), background-color var(--fast), transform var(--dur-press) ease-out; }
  .mark:focus-visible { outline-offset: -2px; color: var(--accent-text); }
  .mark:active { transform: scale(var(--press)); }
  @media (hover: hover) and (pointer: fine) { .mark:hover { color: var(--accent-text); background: var(--row-hover); } }
  @media (pointer: coarse) { .mark { width: 44px; height: 44px; top: 2px; right: 2px; } .withmark .note { padding-right: 52px; } }
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
</style>
