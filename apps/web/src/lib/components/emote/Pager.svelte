<script lang="ts">
  // Previous and next page for the emote page's tabs: which rows show ("1–24 of 26"), then two
  // icon buttons. A button that has nowhere to go is disabled; `busy` holds both while a page loads.
  import IconButton from '$lib/components/IconButton.svelte';

  type Props = { label: string; range: string; prev: boolean; next: boolean; busy?: boolean; onprev: () => void; onnext: () => void };
  let { label, range, prev, next, busy = false, onprev, onnext }: Props = $props();
</script>

{#if prev || next || range}
  <nav class="pager" aria-label={label}>
    {#if range}<span class="range" aria-live="polite">{range}</span>{/if}
    {#if prev || next}
      <IconButton icon="chevron-left" label="Previous page" size="s" tip="top" disabled={!prev || busy} onclick={onprev} />
      <IconButton icon="chevron-right" label="Next page" size="s" tip="top" disabled={!next || busy} onclick={onnext} />
    {/if}
  </nav>
{/if}

<style>
  .pager { display: flex; align-items: center; gap: 2px; flex: none; }
  .range { margin-right: 8px; font-size: var(--t-fine); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
