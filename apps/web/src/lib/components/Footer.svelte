<script lang="ts">
  import Mark from './Mark.svelte';
  const year = new Date().getFullYear();
</script>

<footer class="footer">
  <div class="page">
    <div class="top">
      <a class="brand" href="/"><Mark size={18} /><span>cyex.tv</span></a>
      <nav class="links" aria-label="Footer">
        <a href="/search">Search</a>
        <a href="/developers">Developers</a>
        <a href="https://github.com/cyex-tv/platform">Source</a>
        <a href="https://github.com/cyex-tv/platform/blob/main/docs/takedown.md">Takedowns</a>
      </nav>
    </div>
    <div class="base">
      Not affiliated with, endorsed by, or sponsored by Twitch or Kick. © {year} seryxx
    </div>
  </div>
</footer>

<style>
  .footer { border-top: 1px solid var(--border); margin-top: var(--section-pad); }
  .top { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 24px; padding: 40px 0 32px; }
  .brand { display: flex; align-items: center; gap: 9px; font-size: 16px; font-weight: 600; letter-spacing: -.01em; }
  .links { display: flex; flex-wrap: wrap; gap: 8px 28px; font-size: 14.5px; color: var(--muted); }
  .links a { transition: color var(--dur-color); }
  .links a:hover { color: var(--text); }
  .base { border-top: 1px solid var(--border); padding: 20px 0 36px; font-size: var(--t-fine); color: var(--muted); }
</style>
