<script lang="ts">
  // The profile's About panel (the Inspector profile's right-hand side until an emote is picked).
  // Everything here is public and the same for every viewer (owner rule, 2026/11/20):
  //   Connections  each channel login (a link to the channel) and the set it shows;
  //   Editors      names only (never what they may do), ten at a time;
  //   In numbers   uploads, channels showing a set, connections, editors.
  // Connections and Editors are collapsed and opened on purpose, so an account at its 50 editors
  // keeps the panel as short as a new one. Rows have a fixed height.
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean, count, date } from '$lib/format';
  import { platformName } from '$lib/shell/account';
  import type { PublicProfile } from './data.svelte';
  import { connectionUrl } from './public';
  import { EDITORS_STEP, moreText, shown, showing } from './scale';

  type Props = { accountId: string; profile: PublicProfile | null; uploads: number };
  let { accountId, profile, uploads }: Props = $props();

  const connections = $derived(profile?.identities ?? []);
  const active = $derived(profile?.activeSets ?? []);
  const editors = $derived(profile?.editors ?? []);
  const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const setOn = (platform: string, login: string) => active.find((a) => a.platform === platform && same(a.login, login));
  // A channel showing a set whose login isn't among the connections still gets its row.
  const orphans = $derived(active.filter((a) => !connections.some((c) => c.platform === a.platform && same(c.login, a.login))));
  const linked = $derived(connections.length + orphans.length);

  let openConnections = $state(false);
  let openEditors = $state(false);
  let steps = $state(1);
  $effect(() => {
    void accountId;
    openConnections = false;
    openEditors = false;
    steps = 1;
  });
  const editorsShown = $derived(editors.slice(0, shown(editors.length, steps, EDITORS_STEP)));
  // Only the editors in view are asked for their paint.
  $effect(() => {
    if (openEditors) wantLooks(editorsShown.map((e) => e.id));
  });
  const ids = $props.id();
</script>

