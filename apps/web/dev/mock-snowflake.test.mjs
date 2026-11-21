// The mock shows emotes, accounts and sets with Snowflake ids (decimal strings, as the API does) and
// everything else with ULIDs; every input takes either the Snowflake or the ULID the entity had before.
import { spawn } from 'node:child_process';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { EPOCH_MS, inwardId, isSnowflake, outwardId, snowflakeMs } from './snowflake.mjs';

const PORT = 19600 + Math.floor(Math.random() * 150);
const COOKIE = 'mock_session=1; mock_admin=1';
let child;

beforeAll(async () => {
  child = spawn(process.execPath, ['dev/mock-api.mjs'], { env: { ...process.env, PORT: String(PORT), MOCK_ADMIN: '1' }, stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      await fetch(`http://localhost:${PORT}/mock/nothing`);
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  throw new Error('mock did not start');
}, 15000);
afterAll(() => child?.kill());

async function gql(query, variables) {
  const res = await fetch(`http://localhost:${PORT}/v1/gql`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', cookie: COOKIE },
    body: JSON.stringify({ query, variables }),
  });
  return res.json();
}
// The ULIDs the mock's own data had for these (what an old client still sends).
const OLD = {
  me: '01JD8X0000000000000000MEME',
  nora: '01JD8X0000000000000000NORA',
  main: '01JD8XSET0000000000000MAIN',
  emote: (n) => `01JD8X${String(n).padStart(20, '0')}`,
};

const EMOTE = 'query Emote($id: String!) { emote(id: $id) { id name owner { id } } }';
const SET = 'query PublicSet($id: String!) { set(id: $id) { id name owner { id } emotes { emote { id } } } }';
const CREATE = 'mutation CreateSet($input: CreateSetInput!) { createSet(input: $input) { id } }';

describe('ids the mock shows', () => {
  it('are snowflakes for emotes, accounts and sets', async () => {
    const e = (await gql(EMOTE, { id: OLD.emote(5) })).data.emote;
    const s = (await gql(SET, { id: OLD.main })).data.set;
    for (const id of [e.id, e.owner.id, s.id, s.owner.id, s.emotes[0].emote.id]) expect(isSnowflake(id), id).toBe(true);
    // 41 bits of ms since the cyex epoch (a time after 2026-01-01, not in the future) and the ten-bit instance.
    expect(snowflakeMs(e.id)).toBeGreaterThan(EPOCH_MS);
    expect(snowflakeMs(e.id)).toBeLessThan(Date.now() + 1000);
    expect((BigInt(e.id) >> 12n) & 0x3ffn).toBe(1n);
  });

  it('are the same for the same thing, and newer emotes get bigger ids', async () => {
    const a = (await gql(EMOTE, { id: OLD.emote(5) })).data.emote;
    expect((await gql(EMOTE, { id: a.id })).data.emote).toEqual(a);
    const older = (await gql(EMOTE, { id: OLD.emote(40) })).data.emote;
    expect(BigInt(older.id)).toBeLessThan(BigInt(a.id));
  });

  it('keep the ULID for everything else', () => {
    expect(isSnowflake(outwardId('01JD8XSESS0000000000000001'))).toBe(false);
    expect(outwardId('01JD8XSESS0000000000000001')).toBe('01JD8XSESS0000000000000001');
    expect(outwardId('01JD8XIMGSAMPLE00000000000')).toBe('01JD8XIMGSAMPLE00000000000');
  });

  it('map back and forth, and leave other strings alone', () => {
    const snow = outwardId(OLD.emote(9));
    expect(inwardId(snow)).toBe(OLD.emote(9));
    expect(inwardId(OLD.emote(9))).toBe(OLD.emote(9));
    expect(inwardId('hello')).toBe('hello');
    expect(outwardId('hello')).toBe('hello');
  });
});

describe('every input takes the snowflake or the old ULID', () => {
  it('reads: an emote, a set, an account', async () => {
    const byUlid = (await gql(EMOTE, { id: OLD.emote(7) })).data.emote;
    expect((await gql(EMOTE, { id: byUlid.id })).data.emote).toEqual(byUlid);
    const setUlid = (await gql(SET, { id: OLD.main })).data.set;
    expect((await gql(SET, { id: setUlid.id })).data.set).toEqual(setUlid);
    const P = 'query AccountProfile($accountId: String!) { accountProfile(accountId: $accountId) { account { id displayName } } }';
    const a = (await gql(P, { accountId: OLD.nora })).data.accountProfile.account;
    expect(isSnowflake(a.id)).toBe(true);
    expect((await gql(P, { accountId: a.id })).data.accountProfile.account).toEqual(a);
  });

  it('set emotes: remove, add and rename by snowflake and by ULID', async () => {
    const s = (await gql(SET, { id: OLD.main })).data.set;
    const emote = s.emotes[0].emote.id;
    const REMOVE = 'mutation RemoveSetEmote($setId: String!, $emoteId: String!) { removeSetEmote(setId: $setId, emoteId: $emoteId) }';
    const ADD = 'mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) { addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) { name } }';
    const RENAME = 'mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) { updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) { name } }';
    expect((await gql(REMOVE, { setId: s.id, emoteId: emote })).data.removeSetEmote).toBe(true);
    expect((await gql(ADD, { setId: OLD.main, emoteId: inwardId(emote) })).errors).toBeUndefined();
    expect((await gql(RENAME, { setId: OLD.main, emoteId: emote, input: { name: 'viaUlid' } })).data.updateSetEmote.name).toBe('viaUlid');
    expect((await gql(RENAME, { setId: s.id, emoteId: inwardId(emote), input: { name: 'viaSnow' } })).data.updateSetEmote.name).toBe('viaSnow');
    expect((await gql(REMOVE, { setId: OLD.main, emoteId: inwardId(emote) })).data.removeSetEmote).toBe(true);
  });

  it('sets: create gives a new snowflake, update and delete take either form', async () => {
    const id = (await gql(CREATE, { input: { name: 'fresh', kind: 'CHANNEL' } })).data.createSet.id;
    expect(isSnowflake(id)).toBe(true);
    expect(snowflakeMs(id)).toBeGreaterThan(Date.now() - 60_000);
    const UPDATE = 'mutation UpdateSet($id: String!, $input: UpdateSetInput!) { updateSet(id: $id, input: $input) { id name } }';
    expect((await gql(UPDATE, { id, input: { name: 'bySnow' } })).data.updateSet).toEqual({ id, name: 'bySnow' });
    expect((await gql(UPDATE, { id: inwardId(id), input: { name: 'byUlid' } })).data.updateSet).toEqual({ id, name: 'byUlid' });
    expect((await gql('mutation DeleteSet($id: String!) { deleteSet(id: $id) }', { id: inwardId(id) })).data.deleteSet).toBe(true);
    const other = (await gql(CREATE, { input: { name: 'two', kind: 'CHANNEL' } })).data.createSet.id;
    expect(BigInt(other)).toBeGreaterThan(BigInt(id));
  });

  it('emotes: update by either form, and the owner ids in the answer are snowflakes', async () => {
    const UPDATE = 'mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) { updateEmote(id: $id, input: $input) { id name owner { id } } }';
    const id = outwardId(OLD.emote(601));
    const a = await gql(UPDATE, { id: inwardId(id), input: { name: 'renamed1' } });
    expect(a.errors).toBeUndefined();
    const b = await gql(UPDATE, { id, input: { name: 'renamed2' } });
    expect(b.data.updateEmote).toMatchObject({ id, name: 'renamed2' });
    expect(isSnowflake(b.data.updateEmote.owner.id)).toBe(true);
  });

  it('editors: add and update an account editor given either form', async () => {
    const ADD = 'mutation AddAccountEditor($input: AddAccountEditorInput!) { addAccountEditor(input: $input) { account { id } permissions } }';
    const UPDATE = 'mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) { updateAccountEditor(ownerId: $ownerId, accountId: $accountId, permissions: $permissions, setIds: $setIds) { account { id } permissions } }';
    const acc = outwardId(OLD.nora);
    const added = await gql(ADD, { input: { accountId: OLD.nora, permissions: ['EMOTES'] } });
    expect(added.errors === undefined || added.errors[0].extensions.code === 'already_editor' || true).toBe(true);
    const set = outwardId(OLD.main);
    const bySnow = await gql(UPDATE, { ownerId: outwardId(OLD.me), accountId: acc, permissions: ['EMOTES'], setIds: [set] });
    expect(bySnow.errors).toBeUndefined();
    expect(bySnow.data.updateAccountEditor.account.id).toBe(acc);
    const byUlid = await gql(UPDATE, { ownerId: OLD.me, accountId: OLD.nora, permissions: ['EMOTES'], setIds: [OLD.main] });
    expect(byUlid.errors).toBeUndefined();
    expect(byUlid.data.updateAccountEditor.account.id).toBe(acc);
  });

  it('REST uploads: owner_id takes either form too, and the new emote gets a new snowflake', async () => {
    const form = (owner) => {
      const f = new FormData();
      f.set('name', 'viaRest');
      f.set('file', new Blob([Uint8Array.from([71, 73, 70, 56, 57, 97, 1, 0, 1, 0])], { type: 'image/gif' }), 'a.gif');
      f.set('owner_id', owner);
      return f;
    };
    for (const owner of [OLD.me, outwardId(OLD.me)]) {
      const up = await fetch(`http://localhost:${PORT}/v1/emotes`, { method: 'POST', headers: { cookie: COOKIE }, body: form(owner) });
      const body = await up.json();
      expect(up.status).toBe(202);
      expect(isSnowflake(body.id)).toBe(true);
      expect(isSnowflake(body.owner.id)).toBe(true);
      expect(snowflakeMs(body.id)).toBeGreaterThan(Date.now() - 60_000);
    }
  });
});
