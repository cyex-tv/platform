<script lang="ts">
  // The one empty, loading and error state of the signed-in pages: a hairline panel with an icon
  // (or spinner), a title, a sentence and an action. Errors are announced (role=alert), loading
  // is a status. `bare` drops the panel for a state inside another panel.
  import type { Snippet } from 'svelte';

  type Props = {
    kind?: 'empty' | 'loading' | 'error';
    title?: string;
    children?: Snippet;
    actions?: Snippet;
    bare?: boolean;
  };
  let { kind = 'empty', title, children, actions, bare = false }: Props = $props();
</script>

<div class="state" class:bare role={kind === 'error' ? 'alert' : kind === 'loading' ? 'status' : undefined} aria-busy={kind === 'loading' ? 'true' : undefined}>
  {#if kind === 'loading'}
    <span class="ui-spinner" aria-hidden="true"></span>
  {:else if kind === 'error'}
    <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
  {:else}
    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="3" /><path d="M8 14s1.5 2 4 2 4-2 4-2M9 9h.01M15 9h.01" /></svg>
  {/if}
  {#if title}<svelte:element this={kind === 'loading' ? 'p' : 'h2'} class="t">{title}</svelte:element>{/if}
  {#if children}<p>{@render children()}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</div>

<style>
  .state { display: grid; justify-items: center; gap: 10px; min-height: 180px; align-content: center; padding: 40px 20px; text-align: center;
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); color: var(--muted); }
  .bare { background: none; border: 0; border-radius: 0; min-height: 0; padding: 32px 16px; }
  svg { width: 28px; height: 28px; fill: none; stroke: var(--dim); stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
  .t { margin: 2px 0 0; font-size: var(--t-ui); font-weight: 600; color: var(--text); }
  p:not(.t) { margin: 0; max-width: 52ch; font-size: var(--t-small); line-height: 1.5; text-wrap: pretty; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; margin-top: 6px; }
</style>
