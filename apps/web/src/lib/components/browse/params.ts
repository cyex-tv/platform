// The browse page's whole state lives in the URL: q, sort, kind, zw, wide, nsfw, tag, view
// (grid or table), exact, select (multi-select on) and n (pages loaded). A reload or a shared link gives the same view. Pure, so it is unit tested.

export type Sort = 'relevance' | 'top' | 'new';
export type Kind = 'all' | 'animated' | 'still';
export type Layout = '' | 'grid' | 'table';

export type Browse = {
  q: string;
  sort: Sort;
  kind: Kind;
  zeroWidth: boolean;
  wide: boolean;
  nsfw: boolean;
  /** One tag the emotes must carry (lower case), or ''. */
  tag: string;
  /** The result layout asked for in the URL; '' leaves it to the device's remembered choice. */
  layout: Layout;
  /** The emote's name must equal the query (any case), not just contain it. */
  exact: boolean;
  /** Multi-select is on. */
  select: boolean;
  /** How many pages are loaded (1 to MAX_PAGES). */
  pages: number;
};

export const PER_PAGE = 48;
export const MAX_PAGES = 10;

/** The API's name for a sort. Relevance only counts with a query. */
export function apiSort(s: Sort): 'RELEVANCE' | 'TOP' | 'NEW' {
  return s === 'new' ? 'NEW' : s === 'top' ? 'TOP' : 'RELEVANCE';
}

export function defaultSort(q: string): Sort {
  return q ? 'relevance' : 'top';
}

export function parse(p: URLSearchParams): Browse {
  const q = (p.get('q') ?? '').trim().slice(0, 100);
  const s = p.get('sort');
  const sort: Sort = s === 'top' || s === 'new' || (s === 'relevance' && q) ? s : defaultSort(q);
  const k = p.get('kind');
  const n = Number.parseInt(p.get('n') ?? '1', 10);
  return {
    q,
    sort,
    kind: k === 'animated' || k === 'still' ? k : 'all',
    zeroWidth: p.get('zw') === '1',
    wide: p.get('wide') === '1',
    nsfw: p.get('nsfw') === '1',
    tag: cleanTag(p.get('tag') ?? ''),
    layout: p.get('view') === 'table' ? 'table' : p.get('view') === 'grid' ? 'grid' : '',
    exact: p.get('exact') === '1' && !!q,
    select: p.get('select') === '1',
    pages: Math.min(MAX_PAGES, Math.max(1, Number.isFinite(n) ? n : 1)),
  };
}

/** The query string for a state; defaults are left out. */
export function toSearch(b: Browse): string {
  const p = new URLSearchParams();
  if (b.q) p.set('q', b.q);
  if (b.sort !== defaultSort(b.q)) p.set('sort', b.sort);
  if (b.kind !== 'all') p.set('kind', b.kind);
  if (b.zeroWidth) p.set('zw', '1');
  if (b.wide) p.set('wide', '1');
  if (b.nsfw) p.set('nsfw', '1');
  if (b.tag) p.set('tag', b.tag);
  if (b.layout) p.set('view', b.layout);
  if (b.exact && b.q) p.set('exact', '1');
  if (b.select) p.set('select', '1');
  if (b.pages > 1) p.set('n', String(b.pages));
  const s = p.toString();
  return s ? `?${s}` : '';
}

/** A change to the view (query, sort or filters) starts again at the first page. */
export function change(b: Browse, over: Partial<Browse>): Browse {
  const next = { ...b, ...over, pages: over.pages ?? 1 };
  if (next.sort === 'relevance' && !next.q) next.sort = 'top';
  if (!next.q) next.exact = false;
  // Typing a first query switches the default order to relevance.
  if (over.q !== undefined && over.sort === undefined) {
    if (!b.q && over.q && b.sort === 'top') next.sort = 'relevance';
    if (b.q && !over.q && b.sort === 'relevance') next.sort = 'top';
  }
  return next;
}

export type Active = { key: 'kind' | 'zeroWidth' | 'wide' | 'nsfw' | 'tag' | 'exact'; label: string };

/** The filters that are on, for chips. */
export function active(b: Browse): Active[] {
  const out: Active[] = [];
  if (b.kind !== 'all') out.push({ key: 'kind', label: b.kind === 'animated' ? 'Animated' : 'Still' });
  if (b.zeroWidth) out.push({ key: 'zeroWidth', label: 'Zero-width' });
  if (b.wide) out.push({ key: 'wide', label: 'Wide' });
  if (b.nsfw) out.push({ key: 'nsfw', label: 'NSFW shown' });
  if (b.tag) out.push({ key: 'tag', label: `#${b.tag}` });
  if (b.exact && b.q) out.push({ key: 'exact', label: 'Exact name' });
  return out;
}

export const CLEARED = { kind: 'all', zeroWidth: false, wide: false, nsfw: false, tag: '', exact: false } as const;

/** A tag as the URL keeps it: trimmed, lower case, no leading #, at most 40 characters. */
export function cleanTag(t: string): string {
  return t.trim().replace(/^#+/, '').toLowerCase().slice(0, 40);
}

/** The text the API searches: the query, or the tag alone when there is no query (search matches tags). */
export function apiQuery(b: Browse): string {
  return b.q || b.tag;
}

/** Whether some filters run here, after the API (so the API's total is not the count shown). */
export function clientSide(b: Browse): boolean {
  return b.zeroWidth || b.wide || !!b.tag || (b.exact && !!b.q);
}

/**
 * Exact name, zero-width, wide and tag are filtered here (the API has no args for them), from the emote's
 * flag, its 1x file and its tags.
 */
export function matches(
  b: Browse,
  e: { name?: string; flags?: { zeroWidth: boolean } | null; tags?: string[]; files: { name: string; width: number; height: number }[] },
): boolean {
  if (b.exact && b.q && (e.name ?? '').toLowerCase() !== b.q.toLowerCase()) return false;
  if (b.tag && !(e.tags ?? []).some((t) => t.toLowerCase() === b.tag)) return false;
  if (b.zeroWidth && !e.flags?.zeroWidth) return false;
  if (b.wide) {
    const one = e.files.find((f) => f.name === '1x.webp');
    if (!one || one.width / one.height < 1.75) return false;
  }
  return true;
}

/** The most common tags across `items`, most used first (ties by name), `n` at most. */
export function topTags(items: { tags?: string[] }[], n = 12): { tag: string; count: number }[] {
  const seen = new Map<string, number>();
  for (const e of items)
    for (const t of new Set((e.tags ?? []).map(cleanTag).filter(Boolean))) seen.set(t, (seen.get(t) ?? 0) + 1);
  return [...seen]
    .map(([tag, count]) => ({ tag, count }))
    .sort((a, b) => b.count - a.count || a.tag.localeCompare(b.tag))
    .slice(0, n);
}

/** The columns the table can sort by, and the sort each one stands for (the API has no other). */
export const COLUMN_SORT = { channels: 'top', added: 'new' } as const;
