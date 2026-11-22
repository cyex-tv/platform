import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls: { op: string; vars: any }[] = [];
const role = (o: Record<string, unknown>) => ({ description: null, position: 10, permissions: [], denies: [], public: false, invisible: false, builtIn: null, editable: true, assignable: true, ...o });
const ROLES = [
  role({ id: 'R1', name: 'Moderator', position: 50, permissions: ['emotes.review'] }),
  role({ id: 'RU', name: 'User', position: 0, builtIn: 'USER', permissions: ['emotes.upload'] }),
];

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), beforeNavigate: vi.fn(), afterNavigate: vi.fn(), invalidateAll: vi.fn() }));
const state = vi.hoisted(() => ({ url: new URL('http://localhost/admin/roles') }));
vi.mock('$app/state', () => ({ page: { get url() { return state.url; }, params: {}, route: { id: '/' } } }));
vi.mock('$lib/api', async (orig) => ({
  ...(await orig<typeof import('$lib/api')>()),
  onSignedOut: () => () => {},
  gql: vi.fn(async (doc: unknown, vars: any) => {
    const op = String(doc).match(/(?:query|mutation)\s+(\w+)/)?.[1] ?? '';
    calls.push({ op, vars });
    switch (op) {
      case 'Roles': return { roles: ROLES, permissions: ['emotes.upload', 'emotes.review', 'users.block'] };
      case 'AccountRoles': return { accountRoles: [] };
      case 'RoleLimits': case 'LimitDefaults': return { roleLimits: [], limitDefaults: [] };
      case 'UpdateRole': return { updateRole: { id: vars.id } };
      case 'CreateRole': return { createRole: { id: 'NEW' } };
      default: return { entitlementHolders: [] };
    }
  }),
}));
vi.mock('$lib/tool/session.svelte', () => ({
  session: { me: { id: 'ME', permissions: ['emotes.upload', 'emotes.review', 'users.block', 'roles.manage'] } },
  can: (k: string) => ['emotes.upload', 'emotes.review', 'users.block', 'roles.manage'].includes(k),
}));

import Page from './+page.svelte';

const sent = (op: string) => calls.filter((c) => c.op === op).at(-1)?.vars;

beforeEach(() => {
  calls.length = 0;
  state.url = new URL('http://localhost/admin/roles');
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute('open', ''); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute('open'); };
});

describe('the role editor sends allows, denies and invisible', () => {
  it('Deny puts the permission in denies and not in permissions', async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findByRole('list', { name: 'Roles' });
    await user.click(screen.getByRole('button', { name: /New role/ }));
    const dialog = await screen.findByRole('dialog', { name: 'New role' });
    await user.type(within(dialog).getByLabelText('Name'), 'Muted');
    await user.click(within(within(dialog).getByRole('radiogroup', { name: 'Upload emotes' })).getByRole('radio', { name: 'Deny' }));
    await user.click(within(within(dialog).getByRole('radiogroup', { name: 'Review uploads' })).getByRole('radio', { name: 'Allow' }));
    expect(within(dialog).getByText(/1 allowed · 1 denied/)).toBeTruthy();
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(sent('CreateRole')).toBeTruthy());
    const input = sent('CreateRole').input;
    expect(input.denies).toEqual(['emotes.upload']);
    expect(input.permissions).toEqual(['emotes.review']);
    expect(input.invisible).toBe(false);
  });

  it('the Invisible switch sends invisible: true and turns show-on-profile off', async () => {
    const user = userEvent.setup();
    render(Page);
    await screen.findByRole('list', { name: 'Roles' });
    await user.click(screen.getByRole('button', { name: /New role/ }));
    const dialog = await screen.findByRole('dialog', { name: 'New role' });
    await user.type(within(dialog).getByLabelText('Name'), 'Quiet');
    const shown = within(dialog).getByRole('checkbox', { name: /Shown on profiles/ }) as HTMLInputElement;
    await user.click(shown);
    expect(shown.disabled).toBe(false);
    await user.click(within(dialog).getByRole('switch', { name: /Invisible/ }));
    expect(shown.disabled).toBe(true);
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(sent('CreateRole')).toBeTruthy());
    expect(sent('CreateRole').input).toMatchObject({ invisible: true, public: false });
  });

  it('editing a role sends both lists, moving an allow to a deny', async () => {
    const user = userEvent.setup();
    state.url = new URL('http://localhost/admin/roles?role=R1');
    render(Page);
    await user.click(await screen.findByRole('button', { name: 'Edit' }));
    const dialog = await screen.findByRole('dialog', { name: /Edit .*Moderator/ });
    const review = within(dialog).getByRole('radiogroup', { name: 'Review uploads' });
    expect(within(review).getByRole('radio', { name: 'Allow' }).getAttribute('aria-checked')).toBe('true');
    await user.click(within(review).getByRole('radio', { name: 'Deny' }));
    await user.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(sent('UpdateRole')).toBeTruthy());
    expect(sent('UpdateRole').input).toMatchObject({ permissions: [], denies: ['emotes.review'], invisible: false });
  });

  it('the User role offers no Deny', async () => {
    const user = userEvent.setup();
    state.url = new URL('http://localhost/admin/roles?role=RU');
    render(Page);
    await user.click(await screen.findByRole('button', { name: 'Edit' }));
    const dialog = await screen.findByRole('dialog', { name: /Edit .*User/ });
    expect(within(dialog).queryByRole('radio', { name: 'Deny' })).toBeNull();
    expect(within(dialog).getByText(/Deny isn’t offered here/)).toBeTruthy();
  });
});
