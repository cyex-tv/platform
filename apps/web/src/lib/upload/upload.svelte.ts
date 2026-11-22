// The upload dialog, open from anywhere: the header's Upload button, an empty "Your emotes", any
// old link to /me/upload or /upload. One dialog for the whole site lives in the root layout; what
// you picked stays while you browse, so closing it and opening it again finds it where you left it.
// There is no upload page (owner, 2026/11/22): /me/upload and /upload redirect to the page you came
// from with `?upload` in the address, and the root layout opens the dialog there (`uploadParam`).

export * from './upload-links';

/** `for`: the account to upload as (one you edit with Upload, ADR 0014); null for yourself. */
export const upload = $state<{ open: boolean; for: string | null }>({ open: false, for: null });

export function openUpload(forId: string | null = null) {
  upload.for = forId;
  upload.open = true;
}

export function closeUpload() {
  upload.open = false;
}
