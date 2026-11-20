<script lang="ts">
  // The emote at 1x to 4x in one row, each at its real pixel size, standing on one baseline with
  // its scale and pixel size under it (the row wraps on narrow screens; a size wider than the
  // screen shrinks to fit). `still` shows the first frame; with `still` null the reader's motion
  // setting decides, and before the script runs a <picture> source keeps them still under
  // reduced motion.
  import { ready, stillBy } from '$lib/emote';
  import { motion } from '$lib/shell/motion.svelte';
  import { scaleFiles, type PageEmote } from './emote-page';

  let { emote: e, still = null }: { emote: PageEmote; still?: boolean | null } = $props();

  const files = $derived(scaleFiles(e.files));
  const hasStill = $derived(e.animated && e.files.some((f) => f.name === '1x_static.webp'));
  const stillName = (scale: number) => `${e.host}/${scale}x_static.webp`;
  const moving = (name: string) => `${e.host}/${name}`;
</script>

{#if !ready(e)}
  <p class="wait">{e.state === 'APPROVED' && e.files.length === 0 ? 'The files for this emote are missing.' : 'The files are still being made. This takes a minute or so.'}</p>
{:else}
  <ul class="sizes" aria-label="Sizes">
    {#each files as f (f.name)}
      <li>
        <span class="cell" style:--w="{f.width}px" style:aspect-ratio="{f.width} / {f.height}">
          {#if hasStill && still === null && stillBy(motion.pref) === 'browser'}
            <picture>
              <source media="(prefers-reduced-motion: reduce)" srcset={stillName(f.scale)} />
              <img src={moving(f.name)} width={f.width} height={f.height} alt={f.scale === 4 ? e.name : ''} />
            </picture>
          {:else}
            <img src={hasStill && (still ?? stillBy(motion.pref) === 'still') ? stillName(f.scale) : moving(f.name)} width={f.width} height={f.height} alt={f.scale === 4 ? e.name : ''} />
          {/if}
        </span>
        <span class="cap"><span class="x">{f.scale}x</span><span class="px">{f.width} × {f.height}</span></span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  p { margin: 0; }
  .sizes { list-style: none; margin: 0; padding: 0; max-width: 100%; display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: center;
    gap: 24px clamp(20px, 3.4vw, 48px); }
  li { max-width: 100%; display: grid; justify-items: center; gap: 0; }
  .cell { display: grid; place-items: end center; max-width: 100%; width: min(var(--w), 100%); }
  .cell img { display: block; max-width: 100%; height: auto; }
  /* The baseline: a hairline under each size with a tick in the middle, then scale and pixels. */
  .cap { position: relative; display: grid; justify-items: center; gap: 1px; margin-top: 12px; padding-top: 10px; min-width: 64px;
    border-top: 1px solid var(--border-strong); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .cap::before { content: ''; position: absolute; top: -4px; left: 50%; width: 1px; height: 7px; background: var(--border-strong); }
  .x { font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .px { font-size: var(--t-fine); color: var(--muted); }
  .wait { color: var(--muted); font-size: var(--t-small); text-align: center; max-width: 32ch; margin-inline: auto; padding: 32px 0; }
</style>
