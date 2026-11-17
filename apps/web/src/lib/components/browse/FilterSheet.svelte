<script lang="ts">
  // The filter controls inside the Filters popover / bottom sheet. Choices apply at once.
  import type { Browse, Kind } from './params';

  type Props = { view: Browse; onchange: (over: Partial<Browse>) => void };
  let { view, onchange }: Props = $props();

  const kinds: { value: Kind; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'animated', label: 'Animated' },
    { value: 'still', label: 'Still' },
  ];
</script>

<div class="sheet">
  <fieldset>
    <legend>Type</legend>
    <div class="row">
      {#each kinds as k (k.value)}
        <label class="opt" class:on={view.kind === k.value}>
          <input type="radio" name="kind" value={k.value} checked={view.kind === k.value} onchange={() => onchange({ kind: k.value })} />
          {k.label}
        </label>
      {/each}
    </div>
  </fieldset>

  <div class="sw">
    <label>
      <input type="checkbox" checked={view.zeroWidth} onchange={(e) => onchange({ zeroWidth: e.currentTarget.checked })} />
      <span><b>Zero-width</b><small>Emotes that stack on the one before them.</small></span>
    </label>
    <label>
      <input type="checkbox" checked={view.wide} onchange={(e) => onchange({ wide: e.currentTarget.checked })} />
      <span><b>Wide</b><small>Emotes about twice as wide as tall.</small></span>
    </label>
    <label>
      <input type="checkbox" checked={view.nsfw} onchange={(e) => onchange({ nsfw: e.currentTarget.checked })} />
      <span><b>Show NSFW</b><small>Off by default. Adult emotes are hidden unless you turn this on.</small></span>
    </label>
  </div>
</div>

<style>
  .sheet { display: grid; gap: 16px; padding: 8px; }
  fieldset { border: 0; margin: 0; padding: 0; min-width: 0; }
  legend { padding: 0; margin-bottom: 8px; font-size: var(--t-small); font-weight: 600; color: var(--muted); }
  .row { display: flex; gap: 6px; flex-wrap: wrap; }
  .opt { position: relative; display: inline-flex; align-items: center; height: 38px; padding: 0 14px;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); font-size: var(--t-small); font-weight: 500;
    cursor: pointer; transition: border-color var(--dur-color), background-color var(--dur-color), transform var(--dur-press) ease-out; }
  .opt.on { background: var(--surface-2); border-color: var(--text); }
  .opt input { position: absolute; inset: 0; opacity: 0; margin: 0; cursor: pointer; }
  .opt:has(input:focus-visible) { outline: 2px solid var(--accent-text); outline-offset: 2px; }
  .opt:active { transform: scale(.97); }
  .sw { display: grid; gap: 4px; }
  .sw label { display: flex; gap: 12px; align-items: flex-start; padding: 8px; border-radius: var(--radius-btn); cursor: pointer; min-height: 44px; }
  .sw input { width: 18px; height: 18px; margin: 2px 0 0; accent-color: var(--accent-text); flex: none; }
  .sw span { display: grid; gap: 2px; min-width: 0; }
  .sw b { font-size: var(--t-small); font-weight: 600; }
  .sw small { font-size: 13px; color: var(--muted); line-height: 1.4; }
  @media (hover: hover) and (pointer: fine) {
    .sw label:hover { background: var(--surface-2); }
    .opt:hover { border-color: var(--muted); }
  }
  @media (prefers-reduced-motion: reduce) { .opt { transition: none; } .opt:active { transform: none; } }
</style>
