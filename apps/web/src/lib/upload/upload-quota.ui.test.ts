import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import UploadDropHarness from '../test/UploadDropHarness.svelte';
import { session } from '$lib/tool/session.svelte';

const { gql, quota } = vi.hoisted(() => ({ gql: vi.fn(), quota: { pending: 3, pendingMax: 20 } }));
vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), beforeNavigate: () => {} }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/search') } }));
vi.mock('$lib/api', async (orig) => ({ ...(await orig<typeof import('$lib/api')>()), gql: (...a: unknown[]) => gql(...a), onSignedOut: () => () => {} }));
URL.createObjectURL = () => 'blob:x';
URL.revokeObjectURL = () => {};

const png = () => new File([new Uint8Array(40)], 'cat.png', { type: 'image/png', lastModified: 1 });

beforeEach(() => {
  gql.mockReset();
  gql.mockImplementation(async (doc: unknown) =>
    String(doc).includes('uploadQuota')
      ? { uploadQuota: { accountId: 'ME', uploadsThisHour: 0, uploadsPerHour: 20, ...quota } }
      : Promise.reject(new Error('not in this test')),
  );
  session.me = { id: 'ME', identities: [{ displayName: 'me' }], permissions: [], roles: [] } as unknown as typeof session.me;
});

describe('the upload form’s waiting count', () => {
  it('says how many of your emotes wait for review', async () => {
    Object.assign(quota, { pending: 3, pendingMax: 20 });
    render(UploadDropHarness);
    expect(await screen.findByText('3 of 20 waiting for review.')).toBeTruthy();
    const upload = screen.getByRole('button', { name: 'Upload' }) as HTMLButtonElement;
    expect(upload.disabled).toBe(false);
    expect(gql.mock.calls.find((c) => String(c[0]).includes('uploadQuota'))?.[1]).toEqual({ ownerId: null });
  });

  it('at the cap, Upload is off and the form says why', async () => {
    Object.assign(quota, { pending: 20, pendingMax: 20 });
    render(UploadDropHarness);
    expect(await screen.findByText('You have 20 emotes waiting for review. Upload more once some are reviewed.')).toBeTruthy();
    await fireEvent.drop(document.querySelector('.drop')!, { dataTransfer: { files: [png()] } });
    await waitFor(() => expect((screen.getByRole('button', { name: 'Upload' }) as HTMLButtonElement).disabled).toBe(true));
  });
});
