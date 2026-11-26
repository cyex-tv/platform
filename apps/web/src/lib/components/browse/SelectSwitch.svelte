<script lang="ts">
  // The "Select" switch that turns a grid's multi-select on and off (the browse page has the same
  // look inline): the cyex-app switch, an accent track with a tick on it when on, the word bold.
  //   <SelectSwitch on={selecting} onchange={(v) => setSelecting(v)} />
  let { on, onchange, label = 'Select' }: { on: boolean; onchange: (on: boolean) => void; label?: string } = $props();
</script>

<label class="switch" class:on>
  <span>{label}</span>
  <input type="checkbox" role="switch" checked={on} onchange={(e) => onchange(e.currentTarget.checked)} />
  <span class="track" aria-hidden="true"></span>
</label>

<style>
  .switch { position: relative; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 2px;
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); cursor: pointer; user-select: none; }
  .switch.on { color: var(--text); font-weight: 700; }
  .switch input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .track { position: relative; width: 36px; height: 20px; border-radius: var(--radius-pill); background: var(--border-strong);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text) 40%, transparent); transition: background-color var(--fast) var(--ease); }
  .track::before { content: ''; position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: var(--text);
    transition: translate var(--fast) var(--ease), background-color var(--fast); }
  .track::after { content: ''; position: absolute; top: 6px; left: 9px; width: 2px; height: 8px; border-radius: 1px; background: var(--accent-ink); opacity: 0; }
  .on .track { background: var(--accent); box-shadow: none; }
  .on .track::before { translate: 16px 0; background: var(--accent-ink); }
  .on .track::after { opacity: 1; }
  .switch:has(input:focus-visible) .track { outline: 2px solid var(--focus-ring); outline-offset: 2px; }
  @media (pointer: coarse) { .switch { min-height: 44px; } }
  @media (forced-colors: active) { .on .track { background: Highlight; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .track, :global(html:not([data-motion='full'])) .track::before { transition: none; } }
</style>
