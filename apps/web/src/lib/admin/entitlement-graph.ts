// The entitlement graph as nodes and links: the account (or the item) at the root, then where
// things came from (a shop product and its orders, a code batch, an event, a staff member, the
// built-in roles), then what came from it (roles, badges, paints; or, for an item, the accounts
// holding it). The same item reached two ways is one node with two links, so it is a graph, not
// a tree. Pure: the canvas draws it, the tests pin its shape, the search ranks its nodes.
import { date } from '$lib/format';
import type { Grant } from './graph';

export type NodeKind = 'account' | 'staff' | 'product' | 'order' | 'code' | 'event' | 'builtin' | 'role' | 'badge' | 'paint';

/** Legend order: the root, the sources, then the items. */
export const NODE_KINDS: NodeKind[] = ['account', 'product', 'order', 'code', 'event', 'staff', 'builtin', 'role', 'badge', 'paint'];

export const NODE_LABEL: Record<NodeKind, string> = {
  account: 'Account',
  staff: 'Staff',
  product: 'Product',
  order: 'Order',
  code: 'Code batch',
  event: 'Event',
  builtin: 'Built-in',
  role: 'Role',
  badge: 'Badge',
  paint: 'Paint',
};

export type GNode = {
  /** `kind:key`, stable for a given set of grants (it goes in the address as ?node=). */
  id: string;
  kind: NodeKind;
  /** The text after "Kind: ". Empty for an account until its name is known (see `labelOf`). */
  name: string;
  /** Account and staff nodes: the account. */
  accountId?: string;
  /** Item nodes: the badge, paint or role. */
  itemId?: string;
  orderId?: string;
  codeBatchId?: string;
  /** Steps from the root along the first way it was reached. */
  depth: number;
  /** The node it was first reached from (null for the root): the tree the radial layout uses. */
  parent: string | null;
  /** The grants (indexes into the grants given) that run through it. */
  grants: number[];
  /** Something is held now through it. */
  active: boolean;
  root: boolean;
};

export type GEdge = { source: string; target: string; active: boolean };
export type Graph = { nodes: GNode[]; edges: GEdge[]; root: string; byId: Map<string, GNode> };

export type GraphRoot = { kind: 'ACCOUNT' | 'BADGE' | 'PAINT' | 'ROLE'; id: string; name: string };

type Step = Omit<GNode, 'depth' | 'parent' | 'grants' | 'active' | 'root'>;

const itemNode = (g: Grant): Step => ({ id: `${g.kind.toLowerCase()}:${g.itemId}`, kind: g.kind.toLowerCase() as NodeKind, name: g.itemName || 'Deleted', itemId: g.itemId });

/**
 * Where one grant came from, from the root outwards. A shop product bought more than once
 * (a monthly supporter pack) gets a product node with one order node per order under it; bought
 * once, the order alone. In the holders view an order is one account's, so the product stands in.
 */
function chainOf(g: Grant, ordersOf: Map<string, Set<string>>, holders: boolean): Step[] {
  switch (g.source) {
    case 'PURCHASE': {
      const product = g.orderName ?? 'Shop item';
      if (!g.orderId) return [{ id: 'order:-', kind: 'order', name: 'Shop purchase' }];
      const p: Step = { id: `product:${product}`, kind: 'product', name: product };
      if (holders) return [p];
      const o: Step = { id: `order:${g.orderId}`, kind: 'order', name: product, orderId: g.orderId };
      if ((ordersOf.get(product)?.size ?? 0) < 2) return [o];
      return [p, { ...o, name: g.grantedAt ? date(g.grantedAt) : g.orderId.slice(-8) }];
    }
    case 'CODE':
      return [{ id: `code:${g.codeBatchId ?? '-'}`, kind: 'code', name: g.codeBatchName ?? 'Redeem code', codeBatchId: g.codeBatchId ?? undefined }];
    case 'EVENT':
      return [{ id: `event:${g.reason ?? '-'}`, kind: 'event', name: g.reason || 'Event' }];
    case 'DEFAULT':
      return [{ id: 'builtin:default', kind: 'builtin', name: 'Every account' }];
    case 'OWNER':
      return [{ id: 'builtin:owner', kind: 'builtin', name: 'Owner list' }];
    default:
      return g.grantedBy
        ? [{ id: `staff:${g.grantedBy}`, kind: 'staff', name: '', accountId: g.grantedBy }]
        : [{ id: 'staff:-', kind: 'staff', name: 'Unknown' }];
  }
}

/**
 * Builds the graph. For an account (`root.kind` ACCOUNT) the leaves are its roles, badges and
 * paints; for an item, the accounts holding it. `show: 'held'` leaves out what was taken away.
 */
