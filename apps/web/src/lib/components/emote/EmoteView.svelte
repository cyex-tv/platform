<script lang="ts">
  // The emote page's body, made once per emote (the route keys it by id), in ColumnLayout:
  // a header row (name, creator chip, labels, actions), then panels: the preview (real sizes,
  // background switch, chat) beside the details (facts, review status), then a panel with the
  // channels using it and more from its creator: tabs when there are both, a plain heading when
  // there is one. Phones get the actions in a bar fixed to the bottom of the screen; it comes right
  // after the header in the Tab order.
  import { onMount } from 'svelte';
  import EmoteActions from '$lib/components/EmoteActions.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import { clean, count } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import ActivityTab from './ActivityTab.svelte';
  import EmoteFacts from './EmoteFacts.svelte';
  import PreviewStage from './PreviewStage.svelte';
  import TrustBlock from './TrustBlock.svelte';
  import UsedIn from './UsedIn.svelte';
  import { labels, trust, type ChatPool, type EmoteChannel, type PageEmote } from './emote-page';
  import { Viewer } from './viewer.svelte';
  import { visibilityOf } from '$lib/tool/visibility';

  type Props = {
    emote: PageEmote;
    pool: ChatPool;
    more: PageEmote[];
    channels: EmoteChannel[] | null;
    channelCount: number | null;
    back: { href: string; label: string; back: boolean };
  };
  let { emote: e, pool, more, channels, channelCount, back }: Props = $props();

  // svelte-ignore state_referenced_locally
  const viewer = new Viewer(e.id);
  onMount(() => void viewer.load());

  const public_ = $derived(e.state === 'APPROVED');
  const t = $derived(trust(e.state));
  const tags = $derived(labels(e));
  const name = $derived(clean(e.name));
  const ownerName = $derived(e.owner ? clean(e.owner.displayName) : '');

  // Activity is only for listed emotes: pending, rejected, taken-down and private ones have none,
  // so the tab isn't there and nothing is asked.
  const listed = $derived(public_ && !e.flags.private);
  // Unlisted and private emotes say so beside their labels (ADR 0020): people who got the link
  // know it isn't in search.
  const vis = $derived(visibilityOf(e));

  type TabId = 'channels' | 'more' | 'activity';
  const tabs = $derived<{ id: TabId; label: string; short?: string; icon: IconName; n?: string }[]>([
    ...(channels ? [{ id: 'channels' as const, label: 'Channels', icon: 'tv' as const, n: channelCount !== null ? count(channelCount) : undefined }] : []),
    ...(more.length && e.owner ? [{ id: 'more' as const, label: 'More from creator', short: 'More', icon: 'grid' as const }] : []),
    ...(listed ? [{ id: 'activity' as const, label: 'Activity', icon: 'activity' as const }] : []),
  ]);
  let picked = $state<TabId | null>(null);
  const tab = $derived(tabs.find((x) => x.id === picked)?.id ?? tabs[0]?.id);
  // The Activity tab loads when it is first opened, then keeps its rows while you switch tabs.
  let seen = $state(false);
  $effect(() => {
    if (tab === 'activity') seen = true;
  });

  // Arrow keys move between the tabs, Home and End jump (one tab stop for the list).
  function tabKey(ev: KeyboardEvent) {
    const i = tabs.findIndex((x) => x.id === tab);
    const to = ev.key === 'ArrowRight' ? i + 1 : ev.key === 'ArrowLeft' ? i - 1 : ev.key === 'Home' ? 0 : ev.key === 'End' ? tabs.length - 1 : null;
    if (to === null) return;
    ev.preventDefault();
    const next = tabs[(to + tabs.length) % tabs.length].id;
    picked = next;
    queueMicrotask(() => document.getElementById(`tab-${next}`)?.focus());
  }

  function goBack(ev: MouseEvent) {
    // Back to the browse view the reader came from keeps its scroll and loaded results.
    if (!back.back || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.button !== 0) return;
    ev.preventDefault();
    history.back();
  }
</script>

