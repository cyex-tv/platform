// The browse page's whole state lives in the URL: q, sort, t (the trending window), kind, zw, wide,
// nsfw, tag (repeated, one per tag), exact, select (multi-select on) and n (pages loaded). A reload
// or a shared link gives the same view. Every filter is the API's, so the total is always the real
// one. Pure, so it is unit tested.

export type Sort = 'relevance' | 'trending' | 'top' | 'new';
export type Kind = 'all' | 'animated' | 'still';
/** How far back Trending counts the channels that added an emote. */
export type Window = 'day' | 'week' | 'month';
/** What /search looks for: emotes (the default) or people (`tab=users`). */
export type Tab = 'emotes' | 'users';
export type Flag = 'zeroWidth' | 'wide' | 'nsfw';

export type Browse = {
  tab: Tab;
  q: string;
  sort: Sort;
  /** Only used by the Trending sort; `week` unless the URL says otherwise. */
  window: Window;
  kind: Kind;
  zeroWidth: boolean;
  wide: boolean;
  /** Only NSFW emotes (they are left out otherwise). */
  nsfw: boolean;
  /** Tags the emotes must all carry (lower case, no #), at most MAX_TAGS. */
  tags: string[];
  /** The emote's name must equal the query (any case), not just contain it. */
  exact: boolean;
  /** Multi-select is on. */
  select: boolean;
  /** How many pages are loaded (1 to MAX_PAGES). */
  pages: number;
};

export const PER_PAGE = 48;
/** People per page on the Users tab. */
export const USERS_PER_PAGE = 24;
export const MAX_PAGES = 10;
/** The API looks at ten tags at most. */
export const MAX_TAGS = 10;

export const WINDOWS: { value: Window; label: string; short: string }[] = [
  { value: 'day', label: 'Last day', short: 'Day' },
  { value: 'week', label: 'Last 7 days', short: '7 days' },
  { value: 'month', label: 'Last 30 days', short: '30 days' },
];

/** The API's name for a sort. Relevance only counts with a query. */
export function apiSort(s: Sort): 'RELEVANCE' | 'TRENDING' | 'TOP' | 'NEW' {
  return s === 'new' ? 'NEW' : s === 'top' ? 'TOP' : s === 'trending' ? 'TRENDING' : 'RELEVANCE';
}

export function apiWindow(w: Window): 'DAY' | 'WEEK' | 'MONTH' {
  return w === 'day' ? 'DAY' : w === 'month' ? 'MONTH' : 'WEEK';
}

export function defaultSort(q: string): Sort {
  return q ? 'relevance' : 'top';
}

const SORTS: Sort[] = ['top', 'new', 'trending'];

export function parse(p: URLSearchParams): Browse {
  const q = (p.get('q') ?? '').trim().slice(0, 100);
  const s = p.get('sort') as Sort | null;
  const sort: Sort = s && (SORTS.includes(s) || (s === 'relevance' && q)) ? s : defaultSort(q);
  const k = p.get('kind');
  const t = p.get('t');
  const n = Number.parseInt(p.get('n') ?? '1', 10);
  return {
    tab: p.get('tab') === 'users' ? 'users' : 'emotes',
    q,
    sort,
    window: t === 'day' || t === 'month' ? t : 'week',
    kind: k === 'animated' || k === 'still' ? k : 'all',
    zeroWidth: p.get('zw') === '1',
    wide: p.get('wide') === '1',
    nsfw: p.get('nsfw') === '1',
    tags: cleanTags(p.getAll('tag')),
    exact: p.get('exact') === '1' && !!q,
    select: p.get('select') === '1',
    pages: Math.min(MAX_PAGES, Math.max(1, Number.isFinite(n) ? n : 1)),
  };
}

/** The query string for a state; defaults are left out. */
export function toSearch(b: Browse): string {
  const p = new URLSearchParams();
  if (b.tab === 'users') {
    // People have a name and pages, nothing else: none of the emote filters apply.
    p.set('tab', 'users');
    if (b.q) p.set('q', b.q);
    if (b.pages > 1) p.set('n', String(b.pages));
    return `?${p}`;
  }
  if (b.q) p.set('q', b.q);
  if (b.sort !== defaultSort(b.q)) p.set('sort', b.sort);
  if (b.sort === 'trending' && b.window !== 'week') p.set('t', b.window);
  if (b.kind !== 'all') p.set('kind', b.kind);
  if (b.zeroWidth) p.set('zw', '1');
  if (b.wide) p.set('wide', '1');
  if (b.nsfw) p.set('nsfw', '1');
  for (const t of b.tags) p.append('tag', t);
  if (b.exact && b.q) p.set('exact', '1');
  if (b.select) p.set('select', '1');
  if (b.pages > 1) p.set('n', String(b.pages));
  const s = p.toString();
  return s ? `?${s}` : '';
}

