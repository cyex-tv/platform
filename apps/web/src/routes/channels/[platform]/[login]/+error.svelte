<script lang="ts">
  // Shown instead of the channel page when the channel isn't on cyex.tv (or the API can't be reached).
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { openPalette } from '$lib/components/palette.svelte';
  import EmptyState from '$lib/components/profile/EmptyState.svelte';
  import { clean, iso } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  const missing = $derived(page.status === 404);
  const known = $derived(page.params.platform === 'kick' || page.params.platform === 'twitch');
  const platform = $derived(page.params.platform === 'kick' ? 'Kick' : 'Twitch');
  // A login is user input: cleaned, and cut so a 500-character address stays a sentence.
  const login = $derived.by(() => {
    const l = [...clean(page.params.login ?? '')];
    return l.length > 40 ? `${l.slice(0, 40).join('')}…` : l.join('');
  });
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
  <title>{missing ? 'Channel not found' : 'Something went wrong'} - cyex.tv</title>
</svelte:head>

<ColumnLayout>
<div class="pad">
  {#if missing && known}
    <EmptyState level={1} title="{iso(login)} isn't on cyex.tv yet." role="status">
      Nobody has set up a {platform} channel called <bdi>{login}</bdi> here. A channel shows up once its owner signs in
      with {platform} and picks an emote set. Check the spelling, or look for another channel.
      {#snippet actions()}
        <div class="row">
          <button class="btn btn-primary" type="button" onclick={() => openPalette(login)}>Search channels</button>
          <a class="btn" href="/search">Browse emotes</a>
        </div>
      {/snippet}
    </EmptyState>
  {:else if missing}
    <EmptyState level={1} title="Nothing here." role="status">
      cyex.tv has Twitch and Kick channels.
      {#snippet actions()}
        <div class="row">
          <a class="btn btn-primary" href="/search">Browse emotes</a>
          <a class="btn" href="/">Back to the front page</a>
        </div>
      {/snippet}
    </EmptyState>
  {:else}
    <EmptyState level={1} title="This page can’t load right now." role="alert">
      cyex.tv didn’t answer. It’s usually back within a minute.
      {#snippet actions()}
        <div class="row">
          {#if page.status >= 500}
            <button class="btn btn-primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying' : 'Try again'}</button>
          {/if}
          <a class="btn" href="/">Back to the front page</a>
        </div>
      {/snippet}
    </EmptyState>
  {/if}
</div>
</ColumnLayout>

<style>
  .pad { padding-top: clamp(48px, 8vw, 120px); }
  .row { display: flex; flex-wrap: wrap; gap: 12px; }
</style>
