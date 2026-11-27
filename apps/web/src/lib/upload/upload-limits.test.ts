import { describe, expect, it } from 'vitest';


describe('the limits list under the drop zone', () => {
  it('says the built-in numbers until the account’s own are known', async () => {
    const { uploadLimits, UPLOAD_LIMITS } = await import('./upload-links');
    expect(uploadLimits()).toContain('Up to 8 MB');
    expect(uploadLimits()).toContain('Up to 1000 × 1000 px and 1000 frames');
    expect(uploadLimits()).toContain('Up to 50 fps');
    expect(uploadLimits()).toContain('Between 1:3 tall and 4:1 wide');
    expect([...UPLOAD_LIMITS]).toEqual(uploadLimits());
  });

  it('says the numbers the API gave for the account', async () => {
    const { uploadLimits } = await import('./upload-links');
    const lines = uploadLimits({ maxBytes: 12.5 * 1024 * 1024, maxSide: 512, maxFrames: 200, maxFps: 30, maxAspect: 2, maxTall: 2 });
    expect(lines).toContain('Up to 12,5 MB');
    expect(lines).toContain('Up to 512 × 512 px and 200 frames');
    expect(lines).toContain('Up to 30 fps');
    expect(lines).toContain('Between 1:2 tall and 2:1 wide');
  });
});
