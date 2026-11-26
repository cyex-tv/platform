import { describe, expect, it } from 'vitest';
import type { Grant } from './graph';
import { buildGraph, findCatalog, findNodes, fitView, kindCounts, neighbourhood, radialLayout, stepFrom, visible, type GNode } from './entitlement-graph';

const g = (over: Partial<Grant>): Grant => ({ kind: 'PAINT', itemId: 'P1', itemName: 'Tide', accountId: 'A1', source: 'STAFF', active: true, ...over });
const ROOT = { kind: 'ACCOUNT' as const, id: 'A1', name: 'nora' };

// An account: a monthly product bought twice (two orders), a one-off order, a code batch, an
// event, two staff grants (one taken away), the User role, and one paint reached two ways.
const grants: Grant[] = [
  g({ source: 'DEFAULT', kind: 'ROLE', itemId: 'RU', itemName: 'User' }),
  g({ source: 'PURCHASE', itemId: 'M1', itemName: 'Frost jan', orderId: 'O1', orderName: 'Monthly', grantedAt: '2026-01-01T09:00:00Z' }),
  g({ source: 'PURCHASE', itemId: 'M2', itemName: 'Frost feb', orderId: 'O2', orderName: 'Monthly', grantedAt: '2026-02-01T09:00:00Z' }),
  g({ source: 'PURCHASE', itemId: 'M3', itemName: 'Tide feb', orderId: 'O2', orderName: 'Monthly', grantedAt: '2026-02-01T09:00:00Z' }),
  g({ source: 'PURCHASE', kind: 'BADGE', itemId: 'B1', itemName: 'Founder', orderId: 'O3', orderName: 'Founders bundle' }),
  g({ source: 'CODE', itemId: 'C1', itemName: 'Snowfall', codeBatchId: 'K1', codeBatchName: 'Winter drop' }),
  g({ source: 'EVENT', itemId: 'E1', itemName: 'Pumpkin', reason: 'Halloween 2026' }),
  g({ source: 'STAFF', itemId: 'M1', itemName: 'Frost jan', grantedBy: 'S1' }),
  g({ source: 'STAFF', kind: 'BADGE', itemId: 'B9', itemName: 'Old', grantedBy: 'S2', active: false, revokedAt: '2026-03-01T00:00:00Z' }),
];

const ids = (ns: GNode[]) => ns.map((n) => n.id);

describe('the graph of an account', () => {
  const G = buildGraph(grants, ROOT);

  it('puts the account at the root and each source one step out', () => {
    expect(G.root).toBe('account:A1');
    const first = G.edges.filter((e) => e.source === G.root).map((e) => e.target);
    expect(first).toEqual(['builtin:default', 'product:Monthly', 'order:O3', 'code:K1', 'event:Halloween 2026', 'staff:S1', 'staff:S2']);
  });

  it('fans a product bought more than once out into its orders, named by date; a one-off order stands alone', () => {
    expect(G.byId.get('product:Monthly')!.kind).toBe('product');
    expect(G.edges.filter((e) => e.source === 'product:Monthly').map((e) => e.target)).toEqual(['order:O1', 'order:O2']);
    expect(G.byId.get('order:O1')!.name).toBe('2026/01/01');
    expect(G.byId.get('order:O2')!.depth).toBe(2);
    expect(G.edges.filter((e) => e.source === 'order:O2').map((e) => e.target)).toEqual(['paint:M2', 'paint:M3']);
    const founders = G.byId.get('order:O3')!;
    expect(founders.name).toBe('Founders bundle');
    expect(founders.orderId).toBe('O3');
    expect(founders.depth).toBe(1);
  });

  it('makes one node per item, with a link for every way it was reached', () => {
    const m1 = G.byId.get('paint:M1')!;
    expect(m1.grants).toEqual([1, 7]);
    expect(G.edges.filter((e) => e.target === 'paint:M1').map((e) => e.source)).toEqual(['order:O1', 'staff:S1']);
    // The radial tree follows the first way.
    expect(m1.parent).toBe('order:O1');
  });

  it('groups codes by batch, events by name, staff grants by who gave them', () => {
    expect(G.byId.get('code:K1')).toMatchObject({ kind: 'code', name: 'Winter drop', codeBatchId: 'K1' });
    expect(G.byId.get('event:Halloween 2026')).toMatchObject({ kind: 'event', name: 'Halloween 2026' });
    expect(G.byId.get('staff:S1')).toMatchObject({ kind: 'staff', accountId: 'S1', name: '' });
    expect(G.byId.get('builtin:default')!.name).toBe('Every account');
  });

  it('marks what was taken away, through every node on its way', () => {
    expect(G.byId.get('badge:B9')!.active).toBe(false);
    expect(G.byId.get('staff:S2')!.active).toBe(false);
    expect(G.edges.find((e) => e.target === 'badge:B9')!.active).toBe(false);
    expect(G.byId.get(G.root)!.active).toBe(true);
  });

  it('can show only what is held now', () => {
    const held = buildGraph(grants, ROOT, 'held');
    expect(held.byId.has('badge:B9')).toBe(false);
    expect(held.byId.has('staff:S2')).toBe(false);
  });

  it('counts the nodes of each kind', () => {
    const c = kindCounts(G.nodes);
    expect(c).toMatchObject({ account: 1, product: 1, order: 3, code: 1, event: 1, staff: 2, builtin: 1, role: 1, badge: 2, paint: 5 });
    expect(G.nodes).toHaveLength(18);
  });

  it('knows a node’s neighbours, and leaves out hidden kinds (never the root)', () => {
    expect([...neighbourhood(G.edges, 'paint:M1')].sort()).toEqual(['order:O1', 'paint:M1', 'staff:S1']);
    const v = visible(G, new Set(['paint', 'account'] as const));
    expect(v.nodes.some((n) => n.kind === 'paint')).toBe(false);
    expect(v.nodes.some((n) => n.id === G.root)).toBe(true);
    expect(v.edges.every((e) => !e.target.startsWith('paint:'))).toBe(true);
  });
});

