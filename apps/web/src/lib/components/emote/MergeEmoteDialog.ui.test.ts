// "Merge into another emote" (ADR 0040): the More menu item only with emotes.merge; a pasted link
// resolves to the ID; the confirm screen shows both emotes, the sets that already had the target and
// the "merged already" note; confirming sends mergeEmote and goes to the target; a refusal shows
// its words in the dialog.
import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import { session } from '$lib/tool/session.svelte';
import EmoteActions from '../EmoteActions.svelte';
import MergeEmoteDialog from './MergeEmoteDialog.svelte';

const m = vi.hoisted(() => ({ gql: vi.fn(), goto: vi.fn(), notify: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => m.gql(...a), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: (...a: unknown[]) => m.goto(...a), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/emotes/111') } }));
vi.mock('$lib/tool/notices.svelte', () => ({ notify: (...a: unknown[]) => m.notify(...a) }));

const file = (scale: number) => ({ name: `${scale}x.webp`, format: 'WEBP', scale, width: 32 * scale, height: 32 * scale, size: 900, isStatic: false });
const emote = (id: string, name: string, owner: string, channelCount: number) => ({
  id, name, state: 'APPROVED', animated: false, tags: [], host: `https://cdn.example/emotes/${id}`, createdAt: '2026-11-17T10:00:00.000Z', channelCount,
  visibility: 'PUBLIC', approvals: { publicListing: 'APPROVED', personalUse: 'APPROVED' }, flags: { zeroWidth: false, private: false, wide: false },
  owner: { id: `A-${owner}`, displayName: owner, avatar: null }, files: [1, 2, 3, 4].map(file),
});
const preview = (over: Record<string, unknown> = {}) => ({
  emoteMergePreview: {
    source: emote('111', 'Pogg', 'nora_byte', 3),
    target: emote('222', 'Pog', 'quietoak', 1200),
    followed: false,
    moves: 12,
    collapses: [{ setId: 'X1', setName: 'Subathon', keptName: 'Pog', droppedName: 'Pogg' }],
    ...over,
  },
});

const me = (permissions: string[]) => (session.me = { id: 'A1', permissions, identities: [] } as unknown as typeof session.me);

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
  m.gql.mockReset();
  m.goto.mockReset();
  m.notify.mockReset();
  m.gql.mockImplementation(() => new Promise(() => {}));
  session.me = null;
});

function openDialog() {
  const r = render(MergeEmoteDialog, { emoteId: '111', name: 'Pogg' });
  (r.component as unknown as { open: () => void }).open();
}
async function toConfirm(over: Record<string, unknown> = {}, text = 'https://cyex.tv/emotes/222?tab=activity') {
  m.gql.mockResolvedValueOnce(preview(over));
  openDialog();
  await fireEvent.input(screen.getByLabelText('Link or ID of the emote to merge into'), { target: { value: text } });
  await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
  await screen.findByRole('button', { name: /^Merge into / });
}

describe('the More menu item', () => {
  const menu = async () => {
    const user = userEvent.setup();
    render(EmoteActions, { emoteId: '111', name: 'Pogg', addable: false, reportable: false });
    await user.click(screen.getByRole('button', { name: /more/i }));
  };
  it('is there only with emotes.merge', async () => {
    me(['emotes.merge']);
    await menu();
    expect(screen.getByRole('button', { name: /Merge into another emote/ })).toBeTruthy();
  });
  it('is missing without it', async () => {
    me(['emotes.edit_any']);
    await menu();
    expect(screen.queryByRole('button', { name: /Merge into another emote/ })).toBeNull();
  });
});

describe('the merge pop-up', () => {
  it('resolves a pasted link and a bare ID to the emote asked about', async () => {
    await toConfirm();
    expect(m.gql.mock.calls[0][1]).toEqual({ id: '111', intoId: '222' });
  });

  it('says what a text that is no emote means without asking the API', async () => {
    openDialog();
    await fireEvent.input(screen.getByLabelText('Link or ID of the emote to merge into'), { target: { value: 'https://cyex.tv/users/A9' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect((await screen.findByRole('alert')).textContent).toMatch(/isn’t an emote link or ID/);
    expect(m.gql).not.toHaveBeenCalled();
  });

  it('shows both emotes, what moves and the sets that already had the target', async () => {
    await toConfirm();
    const from = screen.getByRole('region', { name: 'Merge this: Pogg' });
    const to = screen.getByRole('region', { name: 'Into this: Pog' });
    expect(from.textContent).toContain('nora_byte');
    expect(from.textContent).toContain('3 channels');
    expect(to.textContent).toContain('quietoak');
    expect(to.textContent).toContain('1.200 channels');
    expect(from.querySelectorAll('img')).toHaveLength(2);
    const text = document.querySelector('.what')!.textContent!.replace(/\s+/g, ' ');
    expect(text).toContain('12 set entries move to Pog');
    expect(text).toContain('1 set already has Pog');
    expect(text).toContain('Subathon: keeps Pog, drops Pogg');
    expect(text).toContain('can’t be undone');
    expect(screen.queryByText(/was merged into/)).toBeNull();
  });

  it('says when the emote asked for was merged already', async () => {
    await toConfirm({ followed: true });
    expect(screen.getByRole('status').textContent).toBe('That emote was merged into Pog already; this goes to Pog.');
  });

  it('merges, goes to the target and says what moved', async () => {
    await toConfirm();
    m.gql.mockResolvedValueOnce({ mergeEmote: { sourceId: '111', target: emote('222', 'Pog', 'quietoak', 1203), moved: 12, collapsed: [{ setId: 'X1', setName: 'Subathon', keptName: 'Pog', droppedName: 'Pogg' }] } });
    await fireEvent.click(screen.getByRole('button', { name: 'Merge into Pog' }));
    await vi.waitFor(() => expect(m.goto).toHaveBeenCalledWith('/emotes/222'));
    expect(m.gql.mock.calls.at(-1)?.[1]).toEqual({ id: '111', intoId: '222' });
    expect(String(m.notify.mock.calls[0][0]).replace(/[⁨⁩]/g, '')).toBe('Merged Pogg into Pog: 12 sets moved, 1 already had it.');
  });

  it('shows a refusal in plain words, in the first step and in the second', async () => {
    openDialog();
    m.gql.mockRejectedValueOnce(new ApiError('Not found.', 'not_found'));
    await fireEvent.input(screen.getByLabelText('Link or ID of the emote to merge into'), { target: { value: '12345' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    expect((await screen.findByRole('alert')).textContent).toBe('No emote with that link or ID.');

    m.gql.mockResolvedValueOnce(preview());
    await fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await screen.findByRole('button', { name: 'Merge into Pog' });
    m.gql.mockRejectedValueOnce(new ApiError('x', 'already_merged'));
    await fireEvent.click(screen.getByRole('button', { name: 'Merge into Pog' }));
    expect((await screen.findByRole('alert')).textContent).toMatch(/merged into another already/);
    expect(m.goto).not.toHaveBeenCalled();
  });
});
