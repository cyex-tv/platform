import { describe, expect, it } from 'vitest';
import { connectionUrl } from './public';

describe('connectionUrl', () => {
  it('links Twitch and Kick logins, lowercase', () => {
    expect(connectionUrl('TWITCH', 'SomeOne_1')).toBe('https://www.twitch.tv/someone_1');
    expect(connectionUrl('KICK', 'Some-One')).toBe('https://kick.com/some-one');
  });
  it('refuses anything that is not a plain login', () => {
    for (const bad of ['', 'a/b', 'a b', '../x', 'x?y=1', 'x#', 'a'.repeat(65), 'né']) expect(connectionUrl('TWITCH', bad)).toBeNull();
  });
});
