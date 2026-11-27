<script lang="ts">
  // The one person filter (activity "By", the audit log's actor and target, orders' buyer, the
  // review queue's uploader, the admin account pickers). A search field: typing lists the top
  // matching people (avatar, painted name, login, an optional role flag), 150ms after the last key,
  // a newer request aborting the older. Focusing it while it is empty lists the 5 most important
  // people for this list (`top`: the most entries in it, `pinned` first, `fallback` when nobody has
  // any); unfocused, nothing shows. The picked person is a chip with a remove button; the page keeps
  // the ID in its address. An ARIA 1.2 combobox as the header search: ↑ ↓ move the active row
  // (focus stays in the field), Enter picks it, Escape closes the list (a second clears the text),
  // Tab leaves. The list sits in the top layer, so no toolbar or panel can clip it.
  import { onDestroy, tick } from 'svelte';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { lookUp, who } from '$lib/admin/names.svelte';
  import { clean, count as fmt } from '$lib/format';
  import { DEBOUNCE, loginHint, queryOf, searchPeople, searchable, step, topList, type Person } from './user-filter';

  type Props = {
    /** The picked account's ID, '' for anyone. */
    value: string;
    onchange: (id: string, person: Person | null) => void;
    /** The field's name ("By", "Actor", "Uploader"); visible unless `hideLabel`. */
    label: string;
    hideLabel?: boolean;
    placeholder?: string;
    /** The most important people for this list, asked on the first empty focus and again when `topKey` changes. */
    top?: (signal: AbortSignal) => Promise<Person[]>;
    topKey?: string;
    /** What the top list is called ("Most active here"). */
    topLabel?: string;
    /** People always first in the top list (the profile's owner on their own profile). */
    pinned?: Person[];
    /** Who to offer when the top list is empty (the editors, the staff). */
    fallback?: Person[];
    /** A small flag after a name ("Owner", "Editor", "Staff"). */
    flag?: (p: Person) => string | null | undefined;
    /** The search; the site's account search by default. */
    search?: (query: string, signal: AbortSignal) => Promise<Person[]>;
    /** Takes a pasted account ID as it is (admin pickers). */
    ids?: (text: string) => string | null;
    /** The chip's name when the page knows it (else it is looked up). */
    name?: string | null;
    width?: string;
    id?: string;
  };
  let {
    value,
    onchange,
    label,
    hideLabel = false,
    placeholder = 'Anyone',
    top,
    topKey = '',
    topLabel = 'Most active here',
    pinned = [],
    fallback = [],
    flag,
    search = searchPeople,
    ids,
    name = null,
    width = '220px',
    id,
  }: Props = $props();

  const uid = $props.id();
  const inputId = $derived(id ?? `${uid}-q`);
  const listId = `${uid}-list`;
  let input = $state<HTMLInputElement>();
  let root = $state<HTMLDivElement>();
  let drop = $state<HTMLDivElement>();
  let text = $state('');
  let focused = $state(false);
  let dismissed = $state(false);
  let active = $state(-1);
  /** The search field shows instead of the chip (no one picked, or "change" was pressed). */
  let editing = $state(false);

  // Typed results, for `shownFor`; they stay up while the next ones load.
  let found = $state<Person[]>([]);
  let shownFor = $state('');
  let searching = $state(false);
  let failed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let inflight: AbortController | null = null;

  // The most important people, per `topKey`.
  let counted = $state<Person[] | null>(null);
  let countedFor = $state<string | null>(null);
  let topFailed = $state(false);
  let topAsk: AbortController | null = null;

  /** People picked or listed here, so the chip can draw a name without asking again. */
  const seen = new Map<string, Person>();

  const q = $derived(queryOf(text));
  const pastedId = $derived(ids && q ? ids(q) : null);
  const topRows = $derived(topList(counted ?? [], pinned, fallback));
  const rows = $derived<Person[]>(q ? (shownFor === q || found.length ? found : []) : topRows);
  const open = $derived(focused && !dismissed && (!!q || !!top || pinned.length > 0 || fallback.length > 0));
  const activeId = $derived(open && active >= 0 && rows[active] ? `${uid}-o-${rows[active].account.id}` : undefined);
  const showInput = $derived(!value || editing);

  $effect(() => wantLooks(rows.map((r) => r.account.id)));
  $effect(() => {
    if (value && !name && !seen.has(value)) lookUp([value]);
  });
  const chipPerson = $derived.by<Person | null>(() => {
    if (!value) return null;
    const p = seen.get(value) ?? [...pinned, ...fallback, ...(counted ?? [])].find((x) => x.account.id === value);
    if (p) return p;
    const known = who(value);
    const shown = name ?? known?.name;
    return { account: { id: value, displayName: shown ?? '', avatar: known?.avatar ?? null }, identities: [] };
  });
  $effect(() => wantLooks(value ? [value] : []));

  // Live search, DEBOUNCE ms after the last key; a newer request aborts the one before.
  $effect(() => {
    const query = q;
    clearTimeout(timer);
    active = -1;
    if (!query || !searchable(query)) {
      inflight?.abort();
      inflight = null;
      found = [];
      shownFor = query;
      searching = false;
      failed = false;
      return;
    }
    searching = true;
    failed = false;
    timer = setTimeout(async () => {
      inflight?.abort();
      const ctl = new AbortController();
      inflight = ctl;
      try {
        const got = await search(query, ctl.signal);
        if (ctl.signal.aborted) return;
        found = got;
        for (const p of got) seen.set(p.account.id, p);
      } catch {
        if (ctl.signal.aborted) return;
        found = [];
        failed = true;
      }
      shownFor = query;
      searching = false;
      if (inflight === ctl) inflight = null;
    }, DEBOUNCE);
    return () => clearTimeout(timer);
  });

  async function loadTop() {
    if (!top || countedFor === topKey) return;
    topAsk?.abort();
    const ctl = new AbortController();
    topAsk = ctl;
    const key = topKey;
    countedFor = key;
    counted = null;
    topFailed = false;
    try {
      const got = await top(ctl.signal);
      if (ctl.signal.aborted) return;
      counted = got;
      for (const p of got) seen.set(p.account.id, p);
    } catch {
      if (ctl.signal.aborted) return;
      counted = [];
      topFailed = true;
      countedFor = null; // try again on the next focus
    }
  }
  // A new list (other filters): the top people are asked again, now if the field is open.
  $effect(() => {
    void topKey;
    if (countedFor !== null && countedFor !== topKey) {
      countedFor = null;
      if (focused && !q) void loadTop();
    }
  });
  onDestroy(() => {
    inflight?.abort();
    topAsk?.abort();
    clearTimeout(timer);
  });

  // The list floats in the top layer, under the field (above it when there is no room below).
  function place() {
    if (!drop || !root) return;
    const r = root.getBoundingClientRect();
    const w = Math.max(r.width, Math.min(320, window.innerWidth - 16));
    const left = Math.min(Math.max(8, r.left), window.innerWidth - w - 8);
    const below = window.innerHeight - r.bottom;
    drop.style.left = `${left}px`;
    drop.style.width = `${w}px`;
    if (below < 220 && r.top > below) {
      drop.style.top = '';
      drop.style.bottom = `${window.innerHeight - r.top + 4}px`;
    } else {
      drop.style.bottom = '';
      drop.style.top = `${r.bottom + 4}px`;
    }
  }
  $effect(() => {
    if (!drop) return;
    const el = drop as HTMLDivElement & { showPopover?: () => void; hidePopover?: () => void };
    if (open) {
      place();
      try {
        if (!el.matches(':popover-open')) el.showPopover?.();
      } catch {
        /* no popover API: it is positioned fixed all the same */
      }
      const follow = () => place();
      window.addEventListener('scroll', follow, true);
      window.addEventListener('resize', follow);
      return () => {
        window.removeEventListener('scroll', follow, true);
        window.removeEventListener('resize', follow);
      };
    }
    try {
      if (el.matches(':popover-open')) el.hidePopover?.();
    } catch {
      /* as above */
    }
  });

  function onfocus() {
    focused = true;
    dismissed = false;
    if (!q) void loadTop();
  }
  function onfocusout(e: FocusEvent) {
    const to = e.relatedTarget as Node | null;
    if (root?.contains(to) || drop?.contains(to)) return;
    focused = false;
    active = -1;
    if (value) editing = false;
  }

  function pick(p: Person | null) {
    if (p) seen.set(p.account.id, p);
    text = '';
    focused = false;
    editing = false;
    active = -1;
    input?.blur();
    onchange(p ? p.account.id : '', p);
  }
  function pickId(raw: string) {
    text = '';
    editing = false;
    focused = false;
    input?.blur();
    onchange(raw, null);
  }
  async function change() {
    editing = true;
    await tick();
    input?.focus();
  }
  async function clear() {
    onchange('', null);
    editing = false;
    await tick();
    input?.focus();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      dismissed = false;
      if (!q) void loadTop();
      active = step(active, e.key === 'ArrowDown' ? 1 : -1, rows.length);
      if (activeId) document.getElementById(activeId)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter') {
      if (e.isComposing) return;
      e.preventDefault();
      if (open && active >= 0 && rows[active]) pick(rows[active]);
      else if (pastedId) pickId(pastedId);
      else if (open && q && rows.length === 1) pick(rows[0]);
    } else if (e.key === 'Escape') {
      if (open) {
        e.preventDefault();
        e.stopPropagation();
        dismissed = true;
        active = -1;
      } else if (text) {
        e.preventDefault();
        e.stopPropagation();
        text = '';
      } else if (value && editing) {
        e.preventDefault();
        editing = false;
      }
    } else if (e.key === 'Tab') {
      dismissed = true;
    }
  }
  function oninput() {
    dismissed = false;
  }
  const where = (p: string) => (p === 'KICK' ? 'kick' : 'twitch');
