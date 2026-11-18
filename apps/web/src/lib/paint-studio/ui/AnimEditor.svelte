<script lang="ts">
  // The animated paint builder: presets, size and timing, layers (drawn bottom to top) and effects
  // (applied in order), and the selected item's settings. Edits the document in place.
  import type { AnimDoc, Engine, Effect, Layer, ParamDef } from '../engine/index.js';
  import { FPS, SIZES, cachePictures, sizeKey } from '../anim';
  import Glyph from './Glyph.svelte';
  import Library from './Library.svelte';
  import Param from './Param.svelte';
  import Slider from './Slider.svelte';

  type Props = { PL: Engine; doc: AnimDoc; onreplace: (d: AnimDoc) => void };
  let { PL, doc = $bindable(), onreplace }: Props = $props();

  const KINDS = ['layer', 'effect'] as const;
  let sel = $state<{ kind: 'layer' | 'effect'; id: string } | null>(null);
  let adding = $state<'layer' | 'effect' | null>(null);
  const presetNames = $derived(PL.ANIM_PRESETS.map((f, i) => ({ i, name: f().name, scene: i < PL.SCENE_COUNT })));

  const layer = $derived(sel?.kind === 'layer' ? doc.layers.find((l) => l.id === sel!.id) ?? null : null);
  const effect = $derived(sel?.kind === 'effect' ? doc.effects.find((e) => e.id === sel!.id) ?? null : null);
  const item = $derived<Layer | Effect | null>(layer ?? effect);
  const def = $derived(item ? (layer ? PL.LAYERS[item.type] : PL.EFFECTS[item.type]) : null);

  // The selected item's params in their groups, hiding those whose `show` says no.
  const groups = $derived.by(() => {
    if (!def || !item) return [];
    const out: { name: string; params: ParamDef[] }[] = [];
    for (const p of def.params) {
      if (p.show && !p.show(item.params)) continue;
      const g = p.g ?? 'Settings';
      let grp = out.find((x) => x.name === g);
      if (!grp) out.push((grp = { name: g, params: [] }));
      grp.params.push(p);
    }
    return out;
  });

  async function usePreset(i: number) {
    const d = PL.Engine.sanitizeDoc(PL.ANIM_PRESETS[i]()).doc;
    d.width = doc.width;
    d.height = doc.height;
    await cachePictures(PL, d);
    sel = null;
    onreplace(d);
  }
  function surprise() {
    const d = PL.Engine.sanitizeDoc(PL.Surprise.make()).doc;
    d.width = doc.width;
    d.height = doc.height;
    sel = null;
    onreplace(d);
  }

  function add(id: string) {
    if (adding === 'layer') {
      const l = PL.Engine.newLayer(id);
      doc.layers.push(l);
      sel = { kind: 'layer', id: l.id };
    } else {
      const e = PL.Engine.newEffect(id);
      doc.effects.push(e);
      sel = { kind: 'effect', id: e.id };
    }
  }
  function listOf(kind: 'layer' | 'effect') {
    return (kind === 'layer' ? doc.layers : doc.effects) as (Layer | Effect)[];
  }
  // `d` is a step in the list as shown: layers are shown top first, so up means later in the array.
  function move(kind: 'layer' | 'effect', id: string, d: number) {
    const list = listOf(kind);
    const i = list.findIndex((x) => x.id === id);
    const j = kind === 'layer' ? i - d : i + d;
    if (i < 0 || j < 0 || j >= list.length) return;
    const [x] = list.splice(i, 1);
    list.splice(j, 0, x);
  }
  function remove(kind: 'layer' | 'effect', id: string) {
    const list = listOf(kind);
    const i = list.findIndex((x) => x.id === id);
    if (i >= 0) list.splice(i, 1);
    if (sel?.id === id) sel = null;
  }
  function duplicate(kind: 'layer' | 'effect', id: string) {
    const list = listOf(kind);
    const i = list.findIndex((x) => x.id === id);
    if (i < 0) return;
    const copy = PL.clone(list[i]);
    copy.id = `${kind[0]}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`;
    copy.name = `${list[i].name} copy`;
    list.splice(i + 1, 0, copy);
    sel = { kind, id: copy.id };
  }
  function keyMove(e: KeyboardEvent, kind: 'layer' | 'effect', id: string) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    move(kind, id, e.key === 'ArrowUp' ? -1 : 1);
    const target = e.currentTarget as HTMLElement;
    requestAnimationFrame(() => target.focus());
  }
  const shownLayers = $derived([...doc.layers].reverse());

  function setSize(v: string) {
    const s = SIZES.find((x) => x.value === v);
    if (s) {
      doc.width = s.width;
      doc.height = s.height;
    }
  }
