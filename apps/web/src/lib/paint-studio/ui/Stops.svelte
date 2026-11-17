<script lang="ts">
  // Colour stops: a strip showing the ramp, then one row per stop (colour, opacity, position).
  // Positions never go backwards: moving a stop past a neighbour re-sorts the list.
  import { hexToRgba, LIMITS, rgbaToHex, round4, sortStops, type Issue, type Stop } from '../format';
  import Colour from './Colour.svelte';
  import Glyph from './Glyph.svelte';
  import Slider from './Slider.svelte';

  type Props = { stops: Stop[]; onchange: (s: Stop[]) => void; issues?: Issue[]; min?: number; max?: number; label?: string };
  let { stops, onchange, issues = [], min = LIMITS.stopsMin, max = LIMITS.stopsMax, label = 'Colour stops' }: Props = $props();

  const ramp = $derived(`linear-gradient(90deg, ${stops.map((s) => `${s.color} ${s.at * 100}%`).join(', ')})`);
  const general = $derived(issues.find((i) => i.index == null)?.message ?? null);
  const rowIssue = (i: number) => issues.find((x) => x.index === i)?.message ?? null;

  function set(i: number, patch: Partial<Stop>) {
    const next = stops.map((s, j) => (j === i ? { ...s, ...patch } : s));
    onchange(patch.at != null ? sortStops(next) : next);
  }
  function remove(i: number) {
    onchange(stops.filter((_, j) => j !== i));
  }
  // A new stop in the middle of the widest gap, in the colour the ramp has there.
  function add() {
    if (stops.length >= max) return;
    if (stops.length < 2) {
      onchange([...stops, { at: 1, color: stops[0]?.color ?? '#FFFFFF' }]);
      return;
    }
    let gi = 0;
    for (let i = 1; i < stops.length - 1; i++) if (stops[i + 1].at - stops[i].at > stops[gi + 1].at - stops[gi].at) gi = i;
    const a = stops[gi], b = stops[gi + 1];
    const A = hexToRgba(a.color), B = hexToRgba(b.color);
    const mid = rgbaToHex(...(A.map((v, k) => (v + B[k]) / 2) as [number, number, number, number]));
    const next = [...stops];
    next.splice(gi + 1, 0, { at: round4((a.at + b.at) / 2), color: mid });
    onchange(next);
  }
  const reverse = () => onchange(sortStops(stops.map((s) => ({ ...s, at: round4(1 - s.at) })).reverse()));
  const spread = () => onchange(stops.map((s, i) => ({ ...s, at: round4(stops.length > 1 ? i / (stops.length - 1) : 0) })));
</script>

<div class="ps-group" role="group" aria-label={label}>
  <div class="ps-rowhead">
    <strong>{label}</strong>
    <span class="ps-count" class:ps-count-bad={stops.length > max || stops.length < min}>{stops.length} of {max}</span>
  </div>
  <div class="ps-bar" aria-hidden="true"><span style:background-image={ramp}></span></div>
  {#if general}<p class="ps-msg" role="alert">{general}</p>{/if}
  <div>
    {#each stops as s, i (i)}
      <div class="ps-rowcard">
        <div class="ps-rowhead">
          <strong>Stop {i + 1}</strong>
          <button type="button" class="ui-icon" aria-label="Remove stop {i + 1}" title="Remove stop {i + 1}" disabled={stops.length <= min} onclick={() => remove(i)}><Glyph name="trash" /></button>
        </div>
        <Colour label="Stop {i + 1} colour" hideLabel value={s.color} onchange={(c) => set(i, { color: c })} error={rowIssue(i)} />
        <Slider label="Position" unit="%" min={0} max={100} step={0.1} value={round4(s.at * 100)} onchange={(v) => set(i, { at: round4(v / 100) })} />
      </div>
    {/each}
  </div>
  <div class="ui-actions">
    <button type="button" class="ui-btn secondary small" onclick={add} disabled={stops.length >= max}><Glyph name="plus" /> Add stop</button>
    <button type="button" class="ui-btn ghost small" onclick={reverse} disabled={stops.length < 2}>Reverse</button>
    <button type="button" class="ui-btn ghost small" onclick={spread} disabled={stops.length < 2}>Space evenly</button>
  </div>
  {#if stops.length >= max}<p class="ps-hint">That’s the most stops a paint can have.</p>{/if}
</div>
