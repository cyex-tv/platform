<script lang="ts">
  // The header search: the field itself is the search (no popup, no dialog). Typing shows a list
  // attached under the field with the top 5 emotes (the API's name matcher: exact, prefix, word
  // and initials prefix, substring, then near misses) and the top 3 people. Signed in, your
  // favourites whose name holds the text come first (marked with a heart), and emotes you hid for
  // yourself are left out. Focusing the empty
  // field lists the channels you can manage, by your role then name; signed out, one hint line.
  //
  // An ARIA 1.2 combobox: the input owns a listbox, ↑ ↓ move the active row (aria-activedescendant,
  // focus stays in the field), Enter opens the active row or, with none, /search for the text,
  // Escape closes the list (a second Escape clears the text), Tab leaves. Requests wait 150ms
  // after the last key and a newer one aborts the older. The active row is bold with an accent
  // icon, never a filled box (owner rule). `phone`: the field sits in the header row and the list
  // spans the screen under it; `onclose` hands control back to the header.
  import { goto } from '$app/navigation';
  import { fetchWith, gql } from '$lib/api';
  import type { Snippet } from 'svelte';
  import { ready, size, src, srcsetW } from '$lib/emote';
  import { clean } from '$lib/format';
  import type { EditableChannelsQuery, QuickSearchQuery } from '$lib/gql/graphql';
  import { EDITABLE_CHANNELS, MY_FAVOURITES, QUICK_SEARCH } from '$lib/queries';
  import { favouritesFirst, personal } from '$lib/personal/lists.svelte';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ROLE_LABEL, sortChannels, type ChannelRole } from './nav';
  import { announce, emptyText, move, searchHref } from './search';
  import { headerSearch } from './search.svelte';

  type Props = { phone?: boolean; onclose?: () => void };
  let { phone = false, onclose }: Props = $props();

  type Emote = QuickSearchQuery['quickSearch']['emotes'][number];
  type Person = QuickSearchQuery['quickSearch']['users'][number];
  type Managed = EditableChannelsQuery['myEditableChannels'][number];
  type Row =
    | { kind: 'emote'; id: string; href: string; emote: Emote }
    | { kind: 'user'; id: string; href: string; person: Person }
    | { kind: 'channel'; id: string; href: string; item: Managed }
    | { kind: 'all'; id: string; href: string };

  const uid = $props.id();
  const listId = `${uid}-list`;
  let input = $state<HTMLInputElement>();
  let root = $state<HTMLDivElement>();
  let focused = $state(false);
  let dismissed = $state(false); // Escape closed the list; typing or ↓ opens it again
  let active = $state(-1);

  // The results for `shownFor`; they stay up while the next ones load, so the list never blinks.
  let emotes = $state<Emote[]>([]);
  let people = $state<Person[]>([]);
  let shownFor = $state('');
  let searching = $state(false);
  let failed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight: AbortController | null = null;

  // The channels you can manage, asked once per account and again when older than a minute.
  let channels = $state<Managed[] | null>(null);
  let channelsFor = '';
  let channelsAt = 0;
  let channelsFailed = $state(false);

  const me = $derived(session.me);
  const text = $derived(headerSearch.query.trim());
  const rows = $derived.by<Row[]>(() => {
    if (!text) {
      if (!me || !channels) return [];
      return sortChannels(channels).map((c) => ({
        kind: 'channel' as const,
        id: `${uid}-c-${c.channel.platform}-${c.channel.login}`,
        href: `/channels/${c.channel.platform.toLowerCase()}/${encodeURIComponent(c.channel.login)}`,
        item: c,
      }));
    }
    if (shownFor !== text && !emotes.length && !people.length) return [];
    const out: Row[] = [];
    for (const e of emotes) out.push({ kind: 'emote', id: `${uid}-e-${e.id}`, href: `/emotes/${e.id}`, emote: e });
    for (const p of people) out.push({ kind: 'user', id: `${uid}-u-${p.account.id}`, href: `/users/${encodeURIComponent(p.account.id)}`, person: p });
    out.push({ kind: 'all', id: `${uid}-all`, href: searchHref(text) });
    return out;
  });
  const emoteRows = $derived(rows.filter((r) => r.kind === 'emote'));
  const userRows = $derived(rows.filter((r) => r.kind === 'user'));
  const channelRows = $derived(rows.filter((r) => r.kind === 'channel'));
  const allRow = $derived(rows.find((r) => r.kind === 'all'));
  const nothing = $derived(!!text && shownFor === text && !searching && !failed && !emotes.length && !people.length);
  /** Something to show under the field: rows, a state line or the signed-out hint. */
  const content = $derived(!!text || !!me || session.known);
  const open = $derived(focused && !dismissed && content);
  const activeId = $derived(open && active >= 0 ? rows[active]?.id : undefined);

  // One status line for screen readers, read 300ms after results settle (not on every key).
  let status = $state('');
  $effect(() => {
    const t = text;
    const n = [emotes.length, people.length] as const;
    status = '';
    if (!open || searching || !t || shownFor !== t) return;
    const s = setTimeout(() => (status = failed ? 'Search isn’t available right now.' : announce(n[0], n[1], t)), 300);
    return () => clearTimeout(s);
  });

  // Live search, 150ms after the last key; a newer request aborts the one before.
  $effect(() => {
    const q = text;
    clearTimeout(timer);
    active = -1;
    if (!q) {
      inflight?.abort();
      inflight = null;
      emotes = [];
      people = [];
      shownFor = '';
      searching = false;
      failed = false;
      return;
    }
    searching = true;
    failed = false; // a new request: the last one's error is not this one's
    timer = setTimeout(async () => {
      inflight?.abort();
      const ctl = new AbortController();
      inflight = ctl;
      try {
        const signal = fetchWith(ctl.signal);
        const [r, favs] = await Promise.all([
          gql(QUICK_SEARCH, { query: q }, signal),
          // Favourites are a bonus: a failure there never fails the search.
          me ? gql(MY_FAVOURITES, { query: q, perPage: 5 }, signal).catch(() => null) : null,
        ]);
        if (ctl.signal.aborted) return;
        emotes = favouritesFirst(personal.visible((favs?.myFavourites?.items ?? []).filter((e) => !e.flags.nsfw)), personal.visible(r.quickSearch.emotes), 5);
        people = r.quickSearch.users.slice(0, 3);
        failed = false;
      } catch {
        if (ctl.signal.aborted) return;
        emotes = [];
        people = [];
        failed = true;
      }
      shownFor = q;
      searching = false;
      if (inflight === ctl) inflight = null;
    }, 150);
    return () => clearTimeout(timer);
  });

  async function loadChannels() {
    const id = me?.id;
    if (!id) return;
    if (channelsFor === id && channels && Date.now() - channelsAt < 60_000) return;
    if (channelsFor !== id) channels = null;
    channelsFor = id;
    try {
      const r = await gql(EDITABLE_CHANNELS, {});
      if (session.me?.id !== id) return;
      channels = r.myEditableChannels;
      channelsAt = Date.now();
      channelsFailed = false;
    } catch {
      if (session.me?.id === id) channelsFailed = true;
    }
  }

  // Signed out (or someone else signed in): forget the last account's channels.
  $effect(() => {
    if (me?.id !== channelsFor) {
      channels = null;
      channelsFor = '';
    }
  });

  // focusSearch() from anywhere.
  $effect(() => {
    if (headerSearch.focusTick) input?.focus();
  });

  $effect(() => personal.ensure());

  function onfocus() {
    focused = true;
    dismissed = false;
    void loadChannels();
  }

  function onfocusout(e: FocusEvent) {
    if (root?.contains(e.relatedTarget as Node | null)) return;
    focused = false;
    active = -1;
    if (phone && !text) onclose?.();
  }

  async function go(row: Row | undefined) {
    const href = row ? row.href : text ? searchHref(text) : null;
    if (!href) return;
    headerSearch.query = '';
    focused = false;
    input?.blur();
    onclose?.();
    await goto(href);
  }

  function scrollActive() {
    const id = rows[active]?.id;
    if (id) document.getElementById(id)?.scrollIntoView({ block: 'nearest' });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) {
        dismissed = false;
        active = e.key === 'ArrowDown' ? 0 : rows.length - 1;
      } else {
        active = move(active, e.key === 'ArrowDown' ? 1 : -1, rows.length);
      }
      scrollActive();
    } else if (e.key === 'Enter') {
      if (e.isComposing) return;
      e.preventDefault();
      void go(open && active >= 0 ? rows[active] : undefined);
    } else if (e.key === 'Escape') {
      if (open && (rows.length || text)) {
        e.preventDefault();
        dismissed = true;
        active = -1;
      } else if (text) {
        e.preventDefault();
        headerSearch.query = '';
      } else if (phone) {
        e.preventDefault();
        input?.blur();
        onclose?.();
      }
    } else if (e.key === 'Tab') {
      dismissed = true;
    }
  }

  function oninput() {
    dismissed = false;
  }

  function emoteWidth(e: Emote) {
    const s = size(e);
    return Math.min(56, Math.round((28 * s.width) / s.height));
  }
  const platformName = (p: string) => (p === 'KICK' ? 'Kick' : 'Twitch');
  const roleLabel = (r: string) => ROLE_LABEL[r as ChannelRole] ?? '';
