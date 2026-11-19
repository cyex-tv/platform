import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import ProfileHeader from './ProfileHeader.svelte';
import { NOBODY } from './rows';

// The facts line reads as words, also to a screen reader and when copied: a space wherever the
// eye sees one ("Main on lumen_fox", "Editors nora_byte, pixelmoth", "Member since 2026/11/10").
const profile = {
  identities: [{ platform: 'TWITCH', login: 'lumen_fox' }, { platform: 'KICK', login: 'lumenfox_kick' }],
  activeSets: [{ platform: 'TWITCH', login: 'lumen_fox', setId: 'S1', setName: 'Main' }],
  editors: [{ id: 'N', displayName: 'nora_byte' }, { id: 'P', displayName: 'pixelmoth' }],
  roles: [],
  createdAt: '2026-11-10T09:00:00Z',
};
const facts = () => {
  const html = render(ProfileHeader, { props: { account: { id: 'ME', displayName: 'lumen_fox' }, relation: NOBODY, profile: profile as never } }).body;
  const ul = html.slice(html.indexOf('class="facts'), html.lastIndexOf('</ul>'));
  return [...ul.matchAll(/<li[^>]*>([\s\S]*?)<\/li>/g)].map((m) =>
    m[1].replace(/<!--[\s\S]*?-->/g, '').replace(/<svg[\s\S]*?<\/svg>/g, '').replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim(),
  );
};

describe('profile band facts', () => {
  it('spaces every fact like it looks', () => {
    expect(facts()).toEqual([
      'Twitch: lumen_fox (opens in a new tab)',
      'Kick: lumenfox_kick (opens in a new tab)',
      'Main on lumen_fox (Twitch)',
      'Editors nora_byte, pixelmoth',
      'Member since 2026/11/10',
    ]);
  });
});
