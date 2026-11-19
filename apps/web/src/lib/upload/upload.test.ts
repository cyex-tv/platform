import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import UploadForm from './UploadForm.svelte';
import { closeUpload, openUpload, opensDialog, upload } from './upload.svelte';

const click = (o: Partial<MouseEvent> = {}) => ({ button: 0, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, defaultPrevented: false, ...o });
const link = (href: string, attrs: Record<string, string> = {}) => ({ href, target: attrs.target ?? '', hasAttribute: (n: string) => n in attrs });
const at = (path: string) => new URL(`https://cyex.tv${path}`);

describe('the upload dialog', () => {
  it('opens and closes', () => {
    openUpload();
    expect(upload.open).toBe(true);
    closeUpload();
    expect(upload.open).toBe(false);
  });

  it('takes over plain clicks on links to the upload page, from any other page', () => {
    expect(opensDialog(click(), link('/me/upload'), at('/search'))).toBe(true);
    expect(opensDialog(click(), link('https://cyex.tv/upload'), at('/'))).toBe(true);
    expect(opensDialog(click(), link('/me/upload/'), at('/users/1'))).toBe(true);
  });

  it('leaves new tabs, other links and the upload page itself alone', () => {
    for (const o of [{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }, { altKey: true }, { button: 1 }, { defaultPrevented: true }])
      expect(opensDialog(click(o), link('/me/upload'), at('/search')), JSON.stringify(o)).toBe(false);
    expect(opensDialog(click(), link('/me/upload', { target: '_blank' }), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('/me/upload', { download: '' }), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('/me/emotes'), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('https://example.com/me/upload'), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('/me/upload'), at('/me/upload'))).toBe(false);
  });

  it('asks for the guidelines, linked in a new tab, with the formats the API takes', () => {
    const { body } = render(UploadForm, { props: {} });
    expect(body).toMatch(/I accept the <a href="\/guidelines" target="_blank" rel="noopener"[^>]*>emote guidelines<\/a>/);
    expect(body).toContain('type="checkbox"');
    expect(body).toMatch(/accept="[^"]*image\/jpeg[^"]*"/);
    expect(body).not.toContain('image/avif');
  });
});
