import { describe, expect, it } from 'vitest';
import { browserOf } from './pick';

describe('browserOf', () => {
  const ua = {
    chrome: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    edge: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0',
    firefox: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0',
    safari: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15',
    ios: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/131.0 Mobile/15E148 Safari/604.1',
    android: 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36',
  };
  it('names the browsers that take the extension', () => {
    expect(browserOf(ua.chrome)).toBe('chrome');
    expect(browserOf(ua.edge)).toBe('chrome');
    expect(browserOf(ua.firefox)).toBe('firefox');
    expect(browserOf(ua.safari)).toBe('other');
    expect(browserOf(ua.ios)).toBe('other');
    expect(browserOf(ua.android)).toBe('other');
  });
});
