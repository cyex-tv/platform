<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Icon from '$lib/components/Icon.svelte';
  import ProfileCard from '$lib/components/profile/ProfileCard.svelte';
  import EmptyState from '$lib/components/profile/EmptyState.svelte';
  import { currentSection, sectionRail, type Section } from '$lib/components/profile/sections';
  import { useCopied } from '$lib/clipboard.svelte';
  import { clean, count } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';
  import Uploads from './Uploads.svelte';

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

  // Sections are query-param tabs. Channels only when the creator has some on cyex.tv.
  type Key = 'uploads' | 'channels';
  const sections = $derived<Section<Key>[]>([
    { key: 'uploads', label: 'Uploads', icon: 'upload', count: data.total },
    ...(data.channels.length ? [{ key: 'channels' as const, label: 'Channels', icon: 'tv' as const, count: data.channels.length }] : []),
  ]);
  const section = $derived(currentSection(sections, page.url.searchParams));
  const groups = $derived(sectionRail(sections, page.url.pathname, section));
  const link = useCopied();
</script>

{#snippet meta()}
  {count(data.total)} {data.total === 1 ? 'emote' : 'emotes'} uploaded
{/snippet}
{#snippet actions()}
  <button class="btn btn-s copy" type="button" onclick={() => link.copy(location.origin + page.url.pathname)}>
    <Icon name={link.copied ? 'check' : 'copy'} size={16} />
    <span class="labels">
      <span class:off={link.state !== 'idle'}>Copy page link</span>
      <span class:off={!link.copied} aria-hidden="true">Link copied</span>
      <span class:off={!link.failed} aria-hidden="true">Couldn’t copy</span>
    </span>
  </button>
  <span class="sr-only" role="status">{link.copied ? 'Link copied' : link.failed ? 'Couldn’t copy' : ''}</span>
{/snippet}

<svelte:head>
  <title>{data.creator ? `${name} - cyex.tv` : data.error ? 'Something went wrong - cyex.tv' : 'Nothing here - cyex.tv'}</title>
  {#if name}<meta name="description" content="{name}'s emotes on cyex.tv." />{/if}
</svelte:head>

{#if data.error}
  <ColumnLayout>
    <div class="pad">
      <EmptyState level={1} title="This page can’t load right now." role="alert">
        {data.error}
        {#snippet actions()}
          <button class="btn btn-primary" type="button" onclick={retry} disabled={retrying}>{retrying ? 'Trying' : 'Try again'}</button>
        {/snippet}
      </EmptyState>
    </div>
  </ColumnLayout>
{:else if !data.creator}
  <ColumnLayout>
    <div class="pad">
      <EmptyState level={1} title="No public emotes here." role="status">
        This account hasn't published an emote, or there is no account with this address. Check the link, or
        look through what everyone has made.
        {#snippet actions()}
          <a class="btn btn-primary" href="/search">Browse emotes</a>
        {/snippet}
      </EmptyState>
    </div>
  </ColumnLayout>
{:else}
  <RailLayout>
    {#snippet rail()}
      <SideRail label="{name}'s page" {groups} collapse="tabs">
        {#snippet head()}
          <ProfileCard {name} avatar={data.creator?.avatar} kind="Creator" {meta} {actions} />
        {/snippet}
      </SideRail>
    {/snippet}

    <div class="phone-card"><ProfileCard heading={false} {name} avatar={data.creator.avatar} kind="Creator" {meta} {actions} /></div>

    {#if section === 'channels'}
      <section aria-labelledby="ch-h">
        <h2 class="block-title" id="ch-h">Channels</h2>
        <ul class="chans">
          {#each data.channels as ch (ch.platform + ch.login)}
            <li><a class="btn" href="/channels/{ch.platform}/{encodeURIComponent(ch.login)}">{ch.platform === 'twitch' ? 'Twitch' : 'Kick'}: <bdi>{clean(ch.login)}</bdi></a></li>
          {/each}
        </ul>
      </section>
    {:else}
      {#key data.id}
        <Uploads id={data.id} {name} first={data.items} total={data.total} />
      {/key}
    {/if}
  </RailLayout>
{/if}

<style>
  .pad { padding-top: clamp(48px, 8vw, 120px); }
  .chans { list-style: none; margin: 16px 0 0; padding: 0; display: flex; flex-wrap: wrap; gap: 8px; }
  .labels { display: inline-grid; }
  .labels > span { grid-area: 1 / 1; text-align: left; }
  .off { visibility: hidden; }
  .phone-card { display: none; }
  @media (max-width: 899.98px) {
    .phone-card { display: block; padding: 16px 0; border-bottom: 1px solid var(--border); margin-bottom: 4px; }
  }
</style>
