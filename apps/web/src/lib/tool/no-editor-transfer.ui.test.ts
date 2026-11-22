// Owner decision 2026/11/22 (ADR 0014): nobody can ask for someone else's emote to be transferred.
// Only the owner gives an emote away; the editor permission that allowed asking is retired. So the
// permissions picker has no Transfer box, no page offers an editor a Transfer or Hand-over action,
// and nothing answers transfer requests any more (no Accept, Decline or Cancel request).
import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import EditorDialog from './EditorDialog.svelte';

const files = import.meta.glob('/src/**/*.{svelte,ts}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const source = Object.entries(files).filter(([f]) => !/\.test\.ts$|\/lib\/gql\//.test(f));

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
  };
});

describe('no transfer for editors', () => {
  it('the permissions picker offers no Transfer, even to an owner holding every flag', () => {
    const held = new Set(['EMOTES', 'SETS', 'EDITORS', 'PROFILE', 'CHANNELS', 'EMOTE_UPLOAD', 'EMOTE_MANAGE', 'EMOTE_TRANSFER', 'BILLING_PAYMENT', 'BILLING_SUBSCRIPTION']);
    const r = render(EditorDialog, { held: held as never, sets: [], onsave: async () => {}, describe: () => 'no' });
    (r.component as unknown as { open: (...a: unknown[]) => void }).open({ id: 'U', displayName: 'Ana' }, { permissions: [], scope: { all: true, ids: [] } }, 'add');
    const boxes = screen.getAllByRole('checkbox').map((b) => b.closest('label')?.textContent ?? b.getAttribute('aria-label') ?? '');
    expect(boxes.length).toBeGreaterThan(0);
    expect(boxes.some((t) => /upload/i.test(t))).toBe(true);
    expect(boxes.some((t) => /transfer|hand.?over|give/i.test(t))).toBe(false);
    expect(document.body.textContent).not.toMatch(/transfer/i);
  });

  it('nothing on the site asks for, accepts, declines or cancels a transfer request', () => {
    for (const [f, text] of source) {
      expect(text, f).not.toMatch(/ACCEPT_EMOTE_TRANSFER|DECLINE_EMOTE_TRANSFER|CANCEL_EMOTE_TRANSFER|EMOTE_TRANSFER_REQUESTS|acceptEmoteTransfer|emoteTransferRequests/);
      expect(text, f).not.toMatch(/Cancel request|It moves only if you accept|has to accept first/);
    }
  });

  it('the emote editor offers Give only to the owner, and editor grants never unlock it', () => {
    const page = files['/src/routes/(tool)/me/emotes/[id]/+page.svelte'];
    expect(page).toMatch(/const canGive = \$derived\(mine\);/);
    expect(page).not.toMatch(/EMOTE_TRANSFER/);
    const give = page.indexOf('<Section title="Hand-over"');
    expect(give).toBeGreaterThan(0);
    expect(page.lastIndexOf('{#if canGive}', give)).toBeGreaterThan(page.lastIndexOf('{/if}', give));
    // Nowhere else does a grant's transfer flag turn on an action.
    for (const [f, text] of source) if (!f.endsWith('/lib/tool/editors.ts')) expect(text, f).not.toMatch(/'EMOTE_TRANSFER'/);
  });
});
