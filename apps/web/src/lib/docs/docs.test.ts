import { describe, expect, it } from 'vitest';
import spec from '../../../../../openapi/openapi.json';
import { graphqlReference } from './graphql';
import { guides } from './guides';
import { defaultOpen, isOpen, parseGroupState, restGroupId, withAll, withGroup, allOpen } from './groups';
import { restGroups, restOperations } from './rest';

describe('REST reference', () => {
  const ops = restOperations();

  it('lists every path and method in openapi.json', () => {
    const expected = Object.entries(spec.paths).flatMap(([path, item]) =>
      Object.keys(item).map((method) => `${method.toUpperCase()} ${path}`),
    );
    expect(expected.length).toBeGreaterThan(0);
    expect(ops.map((o) => `${o.method} ${o.path}`).sort()).toEqual(expected.sort());
  });

  it('has unique anchors, a sample in each language and the responses of the spec', () => {
    expect(new Set(ops.map((o) => o.id)).size).toBe(ops.length);
    for (const o of ops) {
      expect(o.samples.map((s) => s.lang)).toEqual(['curl', 'js', 'py']);
      expect(o.samples[0].code).toContain('https://api.cyex.tv');
      expect(o.responses.length).toBeGreaterThan(0);
    }
    expect(restGroups().flatMap((g) => g.ops)).toHaveLength(ops.length);
  });

  it('puts path parameters into the sample URL', () => {
    const channel = ops.find((o) => o.path === '/v1/channels/{platform}/{id}')!;
    expect(channel.samples[0].code).toContain('/v1/channels/twitch/12345678');
    expect(channel.samples[2].code).toContain('requests.get("https://api.cyex.tv/v1/channels/twitch/12345678")');
  });
});

describe('guides', () => {
  const ids = new Set([...restOperations().map((o) => o.id), ...graphqlReference().queries.map((o) => o.id), ...graphqlReference().mutations.map((o) => o.id), ...guides.map((g) => g.id), 'ws-events', 'gql-subscription-events']);

  it('link only to sections that exist', () => {
    for (const g of guides)
      for (const b of g.blocks) {
        const runs = 'text' in b ? [b.text] : 'items' in b ? b.items : 'rows' in b && b.t === 'fmt' ? b.rows.map((r) => r[1]) : [];
        for (const para of runs.flat(2)) if (para.href?.startsWith('#')) expect(ids, para.href).toContain(para.href.slice(1));
      }
  });

  it('never use the owner IDs as examples', () => {
    const text = JSON.stringify({ guides, rest: restOperations(), gql: graphqlReference() });
    for (const id of ['9285' + '4357', '3169' + '440']) expect(text).not.toContain(id);
  });
});

describe('GraphQL reference', () => {
  const ref = graphqlReference();

  it('covers queries, mutations and subscriptions', () => {
    expect(ref.queries.map((o) => o.name)).toContain('channel');
    expect(ref.mutations.map((o) => o.name)).toContain('addSetEmote');
    expect(ref.subscriptions.map((o) => o.name)).toEqual(['events']);
  });

  it('documents the types the operations use', () => {
    const names = ref.types.map((t) => t.name);
    expect(names).toEqual(expect.arrayContaining(['Emote', 'Platform', 'EventType', 'UpdateSetInput']));
    expect(names).not.toContain('String');
    expect(names).not.toContain('Query');
  });

  it('writes valid sample requests', () => {
    const channel = ref.queries.find((o) => o.name === 'channel')!;
    expect(channel.document).toContain('query Channel($platform: Platform!, $id: String!)');
    expect(channel.samples[0].code).toContain('https://api.cyex.tv/v1/gql');
    expect(JSON.parse(channel.response).data.channel.login).toBeTruthy();
    expect(channel.document && JSON.parse(channel.variables)).toEqual({ platform: 'TWITCH', id: '12345678' });
    expect(channel.access).toBe('public');
  });

  it('marks what needs a session or a permission', () => {
    expect(ref.mutations.find((o) => o.name === 'addSetEmote')!.access).toBe('signed-in');
    expect(ref.mutations.find((o) => o.name === 'approveEmote')!.access).toBe('restricted');
    expect(ref.mutations.find((o) => o.name === 'addSetEmote')!.samples[0].code).toContain("origin: https://cyex.tv");
  });
});

describe('collapsible groups', () => {
  it('start closed, except the notes', () => {
    expect(defaultOpen('notes')).toBe(true);
    expect(isOpen({}, 'rest-emotes')).toBe(false);
    expect(isOpen({ 'rest-emotes': true, notes: false }, 'rest-emotes')).toBe(true);
    expect(isOpen({ notes: false }, 'notes')).toBe(false);
  });

  it('reads back only what it wrote', () => {
    expect(parseGroupState(null)).toEqual({});
    expect(parseGroupState('not json')).toEqual({});
    expect(parseGroupState('[true]')).toEqual({});
    expect(parseGroupState('{"a":true,"b":"yes","c":false}')).toEqual({ a: true, c: false });
    expect(parseGroupState(JSON.stringify(withGroup({}, 'x', true)))).toEqual({ x: true });
  });

  it('expands and collapses all, and says which the control does next', () => {
    const ids = ['notes', 'a', 'b'];
    expect(allOpen({}, ids)).toBe(false);
    expect(allOpen(withAll(ids, true), ids)).toBe(true);
    expect(allOpen(withAll(ids, false), ids)).toBe(false);
  });

  it('gives each REST tag a distinct, anchor-safe id', () => {
    const ids = restGroups().map((g) => restGroupId(g.tag));
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(id).toMatch(/^rest-[a-z0-9-]+$/);
    expect(restGroupId('Live Events!')).toBe('rest-live-events');
  });
});
