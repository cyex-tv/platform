<script lang="ts">
  // One engine parameter, by type: slider, colour, choice, switch, text, picture, stops or palette.
  import type { Engine, EnginePalette, EngineStop, ParamDef } from '../engine/index.js';
  import { alphaOf, opaque, round4, withAlpha } from '../format';
  import Colour from './Colour.svelte';
  import Glyph from './Glyph.svelte';
  import Slider from './Slider.svelte';
  import Stops from './Stops.svelte';

  type Props = { def: ParamDef; value: unknown; PL: Engine; onchange: (v: unknown) => void };
  let { def, value, PL, onchange }: Props = $props();
  const uid = $props.id();
  let pictureError = $state<string | null>(null);

  const toHexStops = (s: EngineStop[]) => s.map((x) => ({ at: x.at, color: withAlpha(x.color, x.a ?? 1) }));
  const fromHexStops = (s: { at: number; color: string }[]): EngineStop[] => s.map((x) => ({ at: x.at, color: opaque(x.color).toLowerCase(), a: round4(alphaOf(x.color)) }));

  // Pictures are kept as data URLs (at most 512 px a side) and decoded once into the engine's cache.
  async function pickPicture(file: File | undefined) {
    pictureError = null;
    if (!file) return;
    if (!file.type.startsWith('image/')) {
      pictureError = 'Choose a picture file (PNG, JPEG, GIF or WebP).';
      return;
    }
    try {
      const bmp = await createImageBitmap(file);
      const k = Math.min(1, 512 / Math.max(bmp.width, bmp.height));
      const w = Math.max(1, Math.round(bmp.width * k)), h = Math.max(1, Math.round(bmp.height * k));
      const cv = document.createElement('canvas');
      cv.width = w;
      cv.height = h;
      const ctx = cv.getContext('2d', { willReadFrequently: true })!;
      ctx.drawImage(bmp, 0, 0, w, h);
      bmp.close();
      const src = cv.toDataURL('image/png');
      PL.imageCache[src] = { w, h, data: ctx.getImageData(0, 0, w, h).data };
      onchange(src);
    } catch {
      pictureError = 'Couldn’t read that picture. Try another file.';
    }
  }

  const pal = $derived(def.t === 'palette' ? (value as EnginePalette) : null);
  function setPal(patch: Partial<EnginePalette>) {
    if (pal) onchange({ ...pal, ...patch });
  }
  function setColour(i: number, patch: Partial<EnginePalette['colors'][number]>) {
    if (pal) setPal({ colors: pal.colors.map((c, j) => (j === i ? { ...c, ...patch } : c)) });
  }
</script>

{#if def.t === 'range'}
  <Slider label={def.l} min={def.min} max={def.max} step={def.step} value={Number(value)} hint={def.hint} onchange={onchange} />
{:else if def.t === 'color'}
  <Colour label={def.l} alpha={false} value={String(value).toUpperCase()} onchange={(c) => onchange(c.toLowerCase())} />
{:else if def.t === 'select'}
  <div class="ps-field">
    <label for="{uid}-s">{def.l}</label>
    <select id="{uid}-s" class="ui-input" value={String(value)} onchange={(e) => onchange(e.currentTarget.value)}>
      {#each def.o as [k, label] (k)}<option value={k}>{label}</option>{/each}
    </select>
  </div>
{:else if def.t === 'bool'}
  <label class="ui-check"><input class="ui-switch" type="checkbox" role="switch" checked={Boolean(value)} onchange={(e) => onchange(e.currentTarget.checked)} /> {def.l}</label>
{:else if def.t === 'text'}
  <div class="ps-field">
    <label for="{uid}-t">{def.l}</label>
    <input id="{uid}-t" class="ps-text" type="text" maxlength="60" value={String(value ?? '')} oninput={(e) => onchange(e.currentTarget.value)} />
  </div>
{:else if def.t === 'image'}
  <div class="ps-field">
    <label for="{uid}-f">{def.l}</label>
    {#if value}<div class="ps-imgbox"><img src={String(value)} alt="Chosen for this layer" /></div>{/if}
    <input id="{uid}-f" type="file" accept="image/png,image/jpeg,image/gif,image/webp" onchange={(e) => { const f = e.currentTarget.files?.[0]; e.currentTarget.value = ''; pickPicture(f); }} />
    {#if pictureError}<p class="ps-msg" role="alert">{pictureError}</p>{:else}<p class="ps-hint">Large pictures are scaled to 512 px. The picture stays in this paint.</p>{/if}
  </div>
{:else if def.t === 'stops'}
  <Stops label={def.l} stops={toHexStops(value as EngineStop[])} min={1} max={24} onchange={(s) => onchange(fromHexStops(s))} />
{:else if def.t === 'palette' && pal}
  <div class="ps-group" role="group" aria-label={def.l}>
    <div class="ps-rowhead"><strong>{def.l}</strong><span class="ps-count">{pal.colors.length}</span></div>
    <div class="ps-bar" aria-hidden="true"><span style:background-image="linear-gradient(90deg, {PL.paletteToStops(pal).map((s) => `${withAlpha(s.color, s.a)} ${s.at * 100}%`).join(', ')})"></span></div>
    {#each pal.colors as c, i (i)}
      <div class="ps-rowcard">
        <div class="ps-rowhead">
          <strong>Colour {i + 1}</strong>
          <button type="button" class="ui-icon" aria-label="Remove colour {i + 1}" title="Remove" disabled={pal.colors.length <= 1} onclick={() => setPal({ colors: pal.colors.filter((_, j) => j !== i) })}><Glyph name="trash" /></button>
        </div>
        <Colour label="Colour {i + 1}" hideLabel value={withAlpha(c.c, c.a ?? 1)} onchange={(h) => setColour(i, { c: opaque(h).toLowerCase(), a: round4(alphaOf(h)) })} />
        <Slider label="Strength" min={0} max={4} step={0.05} value={c.w} hint="Its share of the colours" onchange={(w) => setColour(i, { w })} />
      </div>
    {/each}
    <div class="ui-actions">
      <button type="button" class="ui-btn secondary small" disabled={pal.colors.length >= 24} onclick={() => setPal({ colors: [...pal.colors, { ...pal.colors[pal.colors.length - 1] }] })}><Glyph name="plus" /> Add colour</button>
    </div>
    <Slider label="Smoothness" min={0} max={1} step={0.01} value={pal.blend} hint="0 gives hard bands, 1 smooth blends" onchange={(blend) => setPal({ blend })} />
    <label class="ui-check"><input class="ui-switch" type="checkbox" role="switch" checked={pal.loop} onchange={(e) => setPal({ loop: e.currentTarget.checked })} /> Loop the last colour back into the first</label>
  </div>
{/if}
