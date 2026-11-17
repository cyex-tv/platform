<script lang="ts">
  // Light or dark, saved in localStorage (cyexTheme) and cross-faded with a view transition
  // unless reduced motion is on. `variant="tool"` is the cyex-app control for the signed-in
  // pages: 32px, the tool's corner, and the instant opacity press instead of the scale.
  import { onMount } from 'svelte';

  let { variant = 'web' }: { variant?: 'web' | 'tool' } = $props();

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
    const bg = getComputedStyle(root).getPropertyValue('--bg').trim();
    document.querySelectorAll('meta[name="theme-color"]').forEach((m) => m.setAttribute('content', bg));
  }

  function toggle() {
    // Read at the moment it runs (cyex-design): reduced motion switches instantly.
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const doc = document as Document & { startViewTransition?: (cb: () => void) => unknown };
    if (!still && doc.startViewTransition) doc.startViewTransition(apply);
    else apply();
  }
</script>

<button class="theme {variant}" type="button" onclick={toggle} aria-label={light ? 'Switch to dark' : 'Switch to light'}>
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
  .theme { width: 40px; height: 40px; flex: none; display: inline-grid; place-items: center; border-radius: var(--radius-btn);
    border: 1px solid var(--border); background: transparent; color: var(--muted); cursor: pointer;
    transition: color var(--dur-color), border-color var(--dur-color), background-color var(--dur-color),
      transform var(--dur-press) ease-out; }
  @media (hover: hover) and (pointer: fine) {
    .theme:hover { color: var(--text); border-color: var(--border-strong); background: var(--surface-2); }
  }
  .theme:active { transform: scale(var(--press)); }
  svg { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }

  .tool { width: 32px; height: 32px; border-radius: var(--radius); border-color: var(--border-strong);
    transition: background-color var(--fast), color var(--fast); }
  .tool svg { stroke-width: 1.8; }
  @media (hover: hover) and (pointer: fine) {
    .tool:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  }
  .tool:active { transform: none; opacity: .6; transition: none; }
  @media (pointer: coarse) { .tool { width: 44px; height: 44px; } }
</style>
