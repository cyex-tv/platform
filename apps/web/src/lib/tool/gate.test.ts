import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Gate } from './gate.svelte';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('Gate', () => {
  it('two clicks 80 ms apart run the action once', async () => {
    const g = new Gate(400);
    const call = vi.fn(async () => {
      await new Promise((r) => setTimeout(r, 30));
      return 'ok';
    });
    const first = g.run(call);
    await vi.advanceTimersByTimeAsync(80);
    const second = g.run(call);
    await vi.advanceTimersByTimeAsync(100);
    expect(await first).toBe('ok');
    expect(await second).toBeUndefined();
    expect(call).toHaveBeenCalledTimes(1);
  });
  it('stays shut for the rest after the answer, then opens', async () => {
    const g = new Gate(400);
    const call = vi.fn(async () => 1);
    await g.run(call);
    expect(g.busy).toBe(true);
    await g.run(call);
    expect(call).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(401);
    expect(g.busy).toBe(false);
    await g.run(call);
    expect(call).toHaveBeenCalledTimes(2);
  });
  it('opens again after a failure (after the rest)', async () => {
    const g = new Gate(0);
    await expect(g.run(async () => Promise.reject(new Error('x')))).rejects.toThrow('x');
    expect(g.busy).toBe(false);
  });
});
