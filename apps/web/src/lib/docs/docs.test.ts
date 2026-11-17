import { describe, expect, it } from 'vitest';
import spec from '../../../../../openapi/openapi.json';
import { graphqlReference } from './graphql';
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
      expect(o.samples.map((s) => s.lang)).toEqual(['curl', 'fetch']);
      expect(o.samples[0].code).toContain('https://api.cyex.tv');
      expect(o.responses.length).toBeGreaterThan(0);
    }
    expect(restGroups().flatMap((g) => g.ops)).toHaveLength(ops.length);
  });

  it('puts path parameters into the sample URL', () => {
    const channel = ops.find((o) => o.path === '/v1/channels/{platform}/{id}')!;
    expect(channel.samples[0].code).toContain('/v1/channels/twitch/123');
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
  });
});
