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
  const added = $derived(date(e.createdAt));
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
    {#if added}<div><dt>Added</dt><dd><time datetime={e.createdAt}>{added}</time></dd></div>{/if}
    {#if e.tags.length}<div class="wide-fact"><dt>Tags</dt><dd>{e.tags.join(', ')}</dd></div>{/if}
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
  .facts { margin: -10px 0 0; display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 20px; }
  .facts > div { display: grid; align-content: start; gap: 2px; padding: 10px 0; border-bottom: 1px solid var(--border); min-width: 0; }
  .wide-fact { grid-column: 1 / -1; }
  dt { color: var(--muted); font-size: var(--t-label); }
  dd { margin: 0; font-size: var(--t-small); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  .figure { font-size: 22px; font-weight: 600; letter-spacing: -.02em; line-height: 1.2; }
  .files { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 4px 18px; }
  .x { color: var(--muted); }
</style>
