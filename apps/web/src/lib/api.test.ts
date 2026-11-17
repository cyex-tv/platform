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
