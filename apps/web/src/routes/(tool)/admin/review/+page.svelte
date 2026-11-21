<script lang="ts">
  // The emote review queue, a workspace: the queue on the left as a contact sheet of small tiles
  // (picture, name, uploader, flags; 50 a page, the server pages) under one toolbar (tabs by kind:
  // public, unlisted and private, resolved; a search by name or uploader; sort and order), and on
  // wide windows the picked upload in a large inspector on the right (ReviewInspector: every size,
  // chat, uploader and their history, duplicates, activity and notes, the verdicts at its foot).
  // Narrow windows open the same upload in the full-screen dialog (ReviewDetail). Keyboard first: the
  // arrows move between cards, Enter opens one, A approves, U and P approve unlisted and
  // private, R rejects, S skips (on to the next, no verdict), Z undoes an approval (5 s), Esc
  // closes; after a verdict the next upload is up. The header counts what is left.
  // Many at once: tick cards (X, Shift for a range, Ctrl+A for the page, at most 100) and a bar
  // approves, approves as unlisted or for the owner only, or rejects them all with one reason, in
  // one call (reviewEmotes) with one audit row each. Ready reasons come from the API; staff with
  // review.settings change them on /admin/review/reasons.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import { pictureFor, ready, size } from '$lib/emote';
  import { display } from '$lib/dpr.svelte';
  import { onDestroy, tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { clean, count, date, dateTime, iso } from '$lib/format';
  import type { EmoteFieldsFragment, EmoteVisibility, ReviewSort, ReviewTab } from '$lib/gql/graphql';
  import { APPROVE, REJECT, REQUEUE_EMOTES, REVIEW_EMOTES, REVIEW_QUEUE, REVIEW_REASONS } from '$lib/queries';
  import IconButton from '$lib/components/IconButton.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ReviewDetail from '$lib/admin/ReviewDetail.svelte';
  import ReviewInspector from '$lib/admin/ReviewInspector.svelte';
  import Bar from '$lib/admin/ui/Bar.svelte';
  import Split from '$lib/admin/ui/Split.svelte';
  import BulkBar from '$lib/admin/BulkBar.svelte';
  import PickBox from '$lib/admin/PickBox.svelte';
  import { batchSummary } from '$lib/admin/bulk';
  import { all as pickAll, EMPTY, range as pickRange, toggle as pickToggle, type Selection, type Step } from '$lib/components/browse/selection';
  import { after, approveFailure, step, toEnd, withheld } from '$lib/admin/queue';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import ReasonDialog from '$lib/tool/ReasonDialog.svelte';
  import { REVIEW_DETAIL_KEYS, REVIEW_KEYS } from '$lib/admin/review-keys';
  import { registerShortcuts } from '$lib/shell/shortcuts.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { dismiss, fail, notify } from '$lib/tool/notices.svelte';
  import { can } from '$lib/tool/session.svelte';
  import { quiet } from '$lib/quiet';

  const PER_PAGE = 50;
  // The ready reasons staff keep (reviewReasons); none until they load, and none if that fails.
  let reasons = $state<string[]>([]);
  $effect(() => {
    void quiet(gql(REVIEW_REASONS, {}).then((r) => (reasons = r.reviewReasons)), 'review reasons');
  });
  const editReasons = can('review.settings');
  type Tab = 'public' | 'limited' | 'resolved';
  type Sort = 'uploaded' | 'reviewed' | 'uploader' | 'name';

  // The address holds the view: tab, search, sort, order, page.
  const p = $derived(page.url.searchParams);
  const tab = $derived<Tab>(p.get('tab') === 'limited' ? 'limited' : p.get('tab') === 'resolved' ? 'resolved' : 'public');
  const resolved = $derived(tab === 'resolved');
  const sort = $derived.by<Sort>(() => {
    const s = p.get('sort');
    if (s === 'uploader' || s === 'name' || (s === 'reviewed' && resolved) || s === 'uploaded') return s;
    return resolved ? 'reviewed' : 'uploaded';
  });
  const desc = $derived(p.has('order') ? p.get('order') === 'desc' : resolved);
  const pageNo = $derived(Math.max(1, Number(p.get('page')) || 1));
  let search = $state(page.url.searchParams.get('q') ?? '');

  function go(next: Record<string, string | null>, replace = false) {
    const q = new URLSearchParams(page.url.searchParams);
    for (const [k, v] of Object.entries(next)) if (v) q.set(k, v); else q.delete(k);
    if (!('page' in next)) q.delete('page');
    const s = q.toString();
    return goto(s ? `?${s}` : '?', { keepFocus: true, noScroll: replace, replaceState: replace });
  }
  let typing: ReturnType<typeof setTimeout> | undefined;
  function onsearch() {
    clearTimeout(typing);
    typing = setTimeout(() => go({ q: search.trim() || null }, true), 250);
  }

  // When the last load started: an approval sent after it is still in what it brought.
  let loadedAt = 0;
  const L = loader(async ({ fetch }) => {
    loadedAt = Date.now();
    const filter = {
      tab: tab.toUpperCase() as ReviewTab,
      search: p.get('q') || null,
      sort: sort.toUpperCase() as ReviewSort,
      descending: desc,
    };
    return (await gql(REVIEW_QUEUE, { page: pageNo, perPage: PER_PAGE, filter }, fetch)).reviewQueue;
  });
  // Approvals waiting out their undo time (see approve below), and the uploads in them.
  const waiting = new SvelteMap<string, Waiting>();
  const held = $derived([...waiting.values()].flatMap((w) => w.es));
  // What the page shows: the loaded queue without the approvals still waiting out their undo time
  // (the server has them pending until they are sent, so every load brings them back).
  const view = $derived(withheld(L.data, held, tab.toUpperCase()));
  const items = $derived(view.items);
  const total = $derived(view.total);
  const pages = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));
  const counts = $derived(L.data ? view.counts : undefined);

  // Its keys (onkeydown below and ReviewDetail's), in the shortcut sheet.
  $effect(() => {
    const a = registerShortcuts(REVIEW_KEYS);
    const b = registerShortcuts(REVIEW_DETAIL_KEYS);
    return () => (a(), b());
  });

  let busy = $state<string | null>(null);
  let rejecting = $state<EmoteFieldsFragment | null>(null);
  let reasonDialog: ReturnType<typeof ReasonDialog>;
  let detail: ReturnType<typeof ReviewDetail>;
  let grid = $state<HTMLUListElement>();

  // Picked cards (on this page only) for a verdict on many.
  let sel = $state<Selection>(EMPTY);
  let selNote = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  const picked = $derived(new Set(sel.ids));
  function pick(step: Step) {
    sel = step.sel;
    if (step.refused) {
      clearTimeout(noteTimer);
      selNote = 'That’s the most at once: 100.';
      noteTimer = setTimeout(() => (selNote = ''), 3000);
    }
  }
  const ids = () => items.map((e) => e.id);
  function pickOne(id: string, shift: boolean) {
    pick(shift ? pickRange(sel, ids(), id) : pickToggle(sel, id));
  }
  // A new page, tab or search, or a card gone: what isn't shown any more drops out.
  $effect(() => {
    if (!L.data) return;
    const shown = new Set(items.map((e) => e.id));
    untrack(() => {
      if (resolved && sel.ids.length) sel = EMPTY;
      else if (sel.ids.some((id) => !shown.has(id))) sel = { ids: sel.ids.filter((id) => shown.has(id)), anchor: sel.anchor && shown.has(sel.anchor) ? sel.anchor : null };
    });
  });

  // The focused card (the grid's one tab stop) and the one the dialog shows.
  let cur = $state<string | null>(null);
  let shownId = $state<string | null>(null);
  let openAfterLoad: 'first' | 'last' | null = null;
  const at = $derived(Math.max(0, items.findIndex((e) => e.id === cur)));
  const shown = $derived(items.find((e) => e.id === shownId) ?? null);
  const shownAt = $derived(items.findIndex((e) => e.id === shownId));

  $effect(() => {
    // A new page: put focus state on its first card, or open the one the dialog walked to.
    if (!L.data) return;
    const list = items;
    if (openAfterLoad && list.length) {
      const e = openAfterLoad === 'first' ? list[0] : list[list.length - 1];
      openAfterLoad = null;
      shownId = cur = e.id;
    } else if (!list.some((e) => e.id === cur)) cur = list[0]?.id ?? null;
  });

  const card = (id: string) => grid?.querySelector<HTMLElement>(`[data-card="${CSS.escape(id)}"]`);
  function focusCard(id: string) {
    cur = id;
    const el = card(id);
    el?.focus();
    el?.scrollIntoView({ block: 'nearest', behavior: 'instant' });
  }
  const columns = () => (grid ? getComputedStyle(grid).gridTemplateColumns.split(' ').filter(Boolean).length : 1);

  // Wide windows (the inspector beside the queue, as Split lays it out): the focused tile is the
  // one inspected; nothing opens. Narrow ones open the dialog.
  let wide = $state(true);
  $effect(() => {
    const m = matchMedia('(min-width: 1100px)');
    wide = m.matches;
    const on = () => {
      wide = m.matches;
      if (wide) detail?.close();
    };
    m.addEventListener('change', on);
    return () => m.removeEventListener('change', on);
  });
  const inspected = $derived(wide ? (items.find((e) => e.id === cur) ?? null) : null);
  const inspectedAt = $derived(inspected ? items.indexOf(inspected) : -1);

  function openDetail(id: string) {
    if (wide) {
      cur = id;
      focusCard(id);
      return;
    }
    shownId = cur = id;
    detail.open();
  }
  function closed() {
    const id = shownId;
    shownId = null;
    if (id && items.some((e) => e.id === id)) tick().then(() => focusCard(id));
    else if (cur) tick().then(() => cur && focusCard(cur));
  }

  function move(delta: 1 | -1) {
    const i = shownAt + delta;
    if (i >= 0 && i < items.length) {
      shownId = cur = items[i].id;
    } else if (delta === 1 && pageNo < pages) {
      openAfterLoad = 'first';
      go({ page: String(pageNo + 1) });
    } else if (delta === -1 && pageNo > 1) {
      openAfterLoad = 'last';
      go({ page: pageNo > 2 ? String(pageNo - 1) : null });
    }
  }

  function onkeydown(ev: KeyboardEvent) {
    if (ev.defaultPrevented || !items.length) return;
    if (document.querySelector('dialog[open]')) return;
    const t = ev.target as HTMLElement | null;
    const inPanel = !!t?.closest('[data-inspector]');
    const onCard = !t || t === document.body || !!t.closest('[data-card]') || inPanel;
    // Ctrl/⌘+A on the grid: every card on this page (up to the cap).
    if ((ev.ctrlKey || ev.metaKey) && !ev.altKey && ev.key.toLowerCase() === 'a' && onCard && !resolved && !t?.closest('input, textarea, [contenteditable]')) {
      ev.preventDefault();
      pick(pickAll(sel, ids()));
      return;
    }
    if (ev.ctrlKey || ev.metaKey || ev.altKey) return;
    if (ev.key === 'Escape' && sel.ids.length && !t?.closest('input, textarea, select')) {
      ev.preventDefault();
      sel = EMPTY;
      return;
    }
    if (t?.closest('input, textarea, select, [contenteditable], [role="radiogroup"]')) return;
    const onButton = !!t?.closest('a, button');
    const next = step(ev.key, at, items.length, columns());
    if (next !== null && onCard && !inPanel && !(onButton && (ev.key === 'Home' || ev.key === 'End'))) {
      ev.preventDefault();
      focusCard(items[next].id);
      return;
    }
    if (!onCard || (onButton && !inPanel)) return;
    if (inPanel && onButton && (ev.key === 'Enter' || ev.key === ' ')) return;
    const e = inPanel ? inspected : items[at];
    if (!e) return;
    const k = ev.key.toLowerCase();
    if (ev.key === 'Enter' || ev.key === ' ') {
      if (inPanel) return;
      ev.preventDefault();
      openDetail(e.id);
    } else if (k === 's' && !ev.shiftKey) {
      ev.preventDefault();
      skip();
    } else if (k === 'x' && !resolved && !ev.repeat) {
      ev.preventDefault();
      pickOne(e.id, ev.shiftKey);
    } else if (!resolved && !ev.repeat && busy !== e.id) {
      if (k === 'a' && ready(e)) (ev.preventDefault(), approve(e));
      else if (k === 'u' && ready(e)) (ev.preventDefault(), approve(e, 'UNLISTED'));
      else if (k === 'p' && ready(e)) (ev.preventDefault(), approve(e, 'PRIVATE'));
      else if (k === 'r') (ev.preventDefault(), askReject(e));
    }
    if (k === 'z' && !ev.repeat && waiting.size) {
      ev.preventDefault();
      undo([...waiting.keys()].pop()!);
    }
  }

  // S: on to the next card (the next page after the last), no verdict; this one stays waiting.
  function skip() {
    if (at < items.length - 1) focusCard(items[at + 1].id);
    else if (pageNo < pages) go({ page: String(pageNo + 1) });
    else notify('That was the last one on this tab.');
  }

  // A verdict takes the card out; the next one gets focus (or the dialog shows it).
  function done(id: string) {
    const q = L.data;
    if (!q) return;
    // Next is reckoned on the cards shown (approvals still waiting are not).
    const i = items.findIndex((e) => e.id === id);
    const n = after(i, items.length);
    const left = items.filter((e) => e.id !== id);
    const nextId = n === null ? null : left[n]?.id ?? null;
    const inDialog = shownId === id;
    const gone = q.items.find((e) => e.id === id);
    const counts = resolved || !gone ? q.counts : gone.visibility === 'PUBLIC' ? { ...q.counts, public: Math.max(0, q.counts.public - 1) } : { ...q.counts, limited: Math.max(0, q.counts.limited - 1) };
    L.set({ ...q, items: q.items.filter((e) => e.id !== id), total: Math.max(0, q.total - 1), counts });
    if (inDialog) {
      if (nextId) shownId = cur = nextId;
      else if (q.total - 1 > 0) {
        openAfterLoad = 'first';
        L.reload();
      } else detail.close();
    } else {
      cur = nextId;
      tick().then(() => (nextId ? focusCard(nextId) : document.querySelector<HTMLElement>('h1')?.focus()));
      if (!left.length && q.total - 1 > 0) L.reload();
    }
  }

  // An approval is not sent at once: the card leaves, a toast offers Undo for UNDO_MS, and only
  // then does the call go out (the API has no way to take an approval back, so waiting is the undo).
  // Leaving the page sends what is waiting (a call marked keepalive survives the page closing or
  // reloading, and the browser asks first while one is pending); Z undoes the latest.
  const UNDO_MS = 5000;
  const VERB: Record<EmoteVisibility, string> = { PUBLIC: 'approved', UNLISTED: 'approved as unlisted', PRIVATE: 'approved for its owner only' };
  // One entry per approval: one card, or a batch picked together (sent as one reviewEmotes call).
  type Waiting = { es: EmoteFieldsFragment[]; visibility: EmoteVisibility | null; timer: ReturnType<typeof setTimeout>; toast: number };
  let batchSeq = 0;

  /** Cards leaving the view (approvals waiting): focus, or the dialog, moves on to what follows. */
  function advance(gone: string[]) {
    const out = new Set(gone);
    const first = items.findIndex((e) => out.has(e.id));
    const left = items.filter((e) => !out.has(e.id));
    const nextId = first < 0 ? cur : (left[after(first, left.length + 1) ?? -1]?.id ?? null);
    const more = total - gone.length > 0;
    if (shownId && out.has(shownId)) {
      if (nextId) shownId = cur = nextId;
      else if (more) {
        openAfterLoad = 'first';
        L.reload();
      } else detail.close();
    } else {
      cur = nextId;
      tick().then(() => (nextId ? focusCard(nextId) : document.querySelector<HTMLElement>('h1')?.focus()));
    }
  }
  /** Sent: the cards are gone on the server too, so they leave what was loaded before the call. */
  function forget(es: EmoteFieldsFragment[], sentAt: number) {
    const q = L.data;
    if (!q) return;
    if (loadedAt >= sentAt) {
      // Loaded while the call was out: it may or may not hold them. Ask again.
      L.reload();
      return;
    }
    const out = new Set(es.map((e) => e.id));
    const pub = es.filter((e) => e.visibility === 'PUBLIC').length;
    const inTab = tab === 'public' ? pub : tab === 'limited' ? es.length - pub : es.length;
    L.set({
      ...q,
      items: q.items.filter((e) => !out.has(e.id)),
      total: Math.max(0, q.total - inTab),
      counts: resolved ? q.counts : { public: Math.max(0, q.counts.public - pub), limited: Math.max(0, q.counts.limited - (es.length - pub)) },
    });
  }

  function approve(e: EmoteFieldsFragment, visibility?: EmoteVisibility) {
    if (waiting.has(e.id)) return;
    const v = visibility ?? null;
    advance([e.id]);
    const toast = notify(`${iso(e.name)} ${VERB[v ?? 'PUBLIC']}.`, false, { ms: UNDO_MS, action: { label: 'Undo', run: () => undo(e.id) } });
    waiting.set(e.id, { es: [e], visibility: v, toast, timer: setTimeout(() => void send(e.id), UNDO_MS) });
  }
  /** The picked cards, approved together: they leave at once, with one Undo for all of them. */
  function approveMany(visibility?: EmoteVisibility) {
    const list = items.filter((e) => picked.has(e.id) && ready(e));
    if (!list.length) return;
    const v = visibility ?? null;
    const key = `batch-${++batchSeq}`;
    sel = EMPTY;
    advance(list.map((e) => e.id));
    const n = list.length;
    const toast = notify(`${count(n)} ${n === 1 ? 'upload' : 'uploads'} ${VERB[v ?? 'PUBLIC']}.`, false, { ms: UNDO_MS, action: { label: 'Undo', run: () => undo(key) } });
    waiting.set(key, { es: list, visibility: v, toast, timer: setTimeout(() => void send(key), UNDO_MS) });
  }
  const keepalive: typeof fetch = (input, init) => fetch(input, { ...init, keepalive: true });
  async function send(key: string, leaving = false) {
    const w = waiting.get(key);
    if (!w) return;
    clearTimeout(w.timer);
    const how = leaving ? keepalive : fetch;
    const sentAt = Date.now();
    try {
      if (w.es.length === 1) await gql(APPROVE, { id: w.es[0].id, visibility: w.visibility }, how);
      else {
        const out = (await gql(REVIEW_EMOTES, { ids: w.es.map((e) => e.id), verdict: 'APPROVE', visibility: w.visibility }, how)).reviewEmotes;
        if (out.failed.length && !leaving) {
          notify(batchSummary(out, VERB[w.visibility ?? 'PUBLIC']), true);
          waiting.delete(key);
          L.reload();
          return;
        }
      }
      // Out of the waiting list and out of what was loaded, in one step, so nothing flickers back.
      forget(w.es, sentAt);
      waiting.delete(key);
    } catch (err) {
      waiting.delete(key); // back on the page from what it already holds
      if (leaving) return; // the page is going; there is nobody to tell
      if ((err as { code?: unknown } | null)?.code === 'signed_out') {
        // The session ended: the signed-out notice says so once. A reload would fail the same way and
        // blank the queue, so the cards come back from what the page already holds.
        fail(err);
      } else {
        notify(approveFailure(err, w.es.length === 1 ? iso(w.es[0].name) : `${count(w.es.length)} uploads`), true);
        L.reload(); // they are still in the queue: bring the cards back
      }
    }
  }
  /** Several cards out at once (a batch rejection): counts down, focus on what follows the first of them. */
  function doneMany(gone: string[]) {
    const q = L.data;
    if (!q) return;
    const out = new Set(gone);
    const first = items.findIndex((e) => out.has(e.id));
    const left = items.filter((e) => !out.has(e.id));
    const leaving = q.items.filter((e) => out.has(e.id));
    const pub = leaving.filter((e) => e.visibility === 'PUBLIC').length;
    const counts = resolved ? q.counts : { public: Math.max(0, q.counts.public - pub), limited: Math.max(0, q.counts.limited - (leaving.length - pub)) };
    L.set({ ...q, items: q.items.filter((e) => !out.has(e.id)), total: Math.max(0, q.total - leaving.length), counts });
    const nextId = left[Math.min(Math.max(0, first), left.length - 1)]?.id ?? null;
    cur = nextId;
    tick().then(() => (nextId ? focusCard(nextId) : document.querySelector<HTMLElement>('h1')?.focus()));
    if (!left.length && q.total - leaving.length > 0) L.reload();
  }
  function undo(id: string) {
    const w = waiting.get(id);
    if (!w) return;
    clearTimeout(w.timer);
    waiting.delete(id);
    dismiss(w.toast);
    // Back in the queue at the end, not where it was (ADR 0024): here at once, and on the server
    // for the next load and every other reviewer. If that call fails only the order is off.
    const ids = w.es.map((e) => e.id);
    if (L.data) L.set(toEnd(L.data, ids));
    gql(REQUEUE_EMOTES, { ids }).catch(() => {});
    notify(`${w.es.length === 1 ? iso(w.es[0].name) + ' is' : count(w.es.length) + ' uploads are'} back in the queue, at the end.`, false);
  }
  const flush = (leaving = false) => [...waiting.keys()].forEach((id) => void send(id, leaving));
  onDestroy(() => flush());
  // Closing or reloading inside the five seconds: ask first, and send what is waiting if they leave.
  function onbeforeunload(ev: BeforeUnloadEvent) {
    if (!waiting.size) return;
    ev.preventDefault();
    ev.returnValue = '';
  }

  function askReject(e: EmoteFieldsFragment) {
    rejecting = e;
    rejectingMany = null;
    reasonDialog.open();
  }
  // The picked cards, rejected together with one reason (no undo: a rejection never waited).
  let rejectingMany = $state<EmoteFieldsFragment[] | null>(null);
  function askRejectMany() {
    const list = items.filter((e) => picked.has(e.id));
    if (!list.length) return;
    rejecting = null;
    rejectingMany = list;
    reasonDialog.open();
  }
  async function reject(reason: string) {
    const many = rejectingMany;
    if (many) {
      try {
        const out = (await gql(REVIEW_EMOTES, { ids: many.map((e) => e.id), verdict: 'REJECT', reason })).reviewEmotes;
        sel = EMPTY;
        doneMany([...out.done]);
        notify(batchSummary(out, 'rejected'), out.failed.length > 0);
        if (out.failed.length) L.reload();
      } catch (err) {
        fail(err);
        throw err;
      }
      return;
    }
    const e = rejecting;
    if (!e) return;
    try {
      await gql(REJECT, { id: e.id, reason });
      notify(`${iso(e.name)} rejected.`);
      done(e.id);
    } catch (err) {
      fail(err);
      throw err;
    }
  }

  // Phones get the short tab names, so the counts still fit.
  let narrow = $state(false);
  $effect(() => {
    const m = matchMedia('(max-width: 520px)');
    narrow = m.matches;
    const on = () => (narrow = m.matches);
    m.addEventListener('change', on);
    return () => m.removeEventListener('change', on);
  });
  const TABS = $derived([
    { value: 'public' as Tab, label: counts ? `Public ${count(counts.public)}` : 'Public', icon: 'eye' as const },
    { value: 'limited' as Tab, label: `${narrow ? 'Unlisted' : 'Unlisted and private'}${counts ? ` ${count(counts.limited)}` : ''}`, icon: 'eye-off' as const },
    { value: 'resolved' as Tab, label: 'Resolved', icon: 'check' as const },
  ]);
  const byText = $derived(sort === 'name' || sort === 'uploader');
  const VIS_CHIP: Record<EmoteVisibility, string> = { PUBLIC: '', UNLISTED: 'Unlisted', PRIVATE: 'Private' };
