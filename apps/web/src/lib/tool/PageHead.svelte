<script lang="ts">
  // The one header of every signed-in page: an optional back link, the title (the public pages'
  // h1 size), a short line under it, and the page's primary action on the right. Under 640px the
  // action drops below the text. The line can be plain text or your own markup.
  import type { Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  type Props = {
    title: string;
    line?: string | Snippet;
    back?: { href: string; label: string };
    actions?: Snippet;
    /** Keep the title's own direction and characters (an emote or set name). */
    name?: boolean;
  };
  let { title, line, back, actions, name = false }: Props = $props();
</script>

<header class="head">
  {#if back}
    <a class="back" href={back.href}><Icon name="arrow-left" size={15} />{back.label}</a>
  {/if}
  <div class="row">
    <div class="text">
      <h1>{#if name}<bdi>{title}</bdi>{:else}{title}{/if}</h1>
      {#if line}<p class="line">{#if typeof line === 'string'}{line}{:else}{@render line()}{/if}</p>{/if}
    </div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
</header>

<style>
  .head { margin-bottom: 24px; }
  .back { display: inline-flex; align-items: center; gap: 6px; min-height: 32px; margin: 0 0 4px -2px; font-size: var(--t-small);
    color: var(--muted); text-decoration: none; }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
  .row { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; }
  .text { min-width: 0; flex: 1 1 280px; }
  h1 { margin: 0; font-size: var(--t-h3-sub); font-weight: 600; line-height: 1.1; letter-spacing: -.03em; overflow-wrap: anywhere; text-wrap: balance; }
  .line { margin: 8px 0 0; max-width: 64ch; font-size: var(--t-ui); line-height: 1.5; color: var(--muted); font-variant-numeric: tabular-nums; text-wrap: pretty; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
</style>
