// The browse page's whole state lives in the URL: q, sort, kind, zw, wide, nsfw and n (pages
// loaded). A reload or a shared link gives the same view. Pure, so it is unit tested.

export type Sort = 'relevance' | 'top' | 'new';
export type Kind = 'all' | 'animated' | 'still';

export type Browse = {
  q: string;
  sort: Sort;
  kind: Kind;
  zeroWidth: boolean;
  wide: boolean;
  nsfw: boolean;
  /** How many pages are loaded (1 to MAX_PAGES). */
  pages: number;
};

export const PER_PAGE = 48;
export const MAX_PAGES = 10;

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
  if (b.pages > 1) p.set('n', String(b.pages));
  const s = p.toString();
  return s ? `?${s}` : '';
}

/** A change to the view (query, sort or filters) starts again at the first page. */
export function change(b: Browse, over: Partial<Browse>): Browse {
  const next = { ...b, ...over, pages: over.pages ?? 1 };
  if (next.sort === 'relevance' && !next.q) next.sort = 'top';
  // Typing a first query switches the default order to relevance.
  if (over.q !== undefined && over.sort === undefined) {
    if (!b.q && over.q && b.sort === 'top') next.sort = 'relevance';
    if (b.q && !over.q && b.sort === 'relevance') next.sort = 'top';
  }
  return next;
}

export type Active = { key: 'kind' | 'zeroWidth' | 'wide' | 'nsfw'; label: string };

/** The filters that are on, for chips. */
export function active(b: Browse): Active[] {
  const out: Active[] = [];
  if (b.kind !== 'all') out.push({ key: 'kind', label: b.kind === 'animated' ? 'Animated' : 'Still' });
  if (b.zeroWidth) out.push({ key: 'zeroWidth', label: 'Zero-width' });
  if (b.wide) out.push({ key: 'wide', label: 'Wide' });
  if (b.nsfw) out.push({ key: 'nsfw', label: 'NSFW shown' });
  return out;
}

export const CLEARED = { kind: 'all', zeroWidth: false, wide: false, nsfw: false } as const;

/** Zero-width and wide are filtered here, from the emote's flag and its 1x file. */
// wire: zeroWidth, wide (move to API args when searchEmotes has them)
export function matches(
  b: Browse,
  e: { flags?: { zeroWidth: boolean } | null; files: { name: string; width: number; height: number }[] },
): boolean {
  if (b.zeroWidth && !e.flags?.zeroWidth) return false;
  if (b.wide) {
    const one = e.files.find((f) => f.name === '1x.webp');
    if (!one || one.width / one.height < 1.75) return false;
  }
  return true;
}