</script>

{#snippet personRow(p: Person, i: number)}
  {@const l = looks[p.account.id]}
  {@const f = flag?.(p)}
  <div
    class="opt"
    id="{uid}-o-{p.account.id}"
    role="option"
    aria-selected={i === active}
    tabindex="-1"
    onpointermove={() => (active = i)}
    onpointerdown={(e) => e.preventDefault()}
    onclick={() => pick(p)}
    onkeydown={() => {}}
  >
    <Avatar src={p.account.avatar} name={p.account.displayName} size={24} tone="tool" />
    <span class="who">
      <span class="nm"><PaintedName name={p.account.displayName} paint={l?.paint} badge={l?.badge} badgeSize={16} /></span>
      {#if p.identities[0]}<span class="login"><Icon name={where(p.identities[0].platform)} size={12} /><bdi>{clean(p.identities[0].login)}</bdi><span class="sr-only">{loginHint(p).slice(p.identities[0].login.length)}</span></span>{/if}
    </span>
    {#if f}<span class="flag">{f}</span>{/if}
    {#if p.count !== undefined && !q}<span class="n" title="{fmt(p.count)} {p.count === 1 ? 'entry' : 'entries'}">{fmt(p.count)}<span class="sr-only"> {p.count === 1 ? 'entry' : 'entries'}</span></span>{/if}
    {#if p.account.id === value}<span class="on" aria-hidden="true"><Icon name="check" size={14} /></span>{/if}
  </div>
{/snippet}

<div class="uf" class:labelled={!hideLabel} style:--uf-w={width} bind:this={root} {onfocusout} data-testid="user-filter">
  <label class="lbl" class:sr-only={hideLabel} for={inputId}>{label}</label>
  {#if showInput}
    <div class="field" class:on={focused}>
      <Icon name="search" size={14} />
      <input
        bind:this={input}
        bind:value={text}
        id={inputId}
        type="search"
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-activedescendant={activeId}
        aria-autocomplete="list"
        placeholder={value && chipPerson ? clean(chipPerson.account.displayName) || placeholder : placeholder}
        autocomplete="off"
        autocapitalize="none"
        spellcheck="false"
        maxlength="100"
        {onfocus}
        {onkeydown}
        {oninput}
      />
      {#if searching && q}<span class="spin" aria-hidden="true"></span>{/if}
    </div>
  {:else if chipPerson}
    {@const l = looks[chipPerson.account.id]}
    <span class="chip">
      <button class="pick" type="button" onclick={change} aria-label="{label}: {clean(chipPerson.account.displayName) || 'an account'}. Change">
        <Avatar src={chipPerson.account.avatar} name={chipPerson.account.displayName || '?'} size={20} tone="tool" />
        <span class="nm">{#if chipPerson.account.displayName}<PaintedName name={chipPerson.account.displayName} paint={l?.paint} badge={l?.badge} badgeSize={14} />{:else}<span class="mono">…{value.slice(-6)}</span>{/if}</span>
      </button>
      <button class="x" type="button" aria-label="Remove {label.toLowerCase()} filter" onclick={clear}><Icon name="x" size={14} /></button>
    </span>
  {/if}

  <div class="drop" bind:this={drop} popover="manual" hidden={!open}>
    <div class="list" id={listId} role="listbox" aria-label={q ? `People matching ${q}` : topLabel}>
      {#if q}
        {#if pastedId}
          <div class="opt id" role="option" aria-selected={false} tabindex="-1" onpointerdown={(e) => e.preventDefault()} onclick={() => pickId(pastedId)} onkeydown={() => {}}>
            <Icon name="user" size={14} /><span>Use the account ID <span class="mono">{pastedId}</span></span>
          </div>
        {/if}
        {#each rows as p, i (p.account.id)}{@render personRow(p, i)}{/each}
        {#if !searchable(q)}
          <p class="line">Type at least two letters.</p>
        {:else if shownFor === q && !searching && failed}
          <p class="line">The search isn’t available right now.</p>
        {:else if shownFor === q && !searching && !rows.length && !pastedId}
          <p class="line">No one matches “<bdi>{q}</bdi>”.</p>
        {:else if !rows.length && searching}
          <p class="line">Searching…</p>
        {/if}
      {:else}
        <p class="group">{topLabel}</p>
        {#each rows as p, i (p.account.id)}{@render personRow(p, i)}{/each}
        {#if top && counted === null && !topFailed}
          <p class="line">Loading…</p>
        {:else if !rows.length}
          <p class="line">{topFailed ? 'Couldn’t load who is most active. Type a name or login.' : 'Nobody yet. Type a name or login.'}</p>
        {/if}
      {/if}
    </div>
  </div>
  <p class="sr-only" role="status">{open && q && shownFor === q && !searching ? (rows.length ? `${rows.length} ${rows.length === 1 ? 'person' : 'people'} found` : 'No one found') : ''}</p>
</div>

<style>
  .uf { display: inline-flex; align-items: center; gap: 8px; min-width: 0; max-width: 100%; }
  /* The label shares the toolbar's text size and one vertical centre with the field and the
     controls beside it (owner, 2026/11/22: "By" sat higher than the field): same size as the
     field's text, a line box no taller than the text, centred in the field's height. */
  .lbl { flex: none; display: inline-flex; align-items: center; height: var(--ctl-h, 34px); font-size: var(--t-small); line-height: 1; color: var(--muted); }
  .field { display: flex; align-items: center; gap: 6px; width: var(--uf-w); max-width: 100%; height: var(--ctl-h, 34px); padding: 0 8px 0 10px;
    border: 1px solid var(--border); border-radius: var(--radius-btn); background: var(--surface); color: var(--muted); cursor: text;
    transition: border-color var(--dur-color), color var(--dur-color); }
  .field.on, .field:focus-within { border-color: var(--border-strong); color: var(--text); outline: 2px solid var(--focus-ring); outline-offset: -1px; }
  .field :global(svg) { flex: none; }
  input { flex: 1; min-width: 0; height: 100%; padding: 0; border: 0; background: none; color: var(--text); font: inherit; font-size: var(--t-small); }
  input:focus, input:focus-visible { outline: none; }
  input::placeholder { color: var(--muted); opacity: 1; }
  input::-webkit-search-cancel-button { display: none; }
  .spin { flex: none; width: 12px; height: 12px; border: 2px solid var(--border-strong); border-top-color: var(--muted); border-radius: 50%; animation: ui-spin .7s linear infinite; }
  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 2s; } }

  /* The picked person: a 3px chip with the painted name; the name reopens the search, × removes it. */
  .chip { display: inline-flex; align-items: center; max-width: var(--uf-w); height: var(--ctl-h, 34px); border: 1px solid var(--border-strong);
    border-radius: var(--radius-btn); background: var(--surface); }
  .pick { flex: 1 1 auto; display: inline-flex; align-items: center; gap: 6px; min-width: 0; height: 100%; padding: 0 4px 0 6px; border: 0; background: none;
    color: var(--text); font: inherit; font-size: var(--t-small); font-weight: 600; cursor: pointer; }
  .pick .nm { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .x { flex: none; position: relative; display: inline-grid; place-items: center; width: 24px; height: 24px; margin-right: 4px; padding: 0; border: 0;
    border-radius: var(--radius-btn); background: none; color: var(--muted); cursor: pointer; }
  .x::after { content: ''; position: absolute; inset: -5px -4px; }
  .pick:focus-visible, .x:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) { .x:hover { color: var(--text); background: var(--surface-2); } .field:hover { border-color: var(--border-strong); } }

  .drop { position: fixed; inset: auto; margin: 0; padding: 0; z-index: var(--z-pop); border: 1px solid var(--border-strong); border-radius: var(--radius-m);
    background: var(--surface); color: var(--text); box-shadow: var(--shadow-pop); overflow: hidden; }
  .drop[hidden] { display: none; }
  .list { max-height: min(340px, 60dvh); overflow-y: auto; overscroll-behavior: contain; padding: 4px; }
  .group { margin: 0; padding: 8px 8px 4px; font-size: var(--t-label, .7rem); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--muted); }
  .opt { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 4px 8px; border-radius: var(--radius-btn); cursor: pointer; font-size: var(--t-small); }
  .opt.id { color: var(--muted); gap: 8px; }
  .who { display: flex; flex-direction: column; min-width: 0; flex: 1; line-height: 1.25; }
  .who .nm { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .login { display: inline-flex; align-items: center; gap: 4px; min-width: 0; font-size: var(--t-fine); color: var(--muted); overflow: hidden; white-space: nowrap; }
  .login :global(svg) { flex: none; color: var(--dim); }
  .flag { flex: none; padding: 0 5px; border: 1px solid var(--border); border-radius: var(--radius-btn); font-size: var(--t-fine); color: var(--muted); line-height: 18px; }
  .n { flex: none; min-width: 2ch; text-align: right; font: var(--t-fine) var(--font-mono); font-variant-numeric: tabular-nums; color: var(--muted); }
  .on { flex: none; display: inline-flex; color: var(--accent-text); }
  /* The active row: bold, its icon in the accent; no fill, border or bar (owner rule). */
  .opt[aria-selected='true'] .who .nm { font-weight: 700; }
  .opt[aria-selected='true'] .login :global(svg) { color: var(--accent-text); }
  .opt[aria-selected='true'] .login { color: var(--text); }
  .line { margin: 0; padding: 8px 8px 10px; color: var(--muted); font-size: var(--t-fine); overflow-wrap: anywhere; }
  .mono { font-family: var(--font-mono); }

  @media (max-width: 639.98px) {
    .uf { width: 100%; }
    .uf.labelled .field, .uf.labelled .chip { flex: 1; width: auto; max-width: none; }
    .uf:not(.labelled) .field, .uf:not(.labelled) .chip { width: 100%; max-width: none; }
    input { font-size: 16px; } /* no zoom on focus (iOS) */
    .opt { min-height: 48px; }
    .x { width: 32px; height: 32px; }
  }
  @media (forced-colors: active) {
    .opt[aria-selected='true'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
</style>
