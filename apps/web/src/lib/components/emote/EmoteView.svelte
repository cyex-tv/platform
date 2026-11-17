<script lang="ts">
  // The emote page's body, made once per emote (the route keys it by id): head with actions,
  // preview stage, facts, review, channels using it and more from its creator. Phones get the
  // preview first and the actions in a sticky bar.
  import { onMount } from 'svelte';
  import EmoteActions from '$lib/components/EmoteActions.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean } from '$lib/format';
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

  function goBack(ev: MouseEvent) {
    // Back to the browse view the reader came from keeps its scroll and loaded results.
    if (!back.back || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.button !== 0) return;
    ev.preventDefault();
    history.back();
  }
</script>

<div class="page emote" class:with-bar={public_}>
  <a class="back" href={back.href} onclick={goBack}><Icon name="arrow-left" />{back.label}</a>

  <div class="layout">
    <header class="head">
      <div class="title">
        <h1><bdi>{name}</bdi></h1>
        <div class="meta">
          {#if e.owner}
            <a class="creator" href="/users/{e.owner.id}">
              <Avatar src={e.owner.avatar} name={e.owner.displayName} size={24} />
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

    <div class="stage-area"><PreviewStage emote={e} {pool} /></div>

    <aside class="side">
      <EmoteFacts emote={e} channels={channelCount} />
      <TrustBlock emote={e} {viewer} />
    </aside>
  </div>

  {#if channels}
    <div class="block"><UsedIn {channels} total={channelCount} /></div>
  {/if}

  {#if more.length && e.owner}
    <section class="block more" aria-labelledby="more-h">
      <div class="block-head">
        <h2 class="block-title" id="more-h">More from {ownerName}</h2>
        <a class="link" href="/users/{e.owner.id}">
          See all
          <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
        </a>
      </div>
      <EmoteGrid items={more} creator={false} skeletons={0} label="More from {ownerName}" />
    </section>
  {/if}

  {#if public_}
    <div class="bar"><EmoteActions emoteId={e.id} {name} {viewer} variant="bar" /></div>
  {/if}
</div>

<style>
  p { margin: 0; }
  .emote { padding-top: clamp(16px, 3vw, 32px); }
  .back { display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small);
    transition: color var(--dur-color); }
  .back :global(.icon) { transition: transform var(--dur-move) var(--ease); }

  .layout { display: grid; grid-template-columns: minmax(0, 1.45fr) minmax(300px, 1fr); gap: 32px 56px; margin-top: 12px;
    grid-template-areas: 'head head' 'notice notice' 'stage side'; align-items: start; }
  .head { grid-area: head; min-width: 0; display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: space-between; gap: 20px 32px; }
  .notice { grid-area: notice; }
  .stage-area { grid-area: stage; min-width: 0; }
  .side { grid-area: side; display: grid; gap: 24px; min-width: 0; }

  .title { min-width: 0; flex: 1 1 320px; }
  h1 { font-size: clamp(36px, 4.6vw, 64px); line-height: 1.02; letter-spacing: -.04em; overflow-wrap: anywhere; text-wrap: balance; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 10px 16px; margin-top: 14px; }
  .creator { display: inline-flex; align-items: center; gap: 8px; min-height: 32px; color: var(--text); font-size: var(--t-ui);
    font-weight: 500; transition: color var(--dur-color); max-width: 100%; min-width: 0; }
  .creator > span:last-child { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .labels { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; gap: 6px; }
  .label { display: inline-flex; align-items: center; height: 22px; padding: 0 7px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); font-size: var(--t-label); color: var(--muted); }

  .notice { padding: 14px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-s); background: var(--surface);
    font-size: var(--t-small); max-width: 72ch; }
  .notice strong { font-weight: 600; }

  .block { margin-top: clamp(48px, 6vw, 72px); }
  .block-head .link { flex: none; white-space: nowrap; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }

  .bar { display: none; }

  @media (hover: hover) and (pointer: fine) {
    .back:hover { color: var(--text); }
    .back:hover :global(.icon) { transform: translateX(-3px); }
    .creator:hover { color: var(--accent-text); }
  }

  @media (min-width: 901px) {
    .stage-area { position: sticky; top: calc(var(--nav-h, 64px) + 16px); }
  }

  @media (max-width: 900px) {
    .layout { grid-template-columns: minmax(0, 1fr); grid-template-areas: 'stage' 'head' 'notice' 'side'; gap: 24px; }
  }

  /* Phones: the actions move to a bar stuck to the bottom of the screen, above the home indicator. */
  @media (max-width: 720px) {
    :global(html:has(.emote.with-bar)) { scroll-padding-bottom: 88px; }
    .head-actions { display: none; }
    .with-bar { padding-bottom: 0; }
    .bar { display: block; position: sticky; bottom: 0; z-index: var(--z-nav); margin: clamp(48px, 8vw, 80px) calc(-1 * var(--page-pad)) 0;
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
