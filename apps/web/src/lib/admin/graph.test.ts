import { describe, expect, it } from 'vitest';
import { branches, holderRows, holdersCsv, how, tally, type Grant } from './graph';

const g = (over: Partial<Grant>): Grant => ({
  kind: 'BADGE', itemId: 'B1', itemName: 'Early', accountId: 'A1', source: 'STAFF', active: true, ...over,
});

describe('entitlement graph', () => {
  const grants: Grant[] = [
    g({ source: 'DEFAULT', kind: 'ROLE', itemId: 'RU', itemName: 'User' }),
    g({ source: 'STAFF', grantedBy: 'S1', grantedAt: '2026-11-10T00:00:00Z' }),
    g({ source: 'STAFF', kind: 'ROLE', itemId: 'R1', itemName: 'Mods', grantedBy: 'S1', grantedAt: '2026-11-12T00:00:00Z' }),
    g({ source: 'PURCHASE', kind: 'PAINT', itemId: 'P1', orderId: 'O1', orderName: 'Starter', grantedAt: '2026-11-11T00:00:00Z' }),
    g({ source: 'PURCHASE', kind: 'PAINT', itemId: 'P2', orderId: 'O2', orderName: 'Gift', active: false, revokedAt: '2026-11-13T00:00:00Z' }),
    g({ source: 'CODE', itemId: 'B2', codeBatchId: 'C1', codeBatchName: 'Launch' }),
    g({ source: 'STAFF', itemId: 'B3', grantedBy: 'S2', active: false, revokedAt: '2026-11-14T00:00:00Z', revokedBy: 'S2' }),
  ];

  it('makes one branch per source: each staff member, order and batch apart', () => {
    const b = branches(grants);
    expect(b.map((x) => x.key)).toEqual(['default', 's:S1', 'o:O1', 'c:C1', 's:S2', 'o:O2']);
    const s1 = b.find((x) => x.key === 's:S1')!;
    // Roles before badges inside a branch.
    expect(s1.grants.map((x) => x.kind)).toEqual(['ROLE', 'BADGE']);
    expect(s1.staffId).toBe('S1');
    expect(b.find((x) => x.key === 'o:O1')!.label).toBe('Order: Starter');
    expect(b.find((x) => x.key === 'c:C1')!.label).toBe('Code batch: Launch');
  });

  it('can show only what is held', () => {
    const b = branches(grants, 'source', 'held');
    expect(b.some((x) => x.key === 's:S2' || x.key === 'o:O2')).toBe(false);
  });

  it('groups by holder for one item', () => {
    const b = branches([g({ accountId: 'A1' }), g({ accountId: 'A2', active: false }), g({ accountId: 'A1', source: 'CODE' })], 'account');
    expect(b.map((x) => x.key)).toEqual(['a:A1', 'a:A2']);
    expect(b[0].grants).toHaveLength(2);
    expect(b[0].href).toBe('/admin/users/A1');
  });

  it('counts what is held and what was taken away', () => {
    expect(tally(grants)).toEqual({ roles: 2, badges: 2, paints: 1, gone: 2 });
  });
});

describe('owners lookup', () => {
  const rows: Grant[] = [
    g({ accountId: 'A1', grantedAt: '2026-11-10T00:00:00Z' }),
    g({ accountId: 'A2', source: 'PURCHASE', orderName: 'Gift', grantedAt: '2026-11-12T00:00:00Z' }),
    g({ accountId: 'A3', active: false, grantedAt: '2026-11-01T00:00:00Z', revokedAt: '2026-11-05T00:00:00Z', revokedBy: 'S1' }),
  ];
  const names: Record<string, string> = { A1: 'zed', A2: 'amy', A3: 'Bob' };
  const nameOf = (id: string) => names[id];

  it('shows who holds it now, or everyone who ever did', () => {
    expect(holderRows(rows, { show: 'held', sort: 'given', desc: true }).map((r) => r.accountId)).toEqual(['A2', 'A1']);
    expect(holderRows(rows, { show: 'ever', sort: 'given', desc: true }).map((r) => r.accountId)).toEqual(['A2', 'A1', 'A3']);
  });
  it('sorts by name, how and when it ended', () => {
    expect(holderRows(rows, { show: 'ever', sort: 'account', desc: false, nameOf }).map((r) => r.accountId)).toEqual(['A2', 'A3', 'A1']);
    expect(holderRows(rows, { show: 'ever', sort: 'how', desc: false }).map((r) => r.accountId)).toEqual(['A2', 'A1', 'A3']);
    // Still held sorts after ended ones going up, and first going down.
    expect(holderRows(rows, { show: 'ever', sort: 'ended', desc: false }).map((r) => r.accountId)[0]).toBe('A3');
    expect(holderRows(rows, { show: 'ever', sort: 'ended', desc: true }).map((r) => r.accountId).at(-1)).toBe('A3');
  });
  it('searches names and IDs', () => {
    expect(holderRows(rows, { show: 'ever', sort: 'given', desc: true, search: 'BO', nameOf }).map((r) => r.accountId)).toEqual(['A3']);
    expect(holderRows(rows, { show: 'ever', sort: 'given', desc: true, search: 'a1', nameOf }).map((r) => r.accountId)).toEqual(['A1']);
  });
  it('writes CSV that spreadsheets read safely', () => {
    expect(how(rows[1])).toBe('Bought · Gift');
    const csv = holdersCsv([g({ accountId: 'A9', reason: '=HYPERLINK("x"), ok', grantedAt: '2026-11-10T00:00:00Z' })], () => 'Ann, "the" mod');
    expect(csv.charCodeAt(0)).toBe(0xfeff); // so Excel reads it as UTF-8
    const [head, line] = csv.slice(1).trim().split('\r\n');
    expect(head.split(',')[0]).toBe('account_id');
    expect(line).toBe('A9,"Ann, ""the"" mod",yes,Given by staff,,2026-11-10T00:00:00Z,"\'=HYPERLINK(""x""), ok",,,');
  });
});
