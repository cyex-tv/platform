<script lang="ts">
  import { stillNow } from '$lib/media.svelte';
  // Pick a layer or an effect. A sidebar of the studio's categories (All first, each with its
  // count) beside cards that show every option as a live preview: the option painted on a sample
  // name (a layer on its own, an effect over a sample gradient). Hovering or focusing a card plays
  // it. The search (name, tags, synonyms) looks across every category.
  import type { Engine } from '../engine/index.js';
  import { untrack } from 'svelte';
  import { searchItems } from '../anim';
  import { cached, drawAll, play, type LibKind } from '../thumbs';
  import Glyph from './Glyph.svelte';

  type Props = { PL: Engine; kind: LibKind | null; onpick: (id: string) => void; onclose: () => void };
  let { PL, kind, onpick, onclose }: Props = $props();
  let dialog: HTMLDialogElement;
  let query = $state('');
  let cat = $state('All');
  const SAMPLE = 'lumen_fox';

  const store = $derived(kind === 'effect' ? PL.EFFECTS : PL.LAYERS);
  const cats = $derived(kind === 'effect' ? PL.EFFECT_CATS : PL.LAYER_CATS);
  const all = $derived(Object.values(store).filter((d) => cats.includes(d.cat)));
  const counts = $derived(new Map(cats.map((c) => [c, all.filter((d) => d.cat === c).length])));
  const found = $derived(searchItems(PL, store, query).filter((d) => cats.includes(d.cat)));
  const grouped = $derived(
    query.trim()
      ? [{ cat: `${found.length} found`, items: found }]
      : cats.filter((c) => cat === 'All' || c === cat).map((c) => ({ cat: c, items: found.filter((d) => d.cat === c) })).filter((g) => g.items.length),
  );

  // Stills, drawn a few at a time for what is on screen; one card plays while hovered or focused.
  let stills = $state<Record<string, string | null>>({});
  let live = $state<{ id: string; url: string } | null>(null);
  let stopLive: (() => void) | null = null;
  const reduce = stillNow();
  $effect(() => {
    const k = kind;
    if (!k) return;
    const ids = grouped.flatMap((g) => g.items.map((d) => d.id)).filter((id) => cached(k, id) === undefined);
    const known: Record<string, string | null> = {};
    for (const g of grouped) for (const d of g.items) if (cached(k, d.id) !== undefined) known[d.id] = cached(k, d.id)!;
    untrack(() => (stills = { ...stills, ...known }));
    return drawAll(PL, k, ids, (id, url) => (stills[id] = url));
  });
  function start(id: string) {
    if (reduce || !kind || live?.id === id) return;
    stop();
    stopLive = play(PL, kind, id, (url) => (live = { id, url }));
  }
  function stop() {
    stopLive?.();
    stopLive = null;
    live = null;
  }
  const bg = (id: string) => {
    const url = live?.id === id ? live.url : stills[id];
    return url ? `background-image: url(${url})` : '';
  };

  $effect(() => {
    if (kind && !dialog.open) {
      query = '';
      cat = 'All';
      dialog.showModal();
    } else if (!kind && dialog.open) dialog.close();
  });

  let listEl = $state<HTMLDivElement>();
  function pickCat(c: string) {
    cat = c;
    query = '';
    listEl?.scrollTo({ top: 0 });
  }
</script>

