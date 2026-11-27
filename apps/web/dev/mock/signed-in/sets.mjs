// Mock API: signed in/sets. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY } from '../config.mjs';
import { member } from '../emotes.mjs';
import { CHANNELS } from '../account.mjs';
import { SETS } from '../set-data.mjs';
import { EDITOR_CHANNELS, EDITOR_OF, SHARED_SETS, editorChannels, editorOfOut, editorSets, flagsOn } from '../editors.mjs';
import { PUBLIC_ONLY, capRange, err, findEmote, findSet, forbidden, mayView, personalSlots, setOut, setRights } from '../sets.mjs';
import { badCursor, pageOf } from '../cursor.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
let newSets = 0;

export function signedInSets(op, body, v, ADMIN) {
  switch (op) {
    case 'MySets':
      return { data: { mySets: SETS.map(setOut), editorSets: editorSets().map(setOut), editorOf: editorOfOut() } };
    case 'MySetNames':
      return { data: { mySets: SETS.map((s) => ({ id: s.id, name: s.name, kind: s.kind, emotes: s.emotes.map((m) => ({ emote: { id: m.emote.id } })) })) } };
    case 'SetEditors': {
      const s = findSet(v.id);
      if (!s || !mayView(s, true)) return err('That set doesn’t exist.', 'not_found');
      if (!setRights(s, ADMIN).emotes) return forbidden();
      return { data: { setEditors: s.editors } };
    }
    case 'SetCapacityRange':
      return { data: { setCapacityRange: capRange(findSet(v.id), ADMIN) } };
    // The set menu: what you may do with one set (capacity range, grants, the channels you change).
    case 'SetMenu':
      return { data: { setCapacityRange: capRange(findSet(v.id), ADMIN), editorOf: editorOfOut(), myChannels: CHANNELS, editorChannels: editorChannels() } };
    case 'EditSet': {
      const s = findSet(v.id);
      const channels = { myChannels: CHANNELS, editorChannels: editorChannels() };
      // Private sets of others answer as if they did not exist; a visible set you can't edit is forbidden.
      if (!s || !mayView(s, true)) return { data: { set: null, setEditors: [], editorOf: editorOfOut(), ...channels } };
      if (!setRights(s, ADMIN).emotes) return forbidden();
      return { data: { set: { ...setOut(s), emotes: undefined }, setEditors: s.editors, editorOf: editorOfOut(), ...channels } };
    }
    // setEmotes: a page of a set's members, oldest first; query keeps the name in the set or the
    // emote's own name (any case, literally); perPage is 1 to 200, default 120.
    case 'SetEmotes':
    case 'SetMembers': {
      const s = findSet(v.id);
      if (!s || !mayView(s, true)) return { data: { setEmotes: { total: 0, items: [], next: null } } };
      const q = (v.query ?? '').trim().toLowerCase();
      const all = [...s.emotes].sort((a, b) => a.addedAt.localeCompare(b.addedAt))
        .filter((m) => !q || m.name.toLowerCase().includes(q) || m.emote.name.toLowerCase().includes(q));
      const p = pageOf(all, v, 120, 200);
      if (!p) return badCursor();
      return { data: { setEmotes: { total: all.length, items: p.items, next: p.next } } };
    }
    case 'CreateSet': {
      const ownerId = v.input.ownerId || ME_ID;
      if (!flagsOn(ownerId).includes('SETS'))
        return err('Only the account’s owner and its editors with the sets permission can make sets for it.', 'forbidden');
      const owner = ownerId === ME_ID ? ME_SUMMARY : EDITOR_OF.find((g) => g.account.id === ownerId).account;
      const list = ownerId === ME_ID ? SETS : SHARED_SETS;
      if (v.input.kind === 'PERSONAL' && [...SETS, ...SHARED_SETS].some((s) => s.kind === 'PERSONAL' && s.owner?.id === ownerId))
        return err('You already have a personal set.', 'personal_set_exists');
      const visibility = v.input.visibility ?? 'LISTED';
      if (visibility !== 'LISTED' && !ADMIN) return PUBLIC_ONLY();
      if (v.input.kind === 'PERSONAL' && visibility === 'PRIVATE')
        return err('Personal sets can’t be private, and the global set is always listed.', 'bad_visibility');
      const s = { id: `01JD8XSETNEW${String(++newSets).padStart(14, "0")}`, name: v.input.name, kind: v.input.kind,
        capacity: v.input.kind === 'PERSONAL' ? 3 : 1000, capacityOverride: null, owner, emotes: [], editors: [], visibility, createdAt: new Date().toISOString() };
      list.push(s);
      return { data: { createSet: setOut(s) } };
    }
    case 'UpdateSet': {
      const s = findSet(v.id);
      if (!s || !mayView(s, true)) return err('Not found.', 'not_found');
      if (v.input.visibility && v.input.visibility !== (s.visibility ?? 'LISTED')) {
        if (s.kind === 'GLOBAL' || (s.kind === 'PERSONAL' && v.input.visibility === 'PRIVATE'))
          return err('Personal sets can’t be private, and the global set is always listed.', 'bad_visibility');
        if (!ADMIN) return PUBLIC_ONLY();
        s.visibility = v.input.visibility;
      }
      if (v.input.resetCapacity) {
        if (s.kind !== 'PERSONAL' || v.input.capacity != null) return err('Only a personal set has an override to take away, and not together with a capacity.', 'bad_capacity');
        if (!ADMIN) return err('A personal set’s slots come from your plan. Only staff can change them.', 'forbidden');
        s.capacityOverride = null;
        s.capacity = personalSlots(s);
      }
      if (v.input.capacity != null) {
        const c = v.input.capacity;
        if (!setRights(s, ADMIN).rename) return forbidden();
        if (s.kind === 'PERSONAL' && !ADMIN) return err('A personal set’s slots come from your plan. Only staff can change them.', 'forbidden');
        if (!Number.isInteger(c) || c < 1 || c > 1_000_000) return err('Capacity is a whole number from 1 to 1.000.000.', 'bad_capacity');
        const r = capRange(s, ADMIN);
        if (c > r.max) return err(`This set can have up to ${r.max.toLocaleString('de-DE')} slots.`, 'capacity_over_limit');
        if (c < s.emotes.length) return err('The set already holds more emotes than that.', 'capacity_too_low');
        s.capacity = c;
        if (s.kind === 'PERSONAL') s.capacityOverride = c;
      }
      if (v.input.name) {
        if (!setRights(s, ADMIN).rename) return forbidden();
        s.name = v.input.name;
      }
      return { data: { updateSet: setOut(s) } };
    }
    case 'DeleteSet': {
      if (!findSet(v.id) || !mayView(findSet(v.id), true)) return err('Not found.', 'not_found');
      if (!setRights(findSet(v.id), ADMIN).manage) return forbidden();
      if ([...CHANNELS, ...EDITOR_CHANNELS].some((c) => c.activeSetId === v.id)) return err('A channel shows this set. Pick another active set first.', 'set_in_use');
      const list = SETS.some((s) => s.id === v.id) ? SETS : SHARED_SETS;
      list.splice(list.findIndex((s) => s.id === v.id), 1);
      return { data: { deleteSet: true } };
    }
    case 'AddSetEmote': {
      const s = findSet(v.setId);
      if (!s || !mayView(s, true)) return err('Not found.', 'not_found');
      if (!setRights(s, ADMIN).emotes) return forbidden();
      const e = findEmote(v.emoteId);
      if (!e) return err('Not found.', 'not_found');
      // A personal set takes only emotes approved for personal use (ADR 0039).
      if (s.kind === 'PERSONAL' && e.approvals?.personalUse !== 'APPROVED')
        return err(e.approvals?.personalUse === 'DENIED' ? 'That emote was not approved for personal use, so it can’t go in a personal set.'
          : 'That emote isn’t approved for personal use yet. A personal set takes only emotes approved for personal use.', 'personal_use_not_approved');
      if (s.emotes.length >= s.capacity) return err('The set is full.', 'set_full');
      if (s.emotes.some((m) => m.name === e.name)) return err('Another emote in this set has that name.', 'name_taken');
      s.emotes.push(member(e, v.input?.name ?? e.name));
      return { data: { addSetEmote: { name: e.name } } };
    }
    case 'UpdateSetEmote': {
      const own = findSet(v.setId);
      if (!own || !mayView(own, true)) return err('Not found.', 'not_found');
      if (!setRights(own, ADMIN).emotes) return forbidden();
      const m = findSet(v.setId).emotes.find((x) => x.emote.id === v.emoteId);
      if (v.input.name) m.name = v.input.name;
      if (v.input.zeroWidth !== undefined) m.zeroWidth = v.input.zeroWidth;
      return { data: { updateSetEmote: { name: m.name, zeroWidth: m.zeroWidth } } };
    }
    case 'RemoveSetEmote': {
      const s = findSet(v.setId);
      if (!s || !mayView(s, true)) return err('Not found.', 'not_found');
      if (!setRights(s, ADMIN).emotes) return forbidden();
      s.emotes = s.emotes.filter((m) => m.emote.id !== v.emoteId);
      return { data: { removeSetEmote: true } };
    }
    case 'RemoveSetEditor': {
      const s = findSet(v.setId);
      s.editors = s.editors.filter((e) => e.id !== v.accountId);
      if (v.accountId === ME_ID) SETS.splice(SETS.indexOf(s), 1);
      return { data: { removeSetEditor: true } };
    }
    default:
      return null;
  }
}
