import { cleanup, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import FactList from './FactList.svelte';
import LinkRows from './LinkRows.svelte';
import Segmented from './Segmented.svelte';

afterEach(cleanup);

describe('FactList', () => {
  it('pairs each label with its value, a value with href as a link', () => {
    render(FactList, { label: 'Order', items: [{ label: 'Total', value: '4,99 €' }, { label: 'Receipt', value: 'Open', href: '/r/1' }] });
    const dl = screen.getByLabelText('Order');
    expect([...dl.querySelectorAll('dt')].map((d) => d.textContent)).toEqual(['Total', 'Receipt']);
    expect(screen.getByRole('link', { name: 'Open' }).getAttribute('href')).toBe('/r/1');
  });
});

describe('LinkRows', () => {
  it('makes the whole row the link, and says when it opens another tab', () => {
    render(LinkRows, { label: 'Related', items: [{ href: '/help', label: 'Help', hint: 'How it works' }, { href: 'https://x.test', label: 'Source', external: true }] });
    expect(screen.getByRole('list', { name: 'Related' })).toBeTruthy();
    expect(screen.getByRole('link', { name: /Help/ }).getAttribute('href')).toBe('/help');
    const out = screen.getByRole('link', { name: /Source/ });
    expect(out.getAttribute('target')).toBe('_blank');
    expect(out.textContent).toContain('opens in a new tab');
  });
});

describe('Segmented without icons', () => {
  it('marks the picked option with the accent tick (the others keep its room, hidden)', () => {
    render(Segmented, { label: 'Motion', value: 'on', onchange: () => {}, options: [{ value: 'sys', label: 'System' }, { value: 'on', label: 'On' }] });
    const on = screen.getByRole('radio', { name: 'On' });
    const sys = screen.getByRole('radio', { name: 'System' });
    expect(on.querySelector('.tick.on')).toBeTruthy();
    expect(sys.querySelector('.tick')).toBeTruthy();
    expect(sys.querySelector('.tick.on')).toBeNull();
  });
});
