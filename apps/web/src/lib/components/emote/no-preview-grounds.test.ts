import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// The owner doesn't need the Dark / Light / Checker / Chat preview grounds: neither the public
// emote page nor the edit page has them; both show the plain size row.
const file = (p: string) => new URL(p, import.meta.url);

describe('emote previews', () => {
  it('has no preview ground switch left', () => {
    expect(existsSync(file('./PreviewStage.svelte'))).toBe(false);
  });

  it('the edit page shows the size row instead', () => {
    const page = readFileSync(file('../../../routes/(tool)/me/emotes/[id]/+page.svelte'), 'utf8');
    expect(page).not.toMatch(/PreviewStage|Checker/);
    expect(page).toMatch(/<ContactSheet label="Sizes"/);
  });
});
