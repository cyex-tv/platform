<script lang="ts">
  // The facts row: type, formats, size at 1x, frames (when known), added date, file sizes and,
  // once the API sends it, how many channels use it.
  import { size } from '$lib/emote';
  import { bytes, count, date } from '$lib/format';
  import { formats, kind, scaleFiles, type PageEmote } from './emote-page';

  type Props = { emote: PageEmote; channels?: number | null; frames?: number | null };
  let { emote: e, channels = null, frames = null }: Props = $props();

  const one = $derived(size(e));
  const files = $derived(scaleFiles(e.files));
  const kinds = $derived(formats(e.files));
</script>

<section class="facts-block" aria-labelledby="facts-h">
  <h2 id="facts-h" class="sr-only">Facts</h2>
  <dl class="facts">
    {#if channels !== null}
      <div class="wide-fact"><dt>Channels using it</dt><dd class="figure">{count(channels)}</dd></div>
    {/if}
    <div><dt>Type</dt><dd>{kind(e)}</dd></div>
    {#if kinds.length}<div><dt>{kinds.length > 1 ? 'Formats' : 'Format'}</dt><dd>{kinds.join(', ')}</dd></div>{/if}
    {#if files.length}<div><dt>Size at 1x</dt><dd>{one.width} × {one.height} px</dd></div>{/if}
    {#if frames}<div><dt>Frames</dt><dd>{count(frames)}</dd></div>{/if}
    <div><dt>Added</dt><dd><time datetime={e.createdAt}>{date(e.createdAt)}</time></dd></div>
    {#if e.tags.length}<div><dt>Tags</dt><dd>{e.tags.join(', ')}</dd></div>{/if}
    {#if files.length}
      <div class="wide-fact">
        <dt>File sizes</dt>
        <dd>
          <ul class="files">
            {#each files as f (f.name)}<li><span class="x">{f.scale}x</span> {bytes(f.size)}</li>{/each}
          </ul>
        </dd>
      </div>
    {/if}
  </dl>
</section>

<style>
  .facts { margin: 0; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 20px; border-top: 1px solid var(--border); }
  .facts > div { display: grid; gap: 4px; padding: 14px 0; border-bottom: 1px solid var(--border); min-width: 0; }
  .wide-fact { grid-column: 1 / -1; }
  dt { color: var(--muted); font-size: var(--t-fine); }
  dd { margin: 0; font-size: var(--t-ui); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  .figure { font-size: var(--t-h3-sub); font-weight: 500; letter-spacing: -.03em; line-height: 1.1; }
  .files { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 18px; }
  .x { color: var(--muted); }
</style>
