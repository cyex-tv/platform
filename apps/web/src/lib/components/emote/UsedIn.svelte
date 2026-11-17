<script lang="ts">
  // The Channels tab's body: up to 8 channels with the emote in their active set (avatar, name,
  // platform). The page hides the tab when the API sends no list.
  import Avatar from '$lib/components/Avatar.svelte';
  import { clean, count } from '$lib/format';
  import type { EmoteChannel } from './emote-page';

  let { channels, total = null }: { channels: EmoteChannel[]; total?: number | null } = $props();
  const shown = $derived(channels.slice(0, 8));
  const platformName = (p: EmoteChannel['platform']) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
</script>

{#if shown.length === 0}
  <p class="fine empty">No channel has it in its set yet.</p>
{:else}
  <ul>
    {#each shown as c (c.platform + c.login)}
      <li>
        <a href="/channels/{c.platform.toLowerCase()}/{encodeURIComponent(c.login)}">
          <Avatar src={c.avatar} name={c.displayName} size={24} />
          <span class="name"><bdi>{clean(c.displayName)}</bdi></span>
          <span class="platform">{platformName(c.platform)}</span>
        </a>
      </li>
    {/each}
  </ul>
  {#if total !== null && total > shown.length}<p class="fine more">{shown.length} of {count(total)} channels</p>{/if}
{/if}

<style>
  p { margin: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 4px 12px; }
  a { display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 6px 10px; border: 1px solid transparent;
    border-radius: var(--radius-btn); color: var(--text); transition: background-color var(--fast), border-color var(--fast); }
  .name { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-small); font-weight: 500; }
  .platform { flex: none; margin-left: 2px; font-size: var(--t-label); color: var(--muted); }
  .empty { padding: 8px 0; }
  .more { margin-top: 12px; color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--row-hover); border-color: var(--border); } }
</style>
