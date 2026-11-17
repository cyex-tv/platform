<script lang="ts">
  import { onMount } from 'svelte';

  let light = $state(false);
  onMount(() => {
    light = document.documentElement.dataset.theme === 'light';
  });

  function apply() {
    light = !light;
    const root = document.documentElement;
    if (light) root.dataset.theme = 'light';
    else delete root.dataset.theme;
    try {
      localStorage.setItem('cyexTheme', light ? 'light' : 'dark');
    } catch {}
    const meta = document.querySelector('meta[name="theme-color"]');
    meta?.setAttribute('content', getComputedStyle(root).getPropertyValue('--bg').trim());
  }

  function toggle() {
    // Read at the moment it runs (cyex-design): reduced motion switches instantly.
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (!still && doc.startViewTransition) doc.startViewTransition(apply);
    else apply();
  }
</script>

<button class="theme" type="button" onclick={toggle} aria-label={light ? 'Switch to dark' : 'Switch to light'}>
  {#if light}
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
  .theme { width: 38px; height: 38px; display: inline-grid; place-items: center; border-radius: var(--radius-btn);
    border: 1px solid var(--border); background: transparent; color: var(--muted); cursor: pointer;
    transition: color var(--dur-color), border-color var(--dur-color), background-color var(--dur-color),
      transform var(--dur-press) ease-out; }
  .theme:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-2); }
  .theme:active { transform: scale(.94); }
  svg { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
</style>
