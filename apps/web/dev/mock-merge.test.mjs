// The mock's emote merge (ADR 0040): emoteMergePreview and mergeEmote for staff with emotes.merge,
// the alias an old ID becomes (`emote` and `emotes(ids)` answer the target with `mergedInto`), what
// leaves search, the set entries that move or collapse, the refusals, and the notification and history lines.
import { spawn } from 'node:child_process';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const PORT = 19950 + Math.floor(Math.random() * 40);
const BASE = `http://localhost:${PORT}`;
let child;

beforeAll(async () => {
  child = spawn(process.execPath, ['dev/mock-api.mjs'], { env: { ...process.env, PORT: String(PORT) }, stdio: 'ignore' });
  for (let i = 0; i < 100; i++) {
    try {
      await fetch(`${BASE}/mock/nothing`);
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  throw new Error('mock did not start');
}, 15000);
afterAll(() => child?.kill());

const SIGNED = 'mock_session=1';
const STAFF = 'mock_session=1; mock_admin=1';
async function gql(query, variables = {}, cookie = STAFF) {
  const res = await fetch(`${BASE}/v1/gql`, { method: 'POST', headers: { 'content-type': 'application/json', cookie }, body: JSON.stringify({ query, variables }) });
  return res.json();
}
const code = (r) => r.errors?.[0]?.extensions?.code;

const MY = `query MyEmotes { myEmotes { items { id name } } }`;
const SETS = `query MySets { mySets { id name emotes { name emote { id } } } }`;
const EMOTE = `query Emote($id: String!) { emote(id: $id) { id name mergedInto } }`;
const EMOTES = `query Emotes($ids: [String!]!) { emotes(ids: $ids) { id name mergedInto } }`;
const SEARCH = `query Search($query: String!) { searchEmotes(query: $query, exact: true) { items { id } } }`;
const PREVIEW = `query EmoteMergePreview($id: String!, $intoId: String!) { emoteMergePreview(id: $id, intoId: $intoId) {
  source { id name } target { id name } followed moves collapses { setId setName keptName droppedName } } }`;
const MERGE = `mutation MergeEmote($id: String!, $intoId: String!) { mergeEmote(id: $id, intoId: $intoId) {
  sourceId target { id name } moved collapsed { setId setName keptName droppedName } } }`;
const HISTORY = `query EmoteHistory($emoteId: String!) { emoteHistory(emoteId: $emoteId) { action after } }`;
const NOTES = `query MyNotifications { myNotifications { kind payload } }`;

// Seeds: the "Main" set holds a few of the account's own emotes and some public ones, "Personal" the first two.
let seeded;
const seeds = () => (seeded ??= seed());
async function seed() {
  const mine = (await gql(MY)).data.myEmotes.items;
  const sets = (await gql(SETS)).data.mySets;
  const main = sets.find((s) => s.name === 'Main');
  const personal = sets.find((s) => s.name === 'Personal');
  const own = new Set(mine.map((e) => e.id));
  const inMain = main.emotes.filter((m) => own.has(m.emote.id));
  const publicInMain = main.emotes.find((m) => !own.has(m.emote.id));
  return { mine, main, personal, collapseSource: inMain[0].emote, target: publicInMain.emote, moveSource: personal.emotes[0].emote, publicName: publicInMain.name };
}

describe('emoteMergePreview', () => {
  it('shows the source, the target, what moves and the sets that already have the target', async () => {
    const s = await seeds();
    const r = (await gql(PREVIEW, { id: s.collapseSource.id, intoId: s.target.id })).data.emoteMergePreview;
    expect(r.source.id).toBe(s.collapseSource.id);
    expect(r.target.id).toBe(s.target.id);
    expect(r.followed).toBe(false);
    expect(r.collapses).toHaveLength(1);
    expect(r.collapses[0]).toMatchObject({ setName: 'Main', keptName: s.publicName, droppedName: s.main.emotes.find((m) => m.emote.id === s.collapseSource.id).name });
    expect(r.moves).toBe(0);
    // Asking changed nothing.
    expect((await gql(EMOTE, { id: s.collapseSource.id })).data.emote.mergedInto).toBeNull();
  });

  it('refuses without the permission, into itself, and for an emote that is not there', async () => {
    const s = await seeds();
    expect(code(await gql(PREVIEW, { id: s.collapseSource.id, intoId: s.target.id }, SIGNED))).toBe('forbidden');
    expect(code(await gql(PREVIEW, { id: s.target.id, intoId: s.target.id }))).toBe('merge_into_self');
    expect(code(await gql(PREVIEW, { id: s.target.id, intoId: '1' }))).toBe('not_found');
    expect(code(await gql(PREVIEW, { id: '1', intoId: s.target.id }))).toBe('not_found');
  });

  it('refuses a target that is not approved', async () => {
    const s = await seeds();
    const form = new FormData();
    form.set('name', 'mergePending');
    form.set('file', new Blob([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0xff, 0xfe])], { type: 'image/png' }), 'a.png');
    const up = await (await fetch(`${BASE}/v1/emotes`, { method: 'POST', body: form, headers: { cookie: SIGNED } })).json();
    expect(code(await gql(PREVIEW, { id: s.collapseSource.id, intoId: up.id }))).toBe('merge_target_unavailable');
  });
});

