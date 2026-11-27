// Mock API: merging emotes (ADR 0040). `emoteMergePreview` and `mergeEmote` (staff with
// emotes.merge; the mock admin holds it), and the alias an old ID becomes: `emote` and
// `emotes(ids)` answer the final target's data with the asked ID and `mergedInto` set. The retired
// source leaves ALL, MINE and search; its set entries move to the target (names kept), a set
// that held both keeps the target's entry; both owners get `emote.merged`, and both emotes' history a line.
import { ME_ID, ME_SUMMARY } from './config.mjs';
import { ALL, MINE } from './emotes.mjs';
import { NOTES } from './account.mjs';
import { PENDING, logEmote } from './review.mjs';
import { ALL_SETS, anyEmote, err } from './sets.mjs';

/** Retired emotes: source id → the emote it was merged into (which may be merged itself later). */
export const MERGED = new Map();

const FORBIDDEN = 'This needs the emotes.merge permission.';
const WORDS = {
  merge_into_self: 'An emote can’t be merged into itself.',
  merge_target_unavailable: 'That emote is deleted, waiting for review, rejected or taken down, so it can’t take another emote’s place.',
};

/** The ID an ID leads to after every merge, and whether any merge was followed. */
export function finalOf(id) {
  let at = id;
  let followed = false;
  for (let i = 0; i < 50 && MERGED.has(at); i++) {
    at = MERGED.get(at);
    followed = true;
  }
  return { id: at, followed };
}

/** `emote` as the API answers it: a merged ID answers its final target with the asked ID and `mergedInto`. */
export function emoteAs(id, find) {
  const f = finalOf(id);
  const e = find(f.id);
  if (!e) return null;
  return f.followed ? { ...e, id, mergedInto: f.id } : { ...e, mergedInto: null };
}

const allSets = ALL_SETS;

function plan(source, target) {
  let moves = 0;
  const collapses = [];
  for (const s of allSets()) {
    const m = s.emotes.find((x) => x.emote.id === source.id);
    if (!m) continue;
    const kept = s.emotes.find((x) => x.emote.id === target.id);
    if (kept) collapses.push({ setId: s.id, setName: s.name, keptName: kept.name, droppedName: m.name });
    else moves += 1;
  }
  return { moves, collapses };
}

/** The checks both operations share: { source, target, followed } or { bad } (an error answer). */
function check(v, ADMIN) {
  if (!ADMIN) return { bad: err(FORBIDDEN, 'forbidden') };
  if (MERGED.has(v.id)) return { bad: err('That emote was merged into another already.', 'already_merged') };
  const source = anyEmote(v.id);
  if (!source) return { bad: err('Not found.', 'not_found') };
  const f = finalOf(v.intoId);
  const target = anyEmote(f.id);
  if (!target) return { bad: err('Not found.', 'not_found') };
  if (target.id === source.id) return { bad: err(WORDS.merge_into_self, 'merge_into_self') };
  if (target.state !== 'APPROVED') return { bad: err(WORDS.merge_target_unavailable, 'merge_target_unavailable') };
  return { source, target, followed: f.followed };
}

export function mergeAnswer(op, v, ADMIN) {
  if (op === 'EmoteMergePreview') {
    const c = check(v, ADMIN);
    if (c.bad) return c.bad;
    const { moves, collapses } = plan(c.source, c.target);
    return { data: { emoteMergePreview: { source: c.source, target: c.target, followed: c.followed, moves, collapses } } };
  }
  if (op !== 'MergeEmote') return null;
  const c = check(v, ADMIN);
  if (c.bad) return c.bad;
  const { source, target } = c;
  const { moves, collapses } = plan(source, target);
  for (const s of allSets()) {
    const i = s.emotes.findIndex((x) => x.emote.id === source.id);
    if (i < 0) continue;
    if (s.emotes.some((x) => x.emote.id === target.id)) s.emotes.splice(i, 1);
    else s.emotes[i] = { ...s.emotes[i], emote: target };
  }
  for (const list of [ALL, MINE, PENDING]) {
    const i = list.indexOf(source);
    if (i >= 0) list.splice(i, 1);
  }
  // Anything merged into the source earlier now points at the target.
  for (const [k, to] of MERGED) if (to === source.id) MERGED.set(k, target.id);
  MERGED.set(source.id, target.id);
  target.channelCount += source.channelCount;
  const after = { source_id: source.id, target_id: target.id, source_name: source.name, target_name: target.name, moved: moves,
    collapsed: collapses.map((x) => ({ set_id: x.setId, kept_name: x.keptName, dropped_name: x.droppedName })), via: 'emotes.merge' };
  logEmote(source.id, 'emote.merged', after);
  logEmote(target.id, 'emote.merged', after);
  const mine = (e) => e.owner?.id === ME_ID;
  const role = mine(source) && mine(target) ? 'both' : mine(source) ? 'source' : mine(target) ? 'target' : null;
  if (role)
    NOTES.unshift({ id: `01JD8XMRG${String(Date.now()).padStart(17, '0')}`, kind: 'emote.merged', read: false, createdAt: new Date().toISOString(),
      payload: JSON.stringify({ role, source_id: source.id, source_name: source.name, target_id: target.id, target_name: target.name, emote_id: target.id, name: target.name,
        actor_id: ME_ID, actor_name: ME_SUMMARY.displayName, moved: moves, collapsed: collapses.length }) });
  return { data: { mergeEmote: { sourceId: source.id, target, moved: moves, collapsed: collapses } } };
}
