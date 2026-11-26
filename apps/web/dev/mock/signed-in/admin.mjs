// Mock API: signed in/admin. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY, OTHERS } from '../config.mjs';
import { IDENTITIES } from '../account.mjs';
import { rolesOf } from '../roles.mjs';
import { AUDIT, HOLD_FIELD, RESTRICTIONS, STATUS, statusOf } from '../admin.mjs';
import { STAFF_NOTES, graphOf } from '../entitlements.mjs';
import { publicAccount } from '../directory.mjs';
import { err } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInAdmin(op, body, v, ADMIN) {
  switch (op) {
    case 'AdminAccount': {
      const a = publicAccount(v.accountId);
      if (!ADMIN) return err('This needs users.view or users.restrict.', 'forbidden');
      if (!a) return err('Not found.', 'not_found');
      const st = statusOf(a.id);
      const idn = (platform, n) => ({ platform, platformUserId: '5550' + a.id.slice(-3) + n, login: a.displayName.toLowerCase(), displayName: a.displayName, avatar: a.avatar });
      return { data: { adminAccount: { ...a, createdAt: '2026-10-02T09:30:00.000Z', ...st, identities: a.id === ME_ID ? IDENTITIES : [idn('TWITCH', 1), ...(a.id.endsWith('NORA') ? [idn('KICK', 2)] : [])] } } };
    }
    case 'User': {
      const a = [...OTHERS, ME_SUMMARY].find((o) => o.displayName === v.login);
      return { data: { channelByLogin: a ? { platform: v.platform, platformChannelId: '5550' + a.id.slice(-3), login: v.login, account: a } : null } };
    }
    case 'UserDetails': {
      return { data: { ...(v.status ? { accountStatus: statusOf(v.accountId) } : {}), ...(v.roles ? { accountRoles: rolesOf(v.accountId) } : {}) } };
    }
    case 'PlaceRestriction': {
      if (v.accountId === ME_ID) return err('The owner’s account can’t be restricted.', 'owner_protected');
      const reason = (v.input.reason ?? '').trim();
      if (!reason) return err('Give a reason of 1 to 500 characters.', 'bad_reason');
      if (v.input.until && Date.parse(v.input.until) <= Date.now()) return err('The end has to be in the future.', 'bad_time');
      const f = HOLD_FIELD[v.input.kind];
      const now = new Date().toISOString();
      const list = (RESTRICTIONS[v.accountId] ??= []);
      for (const r of list) if (r.kind === v.input.kind && r.active) Object.assign(r, { active: false, liftedAt: now, liftedBy: ME_ID });
      list.unshift({ id: 'RS' + Date.now(), kind: v.input.kind, reason, endsAt: v.input.until ?? null, createdBy: ME_ID, createdAt: now, liftedAt: null, liftedBy: null, active: true });
      Object.assign((STATUS[v.accountId] ??= {}), { [f]: true, [f + 'Until']: v.input.until ?? null });
      return { data: { placeRestriction: statusOf(v.accountId) } };
    }
    case 'LiftRestriction': {
      const f = HOLD_FIELD[v.kind];
      const now = new Date().toISOString();
      for (const r of RESTRICTIONS[v.accountId] ?? []) if (r.kind === v.kind && r.active) Object.assign(r, { active: false, liftedAt: now, liftedBy: ME_ID });
      Object.assign((STATUS[v.accountId] ??= {}), { [f]: false, [f + 'Until']: null });
      return { data: { liftRestriction: statusOf(v.accountId) } };
    }
    case 'StaffNotes':
      return { data: { staffNotes: STAFF_NOTES[v.accountId] ?? [] } };
    case 'AddStaffNote': {
      const body = (v.body ?? '').trim();
      if (!body || body.length > 2000) return err('Notes are 1 to 2000 characters.', 'bad_note');
      const n = { id: 'NOTE' + Date.now(), authorId: ME_ID, body, createdAt: new Date().toISOString() };
      (STAFF_NOTES[v.accountId] ??= []).unshift(n);
      return { data: { addStaffNote: n } };
    }
    case 'DeleteStaffNote': {
      for (const list of Object.values(STAFF_NOTES)) {
        const i = list.findIndex((n) => n.id === v.id);
        if (i >= 0) { list.splice(i, 1); return { data: { deleteStaffNote: true } }; }
      }
      return { data: { deleteStaffNote: false } };
    }
    case 'AccountSessions': {
      if (statusOf(v.accountId).blocked) return { data: { accountSessions: [] } };
      return { data: { accountSessions: [
        { id: 'SESS1', kind: 'SITE', device: 'Firefox on Windows', createdAt: '2026-11-15T10:00:00.000Z', lastSeenAt: '2026-11-19T08:30:00.000Z', expiresAt: '2026-12-15T10:00:00.000Z' },
        { id: 'SESS2', kind: 'APP', device: 'cyex deck', createdAt: '2026-11-01T18:00:00.000Z', lastSeenAt: '2026-11-18T22:10:00.000Z', expiresAt: '2027-01-30T18:00:00.000Z' },
      ] } };
    }
    case 'AccountSubscriptions':
      return { data: { accountSubscriptions: [] } };
    case 'EntitlementGraph':
      return { data: { entitlementGraph: graphOf(v.accountId) } };
    case 'EntitlementHolders': {
      // Catalog items stand in for the made-up ones: odd ones held now by most, even ones taken back from some.
      // The catalogue's paints stand in for a monthly one too, so their holders graph fans out.
      const stand = { BADGE: ['BADGE1', 'BADGE2'], PAINT: ['PAINT1', 'PAINT2', 'PAINT3', 'MPNT110', 'MPNT100'] }[v.kind];
      const every = [ME_ID, ...OTHERS.map((o) => o.id)].flatMap((id, i) => graphOf(id).map((g) => ({ ...g, grantedAt: g.grantedAt ? new Date(Date.parse(g.grantedAt) - i * 5 * 3600_000).toISOString() : null })));
      const like = stand && !stand.includes(v.id) && !every.some((g) => g.itemId === v.id) ? stand[[...v.id].reduce((n, c) => n + c.charCodeAt(0), 0) % stand.length] : v.id;
      const all = every.filter((g) => g.kind === v.kind && g.itemId === like).map((g) => ({ ...g, itemId: v.id }));
      return { data: { entitlementHolders: all } };
    }
    case 'AccountRestrictions': {
      const now = Date.now();
      const rows = (RESTRICTIONS[v.accountId] ?? []).map((r) => ({ ...r, active: r.active && (!r.endsAt || Date.parse(r.endsAt) > now) }));
      return { data: { accountRestrictions: rows } };
    }
    case 'Audit': {
      const since = v.since ? Date.parse(v.since) : null;
      const until = v.until ? Date.parse(v.until) : null;
      if ((v.since && Number.isNaN(since)) || (v.until && Number.isNaN(until))) return err('since must be a time like 2026-11-17T00:00:00Z.', 'bad_time');
      let rows = AUDIT.filter((a) => (!v.targetKind || a.targetKind === v.targetKind) && (!v.targetId || a.targetId === v.targetId)
        && (!v.actorId || a.actorId === v.actorId) && (!v.action || a.action === v.action)
        && (since === null || Date.parse(a.createdAt) >= since) && (until === null || Date.parse(a.createdAt) < until));
      if (v.before) rows = rows.filter((a) => a.id < v.before);
      return { data: { auditLog: rows.slice(0, v.limit ?? 50) } };
    }
    default:
      return null;
  }
}
