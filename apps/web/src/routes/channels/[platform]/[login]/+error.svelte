<script lang="ts">
  // Shown instead of the channel page when the channel isn't on cyex.tv (or the API can't be reached).
  import { page } from '$app/state';
  import { openPalette } from '$lib/components/palette.svelte';
  import ProfileState from '$lib/components/profile/ProfileState.svelte';

  const missing = $derived(page.status === 404);
  const platform = $derived(page.params.platform === 'kick' ? 'Kick' : page.params.platform === 'twitch' ? 'Twitch' : null);
  const login = $derived(page.params.login ?? '');
</script>

<svelte:head>
  <title>{missing ? 'Channel not found' : 'Something went wrong'} - cyex.tv</title>
</svelte:head>

<div class="page pad">
  {#if missing && platform}
    <ProfileState title="{login} isn't on cyex.tv yet." role="status">
      Nobody has set up a {platform} channel called {login} here. A channel shows up once its owner signs in
      with {platform} and picks an emote set. Check the spelling, or look for another channel.
      {#snippet actions()}
        <div class="row">
          <button class="btn btn-primary" type="button" onclick={() => openPalette(login)}>Search channels</button>
          <a class="btn" href="/search">Browse emotes</a>
        </div>
      {/snippet}
    </ProfileState>
  {:else}
    <ProfileState title={missing ? 'Nothing here.' : 'This page can’t load right now.'} role="alert">
      {page.error?.message ?? 'Try again in a moment.'}
      {#snippet actions()}
        <div class="row">
          <button class="btn btn-primary" type="button" onclick={() => location.reload()}>Try again</button>
          <a class="btn" href="/">Back to the front page</a>
        </div>
      {/snippet}
    </ProfileState>
  {/if}
</div>

<style>
  .pad { padding-top: clamp(48px, 8vw, 120px); }
  .row { display: flex; flex-wrap: wrap; gap: 12px; }
</style>
