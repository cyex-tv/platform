<script lang="ts">
  // The emote page's body, made once per emote (the route keys it by id), in ColumnLayout:
  // a header row (name, creator chip, labels, actions), then panels: the preview (real sizes,
  // background switch, chat) beside the details (facts, review status), then a tabbed panel with
  // the channels using it and more from its creator. Phones get the actions in a sticky bar.
  import { onMount } from 'svelte';
  import EmoteActions from '$lib/components/EmoteActions.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import EmoteFacts from './EmoteFacts.svelte';
  import PreviewStage from './PreviewStage.svelte';
  import TrustBlock from './TrustBlock.svelte';
  import UsedIn from './UsedIn.svelte';
  import { labels, trust, type ChatPool, type EmoteChannel, type PageEmote } from './emote-page';
  import { Viewer } from './viewer.svelte';

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

  type TabId = 'channels' | 'more';
  const tabs = $derived<{ id: TabId; label: string; n?: string }[]>([
    ...(channels ? [{ id: 'channels' as const, label: 'Channels', n: channelCount !== null ? count(channelCount) : undefined }] : []),
    ...(more.length && e.owner ? [{ id: 'more' as const, label: 'More from creator' }] : []),
  ]);
  let picked = $state<TabId | null>(null);
  const tab = $derived(tabs.find((x) => x.id === picked)?.id ?? tabs[0]?.id);

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
          {#if tags.length}
            <ul class="labels" aria-label="Labels">
              {#each tags as l}<li class="label">{l}</li>{/each}
            </ul>
          {/if}
        </div>
      </div>
      {#if public_}
        <div class="head-actions"><EmoteActions emoteId={e.id} {name} {viewer} /></div>
      {/if}
    </header>

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

    {#if tabs.length}
      <section class="panel tabbed" aria-label="Channels and more from the creator">
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
              >{x.label}{#if x.n}<span class="n">{x.n}</span>{/if}</button>
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
      </section>
    {/if}

    {#if public_}
      <div class="bar"><EmoteActions emoteId={e.id} {name} {viewer} variant="bar" /></div>
    {/if}
  </div>
</ColumnLayout>

<style>
  p { margin: 0; }
  .emote { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; padding-bottom: clamp(24px, 4vw, 40px); }
  .back { justify-self: start; display: inline-flex; align-items: center; gap: 8px; min-height: 36px; color: var(--muted); font-size: var(--t-small);
    transition: color var(--dur-color); }
  .back :global(.icon) { transition: transform var(--dur-move) var(--ease); }

  .head { min-width: 0; display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px 24px; }
  .title { min-width: 0; flex: 1 1 320px; }
  h1 { font-size: clamp(24px, 2.6vw, 30px); line-height: 1.15; letter-spacing: -.025em; overflow-wrap: anywhere; text-wrap: balance; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; margin-top: 10px; }
  .creator { display: inline-flex; align-items: center; gap: 8px; min-height: 32px; padding: 0 10px 0 6px; max-width: 100%; min-width: 0;
    border: 1px solid var(--border-strong); border-radius: var(--radius-btn); background: var(--surface); color: var(--text);
    font-size: var(--t-small); font-weight: 500; transition: border-color var(--fast), background-color var(--fast); }
  .creator > span:last-child { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .labels { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }
  .label { display: inline-flex; align-items: center; height: 24px; padding: 0 8px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); font-size: var(--t-label); color: var(--muted); }

  .notice { padding: 12px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card); background: var(--surface);
    font-size: var(--t-small); max-width: 72ch; }
  .notice strong { font-weight: 600; }

  .panel { min-width: 0; padding: 20px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); }
  .panels { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 16px; align-items: stretch; }
  .side { display: grid; gap: 16px; }

  .tabbed { padding-top: 12px; }
  .tabbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 16px; }
  .tablist { display: flex; gap: 4px; padding: 3px; margin: -3px; overflow-x: auto; scrollbar-width: none; min-width: 0; }
  .tabbtn { display: inline-flex; align-items: center; gap: 8px; height: 36px; padding: 0 12px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-small); font-weight: 500; white-space: nowrap; cursor: pointer;
    transition: background-color var(--fast), color var(--fast); touch-action: manipulation; }
  .tabbtn[aria-selected='true'] { background: var(--accent-dim); color: var(--accent-text); }
  .n { color: var(--dim); font-variant-numeric: tabular-nums; font-size: var(--t-label); }
  .tabbtn[aria-selected='true'] .n { color: inherit; }
  .tabbar .link { flex: none; white-space: nowrap; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }

  .bar { display: none; }

  @media (hover: hover) and (pointer: fine) {
    .back:hover { color: var(--text); }
    .back:hover :global(.icon) { transform: translateX(-3px); }
    .creator:hover { border-color: var(--accent-border); background: var(--surface-hover); }
    .tabbtn:hover:not([aria-selected='true']) { background: var(--row-hover); color: var(--text); }
  }

  @media (max-width: 1000px) { .panels { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 560px) { .panel { padding: 16px; } }

  /* Phones: the actions move to a bar stuck to the bottom of the screen, above the home indicator. */
  @media (max-width: 720px) {
    :global(html:has(.emote.with-bar)) { scroll-padding-bottom: 88px; }
    .head-actions { display: none; }
    .bar { display: block; position: sticky; bottom: 0; z-index: var(--z-nav); margin: 24px calc(-1 * var(--page-pad)) 0;
      padding: 10px var(--page-pad) calc(10px + env(safe-area-inset-bottom, 0px));
      border-top: 1px solid var(--border); background: color-mix(in srgb, var(--bg) 86%, transparent);
      backdrop-filter: blur(14px) saturate(1.4); -webkit-backdrop-filter: blur(14px) saturate(1.4); }
  }
  @media (max-width: 720px) and (max-height: 500px) { .bar { position: static; } }
  @media (max-width: 720px) and ((prefers-reduced-transparency: reduce) or (prefers-contrast: more)) {
    .bar { background: var(--bg); backdrop-filter: none; -webkit-backdrop-filter: none; border-top-color: var(--border-strong); }
  }
  @media (prefers-reduced-motion: reduce) { .back :global(.icon) { transition: none; } }
</style>
