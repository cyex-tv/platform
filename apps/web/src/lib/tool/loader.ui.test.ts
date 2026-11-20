import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import LoaderHarness from '../test/LoaderHarness.svelte';
import type { LoadCtx, Loader } from './loader.svelte';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
const { ApiError } = await import('$lib/api');
const { toasts } = await import('./notices.svelte');
const { runner } = await import('./loader.svelte');

/** A request the test settles by hand. */
function pending() {
  let resolve!: (v: string) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<string>((res, rej) => ((resolve = res), (reject = rej)));
  return { promise, resolve, reject };
}
const state = () => screen.getByTestId('state').textContent;

describe('loader', () => {
  it('starts loading, then shows the data', async () => {
    const run = vi.fn(async (id: string) => `data ${id}`);
    render(LoaderHarness, { id: 'a', run });
    expect(state()).toBe('loading|-|-|-');
    await vi.waitFor(() => expect(state()).toBe('idle|data a|-|-'));
  });

  it('drops a stale response: the newest request wins even when the old one answers last', async () => {
    const first = pending();
    const second = pending();
    const signals: AbortSignal[] = [];
    const run = vi.fn((id: string, ctx: LoadCtx) => {
      signals.push(ctx.signal);
      return id === 'a' ? first.promise : second.promise;
    });
    const { rerender } = render(LoaderHarness, { id: 'a', run });
    await rerender({ id: 'b', run });
    await vi.waitFor(() => expect(run).toHaveBeenCalledTimes(2));
    expect(signals[0].aborted).toBe(true); // the replaced request is cancelled
    expect(signals[1].aborted).toBe(false);

    second.resolve('answer b');
    await vi.waitFor(() => expect(state()).toBe('idle|answer b|-|-'));
    first.resolve('late answer a');
    await new Promise((r) => setTimeout(r, 20));
    expect(state()).toBe('idle|answer b|-|-');
  });

  it('a stale failure is dropped too', async () => {
    const first = pending();
    const run = vi.fn((id: string) => (id === 'a' ? first.promise : Promise.resolve('b ok')));
    const { rerender } = render(LoaderHarness, { id: 'a', run });
    await rerender({ id: 'b', run });
    await vi.waitFor(() => expect(state()).toBe('idle|b ok|-|-'));
    first.reject(new ApiError('old failure', 'server'));
    await new Promise((r) => setTimeout(r, 20));
    expect(state()).toBe('idle|b ok|-|-');
  });

  it('a new input clears the old data while it loads', async () => {
    const second = pending();
    const run = vi.fn((id: string) => (id === 'a' ? Promise.resolve('data a') : second.promise));
    const { rerender } = render(LoaderHarness, { id: 'a', run });
    await vi.waitFor(() => expect(state()).toBe('idle|data a|-|-'));
    await rerender({ id: 'b', run });
    await vi.waitFor(() => expect(state()).toBe('loading|-|-|-'));
  });

  it('says why it failed: the message and the kind of problem', async () => {
    const run = vi.fn(async () => {
      throw new ApiError('No such emote.', 'not_found');
    });
    render(LoaderHarness, { id: 'a', run });
    await vi.waitFor(() => expect(state()).toBe('idle|-|No such emote.|missing'));
  });

  it('uses a plain line for something that is not an Error, and survives a synchronous throw', async () => {
    const run = vi.fn((): Promise<string> => {
      throw new Error('boom');
    });
    render(LoaderHarness, { id: 'a', run });
    await vi.waitFor(() => expect(state()).toContain('boom'));
  });

  it('reload keeps what is on screen while it asks again, then swaps it', async () => {
    const again = pending();
    let calls = 0;
    const run = vi.fn(() => (++calls === 1 ? Promise.resolve('v1') : again.promise));
    let loader!: Loader<string>;
    render(LoaderHarness, { id: 'a', run, expose: (l: Loader<string>) => (loader = l) });
    await vi.waitFor(() => expect(state()).toBe('idle|v1|-|-'));
    const done = loader.reload();
    await vi.waitFor(() => expect(state()).toBe('loading|v1|-|-'));
    again.resolve('v2');
    await done;
    await vi.waitFor(() => expect(state()).toBe('idle|v2|-|-'));
  });

  it('a failed reload keeps the old data off the error and shows the error', async () => {
    let calls = 0;
    const run = vi.fn(() => (++calls === 1 ? Promise.resolve('v1') : Promise.reject(new ApiError('down', 'server'))));
    let loader!: Loader<string>;
    render(LoaderHarness, { id: 'a', run, expose: (l: Loader<string>) => (loader = l) });
    await vi.waitFor(() => expect(state()).toBe('idle|v1|-|-'));
    await loader.reload();
    await vi.waitFor(() => expect(state()).toBe('idle|v1|down|server'));
  });

  it('set replaces the data in place', async () => {
    let loader!: Loader<string>;
    render(LoaderHarness, { id: 'a', run: async () => 'v1', expose: (l: Loader<string>) => (loader = l) });
    await vi.waitFor(() => expect(state()).toBe('idle|v1|-|-'));
    loader.set('edited');
    await vi.waitFor(() => expect(state()).toBe('idle|edited|-|-'));
  });

  it('aborts the request when the page closes', async () => {
    const waiting = pending();
    let signal!: AbortSignal;
    const { unmount } = render(LoaderHarness, {
      id: 'a',
      run: (_id: string, ctx: LoadCtx) => {
        signal = ctx.signal;
        return waiting.promise;
      },
    });
    await vi.waitFor(() => expect(signal).toBeDefined());
    unmount();
    expect(signal.aborted).toBe(true);
  });
});

describe('runner', () => {
  it('ignores a second call while the first runs, and says ok on success', async () => {
    toasts.splice(0);
    const r = runner();
    const gate = pending();
    const first = r.run(() => gate.promise, 'Saved.');
    expect(r.busy).toBe(true);
    expect(await r.run(async () => {}, 'never')).toBe(false);
    gate.resolve('x');
    expect(await first).toBe(true);
    expect(r.busy).toBe(false);
    expect(toasts.map((t) => t.text)).toEqual(['Saved.']);
    toasts.splice(0);
  });

  it('shows a failure as an error toast and gives the button back', async () => {
    toasts.splice(0);
    const r = runner();
    expect(await r.run(async () => Promise.reject(new Error('nope')))).toBe(false);
    expect(r.busy).toBe(false);
    expect(toasts.map((t) => [t.text, t.error])).toEqual([['nope', true]]);
    toasts.splice(0);
  });
});
