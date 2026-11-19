<script lang="ts">
  /**
   * The cyex.tv mark (the owner's logo): three outlined square bars on the 32 grid, the middle one
   * offset right and lit in the accent with a soft glow (dark: light-grey bars and the blue lit
   * bar; light: dark bars and the cyan lit bar with a deeper inner line). Inline SVG, themed by
   * tokens. Crisp from 16 to 32px: under 20px the outline is heavier and the glow is left out
   * (it would only blur the bars). `lit` lights one bar only, for section eyebrows (the other two
   * go quiet); `glow={false}` turns the glow off at any size. Decorative: the wordmark or a label
   * next to it is the name.
   */
  type Bar = 'top' | 'middle' | 'bottom';
  let { size = 22, lit = 'all', glow = true }: { size?: number; lit?: 'all' | Bar; glow?: boolean } = $props();

  const uid = $props.id();
  const small = $derived(size < 20);
  const s = $derived(small ? 2 : 1.4); // outline width in grid units
  const glowing = $derived(glow && !small && (lit === 'all' || lit === 'middle'));
  const BARS: Record<Bar, { x: number; y: number; w: number }> = {
    top: { x: 3, y: 5, w: 15 },
    middle: { x: 10, y: 13, w: 18 },
    bottom: { x: 3, y: 21, w: 15 },
  };
  const H = 6;
  /** The outline's rect: inset by half the stroke so the bar keeps its outer size. */
  const box = (b: Bar, inset = s / 2) => ({ x: BARS[b].x + inset, y: BARS[b].y + inset, width: BARS[b].w - inset * 2, height: H - inset * 2 });
  const tone = (b: Bar) => (lit === 'all' || lit === b ? (b === 'middle' ? 'mid' : 'ink') : 'off');
</script>

<svg class="mark" viewBox="1 3 29 26" width={size} height={Math.round((size * 26) / 29)} aria-hidden="true" focusable="false">
  {#if glowing}
    <defs>
      <filter id="{uid}-halo" filterUnits="userSpaceOnUse" x="0" y="2" width="32" height="28">
        <feGaussianBlur stdDeviation="1.1" />
      </filter>
      <filter id="{uid}-soft" filterUnits="userSpaceOnUse" x="0" y="2" width="32" height="28">
        <feGaussianBlur stdDeviation=".75" />
      </filter>
    </defs>
  {/if}
  {#each ['top', 'bottom'] as const as b (b)}
    {#if glowing && tone(b) === 'ink'}<rect class="ink-glow" {...box(b, s + 0.4)} filter="url(#{uid}-soft)" />{/if}
    <rect class={tone(b)} {...box(b)} stroke-width={s} />
  {/each}
  {#if glowing}
    <rect class="halo" {...box('middle')} stroke-width={s + 0.6} filter="url(#{uid}-halo)" />
    <rect class="mid-fill" {...box('middle', s + 0.5)} filter="url(#{uid}-soft)" />
  {/if}
  <rect class={tone('middle')} {...box('middle')} stroke-width={s} />
  {#if glowing}<rect class="mid-core" {...box('middle')} stroke-width={s * 0.34} />{/if}
</svg>

<style>
  .mark { display: block; flex: none; overflow: visible; }
  rect { fill: none; }
  .ink { stroke: var(--mark-ink); }
  .mid { stroke: var(--accent); }
  .off { stroke: var(--border-strong); }
  .ink-glow { fill: var(--mark-ink); stroke: none; opacity: .2; }
  .halo { stroke: var(--accent); opacity: .9; }
  .mid-fill { fill: var(--mark-lit); stroke: none; opacity: .85; }
  .mid-core { stroke: var(--mark-core); }
  /* Light: dark bars, no glow inside them; the cyan bar gets a deeper inner line. */
  :global([data-theme='light']) .ink-glow { display: none; }
  :global([data-theme='light']) .halo { opacity: .75; }
  @media (prefers-contrast: more) { .halo, .ink-glow, .mid-fill { display: none; } }
  @media (forced-colors: active) {
    .ink, .mid, .off { stroke: CanvasText; }
    .halo, .ink-glow, .mid-fill, .mid-core { display: none; }
  }
</style>
