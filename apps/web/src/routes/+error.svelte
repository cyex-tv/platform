<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';

  const missing = $derived(page.status === 404);
  // Fixed copy per status: a server's own message never reaches the page.
  const lead = $derived(
    missing
      ? 'That page doesn’t exist.'
      : page.status >= 500
        ? 'cyex.tv didn’t answer. It’s usually back within a minute.'
        : 'That didn’t work. Check the address, or go back to the front page.',
  );
  let retrying = $state(false);
  async function retry() {
    retrying = true;
    try {
      await invalidateAll();
    } finally {
      retrying = false;
    }
  }
</script>

<svelte:head>
  <title>{missing ? 'Nothing here' : 'Something went wrong'} - cyex.tv</title>
</svelte:head>

<div class="page nothing">
  <h1>{missing ? 'Nothing here.' : 'Something went wrong.'}</h1>
  <p class="lead">{lead}</p>
  <div class="actions">
    {#if !missing}
      <button class="btn btn-primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying' : 'Try again'}</button>
    {/if}
    <a class="btn" class:btn-primary={missing} href="/">
      Back to the front page
      <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
    </a>
    <a class="btn" href="/search">Browse emotes</a>
  </div>
</div>

<style>
  .nothing { padding-top: clamp(64px, 10vw, 140px); text-align: center; }
  h1 { font-size: clamp(44px, 7vw, 96px); line-height: 1; letter-spacing: -.045em; }
  .lead { margin-inline: auto; }
  .actions { justify-content: center; flex-wrap: wrap; margin-top: 32px; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }
</style>
