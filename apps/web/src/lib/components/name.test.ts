import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import Name from './Name.svelte';

const html = (props: Record<string, unknown>) => render(Name, { props: props as never }).body;

describe('Name', () => {
  it('renders the name in a <bdi>, its own bidi context', () => {
    expect(html({ value: 'lumen_fox' })).toContain('<bdi>lumen_fox</bdi>');
  });

  it('drops bidi controls a stored name still has', () => {
    const out = html({ value: 'abc‮def⁦x⁩' });
    expect(out).toContain('<bdi>abcdefx</bdi>');
    expect(out).not.toMatch(/[‪-‮⁦-⁩]/);
  });

  it('keeps right-to-left names as they are', () => {
    expect(html({ value: 'שלום' })).toContain('<bdi>שלום</bdi>');
  });

  it('shows the fallback for a missing name, and passes class and title on', () => {
    expect(html({ value: null, fallback: 'Unknown' })).toContain('>Unknown</bdi>');
    expect(html({ value: 'x', class: 'who', title: 'x' })).toMatch(/<bdi class="who" title="x">x<\/bdi>/);
  });
});
