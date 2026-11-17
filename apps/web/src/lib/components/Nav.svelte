<script lang="ts">
  import { onMount } from 'svelte';
  import Mark from './Mark.svelte';
  import ThemeToggle from './ThemeToggle.svelte';

  let { signedIn = false }: { signedIn?: boolean } = $props();
  let scrolled = $state(false);
  let open = $state(false);

  onMount(() => {
    const onScroll = () => (scrolled = window.scrollY > 8);
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
    return () => window.removeEventListener('scroll', onScroll);
  });

  const links = [
    { href: '/search', label: 'Search' },
    { href: '/developers', label: 'Developers' },
  ];
</script>

<header class="nav" class:scrolled>
  <a class="brand" href="/" aria-label="cyex.tv home">
    <Mark />
    <span>cyex.tv</span>
  </a>
  <nav class="nav-links" aria-label="Main">
    {#each links as l}
      <a href={l.href}>{l.label}</a>
    {/each}
  </nav>
  <span class="nav-rule" aria-hidden="true"></span>
  <ThemeToggle />
  <a class="btn btn-s nav-cta" href={signedIn ? '/me' : '/signin'}>{signedIn ? 'Your emotes' : 'Sign in'}</a>
  <button
    class="menu"
    type="button"
    aria-expanded={open}
    aria-controls="phone-menu"
    aria-label="Menu"
    onclick={() => (open = !open)}
  >
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
  </button>
</header>
<div id="phone-menu" class="phone" class:open hidden={!open}>
  {#each links as l}
    <a href={l.href} onclick={() => (open = false)}>{l.label}</a>
  {/each}
  <a href={signedIn ? '/me' : '/signin'} onclick={() => (open = false)}>{signedIn ? 'Your emotes' : 'Sign in'}</a>
</div>

<style>
  .nav { position: fixed; inset: 0 0 auto; z-index: 50; height: var(--nav-h);
    display: flex; align-items: center; gap: 14px;
    padding: 0 max(var(--page-pad), (100% - var(--page-max)) / 2 + var(--page-pad));
    transition: background-color var(--dur-move); }
  .nav.scrolled { background: color-mix(in srgb, var(--bg) 78%, transparent);
    backdrop-filter: blur(14px) saturate(1.4); }
  .nav::after { content: ''; position: absolute; inset: 100% 0 auto; height: 18px; pointer-events: none;
    background: linear-gradient(color-mix(in srgb, var(--bg) 55%, transparent), transparent);
    opacity: 0; transition: opacity var(--dur-move); }
  .nav.scrolled::after { opacity: 1; }
  .brand { display: flex; align-items: center; gap: 9px; font-size: 17px; font-weight: 600; letter-spacing: -.01em; margin-right: 14px; }
  .nav-links { margin-left: auto; display: flex; gap: 2px; }
  .nav-links a { padding: 8px 10px; font-size: var(--t-nav); color: var(--muted);
    border-radius: var(--radius-btn); transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .nav-links a:hover { color: var(--text); }
  .nav-links a:active { transform: scale(.96); }
  .nav-rule { width: 1px; height: 20px; background: var(--border-strong); }
  .menu { display: none; width: 38px; height: 38px; place-items: center; border-radius: var(--radius-btn);
    border: 1px solid var(--border); background: transparent; color: var(--muted); cursor: pointer; }
  .menu svg { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
  .phone { position: fixed; z-index: 49; inset: var(--nav-h) 0 auto; background: var(--bg);
    border-bottom: 1px solid var(--border); padding: 8px var(--page-pad) 16px; display: grid; }
  .phone[hidden] { display: none; }
  .phone a { padding: 12px 0; font-size: 16px; }
  .phone.open { animation: drop var(--dur-hover) var(--ease); }
  @keyframes drop { from { opacity: 0; translate: 0 -8px; } }
  @media (max-width: 720px) {
    .nav-links, .nav-rule, .nav-cta { display: none; }
    .brand { margin-right: auto; }
    .menu { display: inline-grid; }
  }
  @media (min-width: 721px) { .phone { display: none; } }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .nav.scrolled { background: var(--bg); backdrop-filter: none; border-bottom: 1px solid var(--border-strong); }
    .nav::after { display: none; }
  }
</style>
