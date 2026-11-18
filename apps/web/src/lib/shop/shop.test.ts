import { describe, expect, it, vi } from 'vitest';
import { ApiError } from '$lib/api';
import {
  centsFrom, centsText, codesFilename, codesText, fromLocalInput, itemState, lockoutMinutes, normaliseCode, orderStatusText, plausibleCode, price,
  redeemMessage, saleEndText, settled, shopMessage, staffMessage, stockText, tidyCode, toLocalInput,
} from './shop';

describe('price', () => {
  it('uses the cyex number format', () => {
    expect(price(499, 'EUR')).toBe('4,99 €');
    expect(price(500, 'EUR')).toBe('5,00 €');
    expect(price(5, 'EUR')).toBe('0,05 €');
    expect(price(123456, 'USD')).toBe('1.234,56 $');
    expect(price(100000, 'GBP')).toBe('1.000,00 £');
  });
  it('is Free at zero and keeps unknown currency codes', () => {
    expect(price(0, 'EUR')).toBe('Free');
    expect(price(1500, 'sek')).toBe('15,00 SEK');
  });
});

describe('centsFrom and centsText', () => {
  it('reads comma and dot prices', () => {
    expect(centsFrom('4,99')).toBe(499);
    expect(centsFrom('4.99')).toBe(499);
    expect(centsFrom(' 5 ')).toBe(500);
    expect(centsFrom('4,5')).toBe(450);
    expect(centsFrom('1000')).toBe(100000);
  });
  it('rejects what is not a price', () => {
    for (const bad of ['', 'abc', '-1', '1,234', '4,999', '1e3', '1.000,00']) expect(centsFrom(bad)).toBeNull();
  });
  it('writes cents as a field value', () => {
    expect(centsText(499)).toBe('4,99');
    expect(centsText(0)).toBe('0,00');
    expect(centsFrom(centsText(12345))).toBe(12345);
  });
});

describe('redeem codes', () => {
  it('ignores case, spaces and dashes', () => {
    expect(normaliseCode('sample-code-0001')).toBe('SAMPLECODE0001');
    expect(normaliseCode(' Sample Code 0001 ')).toBe('SAMPLECODE0001');
    expect(normaliseCode('SAMPLE–CODE—0001')).toBe('SAMPLECODE0001');
    expect(normaliseCode('sample\t code\n0001')).toBe('SAMPLECODE0001');
  });
  it('knows a plausible code', () => {
    expect(plausibleCode('SAMPLECODE0001')).toBe(true);
    expect(plausibleCode('ABC')).toBe(false);
    expect(plausibleCode('SAMPLE_CODE')).toBe(false);
    expect(plausibleCode('')).toBe(false);
  });
  it('tidies what is typed without losing characters', () => {
    expect(tidyCode('sample code  0001')).toBe('SAMPLE-CODE-0001');
  });
});

describe('error messages', () => {
  const e = (code: string, message = 'raw') => new ApiError(message, code);
  it('maps every shop code to a sentence of its own', () => {
    for (const code of ['signed_out', 'item_unavailable', 'sold_out', 'already_owned', 'recipient_not_found', 'checkout_unavailable', 'payment_provider_error', 'checkout_busy', 'checkout_failed', 'unknown_client', 'rate_limited', 'offline']) {
      const m = shopMessage(e(code));
      expect(m).not.toBe('raw');
      expect(m.length).toBeGreaterThan(10);
    }
  });
  it('tunes already_owned for a gift', () => {
    expect(shopMessage(e('already_owned'), true)).toBe('They own everything in it already.');
    expect(shopMessage(e('already_owned'))).toContain('You own everything');
  });
  it('falls back to the API sentence for an unknown code, and never leaks an Error', () => {
    expect(shopMessage(e('weird', 'The API says so.'))).toBe('The API says so.');
    expect(shopMessage(new Error('stack trace'))).not.toContain('stack');
  });
  it('maps every redeem code', () => {
    for (const code of ['bad_code', 'code_disabled', 'code_expired', 'already_redeemed', 'code_used_up', 'signed_out', 'rate_limited']) {
      expect(redeemMessage(e(code))).not.toBe('raw');
    }
  });
  it('says when to try again after a lockout', () => {
    expect(lockoutMinutes('Too many wrong codes. Try again in 7 min.')).toBe(7);
    expect(lockoutMinutes('nothing')).toBeNull();
    expect(redeemMessage(e('locked_out', 'Too many wrong codes. Try again in 7 min.'))).toBe('Too many wrong codes. You can try again in 7 minutes.');
    expect(redeemMessage(e('locked_out', 'Try again in 1 min.'))).toContain('1 minute.');
    expect(redeemMessage(e('locked_out', 'Locked.'))).toBe('Too many wrong codes. Try again in a while.');
  });
  it('maps staff codes', () => {
    expect(staffMessage(e('bad_price_id'))).toContain('pri_');
    expect(staffMessage(e('not_refundable'))).toContain('paid');
    for (const code of ['already_refunded', 'refund_in_progress']) expect(staffMessage(e(code))).not.toBe('raw');
    expect(staffMessage(e('unknown_thing', 'As the API says.'))).toBe('As the API says.');
    expect(staffMessage(new Error('x'))).toBe('That didn’t work. Nothing was changed.');
  });
});

