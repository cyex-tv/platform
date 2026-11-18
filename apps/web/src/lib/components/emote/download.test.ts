import { describe, expect, it } from 'vitest';
import { bytes, ext, fileName, fileUrl, formatsAt, hasBoth, pick, settle, sizes, type DownloadFile } from './download';

const f = (scale: number, format: 'WEBP' | 'AVIF', isStatic = false): DownloadFile => ({
  name: `${scale}x${isStatic ? '_static' : ''}.${format.toLowerCase()}`,
  format,
  scale,
  width: 32 * scale,
  height: 32 * scale,
  size: 1000,
  isStatic,
});
const still = [1, 2, 4].flatMap((s) => [f(s, 'WEBP'), f(s, 'AVIF')]);
const animated = [1, 2].flatMap((s) => [f(s, 'WEBP'), f(s, 'WEBP', true), f(s, 'AVIF', true)]);

describe('download choices', () => {
  it('lists the sizes and formats the files have', () => {
    expect(sizes(still, false)).toEqual([1, 2, 4]);
    expect(formatsAt(still, false, 2)).toEqual(['WEBP', 'AVIF']);
    expect(hasBoth(still)).toBe(false);
    expect(hasBoth(animated)).toBe(true);
  });
  it('animated emotes: WebP only moving, WebP and AVIF still', () => {
    expect(formatsAt(animated, false, 1)).toEqual(['WEBP']);
    expect(formatsAt(animated, true, 1)).toEqual(['WEBP', 'AVIF']);
  });
  it('picks the file, or null', () => {
    expect(pick(animated, { scale: 2, format: 'WEBP', still: true })?.name).toBe('2x_static.webp');
    expect(pick(animated, { scale: 2, format: 'AVIF', still: false })).toBeNull();
    expect(pick(still, { scale: 3, format: 'WEBP', still: false })).toBeNull();
  });
  it('settles a choice the new kind does not have', () => {
    expect(settle(animated, { scale: 2, format: 'AVIF', still: false })).toEqual({ scale: 2, format: 'WEBP', still: false });
    expect(settle(still, { scale: 3, format: 'AVIF', still: false })).toEqual({ scale: 4, format: 'AVIF', still: false });
    expect(settle(animated, { scale: 4, format: 'WEBP', still: false }).scale).toBe(2);
  });
});

describe('file names and urls', () => {
  it('joins host and file', () => expect(fileUrl('https://cdn.cyex.tv/emotes/ab', f(2, 'WEBP'))).toBe('https://cdn.cyex.tv/emotes/ab/2x.webp'));
  it('reads the extension', () => expect(ext(f(1, 'AVIF', true))).toBe('avif'));
  it('names the saved file <name>-<size>.<ext>', () => {
    expect(fileName('Pog', f(2, 'WEBP'), false)).toBe('Pog-2x.webp');
    expect(fileName('Pog', f(4, 'AVIF'), true)).toBe('Pog-4x.avif');
  });
  it('marks the still frame of an animated emote', () => expect(fileName('Pog', f(1, 'WEBP', true), true)).toBe('Pog-1x-still.webp'));
  it('cleans names', () => {
    expect(fileName('a/b\\c:d', f(1, 'WEBP'), false)).toBe('a_b_c_d-1x.webp');
    expect(fileName('..hidden name', f(1, 'WEBP'), false)).toBe('hidden_name-1x.webp');
    expect(fileName('', f(1, 'WEBP'), false)).toBe('emote-1x.webp');
    expect(fileName('x'.repeat(200), f(1, 'WEBP'), false).length).toBeLessThan(95);
  });
  it('formats bytes', () => {
    expect(bytes(0)).toBe('');
    expect(bytes(900)).toBe('900 B');
    expect(bytes(20480)).toBe('20 KB');
    expect(bytes(3 * 1024 * 1024)).toBe('3.0 MB');
  });
});
