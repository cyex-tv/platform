import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ShopItemFieldsFragment } from '$lib/gql/graphql';
import ItemBuy from './ItemBuy.svelte';

const { buy } = vi.hoisted(() => ({ buy: vi.fn() }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('./checkout.svelte', () => ({ buy: (...a: unknown[]) => buy(...a) }));

const item = (over: Partial<ShopItemFieldsFragment> = {}): ShopItemFieldsFragment =>
  ({
    id: '42',
    name: 'Winter pack',
    description: null,
    priceCents: 499,
    currency: 'EUR',
    paddlePriceId: null,
    badges: [],
    paints: [],
    activeFrom: null,
    activeTo: null,
    stockLimit: null,
    stockLeft: null,
    visible: true,
    sortOrder: 0,
    available: true,
    owned: false,
    deleted: false,
    ...over,
  }) as ShopItemFieldsFragment;

function show(props: Partial<{ item: ShopItemFieldsFragment; gift: boolean; signedIn: boolean }> = {}) {
  const onstarted = vi.fn();
  const onclose = vi.fn();
  render(ItemBuy, { item: item(), name: 'lumen_fox', signedIn: true, variant: 'panel', onclose, onstarted, showing: () => true, ...props });
  return { onstarted, onclose };
}

beforeEach(() => buy.mockReset().mockResolvedValue(null));

describe('the item panel', () => {
  it('signed out, offers sign-in that comes back to this item', () => {
    show({ signedIn: false });
    const link = screen.getByRole('link', { name: 'Sign in to buy' });
    expect(link.getAttribute('href')).toBe(`/signin?back=${encodeURIComponent('/shop?item=42')}`);
    expect(screen.queryByRole('radiogroup')).toBeNull();
  });

  it('says what the item holds and the price', () => {
    show();
    expect(screen.getByRole('heading', { level: 2, name: 'Winter pack' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Buy for 4,99 €' })).toBeTruthy();
  });

  it('a gift needs a login before it starts', async () => {
    show({ gift: true });
    await fireEvent.click(screen.getByRole('button', { name: 'Give for 4,99 €' }));
    expect(buy).not.toHaveBeenCalled();
    expect(await screen.findByText('Enter the Twitch or Kick login of who gets it.')).toBeTruthy();
  });

  it('a gift goes to the login typed, without the @, and closes the item when the payment opens', async () => {
    const { onstarted } = show({ gift: true });
    await fireEvent.input(screen.getByPlaceholderText('For example quietoak'), { target: { value: ' @quietoak ' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Give for 4,99 €' }));
    expect(buy).toHaveBeenCalledWith({ itemId: '42', itemName: 'Winter pack', gift: { platform: 'TWITCH', login: 'quietoak' }, to: 'quietoak' });
    await vi.waitFor(() => expect(onstarted).toHaveBeenCalled());
  });

  it('a start that fails says why and stays open', async () => {
    buy.mockResolvedValue('The payment didn’t go through.');
    const { onstarted } = show();
    await fireEvent.click(screen.getByRole('button', { name: 'Buy for 4,99 €' }));
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'The payment didn’t go through.');
    expect(onstarted).not.toHaveBeenCalled();
  });

  it('an owned item opens on the gift and cannot be bought again for yourself', async () => {
    show({ item: item({ owned: true }) });
    expect(screen.getByRole('button', { name: 'Give for 4,99 €' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('radio', { name: 'For me' }));
    expect((screen.getByRole('button', { name: 'Buy for 4,99 €' }) as HTMLButtonElement).disabled).toBe(true);
  });
});
