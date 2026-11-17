<script lang="ts">
  // A designed state for a page or a block: nothing found, nothing set yet, nothing uploaded.
  // A title, one or two sentences of what happened and what to do, then actions.
  // `level` is the heading level (1 when the state is the whole page); `rule` draws the top line.
  import type { Snippet } from 'svelte';

  type Props = { title: string; children?: Snippet; actions?: Snippet; role?: 'status' | 'alert'; level?: 1 | 2; rule?: boolean };
  let { title, children, actions, role, level = 2, rule = false }: Props = $props();
</script>

<section class="state" class:rule {role}>
  <svelte:element this={level === 1 ? 'h1' : 'h2'}>{title}</svelte:element>
  {#if children}<p>{@render children()}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</section>

<style>
  .state { max-width: 52ch; padding: 40px 0 8px; }
  .rule { border-top: 1px solid var(--border); }
  .state > :global(h1), .state > :global(h2) { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; overflow-wrap: anywhere; }
  p { margin: 8px 0 0; color: var(--muted); font-size: var(--t-body); text-wrap: pretty; overflow-wrap: anywhere; }
  .actions { margin-top: 24px; }
</style>