describe('orders', () => {
  it('reads the status in plain words', () => {
    expect(orderStatusText('PENDING')).toBe('Waiting for payment');
    expect(orderStatusText('PAID')).toBe('Paid');
    expect(orderStatusText('REFUNDED')).toBe('Refunded');
    expect(orderStatusText('FAILED')).toBe('Not completed');
  });
  it('polls only while pending', () => {
    expect(settled('PENDING')).toBe(false);
    for (const s of ['PAID', 'REFUNDED', 'FAILED'] as const) expect(settled(s)).toBe(true);
  });
});

describe('items', () => {
  const now = Date.parse('2026-11-18T12:00:00Z');
  const base = { visible: true };
  it('names the staff state', () => {
    expect(itemState({ ...base }, now)).toBe('on_sale');
    expect(itemState({ ...base, deleted: true }, now)).toBe('deleted');
    expect(itemState({ ...base, visible: false }, now)).toBe('hidden');
    expect(itemState({ ...base, activeFrom: '2026-12-01T00:00:00Z' }, now)).toBe('scheduled');
    expect(itemState({ ...base, activeTo: '2026-11-01T00:00:00Z' }, now)).toBe('ended');
    expect(itemState({ ...base, stockLeft: 0 }, now)).toBe('sold_out');
  });
  it('says stock and the sale end', () => {
    expect(stockText({ stockLeft: null })).toBeNull();
    expect(stockText({ stockLeft: 0 })).toBe('Sold out');
    expect(stockText({ stockLeft: 1234 })).toBe('1.234 left');
    expect(saleEndText({ activeTo: null }, now)).toBeNull();
    expect(saleEndText({ activeTo: '2027-01-15T12:00:00Z' }, now)).toMatch(/^On sale until 2027\/01\/1[56]$/);
    expect(saleEndText({ activeTo: '2026-11-01T12:00:00Z' }, now)).toMatch(/^Ended 2026\/11\/0[12]$/);
  });
  it('round-trips the local date-time field', () => {
    expect(toLocalInput(null)).toBe('');
    expect(fromLocalInput('')).toBeNull();
    const iso = '2026-12-01T18:30:00.000Z';
    expect(fromLocalInput(toLocalInput(iso))).toBe(iso);
  });
});

describe('code files', () => {
  it('writes one code per line', () => expect(codesText(['A-1', 'B-2'])).toBe('A-1\nB-2\n'));
  it('names the file safely', () => {
    expect(codesFilename('Stream giveaway #3')).toBe('cyex-tv-codes-stream-giveaway-3.txt');
    expect(codesFilename('../..')).toBe('cyex-tv-codes.txt');
  });
});

describe('the site’s one error wording', () => {
  it('words a dropped network and an unplaceable failure as problem.ts does', async () => {
    const { COPY } = await import('$lib/components/problem');
    expect(shopMessage(new Error('boom'))).toBe(`${COPY.server.title} Nothing was charged.`);
    expect(redeemMessage(new Error('boom'))).toBe(`${COPY.server.title} Try again in a moment.`);
    const offline = new ApiError('whatever the request said', 'offline');
    // With a network the server didn't answer; with none the browser is offline.
    expect(shopMessage(offline)).toBe(`${COPY.server.title} ${COPY.server.body}`);
    vi.stubGlobal('navigator', { onLine: false });
    try {
      expect(shopMessage(offline)).toBe(`${COPY.offline.title} ${COPY.offline.body}`);
      expect(redeemMessage(offline)).toBe(`${COPY.offline.title} ${COPY.offline.body}`);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
