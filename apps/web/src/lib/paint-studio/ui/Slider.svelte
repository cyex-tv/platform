<script lang="ts">
  // A labelled slider with a number field beside it; both edit the same value, clamped to the range.
  type Props = {
    label: string;
    value: number;
    min: number;
    max: number;
    step?: number;
    unit?: string;
    hint?: string;
    error?: string | null;
    onchange: (v: number) => void;
  };
  let { label, value, min, max, step = 1, unit = '', hint, error = null, onchange }: Props = $props();
  const uid = $props.id();
  const decimals = $derived(step >= 1 ? 0 : Math.min(4, Math.ceil(-Math.log10(step) - 1e-9)));
  const clamp = (v: number) => Math.min(max, Math.max(min, v));
  const snap = (v: number) => Number((Math.round(v / step) * step).toFixed(decimals));

  function fromText(t: string) {
    const v = Number(t.replace(',', '.'));
    if (Number.isFinite(v)) onchange(clamp(snap(v)));
  }
</script>

<div class="ps-field">
  <label for="{uid}-n">{label}{unit ? ` (${unit})` : ''}</label>
  <div class="ps-slider">
    <input
      type="range"
      aria-label={label}
      aria-describedby={hint || error ? `${uid}-h` : undefined}
      {min}
      {max}
      {step}
      value={value}
      oninput={(e) => onchange(Number(e.currentTarget.value))}
    />
    <input
      id="{uid}-n"
      class="ps-num"
      type="text"
      inputmode="decimal"
      value={String(Number(value.toFixed(decimals))).replace('.', ',')}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={hint || error ? `${uid}-h` : undefined}
      onchange={(e) => fromText(e.currentTarget.value)}
      onkeydown={(e) => {
        if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
          e.preventDefault();
          onchange(clamp(snap(value + (e.key === 'ArrowUp' ? 1 : -1) * step * (e.shiftKey ? 10 : 1))));
        }
      }}
    />
  </div>
  {#if error}<p class="ps-msg" id="{uid}-h">{error}</p>{:else if hint}<p class="ps-hint" id="{uid}-h">{hint}</p>{/if}
</div>
