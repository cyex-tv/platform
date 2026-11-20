import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import ProfileHeader from './ProfileHeader.svelte';
import { NOBODY } from './rows';

// The facts line: connections and editors are collapsed behind count chips (owner, 2026/11/20),
// so the band says how many and never lists them until opened. It reads as words, also to a
// screen reader and when copied.
const editors = Array.from({ length: 50 }, (_, i) => ({ id: `E${i}`, displayName: `editor_${i}` }));
const profile = {
  identities: [{ platform: 'TWITCH', login: 'lumen_fox' }, { platform: 'KICK', login: 'lumenfox_kick' }],
  activeSets: [{ platform: 'TWITCH', login: 'lumen_fox', setId: 'S1', setName: 'Main' }],
  editors,
  roles: [1, 2, 3, 4, 5].map((n) => ({ id: `R${n}`, name: `Role ${n}` })),
  createdAt: '2026-11-10T09:00:00Z',
};
const html = () => render(ProfileHeader, { props: { account: { id: 'ME', displayName: 'lumen_fox' }, relation: NOBODY, profile: profile as never } }).body;
const text = (h: string) => h.replace(/<!--[\s\S]*?-->/g, '').replace(/<svg[\s\S]*?<\/svg>/g, '').replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
const facts = () => {
  const h = html();
  const ul = h.slice(h.indexOf('class="facts'), h.lastIndexOf('</ul>'));
  return [...ul.matchAll(/<li[^>]*>([\s\S]*?)<\/li>/g)].map((m) => text(m[1]));
};

describe('profile band facts', () => {
  it('collapses connections and editors into count chips, member since stays a line', () => {
    expect(facts()).toEqual(['Connections 2', 'Editors 50', 'Member since 2026/11/10']);
  });
  it('lists no editor, login or set until a chip is opened, however many there are', () => {
    const h = html();
    expect(h).not.toContain('editor_0');
    expect(h).not.toContain('lumenfox_kick');
    expect(h).not.toContain('Main');
    expect(h).toContain('aria-label="Editors, 50"');
    expect(h).toContain('aria-expanded="false"');
  });
  it('shows the first three roles, the rest behind +N', () => {
    const h = html();
    expect(h).toContain('Role 3');
    expect(h).not.toContain('Role 4');
    expect(h).toContain('aria-label="All roles, 2"');
  });
});
