import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import UploadForm from './UploadForm.svelte';
import { closeUpload, openUpload, opensDialog, upload, uploadFor, uploadParam, uploadRedirect, withoutUploadParam } from './upload.svelte';

const click = (o: Partial<MouseEvent> = {}) => ({ button: 0, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, defaultPrevented: false, ...o });
const link = (href: string, attrs: Record<string, string> = {}) => ({ href, target: attrs.target ?? '', hasAttribute: (n: string) => n in attrs });
const at = (path: string) => new URL(`https://cyex.tv${path}`);

describe('the upload dialog', () => {
  it('opens and closes, for you or for an account you edit', () => {
    openUpload();
    expect(upload).toEqual({ open: true, for: null });
    closeUpload();
    expect(upload.open).toBe(false);
    openUpload('A9');
    expect(upload).toEqual({ open: true, for: 'A9' });
    closeUpload();
  });

  it('takes over plain clicks on links to the upload page, from any other page', () => {
    expect(opensDialog(click(), link('/me/upload'), at('/search'))).toBe(true);
    expect(opensDialog(click(), link('https://cyex.tv/upload'), at('/'))).toBe(true);
    expect(opensDialog(click(), link('/me/upload/'), at('/users/1'))).toBe(true);
  });

  it('leaves new tabs and other links alone', () => {
    for (const o of [{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }, { altKey: true }, { button: 1 }, { defaultPrevented: true }])
      expect(opensDialog(click(o), link('/me/upload'), at('/search')), JSON.stringify(o)).toBe(false);
    expect(opensDialog(click(), link('/me/upload', { target: '_blank' }), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('/me/upload', { download: '' }), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('/me/emotes'), at('/search'))).toBe(false);
    expect(opensDialog(click(), link('https://example.com/me/upload'), at('/search'))).toBe(false);
    expect(uploadFor('/me/upload?for=A9', at('/me'))).toBe('A9');
    expect(uploadFor('/me/upload', at('/me'))).toBeNull();
  });

  it('asks for the rules and guidelines, linked in a new tab, with the formats the API takes', () => {
    const { body } = render(UploadForm, { props: {} });
    expect(body).toMatch(/I accept the <a href="\/guidelines" target="_blank" rel="noopener"[^>]*>rules and guidelines<\/a>/);
    expect(body).toContain('type="checkbox"');
    expect(body).toMatch(/accept="[^"]*image\/jpeg[^"]*"/);
    expect(body).not.toContain('image/avif');
  });
});

describe('no upload page (owner, 2026/11/22)', () => {
  const u = (s: string) => new URL(`https://cyex.tv${s}`);
  it('/me/upload and /upload send you back where you came from, with ?upload to open the dialog', () => {
    expect(uploadRedirect(u('/me/upload'), 'https://cyex.tv/search?q=cat')).toBe('/search?q=cat&upload=1');
    expect(uploadRedirect(u('/upload?for=A9'), 'https://cyex.tv/users/A9')).toBe('/users/A9?upload=A9');
  });
  it('goes home without a referer, from another site, or from an upload address', () => {
    expect(uploadRedirect(u('/me/upload'), null)).toBe('/?upload=1');
    expect(uploadRedirect(u('/me/upload'), 'https://example.com/x')).toBe('/?upload=1');
    expect(uploadRedirect(u('/upload'), 'https://cyex.tv/me/upload')).toBe('/?upload=1');
    expect(uploadRedirect(u('/upload'), 'not a url')).toBe('/?upload=1');
  });
  it('reads ?upload and takes it out of the address', () => {
    expect(uploadParam(u('/search'))).toBeNull();
    expect(uploadParam(u('/search?upload=1'))).toEqual({ for: null });
    expect(uploadParam(u('/search?upload=A9'))).toEqual({ for: 'A9' });
    expect(withoutUploadParam(u('/search?q=cat&upload=1#x'))).toBe('/search?q=cat#x');
  });
});
