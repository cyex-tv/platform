import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import EmoteTile from './EmoteTile.svelte';

const emote = {
  id: '01JD8X00000000000000000150',
  name: 'mochiStare',
  host: 'https://cdn.example/emote/1',
  animated: true,
  owner: { displayName: 'sunnyfern' },
  flags: { zeroWidth: true },
  files: [{ name: '1x.webp', width: 64, height: 32, isStatic: false }],
};
const html = () => render(EmoteTile, { props: { emote } }).body;

describe('EmoteTile', () => {
  it('draws no type label over the picture', () => {
    const out = html();
    expect(out).not.toMatch(/class="kinds/);
    const art = out.slice(out.indexOf('class="art'), out.indexOf('class="name'));
    expect(art).not.toMatch(/animated|zero-width|wide/);
  });

  it('keeps the type marks in the accessible name', () => {
    expect(html()).toMatch(/class="sr-only[^"]*">, animated, zero-width, wide</);
  });
});
