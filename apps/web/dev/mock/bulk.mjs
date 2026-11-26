// Mock API: bulk reads and delta sync (ADR 0032). GraphQL `emotes(ids)`, `sets(ids)`,
// `channels(platform, ids)` and `setChanges(id, since)`; the REST lookups are in mock/extension.mjs.
// Caps as the API: 100 emotes, 100 channels, 25 sets (`too_many`). The mock's sets don't count
// versions: every set stays at its version (0), so `since` equal to it is an empty delta and anything
// else a full resync.
import { ALL } from './emotes.mjs';
import { CHANNELS } from './account.mjs';
import { err, findSet, mayView, setOut } from './sets.mjs';

export const SETS_LOOKUP = 25;
export const EMOTES_LOOKUP = 100;
export const CHANNELS_LOOKUP = 100;

/** Each id once, in the order given. */
export const distinct = (ids) => [...new Set((ids ?? []).map(String))];

/** A `SetChanges` for `since`; null for an unknown or hidden set. */
export function changesOf(id, since, session) {
  const s = findSet(id);
  if (!s || !mayView(s, session)) return null;
  const out = setOut(s);
  const version = out.version ?? 0;
  const full = since !== version;
  return { id: out.id, version, since, full, set: full ? out : null, name: out.name, added: [], renamed: [], updated: [], removedIds: [] };
}

/** Answers the bulk and delta operations; null for anything else. */
export function bulkAnswer(q, v, session) {
  if (/\bsets\s*\(\s*ids\s*:/.test(q)) {
    if ((v.ids ?? []).length > SETS_LOOKUP) return err('Look up at most 25 sets at once.', 'too_many');
    return { data: { sets: distinct(v.ids).map(findSet).filter((s) => s && mayView(s, session)).map(setOut) } };
  }
  if (/\bemotes\s*\(\s*ids\s*:/.test(q)) {
    if ((v.ids ?? []).length > EMOTES_LOOKUP) return err('Look up at most 100 emotes at once.', 'too_many');
    return { data: { emotes: distinct(v.ids).map((id) => ALL.find((e) => e.id === id)).filter(Boolean) } };
  }
  if (/\bchannels\s*\(\s*platform\s*:/.test(q)) {
    if ((v.ids ?? []).length > CHANNELS_LOOKUP) return err('Look up at most 100 channels at once.', 'too_many');
    const channels = distinct(v.ids).map((id) => CHANNELS.find((c) => c.platform === v.platform && c.platformChannelId === id)).filter(Boolean);
    return { data: { channels } };
  }
  if (/\bsetChanges\s*\(/.test(q)) return { data: { setChanges: changesOf(v.id, v.since, session) } };
  return null;
}
