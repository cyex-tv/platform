// The mock's two approvals per emote (ADR 0039): uploads start unlisted and pending, reviewApprovals
// decides the listing and personal use apart, the personal queue and its count, the PERSONAL_USE
// search flag, personal sets taking only approved emotes, and transfers by staff alone.
import { spawn } from 'node:child_process';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const PORT = 19800 + Math.floor(Math.random() * 150);
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
async function upload(fields, cookie = SIGNED) {
  const form = new FormData();
  for (const [k, v] of Object.entries(fields)) form.set(k, v);
  form.set('file', new Blob([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 0xff, 0xfe])], { type: 'image/png' }), 'a.png');
  return fetch(`${BASE}/v1/emotes`, { method: 'POST', body: form, headers: { cookie } });
}

const QUEUE = `query ReviewQueue($filter: ReviewQueueFilter) { reviewQueue(filter: $filter, perPage: 100) { total counts { public limited personal }
  items { id name state visibility approvals { publicListing personalUse } } } }`;
const REVIEW = `mutation ReviewApprovals($ids: [String!]!, $input: ApprovalsInput!) { reviewApprovals(ids: $ids, input: $input) { done failed { id code } } }`;
const EMOTE = `query Emote($id: String!) { emote(id: $id) { id state visibility approvals { publicListing personalUse } } }`;

describe('uploads', () => {
  it('start unlisted with both approvals pending, and private stays private', async () => {
    const a = await (await upload({ name: 'apprA' })).json();
    expect(a.visibility).toBe('unlisted');
    expect(a.approvals).toEqual({ public_listing: 'pending', personal_use: 'pending' });
    const b = await (await upload({ name: 'apprB', private: 'true' })).json();
    expect(b.visibility).toBe('private');
    // `public` means "not private": still unlisted until reviewed.
    const c = await (await upload({ name: 'apprC', visibility: 'public' })).json();
    expect(c.visibility).toBe('unlisted');
  });

  it('refuses visibility=unlisted from anyone but staff', async () => {
    const res = await upload({ name: 'apprD', visibility: 'unlisted' });
    expect(res.status).toBe(400);
    expect((await res.json()).error).toBe('bad_visibility');
    expect((await upload({ name: 'apprE', visibility: 'unlisted' }, STAFF)).status).toBe(202);
  });
});

describe('reviewApprovals', () => {
  it('needs something to decide, and a reason to deny', async () => {
    const q = (await gql(QUEUE, { filter: { tab: 'PENDING' } })).data.reviewQueue;
    const id = q.items[0].id;
    expect(code(await gql(REVIEW, { ids: [id], input: {} }))).toBe('bad_approvals');
    expect(code(await gql(REVIEW, { ids: [id], input: { publicListing: 'DENY' } }))).toBe('bad_approvals');
    expect(code(await gql(REVIEW, { ids: [id], input: { publicListing: 'APPROVE' } }, SIGNED))).toBe('forbidden');
  });

  it('approves the listing and personal use apart; only pending parts count', async () => {
    const q = (await gql(QUEUE, { filter: { tab: 'PUBLIC' } })).data.reviewQueue;
    const [one, two] = q.items;
    expect(one.visibility).toBe('UNLISTED');
    // Listing first: listed and usable, personal use still waiting.
    let r = await gql(REVIEW, { ids: [one.id], input: { publicListing: 'APPROVE' } });
    expect(r.data.reviewApprovals).toEqual({ done: [one.id], failed: [] });
    expect((await gql(EMOTE, { id: one.id })).data.emote).toMatchObject({ state: 'APPROVED', visibility: 'PUBLIC', approvals: { publicListing: 'APPROVED', personalUse: 'PENDING' } });
    // The same call again finds nothing pending on the listing.
    r = await gql(REVIEW, { ids: [one.id], input: { publicListing: 'APPROVE' } });
    expect(r.data.reviewApprovals.failed).toEqual([{ id: one.id, code: 'not_pending' }]);
    // It sits in the personal queue now; denying personal use needs a reason and empties it.
    const after = (await gql(QUEUE, { filter: { tab: 'PERSONAL' } })).data.reviewQueue;
    expect(after.items.map((e) => e.id)).toContain(one.id);
    expect(after.counts.personal).toBe(after.total);
    r = await gql(REVIEW, { ids: [one.id], input: { personalUse: 'DENY', reason: 'Too loud for chat.' } });
    expect(r.data.reviewApprovals.done).toEqual([one.id]);
    expect((await gql(QUEUE, { filter: { tab: 'PERSONAL' } })).data.reviewQueue.items.map((e) => e.id)).not.toContain(one.id);
    // onlyPending false changes a decision already made (the edit dialog).
    r = await gql(REVIEW, { ids: [one.id], input: { personalUse: 'APPROVE', onlyPending: false } });
    expect(r.data.reviewApprovals.done).toEqual([one.id]);
    expect((await gql(EMOTE, { id: one.id })).data.emote.approvals.personalUse).toBe('APPROVED');
    // Both at once on a pending upload, and a denied listing stays unlisted but usable.
    r = await gql(REVIEW, { ids: [two.id], input: { publicListing: 'DENY', personalUse: 'APPROVE', reason: 'Not for the listing.' } });
    expect(r.data.reviewApprovals.done).toEqual([two.id]);
    expect((await gql(EMOTE, { id: two.id })).data.emote).toMatchObject({ state: 'APPROVED', visibility: 'UNLISTED', approvals: { publicListing: 'DENIED', personalUse: 'APPROVED' } });
  });

  it('filters any tab by personal use', async () => {
    const r = (await gql(QUEUE, { filter: { tab: 'PERSONAL', personalUse: 'APPROVED' } })).data.reviewQueue;
    expect(r.items).toEqual([]);
  });
});

