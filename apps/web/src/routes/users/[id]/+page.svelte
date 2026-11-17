<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import ProfileHead from '$lib/components/profile/ProfileHead.svelte';
  import EmptyState from '$lib/components/profile/EmptyState.svelte';
  import Uploads from './Uploads.svelte';
  import { clean, count } from '$lib/format';

  let { data } = $props();
  const name = $derived(clean(data.creator?.displayName ?? ''));
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
  <title>{data.creator ? `${name} - cyex.tv` : data.error ? 'Something went wrong - cyex.tv' : 'Nothing here - cyex.tv'}</title>
  {#if name}<meta name="description" content="{name}'s emotes on cyex.tv." />{/if}
</svelte:head>

<div class="page">
  {#if data.error}
    <div class="pad">
      <EmptyState level={1} title="This page can’t load right now." role="alert">
        {data.error}
        {#snippet actions()}
          <button class="btn btn-primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying' : 'Try again'}</button>
        {/snippet}
      </EmptyState>
    </div>
  {:else if !data.creator}
    <div class="pad">
      <EmptyState level={1} title="No public emotes here." role="status">
        This account hasn't published an emote, or there is no account with this address. Check the link, or
        look through what everyone has made.
        {#snippet actions()}
          <a class="btn btn-primary" href="/search">Browse emotes</a>
        {/snippet}
      </EmptyState>
    </div>
  {:else}
    <ProfileHead {name} avatar={data.creator.avatar} kind="Creator">
      {#snippet meta()}
        {count(data.total)} {data.total === 1 ? 'emote' : 'emotes'} uploaded
      {/snippet}
      {#snippet actions()}
        {#if data.channels.length}
          <div class="chans">
            {#each data.channels as ch (ch.platform + ch.login)}
              <a class="btn btn-s" href="/channels/{ch.platform}/{encodeURIComponent(ch.login)}">{ch.platform === 'twitch' ? 'Twitch' : 'Kick'}: <bdi>{clean(ch.login)}</bdi></a>
            {/each}
          </div>
        {/if}
      {/snippet}
    </ProfileHead>

    {#key data.id}
      <Uploads id={data.id} {name} first={data.items} total={data.total} />
    {/key}
  {/if}
</div>

<style>
  .pad { padding-top: clamp(48px, 8vw, 120px); }
  .chans { display: flex; flex-wrap: wrap; gap: 8px; }
</style>