export function buildGraph(grants: Grant[], root: GraphRoot, show: 'all' | 'held' = 'all'): Graph {
  const holders = root.kind !== 'ACCOUNT';
  const rootNode: GNode = holders
    ? { id: `${root.kind.toLowerCase()}:${root.id}`, kind: root.kind.toLowerCase() as NodeKind, name: root.name, itemId: root.id, depth: 0, parent: null, grants: [], active: false, root: true }
    : { id: `account:${root.id}`, kind: 'account', name: root.name, accountId: root.id, depth: 0, parent: null, grants: [], active: false, root: true };
  const byId = new Map<string, GNode>([[rootNode.id, rootNode]]);
  const edges = new Map<string, GEdge>();
  const ordersOf = new Map<string, Set<string>>();
  for (const g of grants)
    if (g.source === 'PURCHASE' && g.orderId && (show === 'all' || g.active)) {
      const k = g.orderName ?? 'Shop item';
      ordersOf.set(k, (ordersOf.get(k) ?? new Set()).add(g.orderId));
    }

  grants.forEach((g, i) => {
    if (show === 'held' && !g.active) return;
    const leaf: Step = holders ? { id: `account:${g.accountId}`, kind: 'account', name: '', accountId: g.accountId } : itemNode(g);
    let from = rootNode;
    from.grants.push(i);
    from.active ||= g.active;
    for (const step of [...chainOf(g, ordersOf, holders), leaf]) {
      let n = byId.get(step.id);
      if (!n) {
        n = { ...step, depth: from.depth + 1, parent: from.id, grants: [], active: false, root: false };
        byId.set(n.id, n);
      }
      if (!n.grants.includes(i)) n.grants.push(i);
      n.active ||= g.active;
      const key = `${from.id}>${n.id}`;
      const e = edges.get(key);
      if (e) e.active ||= g.active;
      else edges.set(key, { source: from.id, target: n.id, active: g.active });
      from = n;
    }
  });
  return { nodes: [...byId.values()], edges: [...edges.values()], root: rootNode.id, byId };
}

/** How many nodes of each kind (for the legend). */
export function kindCounts(nodes: GNode[]): Record<NodeKind, number> {
  const c = Object.fromEntries(NODE_KINDS.map((k) => [k, 0])) as Record<NodeKind, number>;
  for (const n of nodes) c[n.kind]++;
  return c;
}

/** The nodes linked to `id`, and `id` itself. */
export function neighbourhood(edges: GEdge[], id: string): Set<string> {
  const s = new Set([id]);
  for (const e of edges) {
    if (e.source === id) s.add(e.target);
    else if (e.target === id) s.add(e.source);
  }
  return s;
}

/** The graph without the hidden kinds (the root always stays) and the links between what is left. */
export function visible(graph: Graph, hidden: ReadonlySet<NodeKind>): { nodes: GNode[]; edges: GEdge[] } {
  if (!hidden.size) return graph;
  const nodes = graph.nodes.filter((n) => n.root || !hidden.has(n.kind));
  const ids = new Set(nodes.map((n) => n.id));
  return { nodes, edges: graph.edges.filter((e) => ids.has(e.source) && ids.has(e.target)) };
}

// ---- Search ----------------------------------------------------------------------------------

/** Lower case without accents, for matching. */
export const fold = (s: string) => s.normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase();

