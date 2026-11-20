import { describe, expect, it } from 'vitest';
import { groupPayload, lazyGroupIds, navGroups, pageFacts } from './lazy.server';
import { isLazy } from './lazy';

describe('the /developers index and its group bodies', () => {
  const nav = navGroups();

  it('lists every group once, with the live socket drawn by the page', () => {
    expect(new Set(nav.map((g) => g.id)).size).toBe(nav.length);
    expect(nav.map((g) => g.id).slice(-4)).toEqual(['gql-queries', 'gql-mutations', 'gql-subscriptions', 'gql-types']);
    expect(lazyGroupIds()).toEqual(nav.map((g) => g.id).filter((id) => id !== 'live'));
    expect(nav.every((g) => isLazy(g.id) === (g.id !== 'live'))).toBe(true);
  });

  it('has a body for every lazy group that holds exactly the sections its index entries link to', () => {
    for (const g of nav.filter((x) => isLazy(x.id))) {
      const p = groupPayload(g.id)!;
      const ids = p.kind === 'types' ? p.types.map((t) => t.id) : p.ops.map((o) => o.id);
      expect(ids, g.id).toEqual(g.entries.map((e) => e.id));
    }
    expect(groupPayload('nope')).toBeUndefined();
    expect(groupPayload('live')).toBeUndefined();
  });

  it('keeps the filter words in the index, so closed groups are searchable without their bodies', () => {
    const rest = nav.find((g) => g.id.startsWith('rest-'))!;
    expect(rest.entries[0].search).toMatch(/^\/v1\/.* rest$/);
    const all = nav.flatMap((g) => g.entries);
    expect(all.find((e) => e.id === 'ws-events')!.search).toContain('websocket');
    expect(all.every((e) => e.label && e.chip && e.cls.startsWith('m-'))).toBe(true);
  });

  it('draws a subscription with its frames, and knows the type names for links', () => {
    const subs = groupPayload('gql-subscriptions')!;
    expect(subs.kind === 'ops' && subs.ops.every((o) => o.frames?.includes('graphql-transport-ws'))).toBe(true);
    expect(pageFacts().known).toContain('EventType');
  });
});
