<script lang="ts">
  // Shown instead of the channel page when the channel isn't on cyex.tv (or the API can't be reached).
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { openPalette } from '$lib/components/palette.svelte';
  import State from '$lib/components/State.svelte';
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

<ColumnLayout top>
  {#if missing && known}
    <State level={1} role="status" title="{iso(login)} isn't on cyex.tv yet.">
      Nobody has set up a {platform} channel called <bdi>{login}</bdi> here. A channel shows up once its owner signs in
      with {platform} and picks an emote set. Check the spelling, or look for another channel.
      {#snippet actions()}
        <button class="ui-btn primary" type="button" onclick={() => openPalette(login)}>Search channels</button>
        <a class="ui-btn secondary" href="/search">Browse emotes</a>
      {/snippet}
    </State>
  {:else if missing}
    <State level={1} role="status" title="Nothing here.">
      cyex.tv has Twitch and Kick channels.
      {#snippet actions()}
        <a class="ui-btn primary" href="/search">Browse emotes</a>
        <a class="ui-btn secondary" href="/">Back to the front page</a>
      {/snippet}
    </State>
  {:else}
    <State kind="error" level={1} title="This page can’t load right now.">
      cyex.tv didn’t answer. It’s usually back within a minute.
      {#snippet actions()}
        {#if page.status >= 500}
          <button class="ui-btn primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying…' : 'Try again'}</button>
        {/if}
        <a class="ui-btn secondary" href="/">Back to the front page</a>
      {/snippet}
    </State>
  {/if}
</ColumnLayout>
