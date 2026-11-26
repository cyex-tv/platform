<script lang="ts">
  // The slim one-line bar above a list (redesign round 2, like the browse bar): filters at the
  // left, then search, a count, sort or Select at the right; a hairline under it. It wraps on
  // phones. `sticky` keeps it under the header while the list scrolls. `.tb-count` is the bar's
  // mono count ("24 emotes").
  //   <Toolbar label="Notifications">{#snippet end()}<span class="tb-count">5 unread</span>{/snippet}<Segmented …/></Toolbar>
  import type { Snippet } from 'svelte';

  type Props = { label?: string; children?: Snippet; end?: Snippet; sticky?: boolean };
  let { label, children, end, sticky = false }: Props = $props();
</script>

<div class="toolbar" class:sticky role={label ? 'group' : undefined} aria-label={label}>
  {#if children}<div class="tb-start">{@render children()}</div>{/if}
  {#if end}<div class="tb-end">{@render end()}</div>{/if}
</div>

<style>
  .toolbar { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; min-height: 52px; padding: 6px 0; margin-bottom: 16px;
    border-bottom: 1px solid var(--border); }
  .sticky { position: sticky; top: var(--nav-h); z-index: 2; background: var(--bg); }
  .tb-start { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 16px; min-width: 0; }
  .tb-end { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; margin-left: auto; min-width: 0; }
  .toolbar :global(.tb-count) { font-family: var(--font-mono); font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  @media (max-width: 639.98px) { .tb-end { margin-left: 0; width: 100%; } }
</style>
