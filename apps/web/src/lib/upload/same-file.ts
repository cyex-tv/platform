// Before an upload: is this very file already a public emote? The browser hashes the file
// (SHA-256, as the API keys stored files) and asks emotesWithSameFile. The form then says "Looks
// like <emote> by <owner>" with a link, and the upload still goes ahead if the uploader wants:
// it is a hint, never a block. Any failure (no Web Crypto, a slow or failed call) just means no hint.
import { gql } from '$lib/api';
import { EMOTES_WITH_SAME_FILE } from '$lib/queries';

export type SameFileEmote = { id: string; name: string; owner?: { id: string; displayName: string } | null };

/** The file's SHA-256 as 64 lower-case hex digits, or null where Web Crypto isn't there. */
export async function sha256Hex(file: Blob): Promise<string | null> {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) return null;
  const digest = await subtle.digest('SHA-256', await file.arrayBuffer());
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** Public emotes made from the same file as `file` (oldest first), or [] when none or unknown. */
export async function sameFileEmotes(file: Blob): Promise<SameFileEmote[]> {
  try {
    const sha = await sha256Hex(file);
    if (!sha) return [];
    return (await gql(EMOTES_WITH_SAME_FILE, { sha256: sha })).emotesWithSameFile;
  } catch {
    return [];
  }
}

/** The words around the link: "by <owner>", and how many more there are. */
export function sameFileLine(found: readonly SameFileEmote[]): { by: string; more: string } {
  const first = found[0];
  const by = first?.owner?.displayName ? `by ${first.owner.displayName}` : '';
  const rest = found.length - 1;
  return { by, more: rest > 0 ? ` and ${rest} more` : '' };
}
