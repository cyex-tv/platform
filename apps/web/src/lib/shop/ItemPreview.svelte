<script lang="ts">
  // What an item looks like on the buyer's own name, for the inspector: each paint on the dark and
  // the light chat ground side by side and as a chat line, in one hairline frame (no gaps, no
  // boxes); then every badge at its sizes on one baseline (1x, 2x, 3x, like the emote page's
  // sizes) with the pixel size in mono. `full` off shows the first paint only.
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import type { Badge, Paint } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import Sample from './Sample.svelte';

  type Props = { badges: Badge[]; paints: Paint[]; name: string; full?: boolean; uid?: string };
  let { badges, paints, name, full = false, uid = 'pv' }: Props = $props();

  const badge = $derived(badges[0] ?? null);
  const looks = $derived(full ? (paints.length ? paints : [null]) : [paints[0] ?? null]);
  const SIZES = [
    { x: 1, px: 18 },
    { x: 2, px: 36 },
    { x: 3, px: 54 },
  ];
</script>

<div class="pv">
  {#each looks as p (p?.id ?? 'none')}
    <div class="look">
      {#if full && p && looks.length > 1}<span class="cap"><bdi>{clean(p.name)}</bdi></span>{/if}
      <div class="frame">
        <Sample {name} paint={p} {badge} theme="dark" tag="Dark" label="{p ? clean(p.name) : 'Your name'} on the dark chat ground" />
        <Sample {name} paint={p} {badge} theme="light" tag="Light" label="{p ? clean(p.name) : 'Your name'} on the light chat ground" />
        <div class="line"><Sample {name} paint={p} {badge} line label="Your name in a chat line" /></div>
      </div>
    </div>
  {/each}

  {#if full && badges.length}
    <section class="badges" aria-labelledby="{uid}-sizes">
      <h3 class="label" id="{uid}-sizes">Badge sizes<span class="n">{badges.length}</span></h3>
      {#each badges as b (b.id)}
        <div class="badge">
          <p class="bn"><bdi>{clean(b.name)}</bdi>{#if b.tooltip && b.tooltip !== b.name}<span class="sub"><bdi>{clean(b.tooltip)}</bdi></span>{/if}</p>
          <ul class="strip" aria-label="{clean(b.name)} at every size">
            {#each SIZES as s (s.x)}
              <li><span class="px"><BadgeIcon badge={b} size={s.px} label={s.x === 1} /></span><span class="x">{s.x}x</span><span class="dm">{s.px}×{s.px}</span></li>
            {/each}
          </ul>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  p, h3 { margin: 0; }
  .pv { display: grid; gap: 16px; min-width: 0; }
  .look { display: grid; gap: 6px; min-width: 0; }
  .cap { font-size: var(--t-fine); font-weight: 600; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* One frame: dark | light on top, the chat line under them, hairlines between. */
  .frame { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); border: 1px solid var(--border); }
  .frame > :global(.sample:nth-child(2)) { border-left: 1px solid var(--border); }
  .line { grid-column: 1 / -1; border-top: 1px solid var(--border); }
  .label { display: flex; align-items: center; gap: 8px; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .n { font-family: var(--font-mono); letter-spacing: 0; font-variant-numeric: tabular-nums; }
  .badges { display: grid; gap: 10px; }
  .badge { display: grid; gap: 6px; }
  .bn { display: flex; align-items: baseline; gap: 8px; min-width: 0; font-size: var(--t-small); font-weight: 600; }
  .bn bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { min-width: 0; font-size: var(--t-fine); font-weight: 400; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* The sizes on one baseline, cells sharing hairlines, as on the emote page. */
  .strip { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); border: 1px solid var(--border); }
  .strip li { display: grid; justify-items: center; align-content: end; gap: 2px; min-height: 108px; padding: 10px 4px 8px; }
  .strip li + li { border-left: 1px solid var(--border); }
  .px { display: grid; place-items: end center; height: 58px; }
  .x { margin-top: 6px; font-size: var(--t-small); font-weight: 600; }
  .dm { font-family: var(--font-mono); font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
</style>