describe('mergeEmote', () => {
  it('moves set entries, keeps the target entry where a set had both, and retires the source', async () => {
    const s = await seeds();
    expect(code(await gql(MERGE, { id: s.collapseSource.id, intoId: s.target.id }, SIGNED))).toBe('forbidden');

    // A set entry that only has the source moves and keeps its name.
    const personalName = s.personal.emotes.find((m) => m.emote.id === s.moveSource.id).name;
    const moved = (await gql(MERGE, { id: s.moveSource.id, intoId: s.target.id })).data.mergeEmote;
    expect(moved.sourceId).toBe(s.moveSource.id);
    expect(moved.target.id).toBe(s.target.id);
    expect(moved.moved).toBe(1);
    expect(moved.collapsed).toEqual([]);
    const after = (await gql(SETS)).data.mySets.find((x) => x.name === 'Personal');
    expect(after.emotes.find((m) => m.emote.id === s.target.id)?.name).toBe(personalName);
    expect(after.emotes.some((m) => m.emote.id === s.moveSource.id)).toBe(false);

    // A set that held both keeps the target's entry and its name.
    const dropped = s.main.emotes.find((m) => m.emote.id === s.collapseSource.id).name;
    const both = (await gql(MERGE, { id: s.collapseSource.id, intoId: s.target.id })).data.mergeEmote;
    expect(both.moved).toBe(0);
    expect(both.collapsed).toEqual([{ setId: s.main.id, setName: 'Main', keptName: s.publicName, droppedName: dropped }]);
    const main = (await gql(SETS)).data.mySets.find((x) => x.name === 'Main');
    expect(main.emotes.filter((m) => m.emote.id === s.target.id)).toHaveLength(1);
    expect(main.emotes.some((m) => m.emote.id === s.collapseSource.id)).toBe(false);
  });

  it('answers the old ID with the target and mergedInto, one by one and in bulk', async () => {
    const s = await seeds();
    const one = (await gql(EMOTE, { id: s.moveSource.id })).data.emote;
    expect(one).toMatchObject({ id: s.moveSource.id, mergedInto: s.target.id });
    const bulk = (await gql(EMOTES, { ids: [s.moveSource.id, s.target.id] })).data.emotes;
    expect(bulk.find((e) => e.id === s.moveSource.id).mergedInto).toBe(s.target.id);
    expect(bulk.find((e) => e.id === s.target.id).mergedInto).toBeNull();
    const name = (await gql(EMOTE, { id: s.target.id })).data.emote.name;
    expect(one.name).toBe(name);
  });

  it('leaves the retired emote out of search and refuses to merge it again', async () => {
    const s = await seeds();
    const hit = (await gql(SEARCH, { query: s.mine.find((e) => e.id === s.moveSource.id)?.name ?? 'zzzz' })).data.searchEmotes.items;
    expect(hit.some((e) => e.id === s.moveSource.id)).toBe(false);
    expect(code(await gql(MERGE, { id: s.moveSource.id, intoId: s.target.id }))).toBe('already_merged');
    expect(code(await gql(PREVIEW, { id: s.moveSource.id, intoId: s.target.id }))).toBe('already_merged');
  });

  it('follows a merged target to the final one and says so', async () => {
    const s = await seeds();
    const other = s.mine.find((e) => e.id !== s.moveSource.id && e.id !== s.collapseSource.id);
    const r = (await gql(PREVIEW, { id: other.id, intoId: s.collapseSource.id })).data.emoteMergePreview;
    expect(r.followed).toBe(true);
    expect(r.target.id).toBe(s.target.id);
  });

  it('logs emote.merged on both emotes and tells the owner', async () => {
    const s = await seeds();
    for (const id of [s.target.id, s.moveSource.id]) {
      const row = (await gql(HISTORY, { emoteId: id })).data.emoteHistory.find((h) => h.action === 'emote.merged');
      expect(row, id).toBeTruthy();
      const a = JSON.parse(row.after);
      expect(a).toMatchObject({ target_id: s.target.id, via: 'emotes.merge' });
      expect(Array.isArray(a.collapsed)).toBe(true);
    }
    const notes = (await gql(NOTES)).data.myNotifications.filter((n) => n.kind === 'emote.merged');
    expect(notes.length).toBeGreaterThanOrEqual(1);
    expect(JSON.parse(notes[0].payload)).toMatchObject({ role: expect.stringMatching(/source|target|both/), emote_id: s.target.id });
  });
});
