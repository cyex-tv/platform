// The mock's faster reads (ADR 0032): persisted queries over GET (miss, hit, mismatch, mutations
// refused) and the bulk reads' order, de-duplication and caps, as the API answers them.
import { describe, expect, it } from 'vitest';
import { getRequest, hashOf } from './mock/persisted.mjs';
import { bulkAnswer, changesOf } from './mock/bulk.mjs';
import { extensionRead } from './mock/extension.mjs';
import { SETS } from './mock/set-data.mjs';

const params = (o) => new URLSearchParams(Object.entries(o).map(([k, v]) => [k, typeof v === 'string' ? v : JSON.stringify(v)]));
const pq = (hash) => ({ persistedQuery: { version: 1, sha256Hash: hash } });

describe('mock persisted queries over GET', () => {
  it('misses an unknown hash, stores the text, then hits by hash', () => {
    const query = 'query Mock1 { activeAnnouncement { id } }';
    const hash = hashOf(query);
    const miss = getRequest(params({ extensions: pq(hash) }));
    expect(miss.body.errors[0].message).toBe('PersistedQueryNotFound');
    expect(miss.body.errors[0].extensions.code).toBe('persisted_query_not_found');
    expect(getRequest(params({ query, extensions: pq(hash) })).request.query).toBe(query);
    expect(getRequest(params({ extensions: pq(hash) })).request.query).toBe(query);
  });

  it('refuses a wrong hash, a bad extension and mutations', () => {
    expect(getRequest(params({ query: '{ a }', extensions: pq('0'.repeat(64)) })).body.errors[0].extensions.code).toBe('persisted_query_mismatch');
    expect(getRequest(params({ extensions: pq('nope') })).body.errors[0].extensions.code).toBe('bad_persisted_query');
    const m = getRequest(params({ query: 'mutation { revokeOtherSessions }' }));
    expect(m.status).toBe(405);
    expect(m.body.errors[0].extensions.code).toBe('post_required');
  });
});

describe('mock bulk reads', () => {
  it('keep the order asked, list each once, leave out the unknown, and cap', () => {
    const [a, b] = SETS.filter((s) => s.visibility !== 'PRIVATE');
    const out = bulkAnswer('query { sets(ids: $ids) { id } }', { ids: [b.id, 'nope', a.id, b.id] }, false);
    expect(out.data.sets.map((s) => s.id)).toEqual([b.id, a.id]);
    expect(bulkAnswer('query { sets(ids: $ids) { id } }', { ids: Array(26).fill('x') }, false).errors[0].extensions.code).toBe('too_many');
    expect(bulkAnswer('query { emotes(ids: $ids) { id } }', { ids: Array(101).fill('x') }, false).errors[0].extensions.code).toBe('too_many');
    const rest = extensionRead('POST', '/v1/sets/lookup', { ids: Array(26).fill('x') });
    expect(rest.status).toBe(400);
    expect(rest.body.error).toBe('too_many');
  });

  it('answers a delta at the set’s version and a full resync otherwise', () => {
    const s = SETS.find((x) => x.visibility !== 'PRIVATE');
    expect(changesOf(s.id, 0, false).full).toBe(false);
    const full = changesOf(s.id, 5, false);
    expect(full.full).toBe(true);
    expect(full.set.id).toBe(s.id);
  });
});
