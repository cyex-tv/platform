<script lang="ts">
  // Pick a layer or an effect: search (name, tags, synonyms) and categories.
  import type { Engine } from '../engine/index.js';
  import { searchItems } from '../anim';
  import Glyph from './Glyph.svelte';

  type Props = { PL: Engine; kind: 'layer' | 'effect' | null; onpick: (id: string) => void; onclose: () => void };
  let { PL, kind, onpick, onclose }: Props = $props();
  let dialog: HTMLDialogElement;
  let query = $state('');

  const store = $derived(kind === 'effect' ? PL.EFFECTS : PL.LAYERS);
  const cats = $derived(kind === 'effect' ? PL.EFFECT_CATS : PL.LAYER_CATS);
  const found = $derived(searchItems(PL, store, query));
  const grouped = $derived(
    query.trim() ? [{ cat: `${found.length} found`, items: found }] : cats.map((cat) => ({ cat, items: found.filter((d) => d.cat === cat) })).filter((g) => g.items.length),
  );

  $effect(() => {
    if (kind && !dialog.open) {
      query = '';
      dialog.showModal();
    } else if (!kind && dialog.open) dialog.close();
  });
</script>

<dialog class="ui-dialog ps-dialog" bind:this={dialog} aria-labelledby="ps-lib-h" onclose={onclose}>
  <header>
    <h2 id="ps-lib-h">{kind === 'effect' ? 'Add an effect' : 'Add a layer'}</h2>
    <button type="button" class="ui-icon" aria-label="Close" title="Close" onclick={() => dialog.close()}><Glyph name="x" /></button>
  </header>
  <div class="scroll">
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
            <button type="button" class="ps-card" onclick={() => (onpick(d.id), dialog.close())}><b>{d.name}</b>{#if d.desc}<span>{d.desc}</span>{/if}</button>
          {/each}
        </div>
      </section>
    {:else}
      <p class="ps-hint">Nothing matches “{query}”. Try another word.</p>
    {/each}
  </div>
  <footer><button type="button" class="ui-btn secondary" onclick={() => dialog.close()}>Cancel</button></footer>
</dialog>
