import { afterEach, describe, expect, it, vi } from 'vitest';
import { copy, useCopied } from './clipboard.svelte';

describe('clipboard', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('copies through the clipboard API', async () => {
    const writeText = vi.fn(async () => {});
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    expect(await copy('https://cyex.tv/emotes/1')).toBe(true);
    expect(writeText).toHaveBeenCalledWith('https://cyex.tv/emotes/1');
  });

  it('reports a failure when nothing can copy', async () => {
    vi.stubGlobal('navigator', { clipboard: { writeText: async () => Promise.reject(new Error('denied')) } });
    expect(await copy('x')).toBe(false);
  });

  it('holds copied or failed for the given time, then goes idle', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('navigator', { clipboard: { writeText: async () => {} } });
    const c = useCopied(1600);
    expect(c.state).toBe('idle');
    await c.copy('x');
    expect(c.copied).toBe(true);
    vi.advanceTimersByTime(1599);
    expect(c.state).toBe('copied');
    vi.advanceTimersByTime(1);
    expect(c.state).toBe('idle');
    vi.stubGlobal('navigator', {});
    await c.copy('x');
    expect(c.failed).toBe(true);
  });
});
