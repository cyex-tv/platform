import { describe, expect, it } from 'vitest';
import { clean, iso } from './format';

describe('clean', () => {
  it('removes bidi embeddings, overrides and isolates', () => {
    expect(clean('ab‮cd')).toBe('abcd');
    expect(clean('‪‫‬‭‮x')).toBe('x');
    expect(clean('⁦a⁧b⁨c⁩')).toBe('abc');
  });

  it('keeps everything else, marks and joiners included', () => {
    expect(clean('lumen_fox')).toBe('lumen_fox');
    expect(clean('שלום عربي')).toBe('שלום عربي');
    expect(clean('a‍b‎c‏d')).toBe('a‍b‎c‏d');
    expect(clean('👩‍💻 nora')).toBe('👩‍💻 nora');
  });

  it('treats a missing name as empty', () => {
    expect(clean(null)).toBe('');
    expect(clean(undefined)).toBe('');
    expect(clean('')).toBe('');
  });
});

describe('iso', () => {
  it('wraps the cleaned name in an isolate', () => {
    expect(iso('lumen_fox')).toBe('⁨lumen_fox⁩');
    expect(iso(null)).toBe('⁨⁩');
  });

  it("can't be closed early or overridden by the name", () => {
    // A name that brings its own pop-isolate and an override: both are dropped first.
    expect(iso('a⁩‮b')).toBe('⁨ab⁩');
    expect(`${iso('‮gnp.exe')} added.`).toBe('⁨gnp.exe⁩ added.');
  });
});
