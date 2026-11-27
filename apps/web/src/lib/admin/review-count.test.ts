import { describe, expect, it } from 'vitest';
import { adminCountVars, asksForCounts, reviewWaiting } from './review-count';

const only = (...perms: string[]) => (p: string) => perms.includes(p);
const counts = { reviewQueue: { total: 5, counts: { personal: 3 } }, personalQueue: { total: 3 } };

describe('the review count in the admin rail', () => {
  it('asks for each queue only with its own permission', () => {
    expect(adminCountVars(only('emotes.review'))).toEqual({ review: true, reports: true, personal: false });
    expect(adminCountVars(only('emotes.review_personal'))).toEqual({ review: false, reports: false, personal: true });
    expect(adminCountVars(only('emotes.review', 'emotes.review_personal'))).toEqual({ review: true, reports: true, personal: false });
    expect(asksForCounts(adminCountVars(only('audit.view')))).toBe(false);
  });
  it('shows the personal-use count to someone holding only emotes.review_personal', () => {
    expect(reviewWaiting(counts, only('emotes.review_personal'))).toBe(3);
    expect(reviewWaiting({ reviewQueue: undefined, personalQueue: undefined } as never, only('emotes.review_personal'))).toBeUndefined();
  });
  it('shows pending only to a reviewer without the personal permission', () => {
    expect(reviewWaiting(counts, only('emotes.review'))).toBe(5);
  });
  it('adds the two for holders of both', () => {
    expect(reviewWaiting(counts, only('emotes.review', 'emotes.review_personal'))).toBe(8);
  });
  it('shows nothing without either, or before the answer is in', () => {
    expect(reviewWaiting(counts, only('reports.manage'))).toBeUndefined();
    expect(reviewWaiting(null, only('emotes.review'))).toBeUndefined();
  });
});
