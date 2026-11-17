<script lang="ts">
  // The hero's art: real emotes from the library on tiles, composed by hand, still apart from the
  // animated ones playing (their first frame under reduced motion, without any script). One wide
  // tile, one zero-width overlay stacked on its base, and the page's single soft glow behind.
  // Sizes are in container units, so the same composition holds from a phone to 4K.
  import { hasStill, src } from '$lib/emote';
  import type { Cluster, HomeEmote } from './pick';

  let { art }: { art: Cluster } = $props();

  // Centre x, centre y and size in cqw of a 100 x 80 box, and a small turn. Biggest first.
  const SLOTS = [
    { x: 46, y: 38, s: 30, r: -3 },
    { x: 82, y: 20, s: 19, r: 4 },
    { x: 15, y: 18, s: 16, r: -5 },
    { x: 14, y: 52, s: 14, r: 3 },
    { x: 88, y: 58, s: 14, r: -4 },
    { x: 60, y: 8, s: 11, r: 5 },
    { x: 93, y: 40, s: 9, r: -2 },
  ];
  const WIDE = { x: 64, y: 68, s: 16, r: 2 };
  const STACK = { x: 26, y: 70, s: 18, r: -2 };

  const tiles = $derived(art.tiles.slice(0, SLOTS.length).map((e, i) => ({ e, ...SLOTS[i] })));
  const names = $derived(
    [...art.tiles, art.wide, art.base, art.overlay].filter((e): e is HomeEmote => !!e).map((e) => e.name),
  );
</script>

{#snippet pic(e: HomeEmote, eager: boolean, over = false)}
  {#if hasStill(e)}
    <picture>
      <source media="(prefers-reduced-motion: reduce)" srcset={src(e, 4, true)} />
      <img class:over src={src(e, 4)} alt="" loading={eager ? 'eager' : 'lazy'} decoding="async" />
    </picture>
  {:else}
    <img class:over src={src(e, 4)} alt="" loading={eager ? 'eager' : 'lazy'} decoding="async" />
  {/if}
{/snippet}

<div class="cluster" role="img" aria-label="Emotes from the library: {names.join(', ')}">
  <div class="box">
    {#each tiles as t, i (t.e.id)}
      <span class="tile" class:big={i === 0} style:--x={t.x} style:--y={t.y} style:--s={t.s} style:--r="{t.r}deg">
        {@render pic(t.e, true)}
      </span>
    {/each}
    {#if art.wide}
      <span class="tile wide" style:--x={WIDE.x} style:--y={WIDE.y} style:--s={WIDE.s} style:--r="{WIDE.r}deg">
        {@render pic(art.wide, true)}
      </span>
    {/if}
    {#if art.base && art.overlay}
      <span class="tile stack" style:--x={STACK.x} style:--y={STACK.y} style:--s={STACK.s} style:--r="{STACK.r}deg">
        {@render pic(art.base, true)}
        {@render pic(art.overlay, true, true)}
      </span>
    {/if}
  </div>
</div>

<style>
  .cluster { position: relative; container-type: inline-size; width: 100%; }
  /* The one soft glow on the page's first screen. */
  .cluster::before { content: ''; position: absolute; inset: 4% 6%; z-index: 0; pointer-events: none;
    background: radial-gradient(closest-side, var(--accent-glow), transparent); opacity: .7; }
  .box { position: relative; z-index: 1; height: 80cqw; }
  .tile { position: absolute; left: calc(var(--x) * 1cqw); top: calc(var(--y) * 1cqw);
    width: calc(var(--s) * 1cqw); height: calc(var(--s) * 1cqw);
    translate: -50% -50%; rotate: var(--r);
    display: grid; place-items: center;
    border: 1px solid var(--border); border-radius: var(--radius-m);
    background: var(--surface); box-shadow: var(--shadow); }
  .big { border-radius: var(--radius-l); }
  .wide { width: calc(var(--s) * 2cqw); }
  .tile :global(picture) { display: contents; }
  .tile img { grid-area: 1 / 1; display: block; height: 66%; width: auto; max-width: 84%; object-fit: contain; }
  .stack img.over { z-index: 1; }
</style>
