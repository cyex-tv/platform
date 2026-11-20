<script lang="ts" module>
  export type ArtKind = 'extension' | 'deck' | 'api' | 'cosmetics';
</script>

<script lang="ts">
  import { DECK_PANES } from './showcase-geometry';
  // The small picture at the top of each front-page box, in the hero logo's language: open bars
  // on one 120 × 80 grid, ink lines, and exactly one lit part with the logo's glow. Same size,
  // stroke and light in every box, so the four read as a set. When its box is hovered, the lit
  // part moves a few pixels (not under reduced motion).
  type Props = { kind: ArtKind };
  let { kind }: Props = $props();
  const id = $props.id();
  const panes = DECK_PANES;
</script>

<svg class="art" viewBox="0 0 120 80" width="120" height="80" aria-hidden="true" focusable="false">
  <defs>
    <filter id="{id}-glow" filterUnits="userSpaceOnUse" x="-20" y="-20" width="160" height="120">
      <feGaussianBlur stdDeviation="2.4" />
    </filter>
  </defs>

  {#snippet lit(x: number, y: number, w: number, h: number)}
    <g class="lit">
      <rect class="halo" x={x} y={y} width={w} height={h} filter="url(#{id}-glow)" />
      <rect class="fill" x={x + 3} y={y + 3} width={Math.max(0, w - 6)} height={Math.max(0, h - 6)} />
      <rect class="edge" x={x} y={y} width={w} height={h} />
    </g>
  {/snippet}

  {#if kind === 'extension'}
    <!-- A browser window; in its chat, the logo's three lines, the middle one lit. -->
    <rect class="ink" x="13" y="9" width="94" height="62" />
    <path class="ink" d="M13 21h94" />
    <path class="ink dim" d="M20 15h4M28 15h4M36 15h4" />
    <rect class="ink" x="24" y="30" width="40" height="8" />
    {@render lit(36, 43, 48, 8)}
    <rect class="ink" x="24" y="56" width="40" height="8" />
  {:else if kind === 'deck'}
    <!-- An app window: a channel list beside a grid of tiles, one of them lit (the one playing). -->
    <rect class="ink" x="11" y="9" width="98" height="62" />
    <path class="ink" d="M31 9v62" />
    <path class="ink dim" d="M17 18h8M17 26h8M17 34h8" />
    {#each panes as p (p.id)}
      {#if p.lit}
        {@render lit(p.x, p.y, p.w, p.h)}
      {:else}
        <rect class="ink pane" x={p.x} y={p.y} width={p.w} height={p.h} />
      {/if}
    {/each}
  {:else if kind === 'api'}
    <!-- Two ends of a request, and the one lit bar travelling between them. -->
    <rect class="ink" x="9" y="22" width="32" height="36" />
    <rect class="ink" x="79" y="22" width="32" height="36" />
    <path class="ink dim" d="M16 32h18M16 40h12M16 48h16M86 32h18M86 40h14M86 48h10" />
    <path class="ink" d="M41 40h38" />
    {@render lit(50, 35, 20, 10)}
  {:else}
    <!-- Two name plates: the front one with a lit badge, the one behind it quieter. -->
    <rect class="ink dim" x="25" y="45" width="82" height="20" />
    <rect class="ink dim" x="31" y="51" width="8" height="8" />
    <path class="ink dim" d="M45 55h40" />
    <rect class="ink plate" x="13" y="15" width="90" height="24" />
    {@render lit(20, 21, 12, 12)}
    <path class="ink" d="M40 27h46" />
  {/if}
</svg>

<style>
  .art { display: block; width: 120px; height: 80px; overflow: visible; }
  rect, path { fill: none; stroke-width: 2; stroke-linecap: square; }
  .ink { stroke: var(--mark-ink); }
  .dim { stroke: var(--muted); }
  /* The front plate covers the one behind it. */
  .plate { fill: var(--surface); }
  .halo { stroke: var(--accent); stroke-width: 3; opacity: .9; }
  .fill { fill: var(--accent); stroke: none; opacity: .28; }
  .edge { stroke: var(--accent); }
  :global([data-theme='light']) .edge { stroke: color-mix(in srgb, var(--accent) 60%, #00323d); }
  :global([data-theme='light']) .fill { opacity: .45; }
  .lit { transition: transform var(--dur-move) var(--ease); }
  @media (hover: hover) and (pointer: fine) {
    :global(html:not([data-motion='reduce']) .show:hover) .lit { transform: translateX(3px); }
  }
  @media (forced-colors: active) {
    .ink, .dim, .edge { stroke: CanvasText; }
    .halo, .fill { display: none; }
  }
</style>
