// The Edit emote pop-up and the emote page around it (ADR 0038): owner fields for the owner,
// approvals and transfer only with their permissions, no Favourite, both approval cells in the facts
// grid, and a personal set that stays disabled, with the reason, until personal use is approved.
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import { session } from '$lib/tool/session.svelte';
import EditEmoteDialog from './EditEmoteDialog.svelte';
import EmoteView from './EmoteView.svelte';
import EmoteActions from '../EmoteActions.svelte';
import { Viewer } from './viewer.svelte';
import type { PageEmote } from './emote-page';
import { accountIdFromLink, personalBlock } from './approvals';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/emotes/E1') } }));

const file = (scale: number) => ({ name: `${scale}x.webp`, format: 'WEBP' as const, scale, width: 32 * scale, height: 32 * scale, size: 900, isStatic: false });
const emote = (over: Partial<PageEmote> = {}) =>
  ({
    id: 'E1', name: 'fernYawn', state: 'APPROVED', animated: false, tags: ['cat'], host: 'https://cdn.example/emotes/1',
    createdAt: '2026-11-17T10:00:00.000Z', channelCount: 2, visibility: 'PUBLIC',
    approvals: { publicListing: 'APPROVED', personalUse: 'PENDING' },
    flags: { zeroWidth: false, private: false, wide: false },
    owner: { id: 'A1', displayName: 'nora_byte', avatar: null },
    files: [1, 2, 3, 4].map((s) => file(s)),
    ...over,
  }) as unknown as PageEmote;
const me = (permissions: string[]) => (session.me = { id: 'A1', permissions, identities: [] } as unknown as typeof session.me);

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
  gql.mockReset();
  gql.mockImplementation(() => new Promise(() => {}));
  session.me = null;
});

function openDialog(canEdit: boolean, e = emote()) {
  const r = render(EditEmoteDialog, { emote: e, canEdit });
  (r.component as unknown as { open: () => void }).open();
  return r;
}