/** A change to the view (query, sort or filters) starts again at the first page. */
export function change(b: Browse, over: Partial<Browse>): Browse {
  const next = { ...b, ...over, pages: over.pages ?? 1 };
  if (over.tags) next.tags = cleanTags(over.tags);
  // Changing tab keeps the typed name, and starts that tab from its defaults.
  if (over.tab && over.tab !== b.tab) {
    next.sort = defaultSort(next.q);
    next.select = false;
  }
  if (next.sort === 'relevance' && !next.q) next.sort = 'top';
  if (!next.q) next.exact = false;
  // Typing a first query switches the default order to relevance (Trending and New stay).
  if (over.q !== undefined && over.sort === undefined) {
    if (!b.q && over.q && b.sort === 'top') next.sort = 'relevance';
    if (b.q && !over.q && b.sort === 'relevance') next.sort = 'top';
  }
  return next;
}

export type Active = { key: 'kind' | Flag | 'exact' | `tag:${string}`; label: string };

/** The filters that are on, for chips: one per tag, so each can be removed alone. */
export function active(b: Browse): Active[] {
  const out: Active[] = [];
  if (b.kind !== 'all') out.push({ key: 'kind', label: b.kind === 'animated' ? 'Animated' : 'Still' });
  if (b.zeroWidth) out.push({ key: 'zeroWidth', label: 'Zero-width' });
  if (b.wide) out.push({ key: 'wide', label: 'Wide' });
  if (b.nsfw) out.push({ key: 'nsfw', label: 'NSFW' });
  for (const t of b.tags) out.push({ key: `tag:${t}`, label: `#${t}` });
  if (b.exact && b.q) out.push({ key: 'exact', label: 'Exact name' });
  return out;
}

/** The change that turns one active filter off. */
export function without(b: Browse, key: Active['key']): Partial<Browse> {
  if (key.startsWith('tag:')) return { tags: b.tags.filter((t) => t !== key.slice(4)) };
  if (key === 'kind') return { kind: 'all' };
  return { [key]: false } as Partial<Browse>;
}

export const CLEARED: Partial<Browse> = { kind: 'all', zeroWidth: false, wide: false, nsfw: false, tags: [], exact: false };

/** A tag as the URL keeps it: trimmed, lower case, no leading #, at most 30 characters (the API's limit). */
export function cleanTag(t: string): string {
  return t.trim().replace(/^#+/, '').toLowerCase().slice(0, 30);
}

/** Tags cleaned, without empties or repeats, MAX_TAGS at most. */
export function cleanTags(tags: readonly string[]): string[] {
  return [...new Set(tags.map(cleanTag).filter(Boolean))].slice(0, MAX_TAGS);
}

/** The variables of the SEARCH query for a state and a page. */
export function apiVars(b: Browse, page: number) {
  const flags = [b.zeroWidth && 'ZERO_WIDTH', b.wide && 'WIDE', b.nsfw && 'NSFW'].filter(Boolean) as ('ZERO_WIDTH' | 'WIDE' | 'NSFW')[];
  return {
    query: b.q,
    animated: b.kind === 'animated' ? true : b.kind === 'still' ? false : null,
    sort: apiSort(b.sort),
    window: b.sort === 'trending' ? apiWindow(b.window) : null,
    flags: flags.length ? flags : null,
    tags: b.tags.length ? b.tags : null,
    exact: b.exact && !!b.q ? true : null,
    page,
    perPage: PER_PAGE,
  };
}

/** The most common tags across `items`, most used first (ties by name), `n` at most, leaving out `skip`. */
export function topTags(items: { tags?: string[] }[], n = 12, skip: readonly string[] = []): { tag: string; count: number }[] {
  const seen = new Map<string, number>();
  for (const e of items)
    for (const t of new Set((e.tags ?? []).map(cleanTag).filter(Boolean))) if (!skip.includes(t)) seen.set(t, (seen.get(t) ?? 0) + 1);
  return [...seen]
    .map(([tag, count]) => ({ tag, count }))
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag))
    .slice(0, n);
}

/** The columns the table can sort by, and the sort each one stands for (the API has no other). */
export const COLUMN_SORT = { channels: 'top', added: 'new' } as const;
