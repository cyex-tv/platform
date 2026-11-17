<script lang="ts">
  // A small 3px chip, e.g. an active filter. With `onremove` it gets a remove button
  // ("Remove <label>"); with `href` the whole chip is a link (a removable filter in the URL).
  import Icon from './Icon.svelte';

  let { label, onremove, href }: { label: string; onremove?: () => void; href?: string } = $props();
</script>

{#if href}
  <a class="chip" {href} aria-label="Remove {label}">
    <span>{label}</span><Icon name="x" size={14} />
  </a>
{:else}
  <span class="chip" class:static={!onremove}>
    <span>{label}</span>
    {#if onremove}
      <button type="button" class="x" aria-label="Remove {label}" onclick={onremove}><Icon name="x" size={14} /></button>
    {/if}
  </span>
{/if}

<style>
  .chip { display: inline-flex; align-items: center; gap: 4px; height: 32px; padding: 0 4px 0 10px; max-width: 100%;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface);
    color: var(--text); font-size: var(--t-small); font-weight: 500; white-space: nowrap;
    transition: border-color var(--dur-color), background-color var(--dur-color), transform var(--dur-press) ease-out; }
  .chip > span { overflow: hidden; text-overflow: ellipsis; }
  .chip.static { padding-right: 10px; }
  a.chip { padding-right: 8px; color: var(--text); }
  a.chip :global(svg), .x { color: var(--muted); }
  .x { display: inline-grid; place-items: center; width: 24px; height: 24px; padding: 0; border: 0; border-radius: var(--radius-btn);
    background: none; cursor: pointer; transition: color var(--dur-color), background-color var(--dur-color); }
  a.chip:active, .x:active { transform: scale(var(--press)); }
  /* Touch: a 32px remove button with a 44px hit area around it. */
  @media (pointer: coarse) {
    .chip:has(.x) { padding-right: 0; }
    .x { position: relative; width: 32px; height: 32px; }
    .x::after { content: ''; position: absolute; inset: -6px; }
  }
  @media (hover: hover) and (pointer: fine) {
    a.chip:hover { border-color: var(--muted); background: var(--surface-2); }
    a.chip:hover :global(svg), .x:hover { color: var(--text); }
    .x:hover { background: var(--surface-2); }
  }
</style>