describe('the Edit emote pop-up', () => {
  it('shows the owner fields to the owner and nothing staff-only without permissions', () => {
    me([]);
    openDialog(true);
    expect(screen.getByRole('heading', { name: 'Edit emote' })).toBeTruthy();
    expect(screen.getByLabelText('Name')).toBeTruthy();
    expect(screen.getByRole('checkbox', { name: /Zero-width/ })).toBeTruthy();
    expect(screen.getByRole('checkbox', { name: /Private/ })).toBeTruthy();
    expect(screen.queryByText('Approvals')).toBeNull();
    expect(screen.queryByText('Transfer ownership')).toBeNull();
    // Users no longer choose listed or unlisted.
    expect(document.body.textContent).not.toMatch(/unlisted/i);
  });

  it('shows only the approval parts the reviewer holds, with their current state', () => {
    me(['emotes.review']);
    openDialog(false);
    expect(screen.queryByLabelText('Name')).toBeNull();
    const listing = document.querySelector('[data-part="publicListing"]') as HTMLElement;
    expect(within(listing).getByText('Approved')).toBeTruthy();
    expect(document.querySelector('[data-part="personalUse"]')).toBeNull();
    expect(screen.queryByText('Transfer ownership')).toBeNull();
  });

  it('approves with onlyPending false, and a denial needs a reason', async () => {
    me(['emotes.review', 'emotes.review_personal']);
    gql.mockResolvedValue({ reviewApprovals: { done: ['E1'], failed: [] } });
    openDialog(false);
    const personal = document.querySelector('[data-part="personalUse"]') as HTMLElement;
    await fireEvent.click(within(personal).getByRole('button', { name: 'Deny' }));
    const confirm = within(personal).getByRole('button', { name: 'Confirm deny' }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    await fireEvent.input(within(personal).getByRole('textbox'), { target: { value: 'Too noisy.' } });
    await fireEvent.click(confirm);
    expect(gql).toHaveBeenCalledOnce();
    expect(gql.mock.calls[0][1]).toEqual({ ids: ['E1'], input: { personalUse: 'DENY', reason: 'Too noisy.', onlyPending: false } });
  });

  it('shows the API message when a decision fails', async () => {
    me(['emotes.review']);
    gql.mockRejectedValue(new ApiError('You can’t review this.', 'forbidden'));
    openDialog(false);
    const listing = document.querySelector('[data-part="publicListing"]') as HTMLElement;
    await fireEvent.click(within(listing).getByRole('button', { name: 'Deny' }));
    await fireEvent.input(within(listing).getByRole('textbox'), { target: { value: 'No.' } });
    await fireEvent.click(within(listing).getByRole('button', { name: 'Confirm deny' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'You can’t review this.');
  });

  it('offers Transfer ownership only with emotes.transfer, resolving a pasted profile link', async () => {
    me(['emotes.transfer']);
    gql.mockResolvedValueOnce({ account: { id: 'A9', displayName: 'Ana' } });
    openDialog(false);
    expect(screen.getByText('Transfer ownership')).toBeTruthy();
    await fireEvent.input(screen.getByLabelText('Find an account by name or profile link'), { target: { value: 'https://cyex.tv/users/A9' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Search' }));
    await fireEvent.click(await screen.findByRole('button', { name: 'Ana' }));
    gql.mockResolvedValueOnce({ transferEmote: {} });
    await fireEvent.click(screen.getByRole('button', { name: 'Transfer to Ana' }));
    await fireEvent.click(screen.getByRole('button', { name: /Confirm: give to Ana/ }));
    expect(gql.mock.calls.at(-1)?.[1]).toEqual({ id: 'E1', toAccountId: 'A9' });
  });
});

describe('the emote page', () => {
  const view = (e = emote()) => render(EmoteView, { emote: e, more: [], channels: null, channelCount: 2, back: { href: '/search', label: 'All emotes', back: false } });

  it('has no Favourite and opens the pop-up from Edit emote', () => {
    me(['emotes.edit_any']);
    view();
    expect(screen.queryByRole('button', { name: /favourite/i })).toBeNull();
    expect(screen.getAllByRole('button', { name: /Edit emote/ }).length).toBeGreaterThan(0);
    expect(document.querySelector('a[href^="/me/emotes/"]')).toBeNull();
  });

  it('shows Public listing and Personal use as two cells of the facts grid', () => {
    view();
    const cells = [...document.querySelectorAll('.facts > div')].map((d) => d.textContent?.replace(/\s+/g, ' ').trim());
    expect(cells).toContain('Public listingApproved');
    expect(cells).toContain('Personal usePending');
    expect(cells.some((c) => c?.startsWith('Review'))).toBe(false);
  });

  it('has no staff flags panel', () => {
    me(['emotes.flags', 'emotes.edit_any']);
    view();
    expect(screen.queryByText('Emote flags')).toBeNull();
  });
});

describe('adding to a personal set', () => {
  async function actions(approvals: { personalUse: 'PENDING' | 'DENIED' | 'APPROVED'; publicListing: 'APPROVED' }) {
    const viewer = new Viewer('E1');
    viewer.who = 'in';
    viewer.sets = [
      { id: 'S1', name: 'Mine', kind: 'PERSONAL', emotes: [] },
      { id: 'S2', name: 'Channel', kind: 'CHANNEL', emotes: [] },
    ] as never;
    render(EmoteActions, { emoteId: 'E1', name: 'fernYawn', viewer, approvals });
    await fireEvent.click(screen.getByRole('button', { name: /Add to set/ }));
    return viewer;
  }

  it('is disabled with the reason while pending, and for a denial', async () => {
    const v = await actions({ publicListing: 'APPROVED', personalUse: 'PENDING' });
    const personal = screen.getByRole('button', { name: /Mine/ });
    expect(personal.getAttribute('aria-disabled')).toBe('true');
    expect(personal.textContent).toContain('Not approved for personal use yet');
    await fireEvent.click(personal);
    expect(gql.mock.calls.some((c) => (c[1] as { setId?: string })?.setId)).toBe(false);
    expect(screen.getByRole('button', { name: /Channel/ }).getAttribute('aria-disabled')).toBe('false');
    expect(v.adding).toBeNull();
  });

  it('says "Not approved for personal use" when denied, and is open once approved', async () => {
    expect(personalBlock({ personalUse: 'DENIED' })).toBe('Not approved for personal use');
    expect(personalBlock({ personalUse: 'APPROVED' })).toBeNull();
    await actions({ publicListing: 'APPROVED', personalUse: 'APPROVED' });
    expect(screen.getByRole('button', { name: /Mine/ }).getAttribute('aria-disabled')).toBe('false');
  });

  it('maps personal_use_not_approved to its message', async () => {
    const viewer = new Viewer('E1');
    viewer.sets = [{ id: 'S1', name: 'Mine', kind: 'PERSONAL', emotes: [] }] as never;
    gql.mockRejectedValue(new ApiError('409', 'personal_use_not_approved'));
    await viewer.add('S1');
    expect(viewer.addError).toBe('This emote isn’t approved for personal use yet.');
  });
});

describe('profile links', () => {
  it('finds the account id in a pasted link, with or without the origin', () => {
    expect(accountIdFromLink('https://cyex.tv/users/A9')).toBe('A9');
    expect(accountIdFromLink('/users/A9?tab=x')).toBe('A9');
    expect(accountIdFromLink('nora')).toBeNull();
  });
});
