<script lang="ts">
  import { page } from '$app/state';
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ProfileCard from '$lib/components/profile/ProfileCard.svelte';
  import EmptyState from '$lib/components/profile/EmptyState.svelte';
  import { chatLines } from '$lib/components/profile/chat';
  import { currentSection, sectionRail, type Section } from '$lib/components/profile/sections';
  import { useCopied } from '$lib/clipboard.svelte';
  import { clean, count } from '$lib/format';
  import RailLayout from '$lib/shell/RailLayout.svelte';
  import SideRail from '$lib/shell/SideRail.svelte';

  let { data } = $props();
  const c = $derived(data.channel);
  const set = $derived(data.set);
  const key = $derived(c.platform === 'TWITCH' ? ('twitch' as const) : ('kick' as const));
  const platform = $derived(key === 'twitch' ? 'Twitch' : 'Kick');
  const items = $derived(set?.emotes.map((m) => ({ ...m.emote, shownName: m.name })) ?? []);

  const STEP = 120;
  let filter = $state('');
  let shown = $state(STEP);
  const link = useCopied();
  const who = $derived(clean(c.account.displayName));

  const needle = $derived(filter.trim().toLowerCase());
  const matches = $derived(needle ? items.filter((e) => e.shownName.toLowerCase().includes(needle)) : items);
  const visible = $derived(matches.slice(0, shown));
  const lines = $derived(
    chatLines(items.slice(0, 12).map((e) => ({ ...e, name: e.shownName, zeroWidth: e.flags.zeroWidth }))),
  );
  const fill = $derived(set && set.capacity > 0 ? Math.min(100, (items.length / set.capacity) * 100) : 0);

  $effect(() => {
    void needle;
    shown = STEP;
  });

  // Sections are query-param tabs: the set, and how it looks in chat. (Editors: the API has no list yet.)
  type Key = 'set' | 'chat';
  const sections = $derived<Section<Key>[]>([
    { key: 'set', label: 'Active set', icon: 'layers', count: set ? items.length : undefined },
    ...(set && lines.length ? [{ key: 'chat' as const, label: 'Chat preview', icon: 'smile' as const }] : []),
  ]);
  const section = $derived(currentSection(sections, page.url.searchParams));
  const groups = $derived(sectionRail(sections, page.url.pathname, section));
</script>

<svelte:head>
  <title>{who} - cyex.tv</title>
  <meta name="description" content="The cyex.tv emotes in {who}'s {platform} chat." />
</svelte:head>

{#snippet meta()}
  {#if set}Active set: <bdi>{clean(set.name)}</bdi>{:else}No set picked yet{/if}
{/snippet}
{#snippet actions()}
  <button class="btn btn-s copy" type="button" onclick={() => link.copy(location.origin + page.url.pathname)}>
    <Icon name={link.copied ? 'check' : 'copy'} size={16} />
    <span class="labels">
      <span class:off={link.state !== 'idle'}>Copy set link</span>
      <span class:off={!link.copied} aria-hidden="true">Link copied</span>
      <span class:off={!link.failed} aria-hidden="true">Couldn’t copy</span>
    </span>
  </button>
  <span class="sr-only" role="status">{link.copied ? 'Link copied' : link.failed ? 'Couldn’t copy' : ''}</span>
  <a class="btn btn-s" href="/users/{c.account.id}">
    <Icon name="user" size={16} />Creator page
  </a>
{/snippet}

<RailLayout>
  {#snippet rail()}
    <SideRail label="{who}'s channel" {groups} collapse="tabs">
      {#snippet head()}
        <ProfileCard name={who} avatar={c.account.avatar} kind="{platform} channel" {meta} {actions} />
      {/snippet}
    </SideRail>
  {/snippet}

  <div class="phone-card"><ProfileCard name={who} avatar={c.account.avatar} kind="{platform} channel" {meta} {actions} /></div>

  {#if !set}
    <EmptyState rule title="No set picked yet.">
      {who} hasn't chosen an emote set for {platform}, so there is nothing to show here. Their
      emotes appear in chat as soon as they do.
      {#snippet actions()}
        <a class="btn" href="/search">Browse emotes</a>
      {/snippet}
    </EmptyState>
  {:else if section === 'chat'}
    <section class="chat" aria-labelledby="chat-h">
      <h2 class="block-title" id="chat-h">Chat preview</h2>
      <p class="fine">How these emotes look in a {platform} chat. The viewers are made up.</p>
      <div class="frame">
        <ChatPreview platform={key} {lines} visible={6} label="{platform} chat with {who}'s emotes" />
      </div>
    </section>
  {:else if items.length === 0}
    <EmptyState rule title="The set is empty.">
      {who} has a set, with room for {count(set.capacity)} emotes, but hasn't added any yet.
      {#snippet actions()}
        <a class="btn" href="/search">Browse emotes</a>
      {/snippet}
    </EmptyState>
  {:else}
    <section aria-labelledby="emotes-h">
      <div class="bar">
        <h2 class="block-title" id="emotes-h">Active set <span class="n"><bdi>{clean(set.name)}</bdi></span></h2>
        <label class="find">
          <span class="sr-only">Filter this set</span>
          <Icon name="search" size={16} />
          <input class="input" type="search" placeholder="Filter this set" bind:value={filter} autocomplete="off" />
        </label>
      </div>
      <p class="fine slots" aria-live="polite">
        {#if needle}{count(matches.length)} of {count(items.length)} match · {/if}{count(items.length)} of {count(set.capacity)} slots
        {#if fill >= 4}<span class="meter" aria-hidden="true"><span style:width="{fill}%"></span></span>{/if}
      </p>
      {#if matches.length === 0}
        <EmptyState rule title="Nothing matches “{filter.trim()}”.">
          Check the spelling, or clear the filter to see all {count(items.length)} emotes.
          {#snippet actions()}
            <button class="btn btn-s" type="button" onclick={() => (filter = '')}>Clear filter</button>
          {/snippet}
        </EmptyState>
      {:else}
        <EmoteGrid items={visible} creator={false} eager={12} label="{who}'s emotes" />
        {#if matches.length > visible.length}
          <div class="more">
            <button class="btn" type="button" onclick={() => (shown += STEP)}>
              Show more ({count(matches.length - visible.length)} left)
            </button>
          </div>
        {/if}
      {/if}
    </section>
  {/if}
</RailLayout>

<style>
  .labels { display: inline-grid; }
  .labels > span { grid-area: 1 / 1; text-align: left; }
  .off { visibility: hidden; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; }
  .n { margin-left: 6px; font-size: var(--t-small); font-weight: 500; color: var(--dim); }
  .find { position: relative; display: block; flex: 0 1 320px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 38px; padding-left: 36px; }
  @media (max-width: 520px) { .find { flex-basis: 100%; } }
  .slots { display: flex; align-items: center; gap: 12px; margin: 8px 0 20px; font-variant-numeric: tabular-nums; }
  .meter { flex: 0 1 160px; height: 4px; border-radius: 2px; background: var(--border-strong); overflow: hidden; }
  .meter span { display: block; height: 100%; background: var(--accent); }
  .more { display: flex; justify-content: center; padding: 24px 0 0; }
  .chat .fine { margin: 6px 0 20px; }
  .frame { max-width: 520px; }
  .phone-card { display: none; }
  @media (max-width: 899.98px) {
    .phone-card { display: block; padding: 16px 0; border-bottom: 1px solid var(--border); margin-bottom: 20px; }
  }
</style>
