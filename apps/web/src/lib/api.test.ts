import { describe, expect, it } from 'vitest';
import { safeBack, signInUrl } from './api';

describe('safeBack', () => {
  it('keeps paths on this site', () => {
    expect(safeBack('/')).toBe('/');
    expect(safeBack('/me/account?tab=1')).toBe('/me/account?tab=1');
  });

  it('falls back to / for anything else', () => {
    for (const bad of ['//evil.test', '/\\evil.test', '/a\\b', 'https://evil.test', 'evil', '']) {
      expect(safeBack(bad)).toBe('/');
    }
  });

  it('is applied to the sign-in address', () => {
    expect(signInUrl('twitch', '//evil.test')).toContain('return_to=%2F');
    expect(signInUrl('twitch', '//evil.test')).not.toContain('evil');
  });
});

describe('uploadForm', () => {
  it('sends only the flags that are on, and tags comma-separated', async () => {
    const { uploadForm } = await import('./api');
    const f = uploadForm(new Blob(['x']), 'catJAM', ['cat', 'dance'], { zeroWidth: true, private: false, nsfw: true });
    expect(f.get('name')).toBe('catJAM');
    expect(f.get('tags')).toBe('cat,dance');
    expect(f.get('zero_width')).toBe('true');
    expect(f.get('private')).toBeNull();
    expect(f.get('nsfw')).toBe('true');
    expect(f.get('file')).toBeInstanceOf(Blob);
    const plain = uploadForm(new Blob(['x']), 'a', []);
    expect([...plain.keys()]).toEqual(['name', 'file']);
  });
});