<ColumnLayout top>
  <div class="emote" class:with-bar={public_}>
    <a class="back" href={back.href} onclick={goBack}><Icon name="arrow-left" />{back.label}</a>

    <header class="head">
      <div class="title">
        <h1><bdi>{name}</bdi></h1>
        <div class="meta">
          {#if e.owner}
            <a class="creator" href="/users/{e.owner.id}">
              <Avatar src={e.owner.avatar} name={e.owner.displayName} size={20} />
              <span><span class="sr-only">By </span><bdi>{ownerName}</bdi></span>
            </a>
          {/if}
          {#if tags.length || vis !== 'PUBLIC'}
            <ul class="labels" aria-label="Labels">
              {#if vis === 'UNLISTED'}
                <li class="label vis" title="Only people with the link can find it"><Icon name="link" size={12} />Unlisted</li>
              {:else if vis === 'PRIVATE'}
                <li class="label vis" title="Only its creator can add it to sets"><Icon name="shield" size={12} />Private</li>
              {/if}
              {#each tags as l}<li class="label">{l}</li>{/each}
            </ul>
          {/if}
        </div>
      </div>
      {#if public_}
        <div class="head-actions"><EmoteActions emoteId={e.id} {name} {viewer} picture={e} /></div>
      {/if}
    </header>

    {#if public_}
      <div class="bar"><EmoteActions emoteId={e.id} {name} {viewer} variant="bar" picture={e} /></div>
    {/if}

    {#if !public_}
      <p class="notice {t.tone}" role="status"><strong>{t.status}.</strong> Only you and moderators can open this page.</p>
    {/if}

    <div class="panels">
      <div class="panel"><PreviewStage emote={e} {pool} /></div>
      <aside class="panel side" aria-label="Details">
        <EmoteFacts emote={e} channels={channelCount} />
        <TrustBlock emote={e} {viewer} />
      </aside>
    </div>

    {#if tabs.length === 1}
      {@const only = tabs[0]}
      <section class="panel tabbed" aria-labelledby="sec-{only.id}">
        <div class="block-head">
          <h2 class="block-title" id="sec-{only.id}">{only.id === 'more' ? `More from ${ownerName}` : only.label}{#if only.n}<span class="n">{only.n}</span>{/if}</h2>
          {#if only.id === 'more' && e.owner}
            <a class="link" href="/users/{e.owner.id}">
              See all
              <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
            </a>
          {/if}
        </div>
        {#if only.id === 'channels' && channels}
          <UsedIn {channels} total={channelCount} />
        {:else if only.id === 'activity'}
          <ActivityTab emoteId={e.id} />
        {:else}
          <EmoteGrid items={more} creator={false} skeletons={0} label="More from {ownerName}" />
        {/if}
      </section>
    {:else if tabs.length}
      <section class="panel tabbed" aria-label="Channels, more from the creator and activity">
        <div class="tabbar">
          <div class="tablist" role="tablist" aria-label="Sections">
            {#each tabs as x (x.id)}
              <button
                type="button"
                role="tab"
                id="tab-{x.id}"
                class="tabbtn"
                aria-selected={tab === x.id}
                aria-controls="panel-{x.id}"
                tabindex={tab === x.id ? 0 : -1}
                onclick={() => (picked = x.id)}
                onkeydown={tabKey}
              ><Icon name={x.icon} size={16} />{#if x.short}<span class="long">{x.label}</span><span class="short">{x.short}</span>{:else}{x.label}{/if}{#if x.n}<span class="n">{x.n}</span>{/if}</button>
            {/each}
          </div>
          {#if tab === 'more' && e.owner}
            <a class="link" href="/users/{e.owner.id}">
              See all
              <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
            </a>
          {/if}
        </div>
        {#if channels}
          <div role="tabpanel" id="panel-channels" aria-labelledby="tab-channels" hidden={tab !== 'channels'}>
            <UsedIn {channels} total={channelCount} />
          </div>
        {/if}
        {#if more.length && e.owner}
          <div role="tabpanel" id="panel-more" aria-labelledby="tab-more" hidden={tab !== 'more'}>
            <EmoteGrid items={more} creator={false} skeletons={0} label="More from {ownerName}" />
          </div>
        {/if}
        {#if listed && seen}
          <div role="tabpanel" id="panel-activity" aria-labelledby="tab-activity" hidden={tab !== 'activity'}>
            <ActivityTab emoteId={e.id} />
          </div>
        {/if}
      </section>
    {/if}
  </div>
</ColumnLayout>

<style>
  p { margin: 0; }
  .emote { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; padding-bottom: clamp(24px, 4vw, 40px); }
  .back { justify-self: start; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small);
    transition: color var(--dur-color); }
  .back :global(.icon) { transition: transform var(--dur-move) var(--ease); }

  .head { min-width: 0; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; }
  .title { min-width: 0; flex: 1 1 320px; }
  h1 { font-size: var(--t-page); line-height: 1.25; letter-spacing: -.02em; overflow-wrap: anywhere; text-wrap: balance; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; margin-top: 10px; }
  .creator { position: relative; display: inline-flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 10px 0 6px; max-width: 100%; min-width: 0;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface); color: var(--text);
    font-size: var(--t-small); font-weight: 500; transition: border-color var(--fast), background-color var(--fast); }
  .creator::after { content: ''; position: absolute; inset: -6px 0; } /* 44px to hit */
  .creator > span:last-child { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .labels { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }
  .label { display: inline-flex; align-items: center; height: 24px; padding: 0 8px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); font-size: var(--t-label); color: var(--muted); }
  .label.vis { gap: 4px; padding-left: 6px; color: var(--text); border-color: var(--border-strong); }

  .notice { padding: 12px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card); background: var(--surface);
    font-size: var(--t-small); max-width: 72ch; }
  .notice strong { font-weight: 600; }

  .panel { min-width: 0; padding: 20px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .panels { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 16px; align-items: stretch; }
  .side { display: grid; gap: 16px; }

  .tabbed { padding-top: 12px; }
  .tabbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 16px; }
  .tablist { display: flex; gap: 4px; padding: 3px; margin: -3px; overflow-x: auto; scrollbar-width: none; min-width: 0; }
  .tabbtn { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 12px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-small); font-weight: 500; white-space: nowrap; cursor: pointer;
    transition: background-color var(--fast), color var(--fast); touch-action: manipulation; }
  .tabbtn[aria-selected='true'] { color: var(--text); font-weight: 600; }
  .tabbtn :global(.icon) { color: var(--dim); }
  .tabbtn[aria-selected='true'] :global(.icon) { color: var(--accent-text); }
  .n { color: var(--dim); font-variant-numeric: tabular-nums; font-size: var(--t-label); }
  .block-title .n { margin-left: 8px; font-size: var(--t-small); font-weight: 500; }
  .tabbed .block-head { margin-top: 4px; }
  .tabbtn:focus-visible { outline-offset: -2px; }
  .tabbtn[aria-selected='true'] .n { color: inherit; }
  .tabbar .link { flex: none; white-space: nowrap; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }

  .bar { display: none; }
  .short { display: none; }

  @media (hover: hover) and (pointer: fine) {
    .back:hover { color: var(--text); }
    .back:hover :global(.icon) { transform: translateX(-3px); }
    .creator:hover { border-color: var(--accent-border); background: var(--surface-hover); }
    .tabbtn:hover:not([aria-selected='true']) { color: var(--text); }
  }

  @media (pointer: coarse) { .back, .tabbtn { min-height: 44px; } }

  @media (max-width: 1000px) { .panels { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 560px) {
    .panel { padding: 16px; }
    .long { display: none; }
    .short { display: inline; }
    .tablist { gap: 0; }
    .tabbtn { padding: 0 8px; gap: 6px; }
  }

  /* Phones: the actions move to a bar fixed to the bottom of the screen, above the home indicator.
     The page ends 72px higher, so the bar never covers the footer. */
  @media (max-width: 720px) {
    :global(html:has(.emote.with-bar)) { scroll-padding-bottom: 88px; }
    :global(body:has(.emote.with-bar)) { padding-bottom: calc(65px + env(safe-area-inset-bottom, 0px)); }
    .head-actions { display: none; }
    .bar { display: block; position: fixed; left: 0; right: 0; bottom: 0; z-index: var(--z-nav);
      padding: 10px var(--page-pad) calc(10px + env(safe-area-inset-bottom, 0px));
      border-top: 1px solid var(--border); background: color-mix(in srgb, var(--bg) 86%, transparent);
      backdrop-filter: blur(14px) saturate(1.4); -webkit-backdrop-filter: blur(14px) saturate(1.4); }
  }
  @media (max-width: 720px) and (max-height: 500px) {
    .bar { position: static; margin-inline: calc(-1 * var(--page-pad)); border-bottom: 1px solid var(--border); }
    :global(body:has(.emote.with-bar)) { padding-bottom: 0; }
  }
  @media (max-width: 720px) and ((prefers-reduced-transparency: reduce) or (prefers-contrast: more)) {
    .bar { background: var(--bg); backdrop-filter: none; -webkit-backdrop-filter: none; border-top-color: var(--border-strong); }
  }
  @media (prefers-reduced-motion: reduce) { .back :global(.icon) { transition: none; } }
</style>
