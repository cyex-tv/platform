// The Collection tab and its opt-in are gone (owner, 2026/11/20: "just keep cosmetics"). The API
// keeps its fields (additive only); the site neither asks for nor offers them any more.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const src = (p: string) => readFileSync(resolve(__dirname, '../..', p), 'utf8');

describe('no Collection', () => {
  it('the site sends no collection operation and reads no collection flag', () => {
    const q = src('lib/queries.ts');
    expect(q).not.toMatch(/accountCollection|setCollectionPublic|collectionPublic/);
  });
  it('Settings → Account has no switch for it', () => {
    expect(src('routes/(tool)/me/settings/account/+page.svelte')).not.toMatch(/Collection|collection/);
  });
});
