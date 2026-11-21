<script lang="ts">
  // The emote page's body, made once per emote (the route keys it by id): the Inspector layout
  // (redesign, 2026/11/21). A breadcrumb back to where you came from, then two columns:
  //   left  the preview: the emote on one neutral stage (no ground switch, owner rule), the four
  //         sizes under it as buttons (scale, pixels, file size; the one on the stage is bold with
  //         an accent tick), and a line of chat with the emote in it at chat size;
  //   right the name, the creator in their paint and the upload date, flags and tags, the
  //         actions (add to set, edit, more), a grid of facts, then the
  //         tabs (Channels using it, Activity, More by the creator; `?tab=` in the address) with
  //         a pager in the tab bar, and last the staff flags panel.
  // Under 900px the columns stack; phones get the actions in a bar fixed to the bottom.
  import { onMount } from 'svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import EmoteFlagsPanel from '$lib/admin/EmoteFlagsPanel.svelte';
  import EmoteActions from '$lib/components/EmoteActions.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import { hasStill } from '$lib/emote';
  import { bytes, clean, count, date } from '$lib/format';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { ready } from '$lib/emote';
  import { can, session } from '$lib/tool/session.svelte';
  import { editorOf, type Grant } from '$lib/profile/data.svelte';
  import { emoteCan } from '$lib/tool/editors';
  import ActivityTab from './ActivityTab.svelte';
  import ChannelsTab from './ChannelsTab.svelte';
  import Pager from './Pager.svelte';
  import { canReport, fileSize, flagsOf, formats, kind, scaleFiles, tabFrom, tabUrl, tabUrlStale, trust, type EmoteChannel, type PageEmote } from './emote-page';
  import type { PagerState } from './pager';
  import { Viewer } from './viewer.svelte';
  import { bottomBar } from '$lib/tool/bottom-bars.svelte';
  import { registerShortcuts } from '$lib/shell/shortcuts.svelte';
  import { EMOTE_KEYS } from './keys';

  type Props = {
    emote: PageEmote;
    more: PageEmote[];
    channels: EmoteChannel[] | null;
    channelCount: number | null;
    back: { href: string; label: string; back: boolean };
    /** Its place among the trending emotes of the last 7 days (top 100), or null. */
    trending?: { rank: number; channels: number } | null;
  };
  let { emote: e, more, channels, channelCount, back, trending = null }: Props = $props();
  $effect(() => registerShortcuts(EMOTE_KEYS));

  // svelte-ignore state_referenced_locally
  const viewer = new Viewer(e.id);
  onMount(() => void viewer.load());

  const public_ = $derived(e.state === 'APPROVED');
  const t = $derived(trust(e.state));
  const name = $derived(clean(e.name));
  const ownerName = $derived(e.owner ? clean(e.owner.displayName) : '');
  const flags = $derived(flagsOf(e));
  const uploaded = $derived(date(e.createdAt));
  const kinds = $derived(formats(e.files));
  const files = $derived(scaleFiles(e.files));
  const biggest = $derived(files.at(-1));
  // The size on the stage: the biggest until you pick another.
  let picked = $state<number | null>(null);
  const onStage = $derived(files.find((f) => f.scale === picked) ?? biggest);
  const chatFile = $derived(files.find((f) => f.scale === 2) ?? files[0]);
  const hasStillFiles = $derived(e.animated && e.files.some((f) => f.name === '1x_static.webp'));
  const fileUrl = (f: { scale: number; name: string }, frozen: boolean) => (frozen ? `${e.host}/${f.scale}x_static.webp` : `${e.host}/${f.name}`);
  // The creator's name in their paint, as on every tile.
  $effect(() => {
    if (e.owner?.id) wantLooks([e.owner.id]);
  });
  const ownerPaint = $derived(e.owner?.id ? (looks[e.owner.id]?.paint ?? null) : null);
  // Edit: the owner, editors of the owner's account with Manage on every set (ADR 0014), and staff
  // who may change any emote (the API checks again).
  let grants = $state<readonly Grant[] | null>(null);
  $effect(() => {
    const me = session.me;
    if (!me || !e.owner || e.owner.id === me.id) return;
    editorOf()
      .then((g) => (grants = g))
      .catch(() => {});
  });
  const editHref = $derived(
    session.me && (emoteCan(grants, e.owner?.id, session.me.id, 'EMOTE_MANAGE') || can('emotes.edit_any'))
      ? `/me/emotes/${encodeURIComponent(e.id)}`
      : null,
  );

  // Pause: null follows the reader's motion setting (decided by CSS before the script runs).
  let paused = $state<boolean | null>(null);
  let reduced = $state(false);
  $effect(() => {
    const mq = matchMedia('(prefers-reduced-motion: reduce)');
    reduced = mq.matches;
    const on = () => (reduced = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });
  const still = $derived(paused ?? reduced);
  const frozen = $derived(hasStillFiles && still);

  // Activity is only for listed emotes: pending, rejected, taken-down and private ones have none.
  const listed = $derived(public_ && !e.flags.private);

  type TabId = 'channels' | 'activity' | 'more';
  const tabs = $derived<{ id: TabId; label: string; short?: string; icon: IconName; n?: string }[]>([
    ...(channels ? [{ id: 'channels' as const, label: 'Channels', icon: 'tv' as const, n: channelCount !== null ? count(channelCount) : undefined }] : []),
    ...(listed ? [{ id: 'activity' as const, label: 'Activity', icon: 'activity' as const }] : []),
    ...(more.length && e.owner ? [{ id: 'more' as const, label: `More by ${ownerName}`, short: 'More', icon: 'grid' as const }] : []),
  ]);
  // The tab lives in the address (`?tab=activity`; the first tab has none), so a link or a reload
  // opens it and back and forward step between tabs. Switching keeps the scroll and the focus.
  const asked = $derived(page.url.searchParams.get('tab'));
  const tab = $derived(tabFrom(asked, tabs.map((x) => x.id)));
  function show(id: TabId, replace = false) {
    if (id === tab) return;
    void goto(tabUrl(page.url, id, tabs[0]?.id), { noScroll: true, keepFocus: true, replaceState: replace });
  }
  // An unknown tab (or the default one spelled out) is tidied to the clean address.
  $effect(() => {
    if (tabs.length && tabUrlStale(asked, tab, tabs[0]?.id)) void goto(tabUrl(page.url, tab ?? '', tabs[0]?.id), { noScroll: true, keepFocus: true, replaceState: true });
  });
  // The Activity tab loads when it is first opened, then keeps its pages while you switch tabs.
  let seen = $state(false);
  $effect(() => {
    if (tab === 'activity') seen = true;
  });
  let channelPager = $state<PagerState | null>(null);
  let activityPager = $state<PagerState | null>(null);
  const pager = $derived(tab === 'channels' ? channelPager : tab === 'activity' ? activityPager : null);

  // Arrow keys move between the tabs, Home and End jump (one tab stop for the list).
  function tabKey(ev: KeyboardEvent) {
    const i = tabs.findIndex((x) => x.id === tab);
    const to = ev.key === 'ArrowRight' ? i + 1 : ev.key === 'ArrowLeft' ? i - 1 : ev.key === 'Home' ? 0 : ev.key === 'End' ? tabs.length - 1 : null;
    if (to === null) return;
    ev.preventDefault();
    const next = tabs[(to + tabs.length) % tabs.length].id;
    // Arrows walk the tabs without a history entry for each step.
    show(next, true);
    queueMicrotask(() => document.getElementById(`tab-${next}`)?.focus());
  }

  function goBack(ev: MouseEvent) {
    // Back to the browse view the reader came from keeps its scroll and loaded results.
    if (!back.back || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.button !== 0) return;
    ev.preventDefault();
    history.back();
  }
</script>

<div class="wrap emote with-bar">
  <nav class="crumbs" aria-label="Breadcrumb">
    <a class="back" href={back.href} onclick={goBack}><Icon name="arrow-left" />{back.label}</a>
    <span class="sep" aria-hidden="true">/</span>
    <span class="here" aria-current="page"><bdi>{name}</bdi></span>
  </nav>

  {#if !public_}
    <p class="notice {t.tone}" role="status"><strong>{t.status}.</strong> Only you and moderators can open this page.</p>
  {/if}

  <div class="cols">
    <section class="left" aria-label="Preview">
      <div class="stage" data-testid="emote-stage">
        {#if ready(e) && onStage}
          <img class="big" src={fileUrl(onStage, frozen)} width={onStage.width} height={onStage.height} alt={name} />
        {:else}
          <p class="wait">{e.state === 'APPROVED' && e.files.length === 0 ? 'The files for this emote are missing.' : 'The files are still being made. This takes a minute or so.'}</p>
        {/if}
        <div class="stage-tools">
          {#if trending}
            <a class="trend" href="/search?sort=trending" title="{count(trending.channels)} {trending.channels === 1 ? 'channel' : 'channels'} added it in the last 7 days">
              <Icon name="trending" size={14} />Trending <span class="rank">#{trending.rank}</span><span class="sr-only">this week, added by {count(trending.channels)} {trending.channels === 1 ? 'channel' : 'channels'}</span>
            </a>
          {/if}
          {#if hasStill(e)}
            <IconButton icon={still ? 'play' : 'pause'} label={still ? 'Play' : 'Pause'} size="s" tip="top" onclick={() => (paused = !still)} />
          {/if}
        </div>
      </div>

      {#if ready(e) && files.length}
        <div class="sizes" role="group" aria-label="Sizes">
          {#each files as f (f.name)}
            {@const on = onStage?.scale === f.scale}
            {@const h = Math.min(f.height, 24 * f.scale)}
            <button type="button" class="size" class:on aria-pressed={on} onclick={() => (picked = f.scale)}>
              <span class="pic"><img src={fileUrl(f, frozen)} alt="" height={h} style:height="{h}px" style:aspect-ratio="{f.width} / {f.height}" /></span>
              <span class="x">{#if on}<Icon name="check" size={12} stroke={2.4} />{/if}{f.scale}x</span>
              <span class="dm">{f.width} × {f.height}{#if fileSize(f)}<br />{fileSize(f)}{/if}</span>
            </button>
          {/each}
        </div>
        {#if chatFile}
          <p class="chatline">
            <span class="sr-only">How it looks in chat: </span><span class="viewer">nora_byte</span><span>that ending</span><img
              class="inline"
              src={fileUrl(chatFile, frozen)}
              alt={name}
              height="28"
              style:aspect-ratio="{chatFile.width} / {chatFile.height}"
            /><span>again please</span>
          </p>
        {/if}
      {/if}
    </section>

    <section class="right" aria-labelledby="emote-name">
      <header class="head">
        <h1 id="emote-name"><bdi>{name}</bdi></h1>
        <p class="by">
          {#if e.owner}
            <a class="creator" href="/users/{e.owner.id}">
              <Avatar src={e.owner.avatar} name={e.owner.displayName} size={22} />
              <span class="sr-only">Uploaded by </span><PaintedName name={ownerName} paint={ownerPaint} />
            </a>
          {/if}
          {#if uploaded}<span class="when">{e.owner ? 'uploaded' : 'Uploaded'} <time datetime={e.createdAt}>{uploaded}</time></span>{/if}
        </p>
        {#if flags.length || e.tags.length}
          <div class="labels">
            {#if flags.length}
              <ul class="flags" aria-label="Flags">
                {#each flags as f (f.key)}
                  <li class="flag {f.key}" title={f.tip}><Icon name={f.icon} size={13} />{f.label}<span class="sr-only">: {f.tip}</span></li>
                {/each}
              </ul>
            {/if}
            {#if e.tags.length}
              <ul class="tags" aria-label="Tags">
                {#each e.tags as tag (tag)}
                  <li><a class="tag" href="/search?tag={encodeURIComponent(tag)}"><span class="hash" aria-hidden="true">#</span><bdi>{clean(tag)}</bdi></a></li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </header>

      <div class="head-actions">
        <EmoteActions emoteId={e.id} {name} {viewer} picture={e} {editHref} addable={public_} reportable={canReport(e, session.me?.id)} owner={e.owner} />
      </div>

      <dl class="facts">
        {#if channelCount !== null}<div><dt>Channels</dt><dd class="num">{count(channelCount)}</dd></div>{/if}
        {#if trending}<div><dt>Added in 7 days</dt><dd class="num">+{count(trending.channels)}</dd></div>{/if}
        {#if trending}<div><dt>Trending</dt><dd class="num">#{trending.rank}</dd></div>{/if}
        <div><dt>Kind</dt><dd>{kind(e)}</dd></div>
        {#if kinds.length}<div><dt>{kinds.length > 1 ? 'Formats' : 'Format'}</dt><dd>{kinds.join(', ')}</dd></div>{/if}
        {#if biggest && fileSize(biggest)}<div><dt>File at {biggest.scale}x</dt><dd class="num">{bytes(biggest.size)}</dd></div>{/if}
        {#if public_}<div><dt>Review</dt><dd class="ok"><Icon name="check" size={12} stroke={2.4} />Approved</dd></div>{/if}
      </dl>

      {#if tabs.length}
        <section class="tabbed" aria-label="Channels, activity and more by the creator">
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
                  onclick={() => show(x.id)}
                  onkeydown={tabKey}
                ><Icon name={x.icon} size={16} />{#if x.short}<span class="long"><bdi>{x.label}</bdi></span><span class="short">{x.short}</span>{:else}{x.label}{/if}{#if x.n}<span class="n">{x.n}</span>{/if}</button>
              {/each}
            </div>
            {#if pager}
              <Pager label="{tab === 'channels' ? 'Channels' : 'Activity'} pages" {...pager} />
            {:else if tab === 'more' && e.owner}
              <a class="link" href="/users/{e.owner.id}">
                See all
                <svg class="arrow" viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 8h10M9 4l4 4-4 4" /></svg>
              </a>
            {/if}
          </div>
          {#if channels}
            <div role="tabpanel" id="panel-channels" aria-labelledby="tab-channels" hidden={tab !== 'channels'}>
              <ChannelsTab emoteId={e.id} first={channels} total={channelCount} bind:pager={channelPager} />
            </div>
          {/if}
          {#if listed && seen}
            <div role="tabpanel" id="panel-activity" aria-labelledby="tab-activity" hidden={tab !== 'activity'}>
              <ActivityTab emoteId={e.id} name={e.name} bind:pager={activityPager} />
            </div>
          {/if}
          {#if more.length && e.owner}
            <div role="tabpanel" id="panel-more" aria-labelledby="tab-more" hidden={tab !== 'more'}>
              <EmoteGrid items={more} creator={false} skeletons={0} label="More by {ownerName}" />
            </div>
          {/if}
        </section>
      {/if}

      <!-- Staff with emotes.flags change its flags after review (ADR 0024); nothing for anyone else.
           Last in the column: it shows once the session is known, so nothing below it can move. -->
      <EmoteFlagsPanel emote={e} id="emote-flags" onsaved={() => invalidateAll()} />
    </section>
  </div>

  <div class="bar" use:bottomBar>
    <EmoteActions emoteId={e.id} {name} {viewer} variant="bar" picture={e} {editHref} addable={public_} reportable={canReport(e, session.me?.id)} owner={e.owner} />
  </div>
</div>

<style>
  p { margin: 0; }
  .emote { padding-bottom: clamp(24px, 4vw, 48px); }

  /* The breadcrumb: where you came from, then this emote. */
  .crumbs { display: flex; align-items: center; gap: 8px; min-width: 0; min-height: 56px; font-size: var(--t-small); color: var(--muted); }
  .back { flex: none; display: inline-flex; align-items: center; gap: 6px; min-height: 40px; color: var(--muted); font-weight: 600;
    transition: color var(--dur-color); }
  .back :global(.icon) { transition: transform var(--dur-move) var(--ease); }
  .sep { color: var(--dim); }
  .here { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .notice { margin-bottom: 16px; padding: 12px 16px; border: 1px solid var(--border-strong); border-radius: var(--radius-card); background: var(--surface);
    font-size: var(--t-small); max-width: 72ch; }
  .notice strong { font-weight: 600; }

  .cols { display: grid; grid-template-columns: minmax(0, 1.05fr) minmax(0, 1fr); gap: 32px; align-items: start; }
  .left { display: grid; gap: 12px; min-width: 0; position: sticky; top: calc(var(--nav-h) + 16px); }
  .right { display: grid; gap: 20px; min-width: 0; }

  /* The stage: one neutral ground (the surface), the emote at its real size in the middle. */
  .stage { position: relative; display: grid; place-items: center; height: clamp(240px, 28vw, 380px); padding: 24px;
    border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--surface); overflow: hidden; }
  .big { display: block; max-width: 100%; max-height: 100%; width: auto; height: auto; object-fit: contain; }
  .wait { color: var(--muted); font-size: var(--t-small); text-align: center; max-width: 32ch; }
  .stage-tools { position: absolute; top: 10px; right: 10px; display: flex; align-items: center; gap: 8px; }
  .trend { position: relative; display: inline-flex; align-items: center; gap: 6px; height: 28px; padding: 0 10px; border: 1px solid var(--border-strong);
    border-radius: var(--radius-btn); background: var(--surface); font-size: var(--t-small); font-weight: 600; color: var(--text);
    transition: border-color var(--fast); white-space: nowrap; }
  .trend::after { content: ''; position: absolute; inset: -8px 0; } /* 44px to hit */
  .trend :global(.icon) { color: var(--accent-text); }
  .trend .rank { font-variant-numeric: tabular-nums; color: var(--accent-text); }

  /* The sizes: four cells sharing hairlines; the one on the stage is bold with an accent tick. */
  .sizes { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); border: 1px solid var(--border); border-radius: var(--radius-card); overflow: hidden; }
  .size { display: grid; justify-items: center; align-content: end; gap: 4px; min-height: 160px; padding: 12px 6px; border: 0;
    border-left: 1px solid var(--border); background: none; color: var(--muted); font: inherit; cursor: pointer; min-width: 0;
    transition: background-color var(--fast), color var(--fast); touch-action: manipulation; }
  .size:first-child { border-left: 0; }
  .size:focus-visible { outline-offset: -2px; }
  .pic { display: grid; place-items: end center; max-width: 100%; }
  .pic img { display: block; max-width: 100%; width: auto; object-fit: contain; }
  .x { display: inline-flex; align-items: center; gap: 3px; margin-top: 6px; font-size: var(--t-small); font-weight: 600; color: var(--text); }
  .x :global(.icon) { color: var(--accent-text); }
  .size.on { color: var(--text); }
  .size.on .x { font-weight: 700; }
  .dm { font-size: var(--t-fine); line-height: 1.35; font-variant-numeric: tabular-nums; text-align: center; }

  /* One line of chat with the emote at chat size (28px tall). */
  .chatline { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 6px; min-height: 50px; padding: 10px 14px;
    border: 1px solid var(--border); border-radius: var(--radius-card); font-size: var(--t-small); color: var(--text); }
  .viewer { font-weight: 700; color: var(--accent-text); }
  .viewer::after { content: ':'; color: var(--muted); font-weight: 400; }
  .inline { display: block; height: 28px; width: auto; }

  /* The right column. */
  .head { display: grid; gap: 10px; min-width: 0; }
  h1 { font-size: var(--t-page); line-height: 1.15; letter-spacing: -.02em; overflow-wrap: anywhere; }
  .by { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; font-size: var(--t-small); color: var(--muted); min-width: 0; }
  .creator { display: inline-flex; align-items: center; gap: 8px; min-width: 0; max-width: 100%; font-weight: 600; color: var(--text); }
  .creator :global(.pn) { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { font-variant-numeric: tabular-nums; white-space: nowrap; }
  .labels { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 14px; }
  .flags, .tags { list-style: none; margin: 0; padding: 0; display: flex; flex-wrap: wrap; }
  .flags { gap: 4px 12px; }
  .flag { display: inline-flex; align-items: center; gap: 5px; font-size: var(--t-small); font-weight: 500; color: var(--muted); }
  .flag :global(.icon) { color: var(--dim); }
  .flag.nsfw, .flag.unlisted, .flag.private { color: var(--text); }
  .flag.nsfw :global(.icon), .flag.unlisted :global(.icon), .flag.private :global(.icon) { color: var(--accent-text); }
  .tags { gap: 6px; }
  .tag { display: inline-flex; align-items: center; gap: 1px; height: 26px; padding: 0 9px; border-radius: var(--radius);
    background: var(--neutral-chip, var(--surface-2)); color: var(--text); font-size: var(--t-fine); font-weight: 600; transition: background-color var(--fast); }
  .hash { color: var(--dim); }
  .head-actions :global(.actions-row) { justify-content: flex-start; }

  /* The facts: label and value cells sharing hairlines. */
  .facts { margin: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); border: 1px solid var(--border);
    border-radius: var(--radius-card); overflow: hidden; }
  /* Hairlines right of and under every cell; the frame clips the outer ones, so a short last row leaves no grey gap. */
  .facts > div { display: grid; align-content: start; gap: 2px; padding: 10px 12px; min-width: 0; box-shadow: 1px 0 0 var(--border), 0 1px 0 var(--border); }
  dt { font-size: var(--t-fine); color: var(--muted); white-space: nowrap; }
  dd { margin: 0; display: inline-flex; align-items: center; gap: 4px; font-size: var(--t-ui); font-weight: 600; min-width: 0; overflow-wrap: anywhere; }
  dd.num { font-variant-numeric: tabular-nums; }
  dd.ok :global(.icon) { color: var(--accent-text); }

  /* The tabs: a strip over hairline-ruled rows. */
  .tabbed { min-width: 0; }
  .tabbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 44px; border-bottom: 1px solid var(--border); margin-bottom: 4px; }
  .tablist { display: flex; gap: 4px; padding: 3px; margin: -3px -3px -3px -13px; overflow-x: auto; scrollbar-width: none; min-width: 0; }
  .tabbtn { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 10px; border: 0; border-radius: var(--radius-btn);
    background: none; color: var(--muted); font: inherit; font-size: var(--t-ui); font-weight: 500; white-space: nowrap; cursor: pointer;
    transition: color var(--fast); touch-action: manipulation; }
  .tabbtn[aria-selected='true'] { color: var(--text); font-weight: 600; }
  .tabbtn :global(.icon) { color: var(--dim); }
  .tabbtn[aria-selected='true'] :global(.icon) { color: var(--accent-text); }
  .long { max-width: 22ch; overflow: hidden; text-overflow: ellipsis; }
  .n { color: var(--dim); font-variant-numeric: tabular-nums; font-size: var(--t-small); }
  .tabbtn[aria-selected='true'] .n { color: var(--muted); }
  .tabbtn:focus-visible { outline-offset: -2px; }
  .tabbar .link { flex: none; white-space: nowrap; }
  svg.arrow { fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; stroke-linejoin: round; }

  .bar { display: none; }
  .short { display: none; }

  @media (hover: hover) and (pointer: fine) {
    .back:hover { color: var(--text); }
    .back:hover :global(.icon) { transform: translateX(-3px); }
    .creator:hover :global(bdi) { text-decoration: underline; text-underline-offset: 3px; }
    .trend:hover { border-color: var(--accent-border); }
    .tag:hover { background: var(--surface-hover); }
    .size:hover { background: var(--row-hover); color: var(--text); }
    .tabbtn:hover:not([aria-selected='true']) { color: var(--text); }
  }
  @media (pointer: coarse) { .back, .tabbtn { min-height: 44px; } .tag { height: 36px; } }

  @media (max-width: 899.98px) {
    .cols { grid-template-columns: minmax(0, 1fr); gap: 20px; }
    .left { position: static; }
  }
  @media (max-width: 560px) {
    .size { min-height: 120px; padding: 10px 2px; }
    .long { display: none; }
    .short { display: inline; }
    .tablist { gap: 0; }
    .tabbtn { padding: 0 8px; gap: 6px; }
    .tabbar { flex-wrap: wrap; row-gap: 4px; }
    .facts { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }

  /* Phones: the actions move to a bar fixed to the bottom of the screen, above the home indicator.
     The page ends higher, so the bar never covers the footer. */
  @media (max-width: 720px) {
    :global(html:has(.emote.with-bar)) { scroll-padding-bottom: 88px; }
    :global(body:has(.emote.with-bar)) { padding-bottom: calc(65px + env(safe-area-inset-bottom, 0px)); }
    .head-actions { display: none; }
    .bar { display: block; position: fixed; left: 0; right: 0; bottom: 0; z-index: var(--z-nav);
      padding: 10px var(--page-pad) calc(10px + env(safe-area-inset-bottom, 0px));
      border-top: 1px solid var(--border); background: var(--bg); }
  }
  @media (max-width: 720px) and (max-height: 500px) {
    .bar { position: static; margin-inline: calc(-1 * var(--page-pad)); border-bottom: 1px solid var(--border); }
    :global(body:has(.emote.with-bar)) { padding-bottom: 0; }
  }
  @media (max-width: 720px) and ((prefers-reduced-transparency: reduce) or (prefers-contrast: more)) {
    .bar { background: var(--bg); backdrop-filter: none; -webkit-backdrop-filter: none; border-top-color: var(--border-strong); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .back :global(.icon) { transition: none; } }
</style>
