<script lang="ts">
  // The tick on an admin row or card that picks it for a bulk action. A real checkbox (Space and
  // a screen reader treat it as one) drawn as the browse tile's tick: an accent hairline and an
  // accent check when picked, never a filled box. `onpick` gets the click, so Shift picks a range.
  import Icon from '$lib/components/Icon.svelte';

  type Props = { checked: boolean; label: string; disabled?: boolean; onpick: (ev: MouseEvent) => void; class?: string };
  let { checked, label, disabled = false, onpick, class: cls = '' }: Props = $props();
</script>

<label class="pick {cls}" class:on={checked} class:off={disabled}>
  <input type="checkbox" {checked} {disabled} aria-label={label} onclick={(ev) => { ev.stopPropagation(); onpick(ev); }} />
  <span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>
</label>

<style>
  /* A 32px target around a 20px tick. */
  .pick { position: relative; display: inline-grid; place-items: center; width: 32px; height: 32px; flex: none; cursor: pointer; }
  .pick.off { cursor: default; opacity: .5; }
  input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: inherit; }
  .tick { display: inline-grid; place-items: center; width: 20px; height: 20px; border: 1px solid var(--border-strong); border-radius: var(--radius);
    background: var(--bg); color: transparent; transition: border-color var(--fast), color var(--fast); }
  .on .tick { border-color: var(--accent-border); color: var(--accent-text); }
  input:focus-visible + .tick { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) { .pick:not(.off):hover .tick { border-color: var(--accent-text); } }
  @media (forced-colors: active) { .on .tick { color: CanvasText; border-color: Highlight; } }
</style>
