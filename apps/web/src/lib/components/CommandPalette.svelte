<script lang="ts">
  // The command palette: Ctrl K / Cmd K or "/" (outside a field) opens it, so does openPalette().
  // No on-screen shortcut hint anywhere; the keys just work. On a page with its own search field
  // (marked data-page-search), "/" focuses that field instead.
  // Emotes come live from the search (debounced), a text that looks like a login offers its
  // Twitch and Kick channel, and the site's pages are always there. Arrow keys move, Enter goes,
  // Escape closes and focus goes back. No open or close animation: it's keyboard-driven and
  // opened many times a day (Emil). Mount it once, in the root layout.
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import { hasStill, ready, size, src, srcsetW } from '$lib/emote';
  import { clean, count, iso } from '$lib/format';
  import { announce, channelJumps, emptyText, move, pageJumps, typing } from '$lib/palette';
  import type { SearchQuery } from '$lib/gql/graphql';
  import { SEARCH } from '$lib/queries';
  import Icon from './Icon.svelte';
  import { palette } from './palette.svelte';

  type Emote = SearchQuery['searchEmotes']['items'][number];
  type Row =
    | { kind: 'emote'; id: string; href: string; emote: Emote }
    | { kind: 'all'; id: string; href: string; label: string }
    | { kind: 'channel' | 'page'; id: string; href: string; label: string; hint?: string };

  let dialog: HTMLDialogElement;
  let input = $state<HTMLInputElement>();
  let back: HTMLElement | null = null;
  let active = $state(0);
  let emotes = $state<Emote[]>([]);
  let total = $state(0);
  let searching = $state(false);
  let failed = $state(false);
  let seq = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const text = $derived(palette.query.trim());
  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    if (text) {
      for (const e of emotes) out.push({ kind: 'emote', id: `pe-${e.id}`, href: `/emotes/${e.id}`, emote: e });
      out.push({ kind: 'all', id: 'pe-all', href: `/search?q=${encodeURIComponent(text)}`, label: `Search all emotes for “${iso(text)}”` });
      for (const c of channelJumps(text)) out.push({ kind: 'channel', ...c });
    }
    for (const p of pageJumps(text)) out.push({ kind: 'page', ...p });
    return out;
  });
  const emoteRows = $derived(rows.filter((r) => r.kind === 'emote' || r.kind === 'all'));
  const channelRows = $derived(rows.filter((r) => r.kind === 'channel'));
  const pageRows = $derived(rows.filter((r) => r.kind === 'page'));
  const activeId = $derived(rows[active]?.id);
  const nothing = $derived(!!text && !rows.some((r) => r.kind !== 'all'));

  // One status line, read out 300ms after the results settle (not on every key).
  let status = $state('');
  $effect(() => {
    const n = { emotes: emotes.length, channels: channelRows.length, pages: pageRows.length };
    const t = text;
    status = '';
    if (!palette.open || searching) return;
    const timer = setTimeout(() => (status = announce(n, t)), 300);
    return () => clearTimeout(timer);
  });

  // Open and close follow the shared state.
  $effect(() => {
    if (palette.open && !dialog.open) {
      back = document.activeElement as HTMLElement | null;
      dialog.showModal();
      input?.focus();
      input?.select();
      active = 0;
    } else if (!palette.open && dialog.open) {
      dialog.close();
    }
  });

  // Live emote search, 150ms after the last key.
  $effect(() => {
    const q = text;
    clearTimeout(timer);
    active = 0;
    if (!q) {
      emotes = [];
      total = 0;
      searching = false;
      return;
    }
    searching = true;
    const mine = ++seq;
    timer = setTimeout(async () => {
      try {
        const r = await gql(SEARCH, { query: q, animated: null, page: 1, perPage: 6 });
        if (mine !== seq) return;
        emotes = r.searchEmotes.items;
        total = r.searchEmotes.total;
        failed = false;
      } catch {
        if (mine !== seq) return;
        emotes = [];
        failed = true;
      } finally {
        if (mine === seq) searching = false;
      }
    }, 150);
    return () => clearTimeout(timer);
  });

  function onclose() {
    palette.open = false;
    back?.focus?.();
    back = null;
  }

  async function go(row: Row | undefined) {
    if (!row) return;
    palette.open = false;
    palette.query = '';
    await goto(row.href);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      active = move(active, e.key === 'ArrowDown' ? 1 : -1, rows.length);
      document.getElementById(rows[active]?.id ?? '')?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Home' && e.ctrlKey) {
      active = 0;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      go(rows[active]);
    }
  }

  function onwindowkey(e: KeyboardEvent) {
    if ((e.key === 'k' || e.key === 'K') && (e.ctrlKey || e.metaKey) && !e.altKey) {
      e.preventDefault();
      palette.open = !palette.open;
    } else if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !palette.open && !typing(e.target)) {
      e.preventDefault();
      // A page with its own search field (/search) takes "/" there instead of the palette.
      const own = document.querySelector<HTMLInputElement>('input[data-page-search]');
      if (own) own.focus();
      else palette.open = true;
    }
  }

  function onclick(e: MouseEvent) {
    if (e.target === dialog) palette.open = false;
  }

  function emoteWidth(e: Emote) {
    const s = size(e);
    return Math.round((28 * s.width) / s.height);
  }
