// Mock API: set data. See ../mock-api.mjs.
import { ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL, MINE, member } from './emotes.mjs';

export const SETS = [
  { id: '01JD8XSET0000000000000MAIN', name: 'Main', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: [...MINE.slice(3, 7), ...ALL.slice(10, 30)].map((e) => member(e)), editors: [OTHERS[0]], visibility: 'LISTED', createdAt: '2026-11-10T12:05:00.000Z' },
  { id: '01JD8XSET000000000000ALTSS', name: 'Subathon', kind: 'CHANNEL', capacity: 1000, owner: ME_SUMMARY,
    emotes: ALL.slice(40, 46).map((e) => member(e)), editors: [], visibility: 'UNLISTED', createdAt: '2026-11-12T19:40:00.000Z' },
  // Every account has a personal set from sign-up: 3 slots, or 10 for a subscriber role (ADR 0038).
  { id: '01JD8XSET0000000000PERSONME', name: 'Personal', kind: 'PERSONAL', capacity: 3, capacityOverride: null, owner: ME_SUMMARY,
    emotes: MINE.slice(0, 2).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-10T12:10:00.000Z' },
  { id: '01JD8XSET00000000000000OAK', name: 'quietoak main', kind: 'CHANNEL', capacity: 1000, owner: OTHERS[0],
    emotes: ALL.slice(60, 70).map((e) => member(e)), editors: [ME_SUMMARY], visibility: 'LISTED', createdAt: '2026-11-08T09:00:00.000Z' },
];
// MOCK_SET_SIZE=1000: the "Main" set holds that many emotes (a real set can hold 1.000), to try
// paging and the filter in the set editor. The extra ones are copies of the mock emotes under
// their own ids and numbered names.
if (Number(process.env.MOCK_SET_SIZE) > SETS[0].emotes.length) {
  const want = Math.min(1000, Number(process.env.MOCK_SET_SIZE));
  for (let i = SETS[0].emotes.length; i < want; i++) {
    const base = ALL[i % ALL.length];
    SETS[0].emotes.push(member({ ...base, id: `${base.id}-big${i}`, name: `${base.name}${i}`, createdAt: new Date(Date.parse('2026-11-01T00:00:00Z') + i * 60_000).toISOString() }));
  }
}
export const GLOBAL = { id: '01JD8XSET000000000000GLOBL', name: 'Global', kind: 'GLOBAL', capacity: 200, owner: null,
  emotes: ALL.slice(0, 24).map((e) => member(e)), editors: [], visibility: 'LISTED', createdAt: '2026-11-01T00:00:00.000Z' };
SETS.push(GLOBAL);
