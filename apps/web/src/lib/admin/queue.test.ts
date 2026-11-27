import { describe as group, expect, it } from 'vitest';
import { actionDone, actionForKey, actionLabel, actionsFor, after, approvalChips, approvalsInput, approveFailure, byPersonal, decisionOf, describe, kindOf, needsReason, partsOf, settle, sizes, step, toEnd, withheld } from './queue';

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
  const card = (id: string, priv = false) => ({ id, private: priv });
  const held = (id: string, priv = false, main = true, personal = false) => ({ ...card(id, priv), main, personal });
  const data = { items: [card('x'), card('y'), card('z', true)], total: 3, counts: { public: 2, limited: 1, personal: 3 } };

  it('a reload brings nothing back that is still waiting, and the counts leave it out', () => {
    // X and Y approved; X undone: only Y is waiting.
    const view = withheld(data, [held('y')], 'PENDING');
    expect(view.items.map((e) => e.id)).toEqual(['x', 'z']);
    expect(view.total).toBe(2);
    expect(view.counts).toEqual({ public: 1, limited: 1, personal: 3 });
  });
  it('the total drops only for what belongs to the tab shown', () => {
    expect(withheld(data, [held('z', true)], 'PUBLIC').total).toBe(3);
    expect(withheld(data, [held('z', true)], 'LIMITED').total).toBe(2);
    expect(withheld(data, [held('y')], 'RESOLVED')).toBe(data);
  });
  it('a personal-only approval keeps the card in the main queue but leaves the personal count', () => {
    const view = withheld(data, [held('x', false, false, true)], 'PUBLIC');
    expect(view.items.map((e) => e.id)).toEqual(['x', 'y', 'z']);
    expect(view.total).toBe(3);
    expect(view.counts).toEqual({ public: 2, limited: 1, personal: 2 });
  });
  it('in the personal queue only personal verdicts take a card out', () => {
    const view = withheld(data, [held('x', false, true, true), held('y', false, true, false)], 'PERSONAL');
    expect(view.items.map((e) => e.id)).toEqual(['y', 'z']);
    expect(view.total).toBe(2);
    expect(view.counts).toEqual({ public: 0, limited: 1, personal: 2 });
  });
  it('nothing waiting, or nothing loaded', () => {
    expect(withheld(data, [], 'PENDING')).toBe(data);
    expect(withheld(null, [held('y')], 'PENDING')).toEqual({ items: [], total: 0, counts: { public: 0, limited: 0, personal: 0 } });
  });
});

group('toEnd (an undone approval goes back last)', () => {
  const q = { total: 4, items: [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }] };
  it('moves the card to the end, not back to its old place', () => {
    expect(toEnd(q, ['b']).items.map((e) => e.id)).toEqual(['a', 'c', 'd', 'b']);
  });
  it('moves a batch in the order it was approved, keeping the rest', () => {
    const out = toEnd(q, ['c', 'a']);
    expect(out.items.map((e) => e.id)).toEqual(['b', 'd', 'c', 'a']);
    expect(out.total).toBe(4);
  });
  it('leaves the queue alone for cards it doesn’t hold', () => {
    expect(toEnd(q, ['zz'])).toBe(q);
  });
});

group('describe (moderator flags and requeue, ADR 0024)', () => {
  const entry = (action: string, before: object | null, after: object | null) => ({
    id: 'h', action, actorId: 'a', targetId: 't', createdAt: '2026-11-20T10:00:00Z',
    before: before && JSON.stringify(before), after: after && JSON.stringify(after),
  });
  it('names what the flags change did', () => {
    const was = { visibility: 'public', nsfw: false, zero_width: false };
    expect(describe(entry('emote.flags_changed', was, { ...was, visibility: 'unlisted', via: 'emotes.flags' })).text).toBe('Flags: made it unlisted');
    expect(describe(entry('emote.flags_changed', was, { ...was, nsfw: true, zero_width: true })).text).toBe('Flags: marked it NSFW, made it zero-width');
  });
  it('says an undone approval went back last', () => {
    expect(describe(entry('emote.requeued', null, null)).text).toBe('Approval undone: back at the end of the queue');
  });
});

