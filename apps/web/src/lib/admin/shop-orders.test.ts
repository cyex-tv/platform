import { describe, expect, it } from 'vitest';
import { orderFilters, ordersHref, pageRange } from './shop-orders';

const url = (s: string) => new URL(`http://x/admin/shop/orders${s}`);

describe('orderFilters', () => {
  it('reads state, item, account and page', () => {
    expect(orderFilters(url('?status=paid&item=a&account=b&page=3').searchParams)).toEqual({ status: 'PAID', item: 'a', account: 'b', page: 3 });
  });
  it('drops nonsense', () => {
    expect(orderFilters(url('?status=x&page=0').searchParams)).toEqual({ status: '', item: '', account: '', page: 1 });
    expect(orderFilters(url('?page=1.5').searchParams).page).toBe(1);
    expect(orderFilters(url('?page=abc').searchParams).page).toBe(1);
  });
});

describe('ordersHref', () => {
  it('a filter change resets the page and the picked order', () => {
    expect(ordersHref(url('?page=3&order=o1&status=PAID'), { item: 'i1' })).toBe('?status=PAID&item=i1');
  });
  it('paging keeps filters and the order', () => {
    expect(ordersHref(url('?status=PAID&order=o1'), { page: 2 })).toBe('?status=PAID&order=o1&page=2');
    expect(ordersHref(url('?status=PAID&page=2'), { page: 1 })).toBe('?status=PAID');
  });
  it('removes keys and falls back to the path', () => {
    expect(ordersHref(url('?order=o1'), { order: null })).toBe('/admin/shop/orders');
    expect(ordersHref(url('?status=PAID'), { status: '' })).toBe('/admin/shop/orders');
  });
});

describe('pageRange', () => {
  it('counts the rows of a page', () => {
    expect(pageRange(2, 25, 60, 25)).toEqual({ from: 26, to: 50, pages: 3 });
    expect(pageRange(3, 25, 60, 10)).toEqual({ from: 51, to: 60, pages: 3 });
    expect(pageRange(1, 25, 0, 0)).toEqual({ from: 0, to: 0, pages: 1 });
  });
});
