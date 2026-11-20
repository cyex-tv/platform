import { describe, expect, it, vi } from 'vitest';

const { calls } = vi.hoisted(() => ({ calls: [] as unknown[] }));
vi.mock('$lib/api', () => ({
  gql: vi.fn(async (_q: unknown, v: { sha256: string }) => {
    calls.push(v);
    if (v.sha256.startsWith('e3b0')) return { emotesWithSameFile: [{ id: 'E1', name: 'catJAM', owner: { id: 'A1', displayName: 'nora' } }] };
    throw new Error('down');
  }),
  onSignedOut: vi.fn(),
}));

import { sameFileEmotes, sameFileLine, sha256Hex } from './same-file';

describe('same file before an upload', () => {
  it('hashes the file as the API does (SHA-256, hex)', async () => {
    expect(await sha256Hex(new Blob([]))).toBe('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
    expect(await sha256Hex(new Blob(['abc']))).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  });

  it('asks the API with that hash and hands back what it found', async () => {
    const found = await sameFileEmotes(new Blob([]));
    expect(found.map((e) => e.name)).toEqual(['catJAM']);
    expect(calls.at(-1)).toEqual({ sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' });
  });

  it('a failed check is no hint, never an error', async () => {
    expect(await sameFileEmotes(new Blob(['abc']))).toEqual([]);
  });

  it('names the owner and counts the rest', () => {
    expect(sameFileLine([{ id: '1', name: 'a', owner: { id: 'x', displayName: 'nora' } }])).toEqual({ by: 'by nora', more: '' });
    expect(sameFileLine([{ id: '1', name: 'a', owner: null }, { id: '2', name: 'b' }, { id: '3', name: 'c' }])).toEqual({ by: '', more: ' and 2 more' });
  });
});
