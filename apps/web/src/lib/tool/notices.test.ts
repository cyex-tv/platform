import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, gql } from '$lib/api';
import { SIGNED_OUT_TEXT, dismiss, fail, notify, toasts } from './notices.svelte';
import { announceSignedOut, forgetMe, session } from './session.svelte';

afterEach(() => {
  vi.useRealTimers();
  toasts.splice(0);
  forgetMe();
});

describe('the cap of three', () => {
  it('pushes out the oldest passive notice, never one with a live action', () => {
    const undo = notify('Wave approved.', false, { ms: 5000, action: { label: 'Undo', run: () => {} } });
    for (const n of ['a', 'b', 'c', 'd', 'e']) notify(n);
    expect(toasts.some((t) => t.id === undo)).toBe(true);
    expect(toasts.filter((t) => !t.action).map((t) => t.text)).toEqual(['c', 'd', 'e']);
  });

  it('keeps every Undo while the notices around them come and go', () => {
    const ids = ['x', 'y', 'z', 'w'].map((n) => notify(`${n} approved.`, false, { ms: 5000, action: { label: 'Undo', run: () => {} } }));
    for (const n of ['a', 'b', 'c', 'd']) notify(n);
    expect(ids.every((id) => toasts.some((t) => t.id === id))).toBe(true);
  });

  it('counts errors apart from notices', () => {
    for (const n of ['a', 'b', 'c']) notify(n, true);
    notify('n');
    expect(toasts.filter((t) => t.error)).toHaveLength(3);
    dismiss(toasts[0].id);
  });
});

const answer = (code: string) => (async () => new Response(JSON.stringify({ errors: [{ message: 'Sign in to use this.', extensions: { code } }] }))) as unknown as typeof fetch;
const doc = { toString: () => 'query { x }' } as never;
const signedIn = () => (session.me = { id: 'a', createdAt: '', permissions: [], identities: [] } as never);

describe('a session that ends mid-use', () => {
  it('shows one notice with a Sign in link, whichever calls find out', async () => {
    signedIn();
    for (let i = 0; i < 3; i++) await expect(gql(doc, {}, answer('signed_out'))).rejects.toBeInstanceOf(ApiError);
    const notes = toasts.filter((t) => t.text === SIGNED_OUT_TEXT);
    expect(notes).toHaveLength(1);
    expect(notes[0].action?.label).toBe('Sign in');
    expect(notes[0].action?.href).toBeTruthy();
    expect(session.ended).toBe(true); // the header goes to Sign in; the page keeps its account
    expect(session.me).not.toBeNull();
    expect(session.known).toBe(true);
  });

  it('does not add a second red line for the failed call itself', async () => {
    signedIn();
    const err = await gql(doc, {}, answer('signed_out')).catch((e) => e);
    fail(err);
    expect(toasts.filter((t) => t.error)).toHaveLength(1);
  });

  it('stays until it is dismissed', () => {
    vi.useFakeTimers();
    signedIn();
    announceSignedOut();
    vi.advanceTimersByTime(120_000);
    expect(toasts.some((t) => t.text === SIGNED_OUT_TEXT)).toBe(true);
  });

  it('leaves a visitor alone: signed_out is what they are meant to get', async () => {
    session.me = null;
    await gql(doc, {}, answer('signed_out')).catch(() => {});
    expect(toasts).toHaveLength(0);
  });

  it('ignores other errors', async () => {
    signedIn();
    await gql(doc, {}, answer('forbidden')).catch(() => {});
    expect(toasts).toHaveLength(0);
    expect(session.me).not.toBeNull();
  });
});

describe('a toast held by the pointer or focus', () => {
  it('stops its countdown while held and finishes the time it had left on release', async () => {
    const { pauseToast, resumeToast } = await import('./notices.svelte');
    vi.useFakeTimers();
    const id = notify('Saved.');
    vi.advanceTimersByTime(2000);
    pauseToast(id);
    vi.advanceTimersByTime(60_000);
    expect(toasts.some((t) => t.id === id)).toBe(true);
    resumeToast(id);
    vi.advanceTimersByTime(1400);
    expect(toasts.some((t) => t.id === id)).toBe(true);
    vi.advanceTimersByTime(200);
    expect(toasts.some((t) => t.id === id)).toBe(false);
  });

  it('never holds a toast with a live Undo: its send time follows the clock', async () => {
    const { pauseToast } = await import('./notices.svelte');
    vi.useFakeTimers();
    const id = notify('Wave approved.', false, { ms: 5000, action: { label: 'Undo', run: () => {} } });
    pauseToast(id);
    vi.advanceTimersByTime(5001);
    expect(toasts.some((t) => t.id === id)).toBe(false);
  });
});
