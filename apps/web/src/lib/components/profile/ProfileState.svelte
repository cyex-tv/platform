<script lang="ts">
  // A designed state for a profile page: nothing found, nothing set yet, nothing uploaded.
  // A title, one or two sentences of what happened and what to do, then actions.
  import type { Snippet } from 'svelte';

  type Props = { title: string; children?: Snippet; actions?: Snippet; role?: 'status' | 'alert' };
  let { title, children, actions, role }: Props = $props();
</script>

<section class="state" {role}>
  <h2>{title}</h2>
  {#if children}<p>{@render children()}</p>{/if}
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</section>

<style>
  .state { max-width: 52ch; padding: 40px 0 8px; border-top: 1px solid var(--border); }
  h2 { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; }
  p { margin: 8px 0 0; color: var(--muted); font-size: var(--t-body); text-wrap: pretty; }
  .actions { margin-top: 24px; }
</style>