/** How well `name` matches `q` (already folded): 0 exact, 1 prefix, 2 a word's start, 3 inside; null no match. */
export function score(name: string, q: string): number | null {
  const n = fold(name);
  if (!q) return null;
  if (n === q) return 0;
  if (n.startsWith(q)) return 1;
  const at = n.indexOf(q);
  if (at < 0) return null;
  return /[\s\-_:./·(]/.test(n[at - 1]) ? 2 : 3;
}

/**
 * The nodes matching a search, best first: by the name, then "Kind: name" (so "paint: sun"
 * and "order" work), and an exact id. Ties go to the nodes nearer the root, then shorter names.
 */
export function findNodes(nodes: GNode[], query: string, labelOf: (n: GNode) => string = (n) => n.name): GNode[] {
  const q = fold(query.trim());
  if (!q) return [];
  const hits: { n: GNode; s: number; name: string }[] = [];
  for (const n of nodes) {
    const name = labelOf(n);
    const ids = [n.itemId, n.accountId, n.orderId, n.codeBatchId].filter(Boolean).map((x) => fold(x!));
    let s = ids.includes(q) ? 0 : score(name, q);
    if (s === null) {
      const k = score(`${NODE_LABEL[n.kind]}: ${name}`, q);
      if (k !== null) s = k + 4;
    }
    if (s !== null) hits.push({ n, s, name });
  }
  return hits.sort((a, b) => a.s - b.s || a.n.depth - b.n.depth || a.name.length - b.name.length || a.name.localeCompare(b.name)).map((h) => h.n);
}

export type CatalogItem = { kind: 'BADGE' | 'PAINT' | 'ROLE'; id: string; name: string };

/** Badges, paints and roles of the catalogue whose name matches, best first. */
export function findCatalog(items: CatalogItem[], query: string, limit = 6): CatalogItem[] {
  const q = fold(query.trim());
  if (!q) return [];
  return items
    .map((it) => ({ it, s: it.id.toLowerCase() === q ? 0 : score(it.name, q) }))
    .filter((x): x is { it: CatalogItem; s: number } => x.s !== null)
    .sort((a, b) => a.s - b.s || a.it.name.length - b.it.name.length || a.it.name.localeCompare(b.it.name))
    .slice(0, limit)
    .map((x) => x.it);
}

// ---- Layout helpers --------------------------------------------------------------------------

export type Point = { x: number; y: number };

/**
 * The radial layout: the root in the middle, each depth on its own ring, every subtree given an
 * arc in proportion to its leaves, so a product with twelve orders fans out wider than a single
 * staff grant. A ring grows when its nodes would crowd (about `spacing` apart).
 */
export function radialLayout(nodes: { id: string; parent: string | null; depth: number }[], rootId: string, spacing = 22): Map<string, Point> {
  const kids = new Map<string, string[]>();
  for (const n of nodes) if (n.parent !== null && n.id !== rootId) kids.set(n.parent, [...(kids.get(n.parent) ?? []), n.id]);
  const ids = new Set(nodes.map((n) => n.id));
  // A node whose parent is hidden hangs off the root.
  for (const n of nodes)
    if (n.id !== rootId && (n.parent === null || !ids.has(n.parent))) kids.set(rootId, [...(kids.get(rootId) ?? []), n.id]);
  const leaves = new Map<string, number>();
  const count = (id: string, seen: Set<string>): number => {
    if (seen.has(id)) return 0;
    seen.add(id);
    const c = (kids.get(id) ?? []).reduce((s, k) => s + count(k, seen), 0) || 1;
    leaves.set(id, c);
    return c;
  };
  count(rootId, new Set());
  const depthOf = new Map<string, number>([[rootId, 0]]);
  const angle = new Map<string, number>();
  const place = (id: string, from: number, to: number, d: number) => {
    angle.set(id, (from + to) / 2);
    depthOf.set(id, d);
    let at = from;
    for (const k of kids.get(id) ?? []) {
      if (depthOf.has(k)) continue;
      const span = ((to - from) * (leaves.get(k) ?? 1)) / (leaves.get(id) ?? 1);
      place(k, at, at + span, d + 1);
      at += span;
    }
  };
  place(rootId, -Math.PI, Math.PI, 0);
  const perDepth: number[] = [];
  for (const d of depthOf.values()) perDepth[d] = (perDepth[d] ?? 0) + 1;
  const radius: number[] = [0];
  for (let d = 1; d < perDepth.length; d++) radius[d] = Math.max(radius[d - 1] + (d === 1 ? 140 : 110), ((perDepth[d] ?? 0) * spacing) / (2 * Math.PI));
  const out = new Map<string, Point>();
  for (const [id, d] of depthOf) {
    const a = angle.get(id) ?? 0;
    out.set(id, d === 0 ? { x: 0, y: 0 } : { x: Math.cos(a) * radius[d], y: Math.sin(a) * radius[d] });
  }
  return out;
}

export type Dir = 'left' | 'right' | 'up' | 'down';
const DIR: Record<Dir, Point> = { left: { x: -1, y: 0 }, right: { x: 1, y: 0 }, up: { x: 0, y: -1 }, down: { x: 0, y: 1 } };

/** The keyboard's next node from `from` in a direction: the nearest within 60° of it, straighter first. */
export function stepFrom(points: { id: string; x: number; y: number }[], from: string, dir: Dir): string | null {
  const at = points.find((p) => p.id === from);
  if (!at) return points[0]?.id ?? null;
  const v = DIR[dir];
  let best: string | null = null;
  let bestCost = Infinity;
  for (const p of points) {
    if (p.id === from) continue;
    const dx = p.x - at.x;
    const dy = p.y - at.y;
    const dist = Math.hypot(dx, dy);
    if (!dist) continue;
    const cos = (dx * v.x + dy * v.y) / dist;
    if (cos < 0.5) continue;
    const cost = dist * (1 + 2 * (1 - cos));
    if (cost < bestCost) {
      bestCost = cost;
      best = p.id;
    }
  }
  return best;
}

export type View = { x: number; y: number; k: number };

/** The pan and zoom that fit `points` (with their radius) into a w × h box with `pad` around. */
export function fitView(points: { x: number; y: number; r?: number }[], w: number, h: number, pad = 40, max = 2.5): View {
  if (!points.length || w <= 0 || h <= 0) return { x: w / 2, y: h / 2, k: 1 };
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const p of points) {
    const r = p.r ?? 0;
    x0 = Math.min(x0, p.x - r); y0 = Math.min(y0, p.y - r);
    x1 = Math.max(x1, p.x + r); y1 = Math.max(y1, p.y + r);
  }
  const k = Math.max(0.05, Math.min(max, (w - 2 * pad) / Math.max(1, x1 - x0), (h - 2 * pad) / Math.max(1, y1 - y0)));
  return { k, x: w / 2 - ((x0 + x1) / 2) * k, y: h / 2 - ((y0 + y1) / 2) * k };
}
