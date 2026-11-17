<script lang="ts">
  // The theme switch: a 36px square in the AppHeader (and anywhere a page wants one). The state
  // and the switching live in $lib/shell/theme, shared with the account menu's theme row.
  import { onMount } from 'svelte';
  import { syncTheme, theme, toggleTheme } from '$lib/shell/theme.svelte';

  onMount(syncTheme);
</script>

<button class="theme" type="button" onclick={toggleTheme} aria-label={theme.light ? 'Switch to dark' : 'Switch to light'}>
  {#if theme.light}
    <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
      <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" />
    </svg>
  {:else}
    <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
    </svg>
  {/if}
</button>

<style>
  .theme { position: relative; width: 36px; height: 36px; flex: none; display: inline-grid; place-items: center; border-radius: var(--radius-btn);
    border: 1px solid var(--border); background: transparent; color: var(--muted); cursor: pointer;
    transition: color var(--fast), border-color var(--fast), background-color var(--fast), transform var(--dur-press) ease-out; }
  @media (hover: hover) and (pointer: fine) {
    .theme:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-hover); }
  }
  .theme:active { transform: scale(var(--press)); }
  /* Touch: the 36px look keeps a 44px target. */
  @media (pointer: coarse) { .theme::after { content: ''; position: absolute; inset: -4px; } }
  svg { fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
</style>