<div class="about">
  <section class="block">
    {#if linked}
      <button class="dis" type="button" aria-expanded={openConnections} aria-controls="{ids}-conn" onclick={() => (openConnections = !openConnections)}>
        <span class="chev" class:on={openConnections} aria-hidden="true"><Icon name="chevron-right" size={14} /></span>
        <span class="label">Connections</span><span class="n">{count(linked)}</span>
      </button>
      <ul class="rows" id="{ids}-conn" aria-label="Connections" hidden={!openConnections}>
        {#each connections as c (c.platform + c.login)}
          {@const url = connectionUrl(c.platform, c.login)}
          {@const on = setOn(c.platform, c.login)}
          <li class="row">
            <Icon name={c.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
            <span class="main">
              {#if url}
                <a href={url} target="_blank" rel="noopener"><span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi><span class="sr-only">{' (opens in a new tab)'}</span></a>
              {:else}
                <span><span class="sr-only">{`${platformName(c.platform)}: `}</span><bdi>{clean(c.login)}</bdi></span>
              {/if}
              {#if on}<span class="sub">Shows <a href="/sets/{encodeURIComponent(on.setId)}"><bdi>{clean(on.setName)}</bdi></a></span>{/if}
            </span>
            <span class="plat">{platformName(c.platform)}</span>
          </li>
        {/each}
        {#each orphans as a (a.platform + a.login)}
          <li class="row">
            <Icon name={a.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
            <span class="main">
              <span><span class="sr-only">{`${platformName(a.platform)}: `}</span><bdi>{clean(a.login)}</bdi></span>
              <span class="sub">Shows <a href="/sets/{encodeURIComponent(a.setId)}"><bdi>{clean(a.setName)}</bdi></a></span>
            </span>
            <span class="plat">{platformName(a.platform)}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="head-static"><span class="label">Connections</span></p>
      <p class="none">No channels linked.</p>
    {/if}
  </section>

  <section class="block">
    {#if editors.length}
      <button class="dis" type="button" aria-expanded={openEditors} aria-controls="{ids}-ed" onclick={() => (openEditors = !openEditors)}>
        <span class="chev" class:on={openEditors} aria-hidden="true"><Icon name="chevron-right" size={14} /></span>
        <span class="label">Editors</span><span class="n">{count(editors.length)}</span>
      </button>
      <div id="{ids}-ed" hidden={!openEditors}>
        <ul class="rows" aria-label="Editors">
          {#each editorsShown as e (e.id)}
            <li class="row">
              <Avatar src={e.avatar} name={clean(e.displayName)} size={24} />
              <span class="main"><a href="/users/{encodeURIComponent(e.id)}"><PaintedName name={e.displayName} paint={looks[e.id]?.paint} /></a></span>
            </li>
          {/each}
        </ul>
        {#if editorsShown.length < editors.length}
          <div class="more">
            <span class="sub" aria-live="polite">{showing(editorsShown.length, editors.length)}</span>
            <button class="btn btn-s" type="button" onclick={() => steps++}>{moreText(editorsShown.length, editors.length, EDITORS_STEP)}</button>
          </div>
        {/if}
      </div>
    {:else}
      <p class="head-static"><span class="label">Editors</span></p>
      <p class="none">Nobody else edits this channel.</p>
    {/if}
  </section>

  <section class="block">
    <h3 class="head-static"><span class="label">In numbers</span></h3>
    <dl class="kv">
      <dt>Public uploads</dt><dd>{count(uploads)}</dd>
      <dt>Channels showing a set</dt><dd>{count(active.length)}</dd>
      <dt>Connections</dt><dd>{count(linked)}</dd>
      <dt>Editors</dt><dd>{count(editors.length)}</dd>
      {#if profile?.createdAt}<dt>Member since</dt><dd><time datetime={profile.createdAt}>{date(profile.createdAt)}</time></dd>{/if}
    </dl>
  </section>
</div>

<style>
  p, h3 { margin: 0; }
  .about { display: grid; gap: 4px; }
  .block { display: grid; min-width: 0; }
  .block + .block { padding-top: 8px; border-top: 1px solid var(--border); }
  .label { font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .head-static { display: flex; align-items: center; min-height: 36px; font-size: inherit; font-weight: inherit; }
  /* A disclosure head: chevron, the word and how many; the list opens under it. */
  .dis { display: flex; align-items: center; gap: 6px; width: 100%; min-height: 36px; margin: 0 -6px; padding: 0 6px; border: 0;
    border-radius: var(--radius); background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .dis .n { margin-left: auto; font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; }
  .dis[aria-expanded='true'] .label { color: var(--text); }
  .dis:focus-visible { outline-offset: -2px; }
  .chev { display: inline-flex; color: var(--dim); transition: transform var(--fast) var(--ease); }
  .chev.on { transform: rotate(90deg); color: var(--accent-text); }
  .none { padding: 0 0 8px; font-size: var(--t-small); color: var(--muted); }

  .rows { list-style: none; margin: 0 0 8px; padding: 0; display: grid; }
  .rows[hidden] { display: none; }
  .row { display: flex; align-items: center; gap: 10px; height: 44px; min-width: 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .row > :global(svg) { flex: none; color: var(--dim); }
  .main { flex: 1; display: grid; min-width: 0; line-height: 1.3; font-size: var(--t-small); }
  .main > a, .main > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .main a { color: var(--text); font-weight: 600; }
  .sub { font-size: var(--t-fine); color: var(--muted); }
  .sub a { color: var(--muted); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .plat { flex: none; font-size: var(--t-fine); color: var(--dim); }
  .more { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 0; border-top: 1px solid var(--border); }

  .kv { display: grid; grid-template-columns: minmax(0, 1fr) auto; margin: 0 0 4px; }
  .kv dt, .kv dd { margin: 0; padding: 9px 0; border-bottom: 1px solid var(--border); font-size: var(--t-small); }
  .kv dt { color: var(--muted); padding-right: 16px; }
  .kv dd { text-align: right; font-weight: 600; font-variant-numeric: tabular-nums; }
  .kv dt:last-of-type, .kv dd:last-of-type { border-bottom: 0; }

  @media (hover: hover) and (pointer: fine) {
    .dis:hover { background: var(--row-hover); }
    .main a:hover { text-decoration: underline; text-underline-offset: 3px; }
  }
  @media (pointer: coarse) { .dis { min-height: 44px; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .chev { transition: none; } }
</style>
