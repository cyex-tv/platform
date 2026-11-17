import { describe, expect, it } from 'vitest';
import { clean } from './format';

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
