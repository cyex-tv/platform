<script lang="ts">
  // "Chat preview" under a public set: how its emotes look in a Twitch or Kick chat (made-up
  // viewers; the owner's own lines carry their paint). Reuses ChatPreview; it moves only while on
  // screen and never under reduced motion (ChatPreview decides). In the set page's panel
  // (`inPanel`) the panel block names it, so it draws only the Twitch / Kick switch and the chat.
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { looks, NO_LOOK, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import { setChatLines } from '$lib/sets/chat';

  type Member = { name: string; zeroWidth: boolean; emote: Parameters<typeof setChatLines>[0][number]['emote'] };
  type Props = { members: readonly Member[]; owner: { id: string; displayName: string } | null; name: string; inPanel?: boolean };
  let { members, owner, name, inPanel = false }: Props = $props();

  let platform = $state<'twitch' | 'kick'>('twitch');
  $effect(() => {
    if (owner) wantLooks([owner.id]);
  });
  const lines = $derived(setChatLines(members, owner ? { name: owner.displayName, look: looks[owner.id] ?? NO_LOOK } : null));
  const kinds = [
    { key: 'twitch', label: 'Twitch' },
    { key: 'kick', label: 'Kick' },
  ] as const;
</script>

{#snippet pick()}
  <div class="pick" role="group" aria-label="Chat to preview">
    {#each kinds as k (k.key)}
      <button type="button" class:on={platform === k.key} aria-pressed={platform === k.key} onclick={() => (platform = k.key)}>{#if platform === k.key}<Icon name="check" size={13} stroke={2.4} />{/if}{k.label}</button>
    {/each}
  </div>
{/snippet}

{#if lines.length && inPanel}
  <div class="chat in-panel">
    <p class="fine">How this set’s emotes look in a chat. The viewers are made up.</p>
    {@render pick()}
    <div class="frame">
      {#key platform}
        <ChatPreview {platform} {lines} live visible={5} interval={3200} label="{platform === 'twitch' ? 'Twitch' : 'Kick'} chat with {clean(name)}’s emotes" />
      {/key}
    </div>
  </div>
{:else if lines.length}
  <section class="chat" aria-labelledby="chat-h">
    <div class="head">
      <h2 class="block-title" id="chat-h">Chat preview</h2>
      {@render pick()}
    </div>
    <p class="fine">How this set’s emotes look in a chat. The viewers are made up.</p>
    <div class="frame">
      {#key platform}
        <ChatPreview {platform} {lines} live visible={5} interval={3200} label="{platform === 'twitch' ? 'Twitch' : 'Kick'} chat with {clean(name)}’s emotes" />
      {/key}
    </div>
  </section>
{/if}

<style>
  .chat { margin-top: 40px; max-width: 720px; }
  .head { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 8px 16px; }
  .head :global(.block-title) { margin: 0; }
  .in-panel { margin: 0; max-width: none; display: grid; gap: 4px; }
  .in-panel .fine { margin: 0; }
  .in-panel .pick { margin-left: -12px; }
  .fine { margin: 6px 0 16px; font-size: var(--t-small); color: var(--muted); }
  .pick { display: inline-flex; gap: 4px; }
  /* The chosen one is bold, not boxed (owner rule). */
  .pick button { display: inline-flex; align-items: center; gap: 5px; min-height: 40px; padding: 0 12px; border: 0; background: none; font: inherit; font-size: var(--t-small); color: var(--muted);
    border-radius: var(--radius-btn); cursor: pointer; }
  .pick button.on { color: var(--text); font-weight: 700; }
  .pick button :global(svg) { color: var(--accent-text); }
  .pick button:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .pick button:hover { color: var(--text); } }
  @media (pointer: coarse) { .pick button { min-height: 44px; } }
</style>
