import { describe, expect, it, vi } from 'vitest';
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/admin') } }));
import { adminRail, sectionFor, SECTIONS, visibleSections } from './sections';
import { fits } from './fit';
import { withParam } from '../ui/pick';

const only = (...perms: string[]) => (p: string) => perms.includes(p);
const rows = (groups: ReturnType<typeof adminRail>) => groups.flatMap((g) => g.items.map((i) => i.href));

describe('admin sections', () => {
  it('opens a section for any one of its permissions', () => {
    expect(rows(adminRail(only('emotes.takedown'), {}, '/admin'))).toEqual(['/admin', '/admin/reports']);
    expect(rows(adminRail(only('emotes.review'), {}, '/admin'))).toEqual(['/admin', '/admin/review', '/admin/reports']);
  });
  it('keeps Overview for everyone and drops empty groups', () => {
    const g = adminRail(only(), {}, '/admin');
    expect(g).toHaveLength(1);
    expect(g[0].items[0]).toMatchObject({ href: '/admin', current: true });
  });
  it('marks the current row by the longest prefix, Overview only exactly', () => {
    const g = adminRail(() => true, {}, '/admin/review/reasons');
    const cur = g.flatMap((x) => x.items).filter((i) => i.current).map((i) => i.href);
    expect(cur).toEqual(['/admin/review']);
  });
  it('puts the counts on their rows', () => {
    const g = adminRail(() => true, { review: 12, codes: 3 }, '/admin');
    const by = Object.fromEntries(g.flatMap((x) => x.items).map((i) => [i.href, i.count]));
    expect(by['/admin/review']).toBe(12);
    expect(by['/admin/shop/codes']).toBe(3);
    expect(by['/admin/users']).toBeUndefined();
  });
  it('gives every section a line for the overview and a unique address', () => {
    const all = SECTIONS.flatMap((g) => g.items);
    expect(new Set(all.map((s) => s.href)).size).toBe(all.length);
    for (const s of all) expect(s.text.length).toBeGreaterThan(10);
  });
  it('lists only what the permissions open', () => {
    expect(visibleSections(only('audit.view')).map((g) => g.label)).toEqual(['System']);
  });
  it('names the section of a deeper page', () => {
    expect(sectionFor('/admin/cosmetics/paints/abc/studio')?.label).toBe('Paints');
    expect(sectionFor('/admin/users/123')?.label).toBe('Users');
    expect(sectionFor('/admin')).toBeNull();
  });
});

describe('fits', () => {
  it('sticks only while the column fits under the header', () => {
    expect(fits(744, 800, 56)).toBe(true);
    expect(fits(745, 800, 56)).toBe(true); // a pixel of rounding
    expect(fits(900, 800, 56)).toBe(false);
  });
});

describe('withParam', () => {
  it('sets, replaces and removes one parameter, keeping the rest', () => {
    const u = new URL('http://x/admin/cosmetics/badges?q=star');
    expect(withParam(u, 'badge', '42')).toBe('?q=star&badge=42');
    expect(withParam(new URL('http://x/a?badge=1&q=s'), 'badge', '2')).toBe('?badge=2&q=s');
    expect(withParam(new URL('http://x/a?badge=1'), 'badge', null)).toBe('/a');
  });
});
