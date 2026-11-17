<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import State from '$lib/components/State.svelte';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

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

<ColumnLayout top>
  <State kind={missing ? 'empty' : 'error'} level={1} role={missing ? 'status' : 'alert'} title={missing ? 'Nothing here.' : 'Something went wrong.'}>
    {lead}
    {#snippet actions()}
      {#if !missing}
        <button class="ui-btn primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying…' : 'Try again'}</button>
      {/if}
      <a class="ui-btn" class:primary={missing} class:secondary={!missing} href="/">Back to the front page</a>
      <a class="ui-btn secondary" href="/search">Browse emotes</a>
    {/snippet}
  </State>
</ColumnLayout>
