// The mock checks announcements and notification prefs as the API does (apps/api/src/site.rs):
// each kind once, link words up to 40, links up to 500, a time that parses, an emote that is
// approved and public, and at most 50 announcements listed.
import { spawn } from 'node:child_process';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const PORT = 19800 + Math.floor(Math.random() * 150);
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
const code = (r) => r.errors?.[0]?.extensions?.code;

const CREATE = 'mutation($input: AnnouncementInput!) { createAnnouncement(input: $input) { id } }';
const LIST = '{ announcements { id } }';
const PREFS = 'mutation($prefs: [NotificationPrefInput!]!) { setNotificationPrefs(prefs: $prefs) { kind } }';
const ok = { kind: 'INFO', text: 'Hello' };

describe('mock announcements', () => {
  it('refuses what the API refuses', async () => {
    const cases = [
      [{ ...ok, linkUrl: '/shop', linkLabel: 'x'.repeat(41) }, 'bad_label'],
      [{ ...ok, linkUrl: `/${'a'.repeat(500)}` }, 'bad_link'],
      [{ ...ok, linkUrl: '/\\evil' }, 'bad_link'],
      [{ ...ok, startsAt: 'next tuesday' }, 'bad_time'],
      [{ ...ok, endsAt: 'soon' }, 'bad_time'],
      [{ ...ok, emoteId: 'nope' }, 'bad_emote'],
      [{ ...ok, text: '😀'.repeat(201) }, 'bad_text'],
    ];
    for (const [input, want] of cases) expect(code(await gql(CREATE, { input })), JSON.stringify(input).slice(0, 80)).toBe(want);
  });

  it('counts the line in characters, not UTF-16 units', async () => {
    const r = await gql(CREATE, { input: { ...ok, text: '😀'.repeat(200) } });
    expect(r.errors).toBeUndefined();
    expect(code(await gql(CREATE, { input: { ...ok, linkUrl: '/shop', linkLabel: '😀'.repeat(40) } }))).toBeUndefined();
  });

  it('lists at most 50', async () => {
    for (let i = 0; i < 55; i++) await gql(CREATE, { input: { ...ok, text: `n${i}` } });
    expect((await gql(LIST, {})).data.announcements).toHaveLength(50);
  });
});

describe('mock notification prefs', () => {
  it('refuses a kind twice and more kinds than exist', async () => {
    const p = { kind: 'cosmetic.granted', notify: true, toast: true };
    expect(code(await gql(PREFS, { prefs: [p, p] }))).toBe('unknown_kind');
    const many = Array.from({ length: 20 }, () => p);
    expect(code(await gql(PREFS, { prefs: many }))).toBe('too_many');
    expect((await gql(PREFS, { prefs: [p] })).errors).toBeUndefined();
  });
});
