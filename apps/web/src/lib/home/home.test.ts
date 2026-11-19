import { describe, expect, it } from 'vitest';
import { capable, type Env } from './capable';
import { PINNED, isInstaller, newer, pick, validCache } from './deck-release';
import { STORES, listed, storeFor } from './stores';

const DL = 'https://github.com/cyex-app/cyex-deck/releases/download/';
const rel = (tag: string, at: string, o: { draft?: boolean; pre?: boolean; assets?: { name: string; url: string }[] } = {}) => ({
  tag_name: tag,
  draft: !!o.draft,
  prerelease: o.pre ?? true,
  published_at: at,
  assets: (o.assets ?? [{ name: `cyex-deck_${tag.slice(1)}_x64-setup.exe`, url: `${DL}${tag}/cyex-deck_${tag.slice(1)}_x64-setup.exe` }]).map((a) => ({
    name: a.name,
    browser_download_url: a.url,
  })),
});

describe('deck release', () => {
  it('takes the newest release with an installer, pre-releases counted, drafts skipped', () => {
    const list = [
      rel('v0.1.8', '2026-11-07T22:32:00Z'),
      rel('v0.1.10', '2026-11-20T10:00:00Z', { draft: true }),
      rel('v0.1.9', '2026-11-13T21:40:00Z'),
      rel('nightly', '2026-11-19T00:00:00Z'),
    ];
    expect(pick(list)).toEqual({ url: `${DL}v0.1.9/cyex-deck_0.1.9_x64-setup.exe`, version: '0.1.9', prerelease: true });
  });

  it('skips a release without a setup exe, and a lookalike from elsewhere', () => {
    const list = [
      rel('v0.2.0', '2026-12-01T00:00:00Z', { assets: [{ name: 'cyex-deck_0.2.0_x64-setup.exe', url: 'https://evil.test/cyex-deck_0.2.0_x64-setup.exe' }] }),
      rel('v0.1.11', '2026-11-30T00:00:00Z', { assets: [{ name: 'notes.txt', url: `${DL}v0.1.11/notes.txt` }] }),
      rel('v0.1.10', '2026-11-25T00:00:00Z', { pre: false }),
    ];
    expect(pick(list)?.version).toBe('0.1.10');
    expect(pick(list)?.prerelease).toBe(false);
    expect(pick('nope')).toBeNull();
    expect(pick([])).toBeNull();
  });

  it('accepts only the deck repo’s own installers', () => {
    expect(isInstaller(PINNED.url)).toBe(true);
    expect(isInstaller(`${DL}v1/x_x64-setup.exe?x=1`)).toBe(true);
    expect(isInstaller('http://github.com/cyex-app/cyex-deck/releases/download/v1/x_x64-setup.exe')).toBe(false);
    expect(isInstaller('https://github.com/other/repo/releases/download/v1/x_x64-setup.exe')).toBe(false);
    expect(isInstaller(`${DL}../../../other/x_x64-setup.exe`)).toBe(false);
    expect(isInstaller(`${DL}%2e%2e/x_x64-setup.exe`)).toBe(false);
    expect(isInstaller(`https://user@github.com/cyex-app/cyex-deck/releases/download/v1/x_x64-setup.exe`)).toBe(false);
    expect(isInstaller(`${DL}v1/x.msi`)).toBe(false);
    expect(isInstaller(42)).toBe(false);
  });

  it('compares versions the way the deck numbers them', () => {
    expect(newer('0.1.10', '0.1.9')).toBe(true);
    expect(newer('0.1.09', '0.1.9')).toBe(false);
    expect(newer('0.1.9', '0.1.9-beta.1')).toBe(true);
    expect(newer('0.1.9-beta.2', '0.1.9-beta.1')).toBe(true);
    expect(newer('v0.2.0', '0.1.99')).toBe(true);
    expect(newer('', '0.1.0')).toBe(false);
  });

  it('reads back only a well-formed kept answer', () => {
    const ok = { at: 1, url: PINNED.url, version: '0.1.09', prerelease: true };
    expect(validCache(ok)).toEqual(ok);
    expect(validCache({ ...ok, url: 'https://evil.test/a_x64-setup.exe' })).toBeNull();
    expect(validCache({ ...ok, at: 'soon' })).toBeNull();
    expect(validCache(null)).toBeNull();
  });
});

describe('extension stores', () => {
  const ua = {
    chrome: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    edge: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0',
    opera: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 OPR/116.0.0.0',
    firefox: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0',
    safari: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15',
    android: 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36',
  };
  it('names the browser the page is open in', () => {
    expect(storeFor(ua.chrome)).toBe('chrome');
    expect(storeFor(ua.chrome, true)).toBe('brave');
    expect(storeFor(ua.edge)).toBe('edge');
    expect(storeFor(ua.opera)).toBe('opera');
    expect(storeFor(ua.firefox)).toBe('firefox');
    expect(storeFor(ua.safari)).toBeNull();
    expect(storeFor(ua.android)).toBeNull();
  });

  it('has one entry per browser, each going to /download until its listing is live', () => {
    expect(new Set(STORES.map((s) => s.key)).size).toBe(STORES.length);
    for (const s of STORES) expect(listed(s) || s.url === `/download#${s.key}`).toBe(true);
    expect(listed({ ...STORES[0], url: 'https://chromewebstore.google.com/detail/x' })).toBe(true);
  });
});

describe('3D logo gate', () => {
  const fine: Env = { reducedMotion: false, saveData: false, memory: 8, cores: 8, webgl: true };
  it('runs on a capable device, and where the browser doesn’t say how much it has', () => {
    expect(capable(fine)).toBe(true);
    expect(capable({ ...fine, memory: undefined, cores: undefined })).toBe(true);
  });
  it('keeps the flat logo for reduced motion, data saver, small devices and no WebGL', () => {
    expect(capable({ ...fine, reducedMotion: true })).toBe(false);
    expect(capable({ ...fine, saveData: true })).toBe(false);
    expect(capable({ ...fine, memory: 2 })).toBe(false);
    expect(capable({ ...fine, cores: 2 })).toBe(false);
    expect(capable({ ...fine, webgl: false })).toBe(false);
  });
  it('doesn’t probe WebGL when a cheaper check already said no', () => {
    let probed = false;
    const env = { ...fine, reducedMotion: true, get webgl() { probed = true; return true; } };
    capable(env);
    expect(probed).toBe(false);
  });
});