describe('searchEmotes PERSONAL_USE', () => {
  const SEARCH = `query Search($flags: [EmoteFlag!]) { searchEmotes(query: "", flags: $flags, perPage: 100) { total items { id approvals { personalUse } } } }`;
  it('keeps only emotes approved for personal use', async () => {
    const all = (await gql(SEARCH)).data.searchEmotes;
    const some = (await gql(SEARCH, { flags: ['PERSONAL_USE'] })).data.searchEmotes;
    expect(some.total).toBeGreaterThan(0);
    expect(some.total).toBeLessThan(all.total);
    expect(some.items.every((e) => e.approvals.personalUse === 'APPROVED')).toBe(true);
  });
});

describe('personal sets and transfers', () => {
  it('a personal set refuses an emote not approved for personal use', async () => {
    const queue = (await gql(QUEUE, { filter: { tab: 'PERSONAL' } })).data.reviewQueue;
    const waiting = queue.items[0]?.id;
    expect(waiting).toBeTruthy();
    // Every account has its personal set already (ADR 0038, personal set slots).
    const setId = '01JD8XSET0000000000PERSONME';
    expect(setId).toBeTruthy();
    const add = await gql(`mutation AddSetEmote($setId: String!, $emoteId: String!) { addSetEmote(setId: $setId, emoteId: $emoteId) { name } }`, { setId, emoteId: waiting });
    expect(code(add)).toBe('personal_use_not_approved');
  });

  it('only staff move an emote; the request mutations answer 410', async () => {
    const mine = (await gql(`query MyEmotes { myEmotes(perPage: 100) { items { id state } } }`, {}, SIGNED)).data.myEmotes.items.find((e) => e.state === 'APPROVED');
    const found = (await gql(`query Search { searchEmotes(query: "", perPage: 5) { items { owner { id } } } }`)).data.searchEmotes.items;
    const to = found.find((e) => e.owner?.id)?.owner.id;
    const move = `mutation TransferEmote($id: String!, $toAccountId: String!) { transferEmote(id: $id, toAccountId: $toAccountId) { id } }`;
    expect(code(await gql(move, { id: mine.id, toAccountId: to }, SIGNED))).toBe('transfer_staff_only');
    expect((await gql(move, { id: mine.id, toAccountId: to })).errors).toBeUndefined();
    expect(code(await gql(`mutation AcceptEmoteTransfer { acceptEmoteTransfer(id: "x") { id } }`))).toBe('transfer_requests_retired');
    expect(code(await gql(`mutation DeclineEmoteTransfer { declineEmoteTransfer(id: "x") { id } }`))).toBe('transfer_requests_retired');
    expect(code(await gql(`mutation CancelEmoteTransfer { cancelEmoteTransfer(id: "x") { id } }`))).toBe('transfer_requests_retired');
    expect((await gql(`query EmoteTransferRequests { emoteTransferRequests { id } }`)).data.emoteTransferRequests).toEqual([]);
  });
});
