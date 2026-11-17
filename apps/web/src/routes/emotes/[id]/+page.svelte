<script lang="ts">
  import { ready, src } from '$lib/emote';
  import { bytes, date } from '$lib/format';
  import EmoteActions from '$lib/components/EmoteActions.svelte';

  let { data } = $props();
  const e = $derived(data.emote);
  const scales = [1, 2, 3, 4] as const;
  const webp = $derived(e.files.filter((f) => f.format === 'WEBP' && !f.isStatic).sort((a, b) => a.scale - b.scale));
</script>

<svelte:head>
  <title>{e.name} - cyex.tv</title>
  <meta name="description" content="{e.name}, an emote on cyex.tv{e.owner ? ` by ${e.owner.displayName}` : ''}." />
  {#if ready(e)}<meta property="og:image" content={src(e, 4, true)} />{/if}
</svelte:head>

<div class="page">
  <div class="layout">
    <div class="stage" aria-label="Preview">
      {#if ready(e)}
        <img class="big" src={src(e, 4)} alt={e.name} />
      {:else}
        <p class="muted">The files are still being made.</p>
      {/if}
    </div>

    <div class="info">
      <h1>{e.name}</h1>
      {#if e.owner}<p class="lead by">By {e.owner.displayName}</p>{/if}

      {#if ready(e)}
        <div class="sizes" aria-label="Sizes">
          {#each scales as s}
            <figure>
              <span class="cell"><img src={src(e, s)} alt="" loading="lazy" /></span>
              <figcaption class="fine">{s}x</figcaption>
            </figure>
          {/each}
        </div>
      {/if}

      <dl class="facts">
        <div><dt>Added</dt><dd>{date(e.createdAt)}</dd></div>
        <div><dt>Kind</dt><dd>{e.animated ? 'Animated' : 'Still'}{e.flags.zeroWidth ? ', zero-width' : ''}</dd></div>
        {#if e.tags.length}<div><dt>Tags</dt><dd>{e.tags.join(', ')}</dd></div>{/if}
        {#if webp.length}
          <div><dt>Files</dt><dd>{webp.map((f) => `${f.scale}x ${f.width}×${f.height} ${bytes(f.size)}`).join(' · ')}</dd></div>
        {/if}
      </dl>

      {#if e.state === 'APPROVED'}<EmoteActions emoteId={e.id} name={e.name} />{/if}
    </div>
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 40px 72px;
    padding-top: clamp(32px, 5vw, 64px); align-items: start; }
  .stage { display: grid; place-items: center; min-height: 320px; padding: 40px; border-radius: var(--radius-l);
    border: 1px solid var(--border); background: var(--surface); }
  .big { max-width: 100%; height: auto; max-height: 256px; }
  h1 { font-size: var(--t-h2); line-height: 1.02; letter-spacing: -.04em; overflow-wrap: anywhere; }
  .by { margin-top: 12px; }
  .sizes { display: flex; align-items: flex-end; gap: 20px; margin-top: 32px; overflow-x: auto; padding-bottom: 4px; }
  figure { margin: 0; display: grid; justify-items: center; gap: 6px; }
  .cell { display: grid; place-items: center; min-width: 40px; min-height: 40px; }
  .facts { margin: 32px 0 0; border-top: 1px solid var(--border); }
  .facts > div { display: grid; grid-template-columns: 96px minmax(0, 1fr); gap: 16px; padding: 14px 0; border-bottom: 1px solid var(--border); }
  dt { color: var(--muted); font-size: var(--t-ui); }
  dd { margin: 0; font-size: var(--t-ui); font-variant-numeric: tabular-nums; }
  @media (max-width: 900px) { .layout { grid-template-columns: minmax(0, 1fr); } }
</style>
