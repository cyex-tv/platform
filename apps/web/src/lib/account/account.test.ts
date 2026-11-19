import { describe as suite, expect, it } from 'vitest';
import { ApiError } from '$lib/api';
import { deleteMessage, exportMessage, unlinkMessage } from './errors';
import { exportFilename } from './export';
import { badge, describe, parsePayload } from './notifications';

const j = (o: unknown) => JSON.stringify(o);
const strip = (s: string) => s.replace(/[⁨⁩]/g, '');

suite('parsePayload', () => {
  it('reads an object', () => expect(parsePayload('{"a":1}')).toEqual({ a: 1 }));
  it('survives malformed JSON, non-objects and empties', () => {
    for (const bad of ['{', 'nope', '', '[1,2]', '"x"', '12', 'null', undefined, null]) expect(parsePayload(bad)).toEqual({});
  });
});

suite('describe', () => {
  it('emote.approved links to the emote', () => {
    const l = describe('emote.approved', j({ emote_id: 'E1', name: 'KEKW' }));
    expect(strip(l.text)).toBe('KEKW was approved.');
    expect(l.href).toBe('/emotes/E1');
    expect(l.icon).toBe('check');
  });
  it('emote.rejected carries the reason', () => {
    const l = describe('emote.rejected', j({ emote_id: 'E2', name: 'Pog', reason: 'Too blurry.' }));
    expect(strip(l.text)).toBe('Pog was rejected. Reason: Too blurry.');
    expect(l.href).toBe('/me/emotes/E2');
  });
  it('emote.taken_down is understood too', () => {
    expect(strip(describe('emote.taken_down', j({ name: 'X', reason: 'Copy' })).text)).toBe('X was taken down. Reason: Copy');
  });
  it('editor kinds read from the editor’s side', () => {
    const p = { role: 'editor', owner_id: 'O1', owner_name: 'Ana', editor_name: 'Me' };
    expect(strip(describe('editor.added', j(p)).text)).toBe('Ana made you an editor of their account.');
    expect(describe('editor.added', j(p)).href).toBe('/users/O1');
    expect(strip(describe('editor.changed', j(p)).text)).toBe('Ana changed what you can do on their account.');
    expect(strip(describe('editor.removed', j(p)).text)).toBe('You’re no longer an editor of Ana’s account.');
    expect(describe('editor.removed', j(p)).href).toBeNull();
  });
  it('editor kinds read from the owner’s side', () => {
    const p = { role: 'owner', editor_name: 'Bo' };
    expect(strip(describe('editor.added', j(p)).text)).toBe('Bo is now an editor of your account.');
    expect(strip(describe('editor.changed', j(p)).text)).toBe('What Bo can do on your account changed.');
    expect(strip(describe('editor.removed', j(p)).text)).toBe('Bo is no longer an editor of your account.');
    expect(describe('editor.removed', j(p)).href).toBe('/me/editors');
  });
  it('cosmetic kinds name the paint or badge', () => {
    expect(strip(describe('cosmetic.granted', j({ kind: 'paint', name: 'Sunrise', reason: 'Thanks' })).text)).toBe(
      'You got the Sunrise paint. Reason: Thanks',
    );
    expect(strip(describe('cosmetic.revoked', j({ kind: 'badge', name: 'Early' })).text)).toBe('The Early badge was taken away.');
    expect(describe('cosmetic.granted', j({ kind: 'paint' })).href).toBe('/me/cosmetics');
  });
  it('missing fields still give a sentence', () => {
    expect(describe('emote.approved', '{}').text).toBe('Your emote was approved.');
    expect(describe('emote.approved', '{}').href).toBeNull();
    expect(describe('cosmetic.granted', '{}').text).toBe('You got a new cosmetic.');
    expect(strip(describe('editor.added', '{}').text)).toBe('Someone made you an editor of their account.');
  });
  it('malformed payloads and unknown kinds do not throw', () => {
    expect(describe('emote.rejected', '{oops').text).toBe('Your emote was rejected.');
    expect(describe('shop.something', '{}').text).toBe('There’s an update on your account.');
    expect(describe('emote.approved', j({ name: 42, emote_id: { x: 1 } })).text).toBe('Your emote was approved.');
  });
  it('never builds an address from an unsafe ID', () => {
    expect(describe('emote.approved', j({ emote_id: '../admin?x=<script>', name: 'a' })).href).toBeNull();
  });
  it('keeps markup as text and isolates names', () => {
    const l = describe('emote.approved', j({ emote_id: 'E1', name: '<img src=x onerror=alert(1)>' }));
    expect(l.text).toContain('<img src=x onerror=alert(1)>');
    expect(l.text.startsWith('⁨')).toBe(true);
  });
  it('cuts very long names and reasons', () => {
    const l = describe('emote.rejected', j({ name: 'n'.repeat(500), reason: 'r'.repeat(5000) }));
    expect(l.text.length).toBeLessThan(400);
  });
});

suite('badge', () => {
  it('shows nothing at zero, the number up to 99 and 99+ above', () => {
    expect(badge(0)).toBe('');
    expect(badge(-3)).toBe('');
    expect(badge(Number.NaN)).toBe('');
    expect(badge(1)).toBe('1');
    expect(badge(99)).toBe('99');
    expect(badge(100)).toBe('99+');
    expect(badge(120)).toBe('99+');
  });
});

suite('exportFilename', () => {
  it('is cyex-tv-export-<date>.json', () => {
    expect(exportFilename(new Date(2026, 10, 7))).toBe('cyex-tv-export-2026-11-07.json');
    expect(exportFilename(new Date(2027, 0, 31))).toBe('cyex-tv-export-2027-01-31.json');
  });
});

suite('error messages', () => {
  it('maps the delete codes', () => {
    for (const c of ['confirm_mismatch', 'website_only', 'owner_protected', 'restricted']) {
      const m = deleteMessage(new ApiError('raw', c));
      expect(m).not.toBe('raw');
      expect(m.length).toBeGreaterThan(20);
    }
    expect(deleteMessage(new ApiError('server words', 'other'))).toBe('server words');
    expect(deleteMessage(new Error('x'))).toContain('Nothing was deleted');
  });
  it('says rate_limited humanely', () => {
    expect(exportMessage(new ApiError('raw', 'rate_limited'))).toContain('hour');
  });
});

suite('unlinkMessage', () => {
  it('says why a restricted account can’t unlink', () => {
    expect(unlinkMessage(new ApiError('raw', 'restricted'))).toBe('Your account is restricted, so you can’t disconnect a sign-in right now.');
    expect(unlinkMessage(new ApiError('raw', 'last_identity'))).toMatch(/only connection/);
    expect(unlinkMessage(new ApiError('As the API says.', 'other'))).toBe('As the API says.');
    expect(unlinkMessage(new Error('x'))).toContain('Nothing was disconnected');
  });
});