describe('the graph of an item', () => {
  const holders = [
    g({ accountId: 'A1', source: 'PURCHASE', orderId: 'O1', orderName: 'Monthly' }),
    g({ accountId: 'A2', source: 'PURCHASE', orderId: 'O7', orderName: 'Monthly' }),
    g({ accountId: 'A3', source: 'STAFF', grantedBy: 'S1' }),
    g({ accountId: 'A1', source: 'CODE', codeBatchId: 'K1', codeBatchName: 'Winter drop' }),
  ];
  const G = buildGraph(holders, { kind: 'PAINT', id: 'P1', name: 'Tide' });

  it('has the item at the root, sources next (a product, not each holder’s order), the holders last', () => {
    expect(G.root).toBe('paint:P1');
    expect(G.byId.get(G.root)).toMatchObject({ kind: 'paint', name: 'Tide', root: true });
    expect(G.edges.filter((e) => e.source === 'product:Monthly').map((e) => e.target)).toEqual(['account:A1', 'account:A2']);
    expect(G.byId.has('order:O1')).toBe(false);
    // A1 holds it two ways: one node, two links.
    expect(G.edges.filter((e) => e.target === 'account:A1')).toHaveLength(2);
    expect(G.byId.get('account:A3')!.accountId).toBe('A3');
  });
});

describe('find in graph', () => {
  const G = buildGraph(grants, ROOT);
  const names: Record<string, string> = { S1: 'quietoak', S2: 'lumen_fox' };
  const labelOf = (n: GNode) => n.name || (n.accountId ? names[n.accountId] ?? '' : '');

  it('ranks exact, then the start, then a word’s start, then anywhere', () => {
    // Same rank, depth and length: alphabetical.
    expect(ids(findNodes(G.nodes, 'frost', labelOf))).toEqual(['paint:M2', 'paint:M1']);
    // Both at a word's start: the shorter name first.
    expect(ids(findNodes(G.nodes, 'feb', labelOf))).toEqual(['paint:M3', 'paint:M2']);
    expect(ids(findNodes(G.nodes, 'Founder', labelOf))).toEqual(['badge:B1', 'order:O3']);
    expect(ids(findNodes(G.nodes, 'oak', labelOf))).toEqual(['staff:S1']);
  });

  it('finds by "Kind: name", by kind alone, and by an exact id; ignores case and accents', () => {
    expect(ids(findNodes(G.nodes, 'paint: tide', labelOf))).toEqual(['paint:M3']);
    expect(ids(findNodes(G.nodes, 'code batch', labelOf))).toEqual(['code:K1']);
    expect(ids(findNodes(G.nodes, 'o2', labelOf))).toEqual(['order:O2']);
    expect(ids(findNodes(G.nodes, 'SNÖWFALL', labelOf))).toEqual(['paint:C1']);
    expect(findNodes(G.nodes, '   ', labelOf)).toEqual([]);
  });

  it('searches the catalogue by name, best first', () => {
    const cat = [
      { kind: 'PAINT' as const, id: 'X1', name: 'Sample Sunset' },
      { kind: 'BADGE' as const, id: 'X2', name: 'Sunny' },
      { kind: 'ROLE' as const, id: 'X3', name: 'Moderator' },
    ];
    expect(findCatalog(cat, 'sun').map((c) => c.id)).toEqual(['X2', 'X1']);
    expect(findCatalog(cat, 'x3').map((c) => c.id)).toEqual(['X3']);
    expect(findCatalog(cat, 'zzz')).toEqual([]);
  });
});

describe('layout helpers', () => {
  const G = buildGraph(grants, ROOT);

  it('lays the tree out on rings: the root in the middle, each depth further out', () => {
    const p = radialLayout(G.nodes, G.root);
    expect(p.get(G.root)).toEqual({ x: 0, y: 0 });
    const r = (id: string) => Math.hypot(p.get(id)!.x, p.get(id)!.y);
    expect(r('product:Monthly')).toBeCloseTo(140);
    expect(r('order:O1')).toBeGreaterThan(r('product:Monthly'));
    expect(r('paint:M2')).toBeGreaterThan(r('order:O2'));
    expect(p.size).toBe(G.nodes.length);
  });

  it('moves the keyboard to the nearest node in a direction', () => {
    const pts = [{ id: 'a', x: 0, y: 0 }, { id: 'r', x: 100, y: 10 }, { id: 'far', x: 300, y: 0 }, { id: 'up', x: 0, y: -50 }];
    expect(stepFrom(pts, 'a', 'right')).toBe('r');
    expect(stepFrom(pts, 'a', 'up')).toBe('up');
    expect(stepFrom(pts, 'a', 'left')).toBeNull();
  });

  it('fits points into a box', () => {
    const v = fitView([{ x: -100, y: -50 }, { x: 100, y: 50 }], 440, 240, 20);
    expect(v.k).toBeCloseTo(2);
    expect(v.x).toBeCloseTo(220);
    expect(v.y).toBeCloseTo(120);
  });
});