</script>

<svelte:head><title>Review - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} {onbeforeunload} onpagehide={() => flush(true)} />

<Split label="Upload" size="l" open={!!inspected} onclose={() => {}}>
<Bar title="Review queue" count={counts ? `${count(counts.public + counts.limited)} left` : null}>
  {#snippet actions()}
    {#if editReasons}<a class="ui-btn secondary small" href="/admin/review/reasons"><Icon name="settings" size={15} />Reject reasons</a>{/if}
  {/snippet}
  {#snippet tools()}
    <div class="tabs"><Segmented label="Queue" size="s" options={TABS} value={tab} onchange={(v) => go({ tab: v === 'public' ? null : v, sort: null, order: null })} /></div>
    <label class="adm-find grow find">
      <Icon name="search" size={15} />
      <span class="sr-only">Search by emote or uploader</span>
      <input class="ui-input" type="search" bind:value={search} oninput={onsearch} placeholder="Emote or uploader" autocomplete="off" spellcheck="false" maxlength="100" />
    </label>
    <select class="ui-input pick" aria-label="Sort" value={sort} onchange={(e) => go({ sort: e.currentTarget.value })}>
      <option value="uploaded">Upload date</option>
      {#if resolved}<option value="reviewed">Review date</option>{/if}
      <option value="uploader">Uploader</option>
      <option value="name">Name</option>
    </select>
    <select class="ui-input pick" aria-label="Order" value={desc ? 'desc' : 'asc'} onchange={(e) => go({ order: e.currentTarget.value })}>
      <option value="asc">{byText ? 'A to Z' : 'Oldest first'}</option>
      <option value="desc">{byText ? 'Z to A' : 'Newest first'}</option>
    </select>
    <IconButton label="Refresh" icon="refresh" variant="outline" tip="bottom-end" disabled={L.loading} onclick={() => L.reload()} />
  {/snippet}
</Bar>
<p class="sr-only" id="queue-keys">Arrow keys move between uploads. Enter opens one. A approves, U approves as unlisted, P approves for its owner only, R rejects, S skips to the next. X selects for a verdict on many. Z undoes the last approval while its notice shows.</p>

<LoadGate source={L} label="Loading the queue…">
{#snippet children(queue)}
{#if items.length === 0}
  {#if p.get('q')}
    <State title="Nothing matches.">No upload in this tab has that name or uploader.</State>
  {:else if resolved}
    <State title="Nothing resolved yet.">Approved and rejected uploads show up here.</State>
  {:else}
    <State title="Nothing waiting.">New uploads show up here.</State>
  {/if}
{:else}
  <ul class="adm-sheet queue" bind:this={grid} aria-label="Uploads" aria-describedby="queue-keys" aria-busy={L.loading}>
    {#each items as e (e.id)}
      {@const d = size(e)}
      <!-- The tile is the sheet's one tab stop (the arrow keys move it); its tick box stays reachable with Tab. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
      <li class="adm-cell card" class:cur={e.id === cur} class:gone={busy === e.id} class:plain={resolved} data-card={e.id} tabindex={e.id === cur ? 0 : -1}
        class:picked={picked.has(e.id)} aria-current={wide && e.id === cur ? 'true' : undefined}
        aria-label="{clean(e.name)}, by {clean(e.owner?.displayName) || 'unknown'}{picked.has(e.id) ? ', selected' : ''}"
        onfocus={() => (cur = e.id)} onclick={(ev) => { if (!(ev.target as HTMLElement).closest('a, button, label, input')) openDetail(e.id); }}>
        {#if !resolved}
          <PickBox class="tickbox" checked={picked.has(e.id)} label="Select {clean(e.name)}" onpick={(ev) => pickOne(e.id, ev.shiftKey)} />
        {/if}
        <span class="pic">
          {#if ready(e)}
            {@const shownW = Math.round((48 * d.width) / d.height)}
            {@const pic = pictureFor(e, display.dpr, false, shownW)}
            <img src={pic.src} srcset={pic.srcset} sizes={pic.srcset ? `${shownW}px` : undefined} width={d.width} height={d.height} alt="" loading="lazy" decoding="async" />
          {:else}
            <span class="ui-spinner" aria-hidden="true"></span>
          {/if}
        </span>
        <span class="nm" title={clean(e.name)}><bdi>{clean(e.name)}</bdi></span>
        <span class="sub"><bdi>{clean(e.owner?.displayName) || 'Unknown'}</bdi></span>
        <span class="adm-flags fl">
          {#if resolved}<span class={e.state === 'APPROVED' ? 'ok' : e.state === 'PENDING' ? '' : 'warn'}>{e.state === 'APPROVED' ? 'Approved' : e.state === 'REJECTED' ? 'Rejected' : e.state === 'PENDING' ? 'Pending' : 'Taken down'}</span>{/if}
          {#if !ready(e)}<span>No files</span>{/if}
          {#if e.flags.nsfw}<span class="warn">NSFW</span>{/if}
          {#if VIS_CHIP[e.visibility]}<span class="on">{VIS_CHIP[e.visibility]}</span>{/if}
          {#if e.flags.zeroWidth}<span>Zero-width</span>{/if}
          {#if e.flags.wide}<span>Wide</span>{/if}
          <time class="adm-num when" datetime={e.createdAt} title={dateTime(e.createdAt)}>{date(e.createdAt)}</time>
        </span>
      </li>
    {/each}
  </ul>
  {#if pages > 1}
    <nav class="adm-pager" aria-label="Queue pages">
      <button class="ui-btn secondary small" type="button" disabled={pageNo <= 1 || L.loading} onclick={() => go({ page: pageNo > 2 ? String(pageNo - 1) : null })}>Previous</button>
      <span class="ui-hint ui-num">{count((pageNo - 1) * PER_PAGE + 1)}-{count(Math.min(total, pageNo * PER_PAGE))} of {count(total)}</span>
      <button class="ui-btn secondary small" type="button" disabled={pageNo >= pages || L.loading} onclick={() => go({ page: String(pageNo + 1) })}>Next</button>
    </nav>
  {:else}
    <p class="adm-pager total">{count(total)} {total === 1 ? 'upload' : 'uploads'}</p>
  {/if}
{/if}
{/snippet}
</LoadGate>
{#snippet empty()}
  <div class="adm-ip-head"><span class="adm-label">Upload</span></div>
  <p class="none">Pick an upload to see it at every size, its uploader and their history.</p>
{/snippet}
{#snippet panel()}
  {#if inspected}
    <ReviewInspector
      emote={inspected}
      place={(pageNo - 1) * PER_PAGE + inspectedAt + 1}
      {total}
      busy={busy === inspected.id}
      {resolved}
      hasNext={inspectedAt < items.length - 1 || pageNo < pages}
      onskip={skip}
      onapprove={(v) => inspected && approve(inspected, v)}
      onreject={() => inspected && askReject(inspected)}
      onflags={(next) => L.data && L.set({ ...L.data, items: L.data.items.map((x) => (x.id === next.id ? { ...x, ...next } : x)) })}
    />
  {/if}
{/snippet}
</Split>

{#if sel.ids.length && !resolved}
  <BulkBar n={sel.ids.length} status={selNote} onclear={() => (sel = EMPTY)} onall={sel.ids.length < Math.min(items.length, 100) ? () => pick(pickAll(sel, ids())) : undefined}>
    <IconButton label="Approve as unlisted" icon="eye-off" variant="outline" tip="top" onclick={() => approveMany('UNLISTED')} />
    <IconButton label="Approve for their owners only" icon="user" variant="outline" tip="top" onclick={() => approveMany('PRIVATE')} />
    <button class="ui-btn danger small" type="button" onclick={askRejectMany}><Icon name="x" size={15} />Reject {count(sel.ids.length)}</button>
    <button class="ui-btn primary small" type="button" onclick={() => approveMany()}><Icon name="check" size={15} />Approve {count(sel.ids.length)}</button>
  </BulkBar>
{/if}

<ReviewDetail
  bind:this={detail}
  emote={shown}
  place={(pageNo - 1) * PER_PAGE + shownAt + 1}
  {total}
  busy={!!shown && busy === shown.id}
  {resolved}
  hasPrev={shownAt > 0 || pageNo > 1}
  hasNext={shownAt < items.length - 1 || pageNo < pages}
  onprev={() => move(-1)}
  onnext={() => move(1)}
  onskip={() => move(1)}
  onapprove={(v) => shown && approve(shown, v)}
  onreject={() => shown && askReject(shown)}
  onclose={closed}
  onflags={(next) => L.data && L.set({ ...L.data, items: L.data.items.map((x) => (x.id === next.id ? { ...x, ...next } : x)) })}
/>

<ReasonDialog
  bind:this={reasonDialog}
  title={rejectingMany ? `Reject ${count(rejectingMany.length)} ${rejectingMany.length === 1 ? 'upload' : 'uploads'}` : `Reject ${iso(rejecting?.name)}`}
  hint={rejectingMany ? 'Every uploader sees this reason. Be short and concrete.' : 'The uploader sees this. Be short and concrete.'}
  action="Reject"
  danger
  max={500}
  presets={reasons}
  onsubmit={reject}
/>

<style>
  /* The toolbar: tabs, search, sorting on one row; it wraps where the column is narrow. */
  .tabs { flex: none; max-width: 100%; overflow-x: auto; scrollbar-width: none; }
  .tabs::-webkit-scrollbar { display: none; }
  .find { flex: 1 1 180px; min-width: 0; }
  .pick { flex: none; width: auto; padding-left: 10px; }
  @media (max-width: 900px) {
    .tabs { flex-basis: 100%; }
    .find { flex-basis: 100%; max-width: none; }
    .pick { flex: 1 1 0; }
  }

  /* The queue: a contact sheet of small tiles sharing hairlines (admin.css .adm-sheet). The
     focused / inspected tile carries the accent hairline and a bold name; picked ones a tick. */
  .queue { --cell: 148px; }
  .card { height: 148px; padding: 8px 10px; gap: 1px; content-visibility: auto; contain-intrinsic-size: auto 148px; }
  .card :global(.tickbox) { position: absolute; top: 2px; left: 2px; z-index: 1; }
  .card .pic { height: 60px; margin-bottom: 4px; }
  .card .pic img { max-height: 48px; }
  .fl { flex-wrap: nowrap; overflow: hidden; margin-top: 2px; font-size: 12px; }
  .fl .when { margin-left: auto; color: var(--dim); font-size: 11px; }
  .card.cur:not([aria-current]) { box-shadow: inset 0 0 0 1px var(--border-strong); }
  .card:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .card.gone { opacity: .5; }
  .card.picked .nm { font-weight: 700; }
  .card.picked { background: none; }
  .card.picked :global(.tickbox) { color: var(--accent-text); }
  .none { margin: 0; padding: 16px; font-size: var(--t-small); color: var(--dim); }
  .total { margin-top: 16px; }
</style>
