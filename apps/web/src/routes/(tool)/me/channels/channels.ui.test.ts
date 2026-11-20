import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Page from './+page.svelte';

const { gql } = vi.hoisted(() => ({ gql: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ beforeNavigate: () => {}, goto: vi.fn(), afterNavigate: () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/me/channels') } }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));

const channel = { platform: 'TWITCH', platformChannelId: 'c1', login: 'ana', activeSetId: null, allowPersonalEmotes: false, account: { id: 'A', displayName: 'Ana' } };

describe('channel controls while a save is on its way', () => {
  it('stay focusable (aria-disabled, not disabled) so keyboard focus survives the save', async () => {
    let finish!: (v: unknown) => void;
    gql.mockImplementation((_q: unknown, vars: Record<string, unknown>) =>
      vars && 'input' in vars ? new Promise((r) => (finish = r)) : Promise.resolve({ myChannels: [channel], mySets: [], editorOf: [], editorChannels: [], editorSets: [] }),
    );
    render(Page);
    const sw = (await screen.findByRole('checkbox')) as HTMLInputElement;
    sw.focus();
    await fireEvent.click(sw);
    await vi.waitFor(() => expect(sw.getAttribute('aria-disabled')).toBe('true'));
    expect(sw.disabled).toBe(false);
    expect(document.activeElement).toBe(sw);
    finish({ updateChannelSettings: { activeSetId: null, allowPersonalEmotes: true } });
    await vi.waitFor(() => expect(sw.getAttribute('aria-disabled')).toBe('false'));
    expect(document.activeElement).toBe(sw);
  });
});
