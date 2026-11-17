<script lang="ts">
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ProfileHead from '$lib/components/profile/ProfileHead.svelte';
  import ProfileState from '$lib/components/profile/ProfileState.svelte';
  import { chatLines } from '$lib/components/profile/chat';
  import { count } from '$lib/format';

  let { data } = $props();
  const c = $derived(data.channel);
  const set = $derived(data.set);
  const key = $derived(c.platform === 'TWITCH' ? ('twitch' as const) : ('kick' as const));
  const platform = $derived(key === 'twitch' ? 'Twitch' : 'Kick');
  const items = $derived(set?.emotes.map((m) => ({ ...m.emote, shownName: m.name })) ?? []);

  const STEP = 120;
  let filter = $state('');
  let shown = $state(STEP);
  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

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

  async function copy() {
    try {
      await navigator.clipboard.writeText(location.href);
      copied = true;
      clearTimeout(timer);
      timer = setTimeout(() => (copied = false), 2000);
    } catch {
      // Clipboard blocked: the address bar still has the link.
    }
  }
</script>

<svelte:head>
  <title>{c.account.displayName} - cyex.tv</title>
  <meta name="description" content="The cyex.tv emotes in {c.account.displayName}'s {platform} chat." />
</svelte:head>

<div class="page">
  <ProfileHead name={c.account.displayName} avatar={c.account.avatar} kind="{platform} channel">
    {#snippet meta()}
      {#if set}Active set: {set.name}{:else}No set picked yet{/if}
    {/snippet}
    {#snippet actions()}
      <div class="actions">
        <button class="btn btn-s" type="button" onclick={copy}>
          <Icon name={copied ? 'check' : 'copy'} size={16} />
          <span aria-live="polite">{copied ? 'Link copied' : 'Copy set link'}</span>
        </button>
        <a class="btn btn-s" href="/users/{c.account.id}">
          <Icon name="user" size={16} />Creator page
        </a>
      </div>
    {/snippet}
  </ProfileHead>

  {#if !set}
    <ProfileState title="No set picked yet.">
      {c.account.displayName} hasn't chosen an emote set for {platform}, so there is nothing to show here. Their
      emotes appear in chat as soon as they do.
      {#snippet actions()}
        <a class="btn" href="/search">Browse emotes</a>
      {/snippet}
    </ProfileState>
  {:else if items.length === 0}
    <ProfileState title="The set is empty.">
      {c.account.displayName} has a set, with room for {count(set.capacity)} emotes, but hasn't added any yet.
      {#snippet actions()}
        <a class="btn" href="/search">Browse emotes</a>
      {/snippet}
    </ProfileState>
  {:else}
    <section aria-labelledby="emotes-h">
      <div class="bar">
        <h2 class="block-title" id="emotes-h">Emotes</h2>
        <label class="find">
          <span class="sr-only">Filter this set</span>
          <Icon name="search" size={16} />
          <input class="input" type="search" placeholder="Filter this set" bind:value={filter} autocomplete="off" />
        </label>
      </div>
      <p class="fine slots" aria-live="polite">
        {#if needle}{count(matches.length)} of {count(items.length)} match · {/if}{count(items.length)} of {count(set.capacity)} slots
        <span class="meter" aria-hidden="true"><span style:width="{fill}%"></span></span>
      </p>
      {#if matches.length === 0}
        <ProfileState title="Nothing matches “{filter.trim()}”.">
          Check the spelling, or clear the filter to see all {count(items.length)} emotes.
          {#snippet actions()}
            <button class="btn btn-s" type="button" onclick={() => (filter = '')}>Clear filter</button>
          {/snippet}
        </ProfileState>
      {:else}
        <EmoteGrid items={visible} creator={false} eager={12} label="{c.account.displayName}'s emotes" />
        {#if matches.length > visible.length}
          <div class="more">
            <button class="btn" type="button" onclick={() => (shown += STEP)}>
              Show more ({count(matches.length - visible.length)} left)
            </button>
          </div>
        {/if}
      {/if}
    </section>

    {#if lines.length}
      <section class="chat" aria-labelledby="chat-h">
        <h2 class="block-title" id="chat-h">In chat</h2>
        <p class="fine">How these emotes look in a {platform} chat. The viewers are made up.</p>
        <div class="frame">
          <ChatPreview platform={key} {lines} visible={6} label="{platform} chat with {c.account.displayName}'s emotes" />
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .bar { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; }
  .find { position: relative; display: block; flex: 0 1 320px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 40px; padding-left: 36px; }
  @media (max-width: 520px) { .find { flex-basis: 100%; } }
  .slots { display: flex; align-items: center; gap: 12px; margin: 8px 0 20px; font-variant-numeric: tabular-nums; }
  .meter { flex: 0 1 160px; height: 2px; background: var(--border-strong); }
  .meter span { display: block; height: 100%; background: var(--muted); }
  .more { display: flex; justify-content: center; padding: 24px 0 0; }
  .chat { padding-top: clamp(48px, 6vw, 72px); }
  .chat .fine { margin: 6px 0 20px; }
  .frame { max-width: 520px; }
</style>
