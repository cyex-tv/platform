<script lang="ts">
  import { page } from '$app/state';
</script>

<svelte:head>
  <title>{page.status === 404 ? 'Nothing here' : 'Something went wrong'} - cyex.tv</title>
</svelte:head>

<div class="page nothing">
  <h1>{page.status === 404 ? 'Nothing here.' : 'Something went wrong.'}</h1>
  <p class="lead">{page.error?.message ?? 'That page doesn’t exist.'}</p>
  <div class="actions">
    <a class="btn btn-primary" href="/">
      Back to the front page
      <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
    </a>
  </div>
</div>

<style>
  .nothing { padding-top: calc(var(--nav-h) + clamp(64px, 10vw, 140px)); text-align: center; }
  h1 { font-size: clamp(44px, 7vw, 96px); line-height: 1; letter-spacing: -.045em; }
  .lead { margin-inline: auto; }
  .actions { justify-content: center; margin-top: 32px; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
</style>
