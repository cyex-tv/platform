<script lang="ts">
  // "Used in": up to 8 channels with the emote in their active set (avatar, name, platform).
  // The page hides it when the API sends no list.
  import { count } from '$lib/format';
  import type { EmoteChannel } from './emote-page';

  let { channels, total = null }: { channels: EmoteChannel[]; total?: number | null } = $props();
  const shown = $derived(channels.slice(0, 8));
  const platformName = (p: EmoteChannel['platform']) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
</script>

<section class="used" aria-labelledby="used-h">
  <div class="head">
    <h2 id="used-h">Used in</h2>
    {#if total !== null && total > shown.length}<p class="fine">{shown.length} of {count(total)} channels</p>{/if}
  </div>
  {#if shown.length === 0}
    <p class="fine empty">No channel has it in its set yet.</p>
  {:else}
    <ul>
      {#each shown as c (c.platform + c.login)}
        <li>
          <a href="/channels/{c.platform.toLowerCase()}/{encodeURIComponent(c.login)}">
            {#if c.avatar}
              <img class="avatar" src={c.avatar} alt="" width="32" height="32" loading="lazy" />
            {:else}
              <span class="avatar blank" aria-hidden="true">{c.displayName.slice(0, 1).toUpperCase()}</span>
            {/if}
            <span class="name">{c.displayName}</span>
            <span class="platform">{platformName(c.platform)}</span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  p { margin: 0; }
  .head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 12px; }
  h2 { font-size: var(--t-h3); line-height: 1.3; letter-spacing: -.015em; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 4px 16px; }
  a { display: flex; align-items: center; gap: 12px; min-height: 48px; padding: 8px 10px; margin-inline: -10px;
    border-radius: var(--radius-s); color: var(--text); transition: background-color var(--dur-color); }
  .avatar { flex: none; width: 32px; height: 32px; border-radius: 50%; object-fit: cover; background: var(--surface-2); }
  .blank { display: grid; place-items: center; font-size: var(--t-fine); font-weight: 600; color: var(--muted); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-ui); font-weight: 500; }
  .platform { flex: none; font-size: var(--t-label); color: var(--muted); padding: 0 6px; height: 20px; display: inline-flex;
    align-items: center; border: 1px solid var(--border); border-radius: var(--radius-btn); }
  .empty { padding: 8px 0; }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--surface-2); } }
</style>