</script>

{#snippet option(r: Row, inner: Snippet)}
  {@const i = rows.indexOf(r)}
  <a
    class="row"
    class:wide={r.kind === 'all'}
    id={r.id}
    href={r.href}
    role="option"
    aria-selected={i === active}
    tabindex="-1"
    onpointermove={() => (active = i)}
    onpointerdown={(e) => e.preventDefault()}
    onclick={(e) => {
      if (e.ctrlKey || e.metaKey || e.shiftKey || e.button !== 0) return;
      e.preventDefault();
      void go(r);
    }}
  >
    {@render inner()}
  </a>
{/snippet}

<div class="hs" class:phone class:open bind:this={root} {onfocusout}>
  <div class="field">
    <Icon name="search" />
    <input
      bind:this={input}
      bind:value={headerSearch.query}
      type="search"
      role="combobox"
      aria-expanded={open}
      aria-controls={listId}
      aria-activedescendant={activeId}
      aria-autocomplete="list"
      aria-label="Search emotes and people"
      placeholder="Search emotes and people"
      autocomplete="off"
      autocapitalize="none"
      autocorrect="off"
      spellcheck="false"
      enterkeyhint="search"
      maxlength="100"
      {onfocus}
      {onkeydown}
      {oninput}
    />
    {#if searching && text}<span class="spin" aria-hidden="true"></span>{/if}
  </div>

  <div class="drop" hidden={!open}>
    <div class="list" id={listId} role="listbox" aria-label={text ? 'Search results' : 'Your channels'}>
      {#if text}
        {#if emoteRows.length}
          <div class="group" role="group" aria-labelledby="{uid}-ge">
            <p class="group-name" id="{uid}-ge" role="presentation">Emotes</p>
            {#each emoteRows as r (r.id)}
              {#snippet emoteInner()}
                {#if r.kind === 'emote'}
                  <span class="art emote-art">
                    {#if ready(r.emote)}
                      <img src={src(r.emote, 1)} srcset={srcsetW(r.emote)} sizes="{emoteWidth(r.emote)}px" width={emoteWidth(r.emote)} height="28" alt="" decoding="async" />
                    {/if}
                  </span>
                  <span class="label"><bdi>{clean(r.emote.name)}</bdi>{#if personal.favourites.has(r.emote.id)}<span class="fav" title="Favourite"><Icon name="heart" size={12} /><span class="sr-only">, favourite</span></span>{/if}</span>
                  {#if r.emote.owner}<span class="hint"><span>by <bdi>{clean(r.emote.owner.displayName)}</bdi></span></span>{/if}
                {/if}
              {/snippet}
              {@render option(r, emoteInner)}
            {/each}
          </div>
        {/if}
        {#if userRows.length}
          <div class="group" role="group" aria-labelledby="{uid}-gu">
            <p class="group-name" id="{uid}-gu" role="presentation">People</p>
            {#each userRows as r (r.id)}
              {#snippet userInner()}
                {#if r.kind === 'user'}
                  <span class="art"><Avatar src={r.person.account.avatar} name={r.person.account.displayName} size={28} /></span>
                  <span class="label"><bdi>{clean(r.person.account.displayName)}</bdi></span>
                  {#if r.person.identities[0]}<span class="hint"><span><bdi>{clean(r.person.identities[0].login)}</bdi> on {platformName(r.person.identities[0].platform)}</span></span>{/if}
                {/if}
              {/snippet}
              {@render option(r, userInner)}
            {/each}
          </div>
        {/if}
        {#if nothing}
          <p class="line">{emptyText(text)}</p>
        {:else if failed && !emotes.length && !people.length}
          <p class="line">Search isn’t available right now. Press Enter to try the full search.</p>
        {:else if !rows.length}
          <p class="line">Searching…</p>
        {/if}
        {#if allRow}
          <div class="group foot" role="group" aria-label="More">
            {#snippet allInner()}
              <span class="art"><Icon name="arrow-right" /></span>
              <span class="label">All results for “<bdi>{text}</bdi>”</span>
            {/snippet}
            {@render option(allRow, allInner)}
          </div>
        {/if}
      {:else if me}
        <div class="group" role="group" aria-labelledby="{uid}-gc">
          <p class="group-name" id="{uid}-gc" role="presentation">Your channels</p>
          {#each channelRows as r (r.id)}
            {#snippet channelInner()}
              {#if r.kind === 'channel'}
                <span class="art"><Avatar src={r.item.channel.account.avatar} name={r.item.channel.account.displayName} size={28} /></span>
                <span class="label"><bdi>{clean(r.item.channel.login)}</bdi></span>
                <span class="hint"><Icon name={r.item.channel.platform === 'KICK' ? 'kick' : 'twitch'} size={14} />{roleLabel(r.item.role)}</span>
              {/if}
            {/snippet}
            {@render option(r, channelInner)}
          {/each}
          {#if channels === null && !channelsFailed}
            <p class="line">Loading…</p>
          {:else if channels === null}
            <p class="line">Your channels didn’t load. Try again in a moment.</p>
          {:else if !channelRows.length}
            <p class="line">Channels you own or edit show up here.</p>
          {/if}
        </div>
      {:else}
        <p class="line">Type to search emotes by name, or people by login.</p>
      {/if}
    </div>
  </div>
  <p class="sr-only" role="status">{status}</p>
</div>

<style>
  .hs { position: relative; width: 100%; min-width: 0; }
  .field { display: flex; align-items: center; gap: 10px; height: 40px; padding: 0 10px 0 12px; border: 1px solid var(--border);
    border-radius: var(--radius-btn); background: var(--surface); color: var(--muted); cursor: text;
    transition: border-color var(--fast), color var(--fast); }
  .field:focus-within { border-color: var(--border-strong); color: var(--text); outline: 2px solid var(--accent-text); outline-offset: -1px; }
  input { flex: 1; min-width: 0; height: 100%; padding: 0; border: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-nav); }
  input:focus, input:focus-visible { outline: none; }
  input::placeholder { color: var(--muted); opacity: 1; }
  input::-webkit-search-cancel-button { display: none; }
  .spin { flex: none; width: 14px; height: 14px; border: 2px solid var(--border-strong); border-top-color: var(--muted); border-radius: 50%;
    animation: spin .7s linear infinite; }
  @keyframes spin { to { transform: rotate(1turn); } }

  /* Attached to the field: the same width (at least 380px, centred on it), straight under it. */
  .drop { position: absolute; top: calc(100% + 6px); left: 50%; transform: translateX(-50%); width: max(100%, min(380px, calc(100vw - 32px)));
    z-index: var(--z-pop); border: 1px solid var(--border-strong); border-radius: var(--radius-m); background: var(--surface);
    box-shadow: var(--shadow-pop); overflow: hidden; transform-origin: 50% 0; }
  .drop[hidden] { display: none; }
  .list { max-height: min(520px, calc(100dvh - var(--nav-h) - 24px)); overflow-y: auto; overscroll-behavior: contain; padding: 6px; }
  .group + .group { margin-top: 4px; padding-top: 4px; border-top: 1px solid var(--border); }
  .group-name { margin: 0; padding: 8px 10px 6px; font-size: var(--t-fine); font-weight: 500; color: var(--muted); }
  .row { display: flex; align-items: center; gap: 12px; min-height: 44px; padding: 6px 10px; border-radius: var(--radius-btn);
    color: var(--text); font-size: var(--t-nav); text-decoration: none; cursor: pointer; }
  /* The active row: bold with an accent icon; no fill, border or bar (owner rule). */
  .row[aria-selected='true'] .label { font-weight: 700; }
  .row[aria-selected='true'] .art :global(svg) { color: var(--accent-text); }
  .row[aria-selected='true'] .hint { color: var(--text); }
  .art { flex: none; display: inline-grid; place-items: center; min-width: 32px; height: 28px; color: var(--muted); }
  .fav { display: inline-flex; margin-left: 6px; vertical-align: -1px; color: var(--accent-text); }
  .fav :global(svg) { fill: currentColor; }
  .emote-art img { display: block; height: 28px; width: auto; max-width: 56px; object-fit: contain; }
  .label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .hint { margin-left: auto; padding-left: 12px; flex: none; display: inline-flex; align-items: center; gap: 6px; max-width: 50%;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-fine); color: var(--muted); }
  .hint :global(svg) { flex: none; color: var(--dim); }
  .hint > span { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .line { margin: 0; padding: 10px 10px 12px; color: var(--muted); font-size: var(--t-fine); overflow-wrap: anywhere; }
  .foot .row { min-height: 40px; color: var(--muted); }
  .foot .row[aria-selected='true'] { color: var(--text); }

  /* Phones: the field fills the header row, the list spans the screen under the header. */
  .phone .field { height: 40px; }
  .phone input { font-size: 16px; } /* no zoom on focus (iOS) */
  .phone .drop { position: fixed; top: calc(var(--nav-h) + 1px + env(safe-area-inset-top, 0px)); left: 0; right: 0; width: auto; transform: none;
    border-width: 0 0 1px; border-radius: 0; box-shadow: var(--shadow-pop); }
  .phone .list { max-height: calc(100dvh - var(--nav-h) - env(safe-area-inset-top, 0px) - 1px); padding: 6px 10px calc(10px + env(safe-area-inset-bottom, 0px)); }
  .phone .row { min-height: 48px; font-size: var(--t-ui); }

  @media (hover: hover) and (pointer: fine) { .field:hover { border-color: var(--border-strong); } }
  @media (forced-colors: active) {
    .row[aria-selected='true'] { outline: 2px solid Highlight; outline-offset: -2px; }
    .drop { border-color: CanvasText; }
  }
</style>