</script>

<section class="ps-panel" aria-labelledby="ps-apresets-h">
  <header>
    <h2 id="ps-apresets-h">Start from</h2>
    <button type="button" class="ui-btn ghost small" onclick={surprise}><Glyph name="shuffle" /> Surprise me</button>
  </header>
  <div class="ps-body">
    <div class="ps-field">
      <label for="ps-apreset">Preset</label>
      <select id="ps-apreset" class="ui-input" value="" onchange={(e) => e.currentTarget.value !== '' && usePreset(Number(e.currentTarget.value))}>
        <option value="" disabled>Choose a preset…</option>
        <optgroup label="Scenes">
          {#each presetNames.filter((p) => p.scene) as p (p.i)}<option value={p.i}>{p.name}</option>{/each}
        </optgroup>
        <optgroup label="Effects">
          {#each presetNames.filter((p) => !p.scene) as p (p.i)}<option value={p.i}>{p.name}</option>{/each}
        </optgroup>
      </select>
      <p class="ps-hint">A preset replaces the layers and effects. Every animation loops without a seam.</p>
    </div>
    <div class="ps-row">
      <div class="ps-field" style:flex="1 1 200px">
        <label for="ps-size">Size</label>
        <select id="ps-size" class="ui-input" value={sizeKey(doc)} onchange={(e) => setSize(e.currentTarget.value)}>
          {#each SIZES as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
          {#if !SIZES.some((s) => s.value === sizeKey(doc))}<option value={sizeKey(doc)}>{doc.width} × {doc.height}</option>{/if}
        </select>
      </div>
      <div class="ps-field" style:flex="1 1 120px">
        <label for="ps-fps">Frames a second</label>
        <select id="ps-fps" class="ui-input" value={String(doc.fps)} onchange={(e) => (doc.fps = Number(e.currentTarget.value))}>
          {#each FPS as f (f)}<option value={String(f)}>{f}</option>{/each}
        </select>
      </div>
    </div>
    <Slider label="Loop length" unit="s" min={0.4} max={8} step={0.1} value={doc.duration} hint="{Math.max(1, Math.round(doc.duration * doc.fps))} frames" onchange={(v) => (doc.duration = v)} />
  </div>
</section>

{#each KINDS as kind (kind)}
  {@const list = kind === 'layer' ? shownLayers : doc.effects}
  <section class="ps-panel" aria-labelledby="ps-{kind}s-h">
    <header>
      <h2 id="ps-{kind}s-h">{kind === 'layer' ? 'Layers' : 'Effects'}</h2>
      <span class="ps-count">{list.length}</span>
      <button type="button" class="ui-btn secondary small" onclick={() => (adding = kind)}><Glyph name="plus" /> {kind === 'layer' ? 'Add layer' : 'Add effect'}</button>
    </header>
    {#if list.length}
      <ul class="ps-list" aria-label={kind === 'layer' ? 'Layers, top first' : 'Effects, in order'}>
        {#each list as it, i (it.id)}
          <li class="ps-item" class:hidden={!it.visible} aria-current={sel?.id === it.id ? 'true' : undefined}>
            <button type="button" class="ps-pick" aria-pressed={sel?.id === it.id} onclick={() => (sel = { kind, id: it.id })} onkeydown={(e) => keyMove(e, kind, it.id)}>
              <Glyph name={kind === 'layer' ? 'layers' : 'wand'} />
              <span>{it.name}</span>
              {#if it.name !== (kind === 'layer' ? PL.LAYERS : PL.EFFECTS)[it.type]?.name}<small>{(kind === 'layer' ? PL.LAYERS : PL.EFFECTS)[it.type]?.name}</small>{/if}
            </button>
            <button type="button" class="ui-icon" aria-label="{it.visible ? 'Hide' : 'Show'} {it.name}" title={it.visible ? 'Hide' : 'Show'} aria-pressed={!it.visible} onclick={() => (it.visible = !it.visible)}><Glyph name={it.visible ? 'eye' : 'eye-off'} /></button>
            <button type="button" class="ui-icon" aria-label="Move {it.name} up" title="Move up (Alt+Up)" disabled={i === 0} onclick={() => move(kind, it.id, -1)}><Glyph name="up" /></button>
            <button type="button" class="ui-icon" aria-label="Move {it.name} down" title="Move down (Alt+Down)" disabled={i === list.length - 1} onclick={() => move(kind, it.id, 1)}><Glyph name="down" /></button>
            <button type="button" class="ui-icon" aria-label="Duplicate {it.name}" title="Duplicate" onclick={() => duplicate(kind, it.id)}><Glyph name="copy" /></button>
            <button type="button" class="ui-icon" aria-label="Remove {it.name}" title="Remove" onclick={() => remove(kind, it.id)}><Glyph name="trash" /></button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="ps-body"><p class="ps-hint">{kind === 'layer' ? 'No layers yet. Add one, or start from a preset.' : 'No effects. They change everything under them, in order.'}</p></div>
    {/if}
  </section>
{/each}

{#if item && def}
  <section class="ps-panel" aria-labelledby="ps-item-h">
    <header>
      <h2 id="ps-item-h">{item.name}</h2>
      <button type="button" class="ui-icon" aria-label="Close settings" title="Close" onclick={() => (sel = null)}><Glyph name="x" /></button>
    </header>
    <div class="ps-body">
      {#if def.desc}<p class="ps-hint">{def.desc}</p>{/if}
      <div class="ps-field">
        <label for="ps-iname">Name</label>
        <input id="ps-iname" class="ps-text" type="text" maxlength="64" bind:value={item.name} />
      </div>
      {#if layer}
        <div class="ps-row">
          <div class="ps-field" style:flex="1 1 180px">
            <label for="ps-blend">Blend</label>
            <select id="ps-blend" class="ui-input" bind:value={layer.blend}>
              {#each PL.BLENDS as [k, label] (k)}<option value={k}>{label}</option>{/each}
            </select>
          </div>
        </div>
        <Slider label="Opacity" unit="%" min={0} max={100} step={1} value={Math.round(layer.opacity * 100)} onchange={(v) => (layer.opacity = v / 100)} />
      {/if}
      {#each groups as g (g.name)}
        <div class="ps-group">
          <h3 class="ui-label">{g.name}</h3>
          {#each g.params as p (p.k)}
            <Param def={p} {PL} value={item.params[p.k]} onchange={(v) => (item.params[p.k] = v)} />
          {/each}
        </div>
      {/each}
      {#if layer && !PL.LAYERS[layer.type]?.draw}
        <div class="ps-group">
          <h3 class="ui-label">Move and place</h3>
          <Slider label="Scale" min={0.1} max={6} step={0.05} value={layer.xf.scale} onchange={(v) => (layer.xf.scale = v)} />
          <Slider label="Rotate" unit="°" min={-180} max={180} step={1} value={layer.xf.rotate} onchange={(v) => (layer.xf.rotate = v)} />
          <Slider label="Scroll across" min={-8} max={8} step={1} value={layer.xf.sx} hint="Whole trips per loop, so it stays seamless" onchange={(v) => (layer.xf.sx = v)} />
          <Slider label="Scroll down" min={-8} max={8} step={1} value={layer.xf.sy} onchange={(v) => (layer.xf.sy = v)} />
          <Slider label="Spin" min={-8} max={8} step={1} value={layer.xf.spin} hint="Turns per loop" onchange={(v) => (layer.xf.spin = v)} />
        </div>
      {/if}
      {#if layer}
        <div class="ps-group">
          <h3 class="ui-label">Colour</h3>
          <Slider label="Hue" unit="°" min={-180} max={180} step={1} value={layer.tweak.hue} onchange={(v) => (layer.tweak.hue = v)} />
          <Slider label="Saturation" min={0} max={2} step={0.01} value={layer.tweak.sat} onchange={(v) => (layer.tweak.sat = v)} />
          <Slider label="Brightness" min={-0.5} max={0.5} step={0.01} value={layer.tweak.bright} onchange={(v) => (layer.tweak.bright = v)} />
          <Slider label="Contrast" min={0.2} max={2} step={0.01} value={layer.tweak.contrast} onchange={(v) => (layer.tweak.contrast = v)} />
        </div>
      {/if}
    </div>
  </section>
{/if}

<Library {PL} kind={adding} onpick={add} onclose={() => (adding = null)} />
