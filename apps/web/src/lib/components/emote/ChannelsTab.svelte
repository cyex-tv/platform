<script lang="ts">
  // The Channels tab: the channels with the emote in their active set, newest first, 24 to a page
  // (avatar, name, platform). The first page comes with the page; the others are asked for when
  // paged to and kept. The pager itself sits in the tab bar, so this hands its state up (`pager`).
  // Each name is drawn in its account's paint: the page's accounts are asked for together (one
  // request per page of 24, each account once, shared with every other name on the site).
  import { SvelteMap } from 'svelte/reactivity';
  import Avatar from '$lib/components/Avatar.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { channelPage } from './data';
  import { CHANNEL_PAGE, rangeLabel, type EmoteChannel } from './emote-page';
  import type { PagerState } from './pager';

  type Props = { emoteId: string; first: EmoteChannel[]; total: number | null; pager?: PagerState | null };
  let { emoteId, first, total, pager = $bindable(null) }: Props = $props();

  // svelte-ignore state_referenced_locally
  const pages = new SvelteMap<number, EmoteChannel[]>([[0, first]]);
  let at = $state(0);
  let busy = $state(false);
  /** The page that didn't load, to try again. */
  let failed = $state<number | null>(null);

  const rows = $derived(pages.get(at) ?? []);
  $effect(() => wantLooks(rows.map((c) => c.accountId)));
  const last = $derived(total !== null ? Math.max(0, Math.ceil(total / CHANNEL_PAGE) - 1) : rows.length < CHANNEL_PAGE ? at : at + 1);

  async function go(to: number) {
    if (busy || to < 0) return;
    failed = null;
    if (!pages.has(to)) {
      busy = true;
      try {
        pages.set(to, await channelPage(emoteId, to * CHANNEL_PAGE));
      } catch {
        failed = to;
        return;
      } finally {
        busy = false;
      }
    }
    at = to;
  }

  $effect(() => {
    pager = {
      range: rangeLabel(at * CHANNEL_PAGE, rows.length, total),
      prev: at > 0,
      next: at < last,
      busy,
      onprev: () => go(at - 1),
      onnext: () => go(at + 1),
    };
  });
  const platformName = (p: EmoteChannel['platform']) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
</script>

{#if failed !== null}
  <Problem cause="server" onretry={() => failed !== null && go(failed)} home={{ href: '/search', label: 'Browse emotes' }} />
{:else if rows.length === 0}
  <p class="empty">No channel has it in its set yet.</p>
{:else}
  <ul class:busy>
    {#each rows as c (c.platform + c.login)}
      <li>
        <a href="/channels/{c.platform.toLowerCase()}/{encodeURIComponent(c.login)}">
          <Avatar src={c.avatar} name={c.displayName} size={28} />
          <span class="name"><PaintedName name={c.displayName} paint={looks[c.accountId]?.paint} /></span>
          <span class="platform">{platformName(c.platform)}</span>
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  p { margin: 0; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 4px 12px;
    transition: opacity var(--dur-color); }
  ul.busy { opacity: .6; }
  a { display: flex; align-items: center; gap: 10px; min-height: 44px; padding: 6px 10px; border: 1px solid transparent;
    border-radius: var(--radius-btn); color: var(--text); transition: background-color var(--fast), border-color var(--fast); }
  .name { flex: 0 1 auto; display: flex; min-width: 0; font-size: var(--t-ui); font-weight: 500; }
  .platform { flex: none; font-size: var(--t-label); color: var(--muted); }
  .empty { padding: 8px 0; color: var(--muted); font-size: var(--t-small); }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--row-hover); border-color: var(--border); } }
  @media (max-width: 480px) { ul { grid-template-columns: minmax(0, 1fr); } }
</style>
