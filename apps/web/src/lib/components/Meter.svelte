<script lang="ts">
  // How full something is, as a 3px bar (the browse rail's set meter): the accent, yellow from
  // 90 %, red when full. Decorative next to a written count (Capacity) unless `label` is given
  // (then it is a role="meter" with that name).
  //   <Meter used={24} capacity={1000} />
  import { fillPercent } from '$lib/sets/public';

  type Props = { used: number; capacity: number; label?: string; width?: string };
  let { used, capacity, label, width }: Props = $props();
  const pct = $derived(fillPercent(used, capacity));
</script>

<span class="meter" class:full={pct >= 100} class:near={pct >= 90 && pct < 100} style:width
  role={label ? 'meter' : undefined} aria-label={label} aria-valuemin={label ? 0 : undefined} aria-valuemax={label ? 100 : undefined}
  aria-valuenow={label ? pct : undefined} aria-hidden={label ? undefined : 'true'}><i style:width="{pct}%"></i></span>

<style>
  .meter { display: block; width: 100%; height: 3px; border-radius: 2px; background: var(--border); overflow: hidden; }
  i { display: block; height: 100%; border-radius: inherit; background: var(--accent); transition: width var(--base, .2s) var(--ease); }
  .near i { background: var(--yellow); }
  .full i { background: var(--red); }
  @media (forced-colors: active) { i { background: Highlight; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) i { transition: none; } }
</style>
