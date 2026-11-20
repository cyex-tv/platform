import { describe as group, expect, it } from 'vitest';
import { after, approveFailure, describe, kindOf, sizes, step, withheld } from './queue';

const entry = (action: string, before: unknown = null, afterJson: unknown = null) => ({
  id: 'x',
  action,
  before: before === null ? null : JSON.stringify(before),
  after: afterJson === null ? null : JSON.stringify(afterJson),
  createdAt: '2026-11-19T10:00:00Z',
});

group('history lines', () => {
  it('says how it was approved', () => {
    expect(describe(entry('emote.approved', {}, { state: 'approved', visibility: 'unlisted' })).text).toBe('Approved as unlisted');
    expect(describe(entry('emote.approved', {}, { state: 'approved', visibility: 'public' })).text).toBe('Approved');
    expect(describe(entry('emote.approved')).text).toBe('Approved');
  });
  it('keeps reasons and notes as the detail', () => {
    expect(describe(entry('emote.rejected', {}, { reason: 'Blurry' }))).toEqual({ text: 'Rejected', detail: 'Blurry' });
    expect(describe(entry('emote.note_added', null, { body: 'Hi' }))).toEqual({ text: 'Added a note', detail: 'Hi' });
    expect(describe(entry('emote.note_deleted', { body: 'Old' }))).toEqual({ text: 'Deleted a note', detail: 'Old' });
  });
  it('names what an edit changed', () => {
    expect(describe(entry('emote.updated', { tags: [] }, { tags: ['cat', 'wave'] }))).toEqual({ text: 'Set the tags', detail: 'cat, wave' });
    expect(describe(entry('emote.updated', { name: 'a' }, { name: 'b' })).text).toBe('Renamed to b');
    expect(describe(entry('emote.updated', { nsfw: false, private: false }, { nsfw: true, private: true })).text).toBe('Changed NSFW, private');
    expect(describe(entry('emote.updated', { nsfw: true }, { nsfw: true })).text).toBe('Edited');
  });
  it('survives broken JSON and unknown actions', () => {
    expect(describe({ id: 'y', action: 'emote.approved', after: '{nope', createdAt: '' }).text).toBe('Approved');
    expect(describe(entry('emote.some_new_thing')).text).toBe('some new thing');
  });
});

group('pictures', () => {
  const f = (name: string, scale: number, isStatic = false, format = 'WEBP') => ({ name, format, scale, width: 32 * scale, height: 32 * scale, size: 100 * scale, isStatic });
  it('lists the moving WebP at each scale, smallest first', () => {
    const files = [f('2x.webp', 2), f('1x_static.webp', 1, true), f('1x.webp', 1), f('1x_static.avif', 1, true, 'AVIF')];
    expect(sizes(files).map((x) => x.name)).toEqual(['1x.webp', '2x.webp']);
    expect(kindOf({ animated: true, files })).toBe('Animated WebP');
    expect(kindOf({ animated: false, files: [] })).toBe('WebP');
  });
});

group('grid keys', () => {
  it('moves by one and by a row, never off the grid', () => {
    expect(step('ArrowRight', 0, 10, 4)).toBe(1);
    expect(step('ArrowRight', 9, 10, 4)).toBe(9);
    expect(step('ArrowLeft', 0, 10, 4)).toBe(0);
    expect(step('ArrowDown', 1, 10, 4)).toBe(5);
    expect(step('ArrowDown', 7, 10, 4)).toBe(7);
    expect(step('ArrowUp', 5, 10, 4)).toBe(1);
    expect(step('ArrowUp', 2, 10, 4)).toBe(2);
    expect(step('End', 2, 10, 4)).toBe(9);
    expect(step('Home', 7, 10, 0)).toBe(0);
    expect(step('a', 0, 10, 4)).toBeNull();
    expect(step('ArrowRight', 0, 0, 4)).toBeNull();
  });
  it('picks the next card after one leaves', () => {
    expect(after(0, 3)).toBe(0);
    expect(after(2, 3)).toBe(1);
    expect(after(0, 1)).toBeNull();
  });
});

group('approveFailure', () => {
  it('names the upload', () => {
    expect(approveFailure(new Error('Not allowed.'), 'Wave')).toBe('Wave wasn’t approved. Not allowed.');
  });
  it('says a timeout may have gone through', () => {
    expect(approveFailure(Object.assign(new Error('x'), { code: 'timeout' }), 'Wave')).toMatch(/Wave may have been approved: check the queue before trying again/);
  });
});

group('approvals waiting to be sent', () => {
  const card = (id: string, visibility = 'PUBLIC') => ({ id, visibility });
  const data = { items: [card('x'), card('y'), card('z', 'UNLISTED')], total: 3, counts: { public: 2, limited: 1 } };

  it('a reload brings nothing back that is still waiting, and the counts leave it out', () => {
    // X and Y approved; X undone: only Y is waiting.
    const view = withheld(data, [card('y')], 'PENDING');
    expect(view.items.map((e) => e.id)).toEqual(['x', 'z']);
    expect(view.total).toBe(2);
    expect(view.counts).toEqual({ public: 1, limited: 1 });
  });
  it('the total drops only for what belongs to the tab shown', () => {
    expect(withheld(data, [card('z', 'UNLISTED')], 'PUBLIC').total).toBe(3);
    expect(withheld(data, [card('z', 'UNLISTED')], 'LIMITED').total).toBe(2);
    expect(withheld(data, [card('y')], 'RESOLVED')).toBe(data);
  });
  it('nothing waiting, or nothing loaded', () => {
    expect(withheld(data, [], 'PENDING')).toBe(data);
    expect(withheld(null, [card('y')], 'PENDING')).toEqual({ items: [], total: 0, counts: { public: 0, limited: 0 } });
  });
});
