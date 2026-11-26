// Mock API: admin. See ../mock-api.mjs.
import { ME_ID, OWNERS } from './config.mjs';

// Restrictions: what holds now (with ends) and every one placed, newest first.
export const STATUS = {};
export const RESTRICTIONS = {};
export const statusOf = (id) => {
  const st = STATUS[id] ?? {};
  const now = Date.now();
  const live = (on, until) => !!on && (!until || Date.parse(until) > now);
  const out = { id, uploadsBlocked: live(st.uploadsBlocked, st.uploadsBlockedUntil), ticketsBlocked: live(st.ticketsBlocked, st.ticketsBlockedUntil), blocked: live(st.blocked, st.blockedUntil) };
  out.uploadsBlockedUntil = out.uploadsBlocked ? st.uploadsBlockedUntil ?? null : null;
  out.ticketsBlockedUntil = out.ticketsBlocked ? st.ticketsBlockedUntil ?? null : null;
  out.blockedUntil = out.blocked ? st.blockedUntil ?? null : null;
  return out;
};
export const HOLD_FIELD = { UPLOADS: 'uploadsBlocked', TICKETS: 'ticketsBlocked', ACCOUNT: 'blocked' };

export const AUDIT = Array.from({ length: 64 }, (_, i) => ({
  id: '01JD8XAUD' + String(1000 - i).padStart(17, '0'), actorId: i % 5 === 0 ? null : i % 3 === 0 ? OWNERS[1].id : ME_ID,
  action: ['emote.approved', 'set.editor_added', 'role.updated', 'account.restricted'][i % 4],
  targetKind: ['emote', 'set', 'role', 'account'][i % 4],
  targetId: i % 4 === 3 ? OWNERS[1 + (i % 3)].id : '01JD8X' + String(i).padStart(20, '0'),
  before: i % 4 === 3 ? JSON.stringify({ uploads_blocked: false, blocked: false }) : null,
  after: JSON.stringify({ state: 'APPROVED', n: i }), createdAt: new Date(Date.UTC(2026, 10, 16, 22, 0, 0) - i * 600_000).toISOString(),
}));
