import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import AddEmotes from './AddEmotes.svelte';

const { gql, pending } = vi.hoisted(() => ({ gql: vi.fn(), pending: { resolve: null as null | ((v: unknown) => void) } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a) }));

const emote = { id: 'E1', name: 'catJAM', state: 'APPROVED', tags: ['cat'], host: '', files: [], animated: false, owner: { id: 'U', displayName: 'Ana' }, flags: {}, stats: {} };

describe('Add emotes search', () => {
  it('an answer that arrives after the field was emptied is dropped', async () => {
    gql.mockImplementation(() => new Promise((res) => (pending.resolve = res)));
    render(AddEmotes, { setId: 'S', personal: false, taken: new Set<string>(), names: new Set<string>(), full: false, onadded: () => {} });
    const input = document.getElementById('add-find') as HTMLInputElement;
    const form = input.form!;
    await fireEvent.input(input, { target: { value: 'cat' } });
    await fireEvent.submit(form);
    await fireEvent.input(input, { target: { value: '' } });
    await fireEvent.submit(form);
    pending.resolve!({ searchEmotes: { items: [emote], total: 1 } });
    await new Promise((r) => setTimeout(r, 20));
    expect(input.value).toBe('');
    expect(screen.queryAllByRole('button', { name: /^Add / })).toHaveLength(0);
    expect(document.querySelector('.ui-spinner, [aria-busy="true"]')).toBeNull();
  });
});
