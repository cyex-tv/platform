<script lang="ts">
  // Drop shadows under the painted name, drawn in order (at most 4; offsets −16..16 px, blur 0..16 px).
  import { LIMITS, type Issue, type Shadow } from '../format';
  import Colour from './Colour.svelte';
  import Glyph from './Glyph.svelte';
  import Slider from './Slider.svelte';

  type Props = { shadows: Shadow[]; onchange: (s: Shadow[]) => void; issues?: Issue[] };
  let { shadows, onchange, issues = [] }: Props = $props();
  const rowIssue = (i: number) => issues.find((x) => x.index === i)?.message ?? null;
  const general = $derived(issues.find((i) => i.index == null)?.message ?? null);

  const set = (i: number, patch: Partial<Shadow>) => onchange(shadows.map((s, j) => (j === i ? { ...s, ...patch } : s)));
  const move = (i: number, d: number) => {
    const next = [...shadows];
    const [s] = next.splice(i, 1);
    next.splice(i + d, 0, s);
    onchange(next);
  };
</script>

<div class="ps-group" role="group" aria-label="Shadows">
  <div class="ps-rowhead">
    <strong>Shadows</strong>
    <span class="ps-count" class:ps-count-bad={shadows.length > LIMITS.shadowsMax}>{shadows.length} of {LIMITS.shadowsMax}</span>
  </div>
  {#if general}<p class="ps-msg" role="alert">{general}</p>{/if}
  {#if !shadows.length}<p class="ps-hint">No shadows. Each one is drawn under the name, in order.</p>{/if}
  {#each shadows as s, i (i)}
    <div class="ps-rowcard">
      <div class="ps-rowhead">
        <strong>Shadow {i + 1}</strong>
        <button type="button" class="ui-icon" aria-label="Move shadow {i + 1} up" title="Move up" disabled={i === 0} onclick={() => move(i, -1)}><Glyph name="up" /></button>
        <button type="button" class="ui-icon" aria-label="Move shadow {i + 1} down" title="Move down" disabled={i === shadows.length - 1} onclick={() => move(i, 1)}><Glyph name="down" /></button>
        <button type="button" class="ui-icon" aria-label="Remove shadow {i + 1}" title="Remove" onclick={() => onchange(shadows.filter((_, j) => j !== i))}><Glyph name="trash" /></button>
      </div>
      {#if rowIssue(i)}<p class="ps-msg">{rowIssue(i)}</p>{/if}
      <Colour label="Shadow {i + 1} colour" hideLabel value={s.color} onchange={(c) => set(i, { color: c })} />
      <Slider label="Across" unit="px" min={-LIMITS.offsetMax} max={LIMITS.offsetMax} step={0.5} value={s.x} onchange={(v) => set(i, { x: v })} />
      <Slider label="Down" unit="px" min={-LIMITS.offsetMax} max={LIMITS.offsetMax} step={0.5} value={s.y} onchange={(v) => set(i, { y: v })} />
      <Slider label="Blur" unit="px" min={0} max={LIMITS.radiusMax} step={0.5} value={s.radius} onchange={(v) => set(i, { radius: v })} />
    </div>
  {/each}
  <div class="ui-actions">
    <button type="button" class="ui-btn secondary small" disabled={shadows.length >= LIMITS.shadowsMax} onclick={() => onchange([...shadows, { x: 0, y: 1, radius: 2, color: '#00000099' }])}><Glyph name="plus" /> Add shadow</button>
  </div>
</div>