<dialog class="ui-dialog ps-dialog ps-lib" bind:this={dialog} aria-labelledby="ps-lib-h" onclose={() => (stop(), onclose())}>
  <header>
    <h2 id="ps-lib-h">{kind === 'effect' ? 'Add an effect' : 'Add a layer'}</h2>
    <button type="button" class="ui-icon" aria-label="Close" title="Close" onclick={() => dialog.close()}><Glyph name="x" /></button>
  </header>
  <div class="ps-lib-body">
    <nav class="ps-lib-nav" aria-label="Categories">
      {#each ['All', ...cats] as c (c)}
        {@const on = cat === c && !query.trim()}
        <button type="button" class:on aria-current={on ? 'true' : undefined} onclick={() => pickCat(c)}>
          <span class="nm">{c}</span><span class="n">{c === 'All' ? all.length : counts.get(c)}</span>
        </button>
      {/each}
    </nav>
    <div class="scroll" bind:this={listEl}>
      <div class="ps-field">
        <label for="ps-lib-q">Search</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input id="ps-lib-q" class="ps-text" type="search" autocomplete="off" autofocus placeholder={kind === 'effect' ? 'glow, glitch, chrome…' : 'fire, stars, metal…'} bind:value={query} />
      </div>
      {#each grouped as g (g.cat)}
        <section class="ps-group" aria-label={g.cat}>
          <h3 class="ui-label">{g.cat}</h3>
          <div class="ps-cards">
            {#each g.items as d (d.id)}
              <button type="button" class="ps-card ps-libcard" onclick={() => (onpick(d.id), dialog.close())}
                onpointerenter={() => start(d.id)} onpointerleave={stop} onfocus={() => start(d.id)} onblur={stop}>
                <span class="ps-swatch" aria-hidden="true">
                  <span class="ps-painted" class:blank={!stills[d.id] && live?.id !== d.id} style={bg(d.id)}>{SAMPLE}</span>
                </span>
                <b>{d.name}</b>{#if d.desc}<span>{d.desc}</span>{/if}
              </button>
            {/each}
          </div>
        </section>
      {:else}
        <p class="ps-hint">Nothing matches “{query}”. Try another word.</p>
      {/each}
    </div>
  </div>
  <footer><button type="button" class="ui-btn secondary" onclick={() => dialog.close()}>Cancel</button></footer>
</dialog>

<style>
  .ps-lib { width: min(980px, calc(100vw - 24px)); }
  .ps-lib-body { display: grid; grid-template-columns: 200px minmax(0, 1fr); min-height: 0; }
  .ps-lib-nav { display: grid; align-content: start; gap: 2px; padding: 10px 8px; border-right: 1px solid var(--border); overflow-y: auto; }
  .ps-lib-nav button { display: flex; align-items: center; gap: 8px; min-height: 36px; padding: 4px 10px; border: 0; border-radius: var(--radius);
    background: none; font: inherit; font-size: var(--text-sub); color: var(--muted); text-align: left; cursor: pointer; }
  @media (hover: hover) and (pointer: fine) { .ps-lib-nav button:hover { color: var(--text); } }
  .ps-lib-nav button:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .ps-lib-nav button.on { font-weight: 700; color: var(--text); }
  .ps-lib-nav .nm { flex: 1; min-width: 0; }
  .ps-lib-nav .n { font-size: 12px; font-variant-numeric: tabular-nums; color: var(--dim); }
  .ps-lib-nav button.on .n { color: var(--accent-text); font-weight: 700; }
  .ps-lib .scroll { min-height: 0; }
  /* The preview: the option painted on a sample name, on the dark stage the studio previews on. */
  .ps-swatch { display: flex; align-items: center; justify-content: center; justify-self: stretch; width: 100%; height: 56px; margin-bottom: 6px; border: 0; border-radius: var(--radius); background: var(--stage-dark); overflow: hidden; }
  .ps-swatch .ps-painted { flex: none; align-self: center; height: auto; margin: auto; width: max-content; max-width: calc(100% - 12px); font-size: 26px; color: inherit; line-height: 1.2; letter-spacing: -.01em; background-size: 100% 100%; background-repeat: no-repeat; }
  .ps-swatch .ps-painted.blank { -webkit-text-fill-color: var(--dim); }
  @media (max-width: 640px) {
    .ps-lib-body { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); }
    .ps-lib-nav { display: flex; gap: 4px; padding: 6px 8px; border-right: 0; border-bottom: 1px solid var(--border); overflow-x: auto; overflow-y: hidden; }
    .ps-lib-nav button { flex: none; }
  }
</style>
