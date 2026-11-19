<script lang="ts">
  // The cyex.tv logo, drawn flat: three open bars, the middle one lit with a soft glow (blue on
  // dark, cyan on light). It is the hero's first paint, the fallback where there's no 3D, and
  // what reduced motion keeps. Same grid as the 3D scene (logo.ts), so the two line up.
  import { BARS, CENTER, GRID, STROKE } from './logo';

  const id = $props.id();
  const half = STROKE / 2;
  // The view is centred on the logo, as the 3D camera is.
  const view = `${CENTER.x - GRID / 2} ${CENTER.y - GRID / 2} ${GRID} ${GRID}`;
</script>

<svg class="logo" viewBox={view} aria-hidden="true" focusable="false">
  <defs>
    <filter id="{id}-glow" filterUnits="userSpaceOnUse" x="-4" y="-4" width="40" height="40">
      <feGaussianBlur stdDeviation="1.1" />
    </filter>
    <filter id="{id}-soft" filterUnits="userSpaceOnUse" x="-4" y="-4" width="40" height="40">
      <feGaussianBlur stdDeviation=".7" />
    </filter>
  </defs>
  {#each BARS as b, i (i)}
    {@const r = { x: b.x + half, y: b.y + half, width: b.w - STROKE, height: b.h - STROKE }}
    {#if b.lit}
      <rect class="halo" {...r} stroke-width={STROKE * 1.4} filter="url(#{id}-glow)" />
      <rect class="core" x={b.x + STROKE * 1.6} y={b.y + STROKE * 1.6} width={b.w - STROKE * 3.2} height={b.h - STROKE * 3.2} filter="url(#{id}-soft)" />
      <rect class="lit" {...r} stroke-width={STROKE} />
      <rect class="lit-in" x={b.x + STROKE * 0.72} y={b.y + STROKE * 0.72} width={b.w - STROKE * 1.44} height={b.h - STROKE * 1.44} stroke-width={STROKE * 0.32} />
    {:else}
      <rect class="ink-halo" {...r} stroke-width={STROKE} filter="url(#{id}-soft)" />
      <rect class="ink" {...r} stroke-width={STROKE} />
    {/if}
  {/each}
</svg>

<style>
  .logo { display: block; width: 100%; height: 100%; overflow: visible; }
  rect { fill: none; }
  .ink { stroke: var(--mark-ink); }
  .lit { stroke: var(--accent); }
  .halo { stroke: var(--accent); opacity: .85; }
  .core { fill: var(--accent); opacity: .32; }
  /* Dark: the outer bars carry a faint light of their own; the lit bar's inner line is lighter. */
  .ink-halo { stroke: var(--mark-ink); opacity: .22; }
  .lit-in { stroke: color-mix(in srgb, var(--accent) 55%, #ffffff); opacity: .5; }
  /* Light: ink bars are plain, and the cyan bar gets a darker inner line so it holds on white. */
  :global([data-theme='light']) .ink-halo { opacity: 0; }
  :global([data-theme='light']) .lit-in { stroke: color-mix(in srgb, var(--accent) 62%, #00323d); opacity: .9; }
  :global([data-theme='light']) .core { opacity: .38; }
  @media (forced-colors: active) {
    .ink, .lit { stroke: CanvasText; }
    .halo, .core, .ink-halo, .lit-in { display: none; }
  }
</style>
