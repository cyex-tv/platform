<script lang="ts">
  // A picked file before it is uploaded, as cyex.tv will show it: 1x, 2x, 3x and 4x at their real pixel
  // sizes (1x is 32 px tall, as the worker makes it) standing on one baseline with the scale and
  // size under each, the same plain row as the emote page. No ground or chat switch.
  type Props = { src: string; width: number; height: number; name: string };
  let { src, width, height, name }: Props = $props();

  const ratio = $derived(Math.min(4, Math.max(0.25, width / Math.max(1, height))));
  const at = (h: number) => ({ w: Math.round(h * ratio), h });
</script>

<ul class="sizes" aria-label="Sizes">
  {#each [1, 2, 3, 4] as k (k)}
    {@const s = at(32 * k)}
    <li>
      <span class="cell" style:--w="{s.w}px" style:aspect-ratio="{s.w} / {s.h}">
        <img {src} width={s.w} height={s.h} alt={k === 4 ? name : ''} />
      </span>
      <span class="cap"><span class="x">{k}x</span><span class="px">{s.w} × {s.h}</span></span>
    </li>
  {/each}
</ul>

<style>
  .sizes { list-style: none; margin: 0; padding: 0; max-width: 100%; min-width: 0; display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: flex-start;
    gap: 24px clamp(20px, 3.4vw, 48px); }
  li { max-width: 100%; display: grid; justify-items: center; gap: 0; }
  .cell { display: grid; place-items: end center; max-width: 100%; width: min(var(--w), 100%); }
  .cell img { display: block; max-width: 100%; height: auto; }
  .cap { position: relative; display: grid; justify-items: center; gap: 1px; margin-top: 12px; padding-top: 10px; min-width: 64px;
    border-top: 1px solid var(--border-strong); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .cap::before { content: ''; position: absolute; top: -4px; left: 50%; width: 1px; height: 7px; background: var(--border-strong); }
  .x { font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .px { font-size: var(--t-fine); color: var(--muted); }
</style>