</script>

<svelte:window onkeydown={onwindowkey} />

<dialog class="palette" bind:this={dialog} aria-label="Search cyex.tv" {onclose} {onclick}>
  <div class="box">
    <div class="pal-field">
      <Icon name="search" size={18} />
      <input
        bind:this={input}
        bind:value={palette.query}
        type="text"
        role="combobox"
        aria-expanded="true"
        aria-controls="palette-list"
        aria-activedescendant={activeId}
        aria-autocomplete="list"
        aria-label="Search emotes, channels and pages"
        placeholder="Emotes, channels, pages"
        autocomplete="off"
        autocapitalize="none"
        autocorrect="off"
        spellcheck="false"
        enterkeyhint="go"
        {onkeydown}
      />
      <kbd class="esc">Esc</kbd>
    </div>

    <div class="list" id="palette-list" role="listbox" aria-label="Results">
      {#if text}
        <div class="group" role="group" aria-labelledby="pg-emotes">
          <p class="group-name" id="pg-emotes" role="presentation">
            Emotes
            {#if searching}<span class="state">Searching</span>{:else if failed}<span class="state">Search isn’t available right now</span>{:else if total > 6}<span class="state">6 of {count(total)}</span>{/if}
          </p>
          {#each emoteRows as r (r.id)}
            {@const i = rows.indexOf(r)}
            <a
              class="row"
              id={r.id}
              href={r.href}
              role="option"
              aria-selected={i === active}
              tabindex="-1"
              onpointermove={() => (active = i)}
              onclick={(e) => {
                e.preventDefault();
                go(r);
              }}
            >
              {#if r.kind === 'emote'}
                <span class="art">
                  {#if ready(r.emote)}
                    <img
                      src={src(r.emote, 1, hasStill(r.emote))}
                      srcset={srcsetW(r.emote, hasStill(r.emote))}
                      sizes="{emoteWidth(r.emote)}px"
                      width={emoteWidth(r.emote)}
                      height="28"
                      alt=""
                      decoding="async"
                    />
                  {/if}
                </span>
                <span class="label"><bdi>{clean(r.emote.name)}</bdi></span>
                {#if r.emote.owner}<span class="hint"><bdi>{clean(r.emote.owner.displayName)}</bdi></span>{/if}
              {:else if r.kind === 'all'}
                <span class="art"><Icon name="search" /></span>
                <span class="label">{r.label}</span>
              {/if}
            </a>
          {/each}
        </div>
      {/if}

      {#if channelRows.length}
        <div class="group" role="group" aria-labelledby="pg-channels">
          <p class="group-name" id="pg-channels" role="presentation">Channels</p>
          {#each channelRows as r (r.id)}
            {@const i = rows.indexOf(r)}
            <a class="row" id={r.id} href={r.href} role="option" aria-selected={i === active} tabindex="-1"
              onpointermove={() => (active = i)} onclick={(e) => { e.preventDefault(); go(r); }}>
              <span class="art"><Icon name="tv" /></span>
              <span class="label">{r.kind === 'channel' ? r.label : ''}</span>
            </a>
          {/each}
        </div>
      {/if}

      {#if pageRows.length}
        <div class="group" role="group" aria-labelledby="pg-pages">
          <p class="group-name" id="pg-pages" role="presentation">Pages</p>
          {#each pageRows as r (r.id)}
            {@const i = rows.indexOf(r)}
            <a class="row" id={r.id} href={r.href} role="option" aria-selected={i === active} tabindex="-1"
              onpointermove={() => (active = i)} onclick={(e) => { e.preventDefault(); go(r); }}>
              <span class="art"><Icon name="file" /></span>
              <span class="label">{r.kind === 'page' ? r.label : ''}</span>
              {#if r.kind === 'page' && r.hint}<span class="hint">{r.hint}</span>{/if}
            </a>
          {/each}
        </div>
      {/if}

    </div>

    {#if nothing && !searching}<p class="empty">{emptyText(text)}</p>{/if}
    <p class="sr-only" role="status">{status}</p>

    <p class="keys" aria-hidden="true">
      <span><kbd>↑</kbd><kbd>↓</kbd> to move</span>
      <span><kbd>Enter</kbd> to open</span>
      <span><kbd>Esc</kbd> to close</span>
    </p>
  </div>
</dialog>

<style>
  :global(html:has(dialog.palette[open])) { overflow: hidden; }
  .palette { width: min(640px, calc(100% - 32px)); max-width: none; max-height: none; margin: 12vh auto auto; padding: 0;
    border: 0; background: transparent; color: var(--text); overflow: visible; }
  .palette::backdrop { background: var(--scrim); }
  .box { display: flex; flex-direction: column; max-height: min(560px, 76dvh); border: 1px solid var(--border-strong);
    border-radius: var(--radius-m); background: var(--surface); box-shadow: var(--shadow-pop); overflow: hidden; }
  .pal-field { flex: none; display: flex; align-items: center; gap: 12px; padding: 0 14px 0 16px; height: 56px;
    border-bottom: 1px solid var(--border); color: var(--muted); border-radius: var(--radius-m) var(--radius-m) 0 0; }
  /* The input draws no outline of its own; the whole field row shows focus. */
  .pal-field:focus-within { outline: 2px solid var(--accent-text); outline-offset: -2px; }
  input { flex: 1; min-width: 0; height: 100%; border: 0; background: none; color: var(--text); font: inherit; font-size: 17px; }
  input:focus, input:focus-visible { outline: none; }
  input::placeholder { color: var(--muted); }
  kbd { display: inline-grid; place-items: center; min-width: 22px; height: 22px; padding: 0 6px; border: 1px solid var(--border-strong);
    border-radius: var(--radius-btn); background: var(--surface-2); color: var(--muted); font-size: 12px; font-weight: 500; line-height: 1; }
  .list { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; padding: 6px; }
  .group + .group { margin-top: 4px; padding-top: 4px; border-top: 1px solid var(--border); }
  .group-name { display: flex; justify-content: space-between; gap: 12px; margin: 0; padding: 8px 10px 6px;
    font-size: var(--t-fine); font-weight: 500; color: var(--muted); }
  .state { font-weight: 400; font-variant-numeric: tabular-nums; }
  .row { display: flex; align-items: center; gap: 12px; min-height: 44px; padding: 6px 10px; border-radius: var(--radius-btn);
    color: var(--text); font-size: var(--t-ui); cursor: pointer; }
  .row[aria-selected='true'] { background: var(--surface-2); box-shadow: 0 0 0 1px var(--border-strong) inset; }
  .art { flex: none; display: inline-grid; place-items: center; min-width: 32px; height: 28px; color: var(--muted); }
  .art img { display: block; height: 28px; width: auto; max-width: 64px; }
  .label { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .hint { margin-left: auto; padding-left: 12px; flex: none; max-width: 45%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: var(--t-small); color: var(--muted); }
  .empty { flex: none; margin: 0; padding: 8px 16px 16px; color: var(--muted); font-size: var(--t-small); overflow-wrap: anywhere; }
  .keys { flex: none; display: flex; flex-wrap: wrap; gap: 6px 18px; margin: 0; padding: 10px 16px; border-top: 1px solid var(--border);
    font-size: var(--t-fine); color: var(--muted); }
  .keys span { display: inline-flex; align-items: center; gap: 4px; }
  /* Phones and touch: no keyboard hints. */
  @media (pointer: coarse), (max-width: 520px) { .keys, .esc { display: none; } }
  @media (max-width: 520px) { .palette { margin-top: max(12px, env(safe-area-inset-top, 0px)); } }
</style>
