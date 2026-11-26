import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { EmoteFieldsFragment } from '$lib/gql/graphql';
import ReviewInspector from './ReviewInspector.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

const file = (scale: number) => ({ name: `${scale}x.webp`, format: 'WEBP', isStatic: false, scale, width: 32 * scale, height: 32 * scale, size: 1000 * scale });
const emote = (id: string, name: string, state = 'PENDING') =>
  ({
    id, name, state, visibility: 'PUBLIC', animated: false, tags: ['meme'], createdAt: '2026-11-21T10:00:00Z',
    flags: { zeroWidth: false, wide: false }, owner: { id: 'OWN', displayName: 'byteNod' },
    files: [1, 2, 3, 4].map(file),
  }) as unknown as EmoteFieldsFragment;

function answer() {
  gql.mockImplementation(async (_doc: unknown, vars: Record<string, unknown>) => {
    if ('accountId' in vars) return { accountEmotes: { total: 4, items: [emote('E1', 'pupHmm'), emote('E2', 'oldOne', 'APPROVED'), emote('E3', 'nope', 'REJECTED'), emote('E4', 'fine', 'APPROVED')] } };
    return { emoteHistory: [], emoteDuplicates: [], emoteNotes: [] };
  });
}

const base = { place: 3, total: 41, busy: false, resolved: false, hasNext: true, onskip: () => {}, onreject: () => {}, onapprove: () => {} };

describe('ReviewInspector', () => {
  it('shows the upload at every size, its place and the uploader’s history', async () => {
    answer();
    render(ReviewInspector, { ...base, emote: emote('E1', 'pupHmm') });
    expect(screen.getByRole('heading', { name: 'pupHmm' })).toBeTruthy();
    expect(screen.getByText('3 / 41')).toBeTruthy();
    expect(screen.getAllByRole('figure')).toHaveLength(4);
    expect(screen.queryByText('NSFW')).toBeNull();
    // Earlier uploads leave this one out: two approved, one rejected.
    await screen.findByText('oldOne');
    expect(screen.getByText('approved').parentElement?.textContent).toContain('2');
    expect(screen.getByText('rejected').parentElement?.textContent).toContain('1');
  });

  it('sends the verdicts from its foot', async () => {
    answer();
    const onapprove = vi.fn();
    const onreject = vi.fn();
    const onskip = vi.fn();
    render(ReviewInspector, { ...base, emote: emote('E1', 'pupHmm'), onapprove, onreject, onskip });
    await fireEvent.click(screen.getByRole('button', { name: 'Approve' }));
    expect(onapprove).toHaveBeenCalledWith();
    await fireEvent.click(screen.getByRole('button', { name: 'Approve as unlisted' }));
    expect(onapprove).toHaveBeenLastCalledWith('UNLISTED');
    await fireEvent.click(screen.getByRole('button', { name: 'Reject' }));
    expect(onreject).toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Skip to the next, no verdict' }));
    expect(onskip).toHaveBeenCalled();
  });

  it('has no verdicts on resolved uploads', () => {
    answer();
    render(ReviewInspector, { ...base, resolved: true, emote: emote('E1', 'pupHmm', 'APPROVED') });
    expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull();
  });
});
