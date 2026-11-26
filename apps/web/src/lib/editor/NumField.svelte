<script lang="ts">
  // A whole-number field: type and press Enter (or leave it) to set; ↑ ↓ step by 1, Shift by 10.
  // It shows `value` again whenever that changes from outside.
  let {
    label,
    value,
    min = 1,
    max = 100_000,
    unit = 'px',
    onset,
    id,
  }: { label: string; value: number; min?: number; max?: number; unit?: string; onset: (v: number) => void; id?: string } = $props();
  let text = $state('');
  $effect(() => {
    text = String(Math.round(value));
  });

  function set(v: number) {
    const n = Math.min(max, Math.max(min, Math.round(v)));
    text = String(n);
    if (n !== Math.round(value)) onset(n);
  }

  function commit() {
    const v = Number(text.replace(',', '.'));
    if (!text.trim() || !Number.isFinite(v)) text = String(Math.round(value));
    else set(v);
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const k = (e.shiftKey ? 10 : 1) * (e.key === 'ArrowUp' ? 1 : -1);
      set((Number(text) || value) + k);
    } else if (e.key === 'Escape' && text !== String(Math.round(value))) {
      e.preventDefault();
      e.stopPropagation();
      text = String(Math.round(value));
    }
  }
</script>

<label class="nf">
  <span class="ui-label">{label}</span>
  <span class="box">
    <input {id} inputmode="numeric" bind:value={text} onkeydown={keydown} onblur={commit} aria-label="{label} in {unit}" />
    {#if unit}<span class="u" aria-hidden="true">{unit}</span>{/if}
  </span>
</label>

<style>
  .nf { display: grid; gap: 4px; min-width: 0; }
  .box { position: relative; display: flex; align-items: center; }
  input { width: 100%; min-width: 0; height: 32px; padding: 0 26px 0 8px; border: 1px solid var(--field-border, var(--border-strong)); border-radius: var(--radius-btn);
    background: var(--bg); color: var(--text); font: inherit; font-size: var(--t-fine); font-weight: 500; font-variant-numeric: tabular-nums; text-align: right; }
  input:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 1px; border-color: var(--accent-border); }
  .u { position: absolute; right: 8px; font-size: 11px; color: var(--dim); pointer-events: none; }
  @media (pointer: coarse) { input { height: 40px; font-size: 16px; } }
</style>
