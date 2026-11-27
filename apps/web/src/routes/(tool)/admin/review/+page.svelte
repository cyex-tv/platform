<script lang="ts">
  import { media, PANEL, SMALL } from '$lib/media.svelte';
  // The emote review queue, a workspace: the queue on the left as a contact sheet of small tiles
  // (picture, name, uploader, flags; 50 a page, the server pages) under one toolbar (tabs by kind:
  // public, unlisted and private, personal use, resolved; a search by name or uploader; one
  // uploader (UserFilter: focused empty, the 5 with the most uploads in this tab); sort and order),
  // and on wide windows the picked upload in a large inspector on the right (ReviewInspector: every
  // size, chat, uploader and their history, duplicates, activity and notes, the verdicts at its
  // foot). Narrow windows open the same upload in the full-screen dialog (ReviewDetail).
  // Two questions per upload (ADR 0038): may it be listed publicly, may it be a personal emote.
  // "Approve both" is the main action; a reviewer holding one of emotes.review and
  // emotes.review_personal gets the one they can do. Keyboard first: the arrows move between cards,
  // Enter opens one, A approves (both), L the listing, P personal use, D and N deny the listing or
  // personal use (with a reason), R rejects the upload outright, S skips (on to the next, no
  // verdict), Z undoes an approval (5 s), Esc closes; after a verdict the next upload is up. The
  // Personal use tab is its own queue: approved uploads whose personal use waits (A approves, D
  // denies). The header counts what is left.
  // Many at once: tick cards (X, Shift for a range, Ctrl+A for the page, at most 100) and a bar
  // applies the same choice to all of them, in one call (reviewApprovals; rejections reviewEmotes)
  // with one audit row each. Ready reasons come from the API; staff with review.settings change
  // them on /admin/review/reasons.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fetchWith, gql } from '$lib/api';
  import UserFilter from '$lib/components/UserFilter.svelte';
  import type { Person } from '$lib/components/user-filter';
  import { pictureFor, ready, size } from '$lib/emote';
  import { display } from '$lib/dpr.svelte';
  import { onDestroy, tick, untrack } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { clean, count, date, dateTime, iso } from '$lib/format';
  import type { EmoteFieldsFragment, ReviewSort, ReviewTab } from '$lib/gql/graphql';
  import { REJECT, REQUEUE_EMOTES, REVIEW_APPROVALS, REVIEW_EMOTES, REVIEW_QUEUE, REVIEW_TOP_UPLOADERS, REVIEW_REASONS } from '$lib/queries';
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
  import { actionDone, actionForKey, actionLabel, actionsFor, after, approvalChips, approvalsInput, approveFailure, decisionOf, needsReason, partsOf, settle, step, toEnd, withheld, type Action, type Approvals, type Decision, type Mode, type Reviewer } from '$lib/admin/queue';
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
  // What this reviewer may decide: the listing (emotes.review) and personal use (emotes.review_personal).
  const reviewer = $derived<Reviewer>({ listing: can('emotes.review'), personal: can('emotes.review_personal') });
  type Tab = 'public' | 'limited' | 'personal' | 'resolved';
  type Sort = 'uploaded' | 'reviewed' | 'uploader' | 'name';

  // The address holds the view: tab, search, uploader, sort, order, page.
  const p = $derived(page.url.searchParams);
  // Someone who only reviews personal use has just that queue.
  const tab = $derived<Tab>(!reviewer.listing || p.get('tab') === 'personal' ? 'personal' : p.get('tab') === 'limited' ? 'limited' : p.get('tab') === 'resolved' ? 'resolved' : 'public');
  const mode = $derived<Mode>(tab === 'personal' ? 'personal' : 'main');
  const actions = $derived(actionsFor(mode, reviewer));
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
      uploaderId: p.get('uploader') || null,
      sort: sort.toUpperCase() as ReviewSort,
      descending: desc,
    };
    return (await gql(REVIEW_QUEUE, { page: pageNo, perPage: PER_PAGE, filter }, fetch)).reviewQueue;
  });
  const uploader = $derived(p.get('uploader') ?? '');
  const topUploaders = async (signal: AbortSignal): Promise<Person[]> =>
    (await gql(REVIEW_TOP_UPLOADERS, { filter: { tab: tab.toUpperCase() as ReviewTab, search: p.get('q') || null }, limit: 5 }, fetchWith(signal))).reviewTopUploaders;

  // Approvals waiting out their undo time (see approve below), and the uploads in them.
  const waiting = new SvelteMap<string, Waiting>();
  const held = $derived(
    [...waiting.values()].flatMap((w) => {
      const x = partsOf(w.action);
      return w.es.map((e) => ({ id: e.id, private: e.flags.private, main: x.main, personal: x.personal && e.approvals?.personalUse === 'PENDING' }));
    }),
  );
  // What the page shows: the loaded queue without the approvals still waiting out their undo time
  // (the server has them pending until they are sent, so every load brings them back).
  const view = $derived(withheld(L.data, held, tab.toUpperCase()));
  const items = $derived(view.items);
  const total = $derived(view.total);
  const pages = $derived(Math.max(1, Math.ceil(total / PER_PAGE)));
  const counts = $derived(L.data ? view.counts : undefined);
  const left = $derived(counts ? (reviewer.listing ? counts.public + counts.limited : counts.personal) : null);

  // Its keys (onkeydown below and ReviewDetail's), in the shortcut sheet.
  $effect(() => {
    const a = registerShortcuts(REVIEW_KEYS);
    const b = registerShortcuts(REVIEW_DETAIL_KEYS);
    return () => (a(), b());
  });

  let busy = $state<string | null>(null);
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
    const m = matchMedia(PANEL);
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
      const a = actionForKey(ev.key, mode, reviewer);
      if (a && (needsReason(a) || ready(e))) (ev.preventDefault(), act(e, a));
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

  // An approval is not sent at once: the card leaves, a toast offers Undo for UNDO_MS, and only
  // then does the call go out (the API has no way to take an approval back, so waiting is the undo).
  // Leaving the page sends what is waiting (a call marked keepalive survives the page closing or
  // reloading, and the browser asks first while one is pending); Z undoes the latest.
  const UNDO_MS = 5000;
  type Card = EmoteFieldsFragment;
  // One entry per approval: one card, or a batch picked together (sent as one reviewApprovals call).
  type Waiting = { es: Card[]; action: Action; decision: Decision; uuid: string; timer: ReturnType<typeof setTimeout>; toast: number };
  let batchSeq = 0;

  /** Cards leaving the view (approvals waiting): focus, or the dialog, moves on to what follows. */
  function advance(gone: string[]) {
    const out = new Set(gone);
    const first = items.findIndex((e) => out.has(e.id));
    const rest = items.filter((e) => !out.has(e.id));
    const nextId = first < 0 ? cur : (rest[after(first, rest.length + 1) ?? -1]?.id ?? null);
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
  /** Whether a verdict takes the upload out of the tab shown (a listing verdict in the main queue, a personal one in the personal queue). */
  const leaves = (a: Action) => (mode === 'personal' ? partsOf(a).personal : partsOf(a).main);
  /**
   * The loaded queue after a verdict on `es`: the cards that leave this tab are gone (and the total
   * drops), the tab counts go down, and a card that stays shows its new approvals.
   */
  function shrink<Q extends NonNullable<typeof L.data>>(q: Q, es: Card[], a: Action): Q {
    const out = new Set(es.map((e) => e.id));
    const x = partsOf(a);
    const main = x.main ? es : [];
    const pub = main.filter((e) => !e.flags.private).length;
    const pers = x.personal ? es.filter((e) => e.approvals?.personalUse === 'PENDING').length : 0;
    const counts = resolved ? q.counts : { public: Math.max(0, q.counts.public - pub), limited: Math.max(0, q.counts.limited - (main.length - pub)), personal: Math.max(0, q.counts.personal - pers) };
    if (!leaves(a)) {
      const d = decisionOf(a);
      return { ...q, counts, items: q.items.map((e) => (out.has(e.id) ? { ...e, approvals: settle(e.approvals, d) } : e)) };
    }
    const inTab = mode === 'personal' ? es.length : tab === 'public' ? pub : tab === 'limited' ? main.length - pub : main.length;
    return { ...q, items: q.items.filter((e) => !out.has(e.id)), total: Math.max(0, q.total - inTab), counts };
  }
  /** Sent: the cards are decided on the server too, so they leave what was loaded before the call. */
  function forget(es: Card[], a: Action, sentAt: number) {
    const q = L.data;
    if (!q) return;
    if (loadedAt >= sentAt) {
      // Loaded while the call was out: it may or may not hold them. Ask again.
      L.reload();
      return;
    }
    L.set(shrink(q, es, a));
  }

  /** An approval on one card, or on the picked ones: they leave at once, with one Undo for all of them. */
  function approve(es: Card[], action: Action) {
    const decision = decisionOf(action);
    if (!decision || !es.length || es.some((e) => waiting.has(e.id))) return;
    const key = es.length === 1 ? es[0].id : `batch-${++batchSeq}`;
    if (es.length > 1) sel = EMPTY;
    if (leaves(action)) advance(es.map((e) => e.id));
    const n = es.length;
    const what = n === 1 ? iso(es[0].name) : `${count(n)} ${n === 1 ? 'upload' : 'uploads'}`;
    const toast = notify(`${what} ${actionDone(action)}.`, false, { ms: UNDO_MS, action: { label: 'Undo', run: () => undo(key) } });
    waiting.set(key, { es, action, decision, uuid: crypto.randomUUID(), toast, timer: setTimeout(() => void send(key), UNDO_MS) });
  }
  const keepalive: typeof fetch = (input, init) => fetch(input, { ...init, keepalive: true });
  async function send(key: string, leaving = false) {
    const w = waiting.get(key);
    if (!w) return;
    clearTimeout(w.timer);
    const how = leaving ? keepalive : fetch;
    const sentAt = Date.now();
    try {
      const out = (await gql(REVIEW_APPROVALS, { ids: w.es.map((e) => e.id), input: approvalsInput(w.decision), idempotencyKey: w.uuid }, how)).reviewApprovals;
      if (out.failed.length && !leaving) {
        notify(w.es.length === 1 ? approveFailure(new Error(out.failed[0].message), iso(w.es[0].name)) : batchSummary(out, actionDone(w.action)), true);
        waiting.delete(key);
        L.reload();
        return;
      }
      // Out of the waiting list and out of what was loaded, in one step, so nothing flickers back.
      forget(w.es, w.action, sentAt);
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
  function undo(id: string) {
    const w = waiting.get(id);
    if (!w) return;
    clearTimeout(w.timer);
    waiting.delete(id);
    dismiss(w.toast);
    // Back in the queue at the end, not where it was (ADR 0024): here at once, and, for the main
    // queue, on the server for the next load and every other reviewer. If that call fails only the
    // order is off.
    const ids = w.es.map((e) => e.id);
    if (L.data) L.set(toEnd(L.data, ids));
    if (w.decision.listing) gql(REQUEUE_EMOTES, { ids }).catch(() => {});
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

  /** What a button or key asks for on one card. */
  function act(e: Card, a: Action) {
    if (needsReason(a)) ask([e], a);
    else approve([e], a);
  }
  /** The same choice on every picked card. */
  function actMany(a: Action) {
    const list = items.filter((e) => picked.has(e.id) && (needsReason(a) || ready(e)));
    if (!list.length) return;
    if (needsReason(a)) ask(list, a);
    else approve(list, a);
  }

  // A rejection or a denial asks for a reason first and is sent at once (no undo: it never waited).
  let asking = $state<{ es: Card[]; action: Action } | null>(null);
  function ask(es: Card[], action: Action) {
    asking = { es, action };
    reasonDialog.open();
  }
  /** Cards that were decided at once (rejected, denied): out of the view, focus on what follows. */
  function decided(es: Card[], action: Action) {
    const q = L.data;
    if (!q) return;
    const gone = es.map((e) => e.id);
    if (leaves(action)) {
      advance(gone);
      const rest = items.filter((e) => !gone.includes(e.id));
      L.set(shrink(q, es, action));
      if (!rest.length && total - gone.length > 0) L.reload();
    } else L.set(shrink(q, es, action));
  }
  async function submitReason(reason: string) {
    const a = asking;
    if (!a) return;
    const { es, action } = a;
    try {
      let out: { done: readonly string[]; failed: readonly { id: string; code: string; message: string }[] };
      if (action === 'reject' && es.length === 1) {
        await gql(REJECT, { id: es[0].id, reason });
        out = { done: [es[0].id], failed: [] };
      } else if (action === 'reject') {
        out = (await gql(REVIEW_EMOTES, { ids: es.map((e) => e.id), verdict: 'REJECT', reason })).reviewEmotes;
      } else {
        out = (await gql(REVIEW_APPROVALS, { ids: es.map((e) => e.id), input: approvalsInput(decisionOf(action)!, reason), idempotencyKey: crypto.randomUUID() })).reviewApprovals;
        if (es.length === 1 && out.failed.length) throw new Error(out.failed[0].message || 'That didn’t go through.');
      }
      if (es.length > 1) sel = EMPTY;
      decided(es.filter((e) => out.done.includes(e.id)), action);
      notify(es.length === 1 ? `${iso(es[0].name)} ${actionDone(action)}.` : batchSummary(out, actionDone(action)), out.failed.length > 0);
      if (out.failed.length && es.length > 1) L.reload();
    } catch (err) {
      fail(err);
      throw err;
    }
  }
  const reasonTitle = $derived.by(() => {
    if (!asking) return 'Reason';
    const n = asking.es.length;
    const what = n === 1 ? iso(asking.es[0].name) : `${count(n)} uploads`;
    return asking.action === 'reject' ? `Reject ${what}` : asking.action === 'deny-listing' ? `Deny public listing for ${what}` : `Deny personal use for ${what}`;
  });

  /** A resolved upload's approvals were changed in the detail: its card shows them. */
  function changed(id: string, a: Approvals) {
    const q = L.data;
    if (q) L.set({ ...q, items: q.items.map((e) => (e.id === id ? { ...e, approvals: { ...e.approvals, ...a } } : e)) });
  }

  // Phones get the short tab names, so the counts still fit.
  const narrow = $derived(media(SMALL).matches);
  const TABS = $derived([
    ...(reviewer.listing
      ? [
          { value: 'public' as Tab, label: counts ? `Public ${count(counts.public)}` : 'Public', icon: 'eye' as const },
          { value: 'limited' as Tab, label: `${narrow ? 'Private' : 'Private uploads'}${counts ? ` ${count(counts.limited)}` : ''}`, icon: 'lock' as const },
        ]
      : []),
    { value: 'personal' as Tab, label: `${narrow ? 'Personal' : 'Personal use'}${counts ? ` ${count(counts.personal)}` : ''}`, icon: 'user' as const },
    ...(reviewer.listing ? [{ value: 'resolved' as Tab, label: 'Resolved', icon: 'check' as const }] : []),
  ]);
  const byText = $derived(sort === 'name' || sort === 'uploader');
</script>

<svelte:head><title>Review - Admin - cyex.tv</title></svelte:head>

<svelte:window {onkeydown} {onbeforeunload} onpagehide={() => flush(true)} />

<Split label="Upload" size="l" open={!!inspected} onclose={() => {}}>
<Bar title="Review queue" count={left !== null ? `${count(left)} left` : null}>
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
    <UserFilter label="Uploader" hideLabel placeholder="Any uploader" width="180px" value={uploader} onchange={(id) => go({ uploader: id || null })}
      top={topUploaders} topKey="{tab}|{p.get('q') ?? ''}" topLabel="Most uploads here" />
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
<p class="sr-only" id="queue-keys">Arrow keys move between uploads. Enter opens one. A approves both the public listing and personal use, L approves the listing, P approves personal use, D denies the listing, N denies personal use, R rejects, S skips to the next. X selects for a verdict on many. Z undoes the last approval while its notice shows.</p>

<LoadGate source={L} label="Loading the queue…">
{#snippet children(queue)}
{#if items.length === 0}
  {#if p.get('q') || uploader}
    <State title="Nothing matches.">No upload in this tab has that name or uploader.</State>
  {:else if resolved}
    <State title="Nothing resolved yet.">Approved and rejected uploads show up here.</State>
  {:else if tab === 'personal'}
    <State title="No personal use waiting.">Approved uploads show up here until someone decides whether they may be personal emotes.</State>
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
          {#if e.flags.private}<span class="on">Private</span>{/if}
          {#each approvalChips(e, resolved) as c (c.text)}<span class={c.tone}>{c.text}</span>{/each}
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
      {mode}
      {reviewer}
      onact={(a) => inspected && act(inspected, a)}
      onapprovals={changed}
    />
  {/if}
{/snippet}
</Split>

{#if sel.ids.length && !resolved && actions.length}
  {@const prim = actions[0]}
  <BulkBar n={sel.ids.length} status={selNote} onclear={() => (sel = EMPTY)} onall={sel.ids.length < Math.min(items.length, 100) ? () => pick(pickAll(sel, ids())) : undefined}>
    {#each actions.filter((a) => a !== 'reject' && a !== prim) as a (a)}
      <IconButton label={actionLabel(a, reviewer, mode)} icon={a === 'listing' ? 'eye' : a === 'personal' ? 'user' : a === 'deny-listing' ? 'eye-off' : 'user-x'} variant="outline" tip="top" onclick={() => actMany(a)} />
    {/each}
    {#if actions.includes('reject')}<button class="ui-btn danger small" type="button" onclick={() => actMany('reject')}><Icon name="x" size={15} />Reject {count(sel.ids.length)}</button>{/if}
    <button class="ui-btn primary small" type="button" onclick={() => actMany(prim)}><Icon name="check" size={15} />{actionLabel(prim, reviewer, mode)} {count(sel.ids.length)}</button>
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
  {mode}
  {reviewer}
  onact={(a) => shown && act(shown, a)}
  onclose={closed}
  onapprovals={changed}
/>

<ReasonDialog
  bind:this={reasonDialog}
  title={reasonTitle}
  hint={asking?.es.length === 1 ? 'The uploader sees this. Be short and concrete.' : 'Every uploader sees this reason. Be short and concrete.'}
  action={asking?.action === 'reject' ? 'Reject' : 'Deny'}
  danger
  max={500}
  presets={reasons}
  onsubmit={submitReason}
/>

<style>
  /* The toolbar: tabs, search, sorting on one row; it wraps where the column is narrow. */
  .tabs { flex: none; max-width: 100%; overflow-x: auto; scrollbar-width: none; }
  .tabs::-webkit-scrollbar { display: none; }
  .find { flex: 1 1 180px; min-width: 0; }
  .pick { flex: none; width: auto; padding-left: 10px; }
  @media (max-width: 899.98px) {
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
