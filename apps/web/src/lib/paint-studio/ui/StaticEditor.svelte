<script lang="ts">
  // The gradient paint: presets, kind, angle or shape, repeat, stops.
  import { clonePaint, type Issue, type StudioPaint } from '../format';
  import { paintStyleText } from '../css';
  import { presetGround, STATIC_PRESETS } from '../presets';
  import Glyph from './Glyph.svelte';
  import Seg from './Seg.svelte';
  import Slider from './Slider.svelte';
  import Stops from './Stops.svelte';

  type Props = { paint: StudioPaint; issues: Issue[]; onchange: (p: Partial<StudioPaint>) => void; onsurprise?: () => void; part?: 'lib' | 'params' | 'both' };
  let { paint, issues, onchange, onsurprise, part = 'both' }: Props = $props();
  const angleIssue = $derived(issues.find((i) => i.field === 'angle')?.message ?? null);

  function usePreset(p: StudioPaint) {
    const c = clonePaint(p);
    onchange({ kind: c.kind, angle: c.angle, repeat: c.repeat, shape: c.shape, stops: c.stops, shadows: c.shadows, color: c.color });
  }
</script>

{#if part !== 'params'}
<section class="ps-panel" aria-labelledby="ps-presets-h">
  <header>
    <h2 id="ps-presets-h">Presets</h2>
    {#if onsurprise}<button type="button" class="ui-btn ghost small" onclick={onsurprise}><Glyph name="shuffle" /> Surprise me</button>{/if}
  </header>
  <div class="ps-body">
    <div class="ps-presets">
      {#each STATIC_PRESETS as p (p.name)}
        <button type="button" class="ps-preset {presetGround(p)}" onclick={() => usePreset(p)} aria-label="Use the {p.name} preset"><span class="ps-painted" style={paintStyleText(p)}>{p.name}</span></button>
      {/each}
    </div>
    <p class="ps-hint">A preset replaces the gradient and shadows, and keeps the name.</p>
  </div>
</section>
{/if}

{#if part !== 'lib'}
<section class="ps-panel" aria-labelledby="ps-gradient-h">
  <header><h2 id="ps-gradient-h">Gradient</h2></header>
  <div class="ps-body">
    <div class="ps-field">
      <span class="ps-flabel">Kind</span>
      <Seg
        label="Gradient kind"
        value={paint.kind === 'RADIAL_GRADIENT' ? 'RADIAL_GRADIENT' : 'LINEAR_GRADIENT'}
        onchange={(v) => onchange({ kind: v })}
        options={[
          { value: 'LINEAR_GRADIENT', label: 'Linear' },
          { value: 'RADIAL_GRADIENT', label: 'Radial' },
        ]}
      />
    </div>
    {#if paint.kind === 'LINEAR_GRADIENT'}
      <Slider label="Angle" unit="°" min={0} max={360} step={1} value={paint.angle} error={angleIssue} hint="0° runs bottom to top, 90° left to right." onchange={(v) => onchange({ angle: Math.round(v) })} />
    {:else}
      <div class="ps-field">
        <span class="ps-flabel">Shape</span>
        <Seg label="Radial shape" value={paint.shape} onchange={(v) => onchange({ shape: v })} options={[{ value: 'CIRCLE', label: 'Circle' }, { value: 'ELLIPSE', label: 'Ellipse' }]} />
      </div>
    {/if}
    <label class="ui-check"><input class="ui-switch" type="checkbox" role="switch" checked={paint.repeat} onchange={(e) => onchange({ repeat: e.currentTarget.checked })} /> Repeat the stops along the name</label>
    <Stops stops={paint.stops} issues={issues.filter((i) => i.field === 'stops')} onchange={(s) => onchange({ stops: s })} />
  </div>
</section>
{/if}