group('approvals history lines (ADR 0039)', () => {
  const e = (action: string, before: object | null, after: object | null) => ({
    id: 'h', action, actorId: 'a', createdAt: '2026-11-27T10:00:00Z',
    before: before && JSON.stringify(before), after: after && JSON.stringify(after),
  });
  it('says what approvals_changed changed, with the reason for a denial', () => {
    const was = { public_listing: 'pending', personal_use: 'pending' };
    expect(describe(e('emote.approvals_changed', was, { ...was, public_listing: 'approved' })).text).toBe('Public listing approved');
    expect(describe(e('emote.approvals_changed', was, { ...was, personal_use: 'denied', reason: 'Too big' })).text).toBe('Personal use denied: Too big');
    expect(describe(e('emote.approvals_changed', { public_listing: 'approved', personal_use: 'denied' }, { public_listing: 'denied', personal_use: 'denied', reason: 'Spam' })).text).toBe('Public listing denied: Spam');
    expect(describe(e('emote.approvals_changed', was, { public_listing: 'denied', personal_use: 'denied', reason: 'No' })).text).toBe('Public listing denied, personal use denied: No');
    expect(describe(e('emote.approvals_changed', null, null)).text).toBe('Changed the approvals');
  });
  it('reads the first verdict (emote.approved) with its approvals, and old rows as before', () => {
    expect(describe(e('emote.approved', {}, { state: 'approved', public_listing: 'approved', personal_use: 'approved' })).text).toBe('Approved for public listing and personal use');
    expect(describe(e('emote.approved', {}, { state: 'approved', public_listing: 'approved', personal_use: 'pending' })).text).toBe('Public listing approved');
    expect(describe(e('emote.approved', {}, { state: 'approved', public_listing: 'denied', personal_use: 'pending', reason: 'Off topic' })).text).toBe('Public listing denied: Off topic');
    expect(describe(e('emote.approved', {}, { state: 'approved', visibility: 'unlisted' })).text).toBe('Approved as unlisted');
  });
  it('says what a merge did', () => {
    const after = { source_name: 'Pogg', target_name: 'Pog', moved: 12, collapsed: [{ set_id: 's', kept_name: 'Pog', dropped_name: 'Pogg' }] };
    expect(describe(e('emote.merged', null, after)).text).toBe('Merged Pogg into Pog (12 moved, 1 already had it)');
    expect(describe(e('emote.merged', null, null)).text).toBe('Merged into another emote');
  });
  it('says staff transferred it', () => {
    expect(describe(e('emote.transferred', { owner: 'a' }, { owner: 'b', via: 'emotes.transfer' })).text).toBe('Transferred to another account by staff');
    expect(describe(e('emote.transferred', null, null)).text).toBe('Transferred to another account');
  });
});

