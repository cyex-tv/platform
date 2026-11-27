// The emote page's More menu: no Copy name; Download asks for the format (GIF, WebP or AVIF as the
// emote's files allow), the type and the size, and saves `name_2x.ext`. A GIF is made in the browser.
import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EmoteActions from '../EmoteActions.svelte';
import type { PageEmote } from './emote-page';

const m = vi.hoisted(() => ({ save: vi.fn(), saveBlob: vi.fn(), makeGif: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: () => new Promise(() => {}), onSignedOut: () => () => {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn(), afterNavigate: vi.fn(), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/emotes/E1') } }));
vi.mock('./download', async (orig) => ({ ...(await orig<typeof import('./download')>()), save: m.save, saveBlob: m.saveBlob, makeGif: m.makeGif }));

const file = (scale: number, format: 'WEBP' | 'AVIF', isStatic = false) => ({
  name: `${scale}x${isStatic ? '_static' : ''}.${format.toLowerCase()}`, format, scale, width: 32 * scale, height: 32 * scale, size: 900 * scale, isStatic,
});
const stillFiles = [1, 2].flatMap((s) => [file(s, 'WEBP'), file(s, 'AVIF')]);
const movingFiles = [1, 2].flatMap((s) => [file(s, 'WEBP'), file(s, 'WEBP', true), file(s, 'AVIF', true)]);
const picture = (animated: boolean) => ({ host: 'https://cdn.example/emotes/1', animated, files: (animated ? movingFiles : stillFiles) as PageEmote['files'] });

async function openDownload(animated: boolean) {
  const user = userEvent.setup();
  render(EmoteActions, { emoteId: 'E1', name: 'fern yawn', picture: picture(animated), addable: false, reportable: false });
  await user.click(screen.getByRole('button', { name: /more/i }));
  return user;
}
const go = () => document.querySelector<HTMLButtonElement>('button.go')!;
const formats = () => [...document.querySelectorAll('.dl fieldset')].find((f) => f.querySelector('legend')?.textContent === 'Format')?.querySelectorAll('.t');

beforeEach(() => {
  m.save.mockReset().mockResolvedValue(true);
  m.saveBlob.mockReset();
  m.makeGif.mockReset();
});

describe('More menu and Download', () => {
  it('has no Copy name, and still has Copy link and Download', async () => {
    await openDownload(false);
    expect(screen.queryByRole('button', { name: /copy name/i })).toBeNull();
    expect(screen.getByRole('button', { name: /copy link/i })).toBeTruthy();
    expect(screen.getByRole('button', { name: /download/i })).toBeTruthy();
  });

  it('a still emote: Format GIF, WebP, AVIF, no Type; defaults to WebP at the largest size', async () => {
    const user = await openDownload(false);
    await user.click(screen.getByRole('button', { name: /download/i }));
    expect([...formats()!].map((e) => e.textContent)).toEqual(['GIF', 'WebP', 'AVIF']);
    expect(screen.queryByText('Type')).toBeNull();
    expect(go().textContent).toMatch(/2 KB/);
    await user.click(go());
    expect(m.save).toHaveBeenCalledWith('https://cdn.example/emotes/1/2x.webp', 'fern_yawn_2x.webp');
  });

  it('an animated emote: AVIF only for the still frame; names the still file', async () => {
    const user = await openDownload(true);
    await user.click(screen.getByRole('button', { name: /download/i }));
    expect([...formats()!].map((e) => e.textContent)).toEqual(['GIF', 'WebP']);
    await user.click(screen.getByRole('radio', { name: 'Still frame' }));
    expect([...formats()!].map((e) => e.textContent)).toEqual(['GIF', 'WebP', 'AVIF']);
    await user.click(screen.getByRole('radio', { name: 'AVIF' }));
    await user.click(screen.getByRole('radio', { name: /^1x/ }));
    await user.click(go());
    expect(m.save).toHaveBeenCalledWith('https://cdn.example/emotes/1/1x_static.avif', 'fern_yawn_1x_still.avif');
  });

  it('GIF: says it is made here, makes it from the WebP and saves name_2x.gif', async () => {
    m.makeGif.mockResolvedValue(new Blob(['GIF89a'], { type: 'image/gif' }));
    const user = await openDownload(true);
    await user.click(screen.getByRole('button', { name: /download/i }));
    await user.click(screen.getByRole('radio', { name: 'GIF' }));
    expect(screen.getByText(/made in your browser/i)).toBeTruthy();
    await user.click(go());
    await waitFor(() => expect(m.saveBlob).toHaveBeenCalled());
    expect(m.makeGif.mock.calls[0][0]).toBe('https://cdn.example/emotes/1/2x.webp');
    expect(m.saveBlob.mock.calls[0][1]).toBe('fern_yawn_2x.gif');
    expect(m.save).not.toHaveBeenCalled();
  });

  it('GIF that fails: an inline error that points to WebP, and the menu stays open', async () => {
    m.makeGif.mockRejectedValue(new Error('cors'));
    const user = await openDownload(true);
    await user.click(screen.getByRole('button', { name: /download/i }));
    await user.click(screen.getByRole('radio', { name: 'GIF' }));
    await user.click(go());
    expect((await screen.findByRole('alert')).textContent).toMatch(/try webp/i);
    expect(m.saveBlob).not.toHaveBeenCalled();
    expect(go()).toBeTruthy();
  });
});
