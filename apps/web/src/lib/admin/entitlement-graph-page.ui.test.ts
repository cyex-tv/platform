// Admin → Entitlement graph: the picked node comes from the address (?node=) and shows in the
// inspector; the legend hides a kind; "Find in graph" lists matches and Enter picks one; the
// List view shows the same grants as rows.
import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import type { Grant } from './graph';
import Page from '../../routes/(tool)/admin/entitlements/+page.svelte';

const { gql, goto, url } = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), url: { current: new URL('http://x/admin/entitlements?account=A1') } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/state', () => ({ page: { get url() { return url.current; } } }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => goto(...a), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$lib/tool/session.svelte', async (orig) => ({ ...(await orig<typeof import('$lib/tool/session.svelte')>()), can: () => true }));

const g = (over: Partial<Grant>): Grant => ({
  kind: 'PAINT', itemId: 'P1', itemName: 'Tide', accountId: 'A1', source: 'STAFF', reason: null, grantedBy: null, grantedAt: null, orderId: null, orderName: null,
  orderStatus: null, codeBatchId: null, codeBatchName: null, revokedAt: null, revokedBy: null, revokeReason: null, active: true, ...over,
});
const GRANTS: Grant[] = [
  g({ source: 'DEFAULT', kind: 'ROLE', itemId: 'RU', itemName: 'User' }),
  g({ source: 'PURCHASE', itemId: 'M1', itemName: 'Frost jan', orderId: 'O1', orderName: 'Monthly', orderStatus: 'paid', grantedAt: '2026-01-01T09:00:00Z' }),
  g({ source: 'PURCHASE', itemId: 'M2', itemName: 'Frost feb', orderId: 'O2', orderName: 'Monthly', orderStatus: 'paid', grantedAt: '2026-02-01T09:00:00Z' }),
  g({ source: 'CODE', kind: 'BADGE', itemId: 'B1', itemName: 'Snow', codeBatchId: 'K1', codeBatchName: 'Winter drop', grantedAt: '2026-01-08T12:00:00Z' }),
];

beforeAll(() => {
  // jsdom draws nothing; the graph copes with no 2D context.
  HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext;
});

function answer() {
  gql.mockImplementation(async (doc: unknown) => {
    const q = String(doc);
    if (q.includes('entitlementGraph')) return { entitlementGraph: GRANTS };
    if (q.includes('entitlementHolders')) return { entitlementHolders: [] };
    if (q.includes('cosmetics')) return { cosmetics: { badges: [], paints: [{ id: 'X1', name: 'Frosted glass' }] } };
    if (q.includes('roles')) return { roles: [] };
    if (q.includes('account(')) return { account: { id: 'A1', displayName: 'nora_byte', avatar: null } };
    return {};
  });
}

afterEach(() => {
  cleanup();
  goto.mockReset();
  url.current = new URL('http://x/admin/entitlements?account=A1');
});

describe('the entitlement graph page', () => {
  it('draws the graph with its counts, and shows the node the address picks in the inspector', async () => {
    url.current = new URL('http://x/admin/entitlements?account=A1&node=paint%3AM1');
    answer();
    render(Page);
    const graph = await screen.findByRole('application', { name: /Entitlement graph of nora_byte/ });
    expect(graph.getAttribute('aria-roledescription')).toBe('graph');
    const desc = document.getElementById(graph.getAttribute('aria-describedby')!)!;
    // Account, product, two orders, code batch, built-in, role, badge, two paints.
    expect(desc.textContent).toContain('10 nodes and 9 links.');
    const panel = await screen.findByRole('complementary', { name: 'Paint' });
    expect(within(panel).getByRole('heading', { name: 'Frost jan' })).toBeTruthy();
    expect(within(panel).getByText('Bought')).toBeTruthy();
    expect(within(panel).getByRole('link', { name: /Who holds it/ }).getAttribute('href')).toBe('/admin/entitlements?paint=M1');
  });

  it('hides a kind from the legend, and drops a pick of that kind', async () => {
    url.current = new URL('http://x/admin/entitlements?account=A1&node=paint%3AM1');
    answer();
    const user = userEvent.setup();
    render(Page);
    const legend = await screen.findByRole('list', { name: /Kinds/ });
    const paint = within(legend).getByRole('button', { name: /Paint\s*2/ });
    expect(paint.getAttribute('aria-pressed')).toBe('true');
    await user.click(paint);
    expect(paint.getAttribute('aria-pressed')).toBe('false');
    const graph = screen.getByRole('application');
    expect(document.getElementById(graph.getAttribute('aria-describedby')!)!.textContent).toContain('2 paint (hidden)');
    expect(goto).toHaveBeenLastCalledWith('?account=A1', expect.objectContaining({ replaceState: true }));
  });

  it('finds nodes in the graph and items in the catalogue; Enter picks the first match', async () => {
    answer();
    const user = userEvent.setup();
    render(Page);
    await screen.findByRole('application');
    const find = screen.getByRole('combobox', { name: /Find in graph/ });
    await user.type(find, 'frost');
    const list = await screen.findByRole('listbox', { name: 'Matches' });
    const opts = within(list).getAllByRole('option').map((o) => o.textContent?.trim());
    expect(opts).toEqual(['Paint: Frost feb', 'Paint: Frost jan', 'Paint: Frosted glass']);
    expect(screen.getByText('2 matches')).toBeTruthy();
    await user.keyboard('{Enter}');
    expect(goto).toHaveBeenLastCalledWith('?account=A1&node=paint%3AM2', expect.anything());
    // The catalogue's paint opens its holders.
    await user.clear(find);
    await user.type(find, 'frosted');
    await user.keyboard('{Enter}');
    expect(goto).toHaveBeenLastCalledWith('?paint=X1');
  });

  it('has a List view with the same grants as rows; a row picks its node', async () => {
    answer();
    const user = userEvent.setup();
    render(Page);
    await screen.findByRole('application');
    await user.click(screen.getByRole('radio', { name: /List/ }));
    expect(screen.queryByRole('application')).toBeNull();
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows).toHaveLength(4);
    await user.click(within(rows[1]).getByText('Frost jan'));
    expect(goto).toHaveBeenLastCalledWith('?account=A1&node=paint%3AM1', expect.anything());
  });
});
