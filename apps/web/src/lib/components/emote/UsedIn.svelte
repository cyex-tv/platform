<script lang="ts">
  // "Used in": up to 8 channels with the emote in their active set (avatar, name, platform).
  // The page hides it when the API sends no list.
  import Avatar from '$lib/components/Avatar.svelte';
  import { clean, count } from '$lib/format';
  import type { EmoteChannel } from './emote-page';

  let { channels, total = null }: { channels: EmoteChannel[]; total?: number | null } = $props();
  const shown = $derived(channels.slice(0, 8));
  const platformName = (p: EmoteChannel['platform']) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
</script>

<section class="used" aria-labelledby="used-h">
  <div class="block-head">
    <h2 class="block-title" id="used-h">Used in</h2>
    {#if total !== null && total > shown.length}<p class="fine">{shown.length} of {count(total)} channels</p>{/if}
  </div>
  {#if shown.length === 0}
    <p class="fine empty">No channel has it in its set yet.</p>
  {:else}
    <ul>
      {#each shown as c (c.platform + c.login)}
        <li>
          <a href="/channels/{c.platform.toLowerCase()}/{encodeURIComponent(c.login)}">
            <Avatar src={c.avatar} name={c.displayName} size={32} />
            <span class="name"><bdi>{clean(c.displayName)}</bdi></span>
            <span class="platform">{platformName(c.platform)}</span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  p { margin: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 4px 16px; }
  a { display: flex; align-items: center; gap: 12px; min-height: 48px; padding: 8px 10px; margin-inline: -10px;
    border-radius: var(--radius-s); color: var(--text); transition: background-color var(--dur-color); }
  .name { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-ui); font-weight: 500; }
  .platform { flex: none; font-size: var(--t-label); color: var(--muted); padding: 0 6px; height: 20px; display: inline-flex;
    align-items: center; border: 1px solid var(--border); border-radius: var(--radius-btn); }
  .empty { padding: 8px 0; }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--surface-2); } }
</style>