group('review actions (ADR 0039)', () => {
  const both = { listing: true, personal: true };
  const listingOnly = { listing: true, personal: false };
  const personalOnly = { listing: false, personal: true };

  it('makes Approve both the primary action for a reviewer who holds both permissions', () => {
    expect(actionsFor('main', both)[0]).toBe('both');
    expect(actionLabel('both', both)).toBe('Approve both');
    expect(decisionOf('both')).toEqual({ listing: 'APPROVE', personal: 'APPROVE' });
  });
  it('gives the one they can do to a reviewer with only one', () => {
    expect(actionsFor('main', listingOnly)).toEqual(['listing', 'deny-listing', 'reject']);
    expect(actionLabel('listing', listingOnly)).toBe('Approve');
    expect(actionsFor('main', personalOnly)).toEqual([]);
    expect(actionsFor('personal', personalOnly)).toEqual(['personal', 'deny-personal']);
    expect(actionsFor('personal', listingOnly)).toEqual([]);
  });
  it('has no reject in the personal queue', () => {
    expect(actionsFor('personal', both)).not.toContain('reject');
  });
  it('maps keys to what the queue offers', () => {
    expect(actionForKey('a', 'main', both)).toBe('both');
    expect(actionForKey('a', 'main', listingOnly)).toBe('listing');
    expect(actionForKey('A', 'personal', both)).toBe('personal');
    expect(actionForKey('d', 'main', both)).toBe('deny-listing');
    expect(actionForKey('d', 'personal', both)).toBe('deny-personal');
    expect(actionForKey('n', 'main', both)).toBe('deny-personal');
    expect(actionForKey('r', 'main', both)).toBe('reject');
    expect(actionForKey('r', 'personal', both)).toBeNull();
    expect(actionForKey('p', 'main', listingOnly)).toBeNull();
    expect(actionForKey('u', 'main', both)).toBeNull();
  });
  it('needs a reason to deny or reject, never to approve', () => {
    expect(needsReason('deny-listing')).toBe(true);
    expect(needsReason('deny-personal')).toBe(true);
    expect(needsReason('reject')).toBe(true);
    expect(needsReason('both')).toBe(false);
    expect(needsReason('personal')).toBe(false);
  });
  it('builds the reviewApprovals input and leaves a part out when it is not decided', () => {
    expect(approvalsInput({ listing: 'APPROVE', personal: 'APPROVE' })).toEqual({ publicListing: 'APPROVE', personalUse: 'APPROVE' });
    expect(approvalsInput({ personal: 'DENY' }, 'Too big')).toEqual({ personalUse: 'DENY', reason: 'Too big' });
    expect(decisionOf('reject')).toBeNull();
  });
  it('knows which queues a verdict takes an upload out of', () => {
    expect(partsOf('both')).toEqual({ main: true, personal: true });
    expect(partsOf('listing')).toEqual({ main: true, personal: false });
    expect(partsOf('deny-personal')).toEqual({ main: false, personal: true });
    expect(partsOf('reject')).toEqual({ main: true, personal: true });
  });
  it('words the toast', () => {
    expect(actionDone('both')).toBe('approved for public listing and personal use');
    expect(actionDone('deny-personal')).toBe('denied personal use');
  });
  it('shows an upload approvals after a verdict', () => {
    const was = { publicListing: 'PENDING', personalUse: 'PENDING' } as const;
    expect(settle(was, { listing: 'APPROVE' })).toEqual({ publicListing: 'APPROVED', personalUse: 'PENDING' });
    expect(settle(was, { personal: 'DENY' })).toEqual({ publicListing: 'PENDING', personalUse: 'DENIED' });
    expect(settle(was, null)).toEqual({ publicListing: 'DENIED', personalUse: 'DENIED' });
    expect(settle(undefined, { listing: 'APPROVE' })).toBeUndefined();
  });
  it('chips only what is decided', () => {
    expect(approvalChips({ approvals: { publicListing: 'PENDING', personalUse: 'PENDING' } }, false)).toEqual([]);
    expect(approvalChips({ approvals: { publicListing: 'APPROVED', personalUse: 'DENIED' } }, true)).toEqual([{ text: 'Listed', tone: 'ok' }, { text: 'No personal', tone: 'warn' }]);
    expect(approvalChips({}, true)).toEqual([]);
  });
  it('filters an account uploads by personal use', () => {
    const rows = [
      { id: '1', approvals: { publicListing: 'APPROVED', personalUse: 'APPROVED' } as const },
      { id: '2', approvals: { publicListing: 'APPROVED', personalUse: 'PENDING' } as const },
      { id: '3' },
    ];
    expect(byPersonal(rows, 'all')).toHaveLength(3);
    expect(byPersonal(rows, 'APPROVED').map((r) => r.id)).toEqual(['1']);
    expect(byPersonal(rows, 'PENDING').map((r) => r.id)).toEqual(['2']);
    expect(byPersonal(rows, 'DENIED')).toEqual([]);
  });
});
