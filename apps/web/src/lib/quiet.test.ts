import { describe, expect, it, vi } from 'vitest';
import { quiet } from './quiet';

describe('quiet', () => {
  it('passes the value through', async () => {
    expect(await quiet(Promise.resolve(3), 'x')).toBe(3);
  });

  it('turns a failure into undefined and tells onFail', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const onFail = vi.fn();
    expect(await quiet(Promise.reject(new Error('down')), 'list', onFail)).toBeUndefined();
    expect(onFail).toHaveBeenCalledOnce();
    warn.mockRestore();
  });
});
