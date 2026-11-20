<script lang="ts">
  // Two icons in one cell that cross-fade when `on` flips (copy → check): opacity and a small
  // scale, 150ms (--dur-pop). Both stay mounted, so a quick second click reverses from where it
  // is. Under reduced motion the swap is instant (tokens.css zeroes transitions; "Off" keeps them).
  import Icon, { type IconName } from './Icon.svelte';

  let { a, b, on, size = 16 }: { a: IconName; b: IconName; on: boolean; size?: number } = $props();
</script>

<span class="swap" class:on>
  <Icon name={a} {size} />
  <Icon name={b} {size} />
</span>

<style>
  .swap { display: inline-grid; flex: none; }
  .swap :global(.icon) { grid-area: 1 / 1; transition: opacity var(--dur-pop) ease-out, scale var(--dur-pop) ease-out; }
  .swap :global(.icon:last-child), .swap.on :global(.icon:first-child) { opacity: 0; scale: .6; }
  .swap.on :global(.icon:last-child) { opacity: 1; scale: 1; }
</style>
