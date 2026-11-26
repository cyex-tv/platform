// Mock API: signed in/channels. See ../mock-api.mjs.
import { CHANNELS } from '../account.mjs';
import { SETS } from '../set-data.mjs';
import { coversSet, editorChannels, editorOfOut, editorSets, flagsOn } from '../editors.mjs';
import { err, findSet } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInChannels(op, body, v, ADMIN) {
  switch (op) {
    case 'MyChannels': {
      const short = (s) => ({ id: s.id, name: s.name, kind: s.kind, owner: s.owner ? { id: s.owner.id } : null });
      return { data: { myChannels: CHANNELS, mySets: SETS.map(short), editorChannels: editorChannels(), editorSets: editorSets().map(short), editorOf: editorOfOut() } };
    }
    case 'UpdateChannel': {
      const c = [...CHANNELS, ...editorChannels()].find((x) => x.platform === v.platform && x.platformChannelId === v.id);
      const held = c ? flagsOn(c.account.id) : [];
      // Picking the active set: SETS (or CHANNELS, as before); the other settings: CHANNELS.
      if (!c || (!held.includes('CHANNELS') && !held.includes('SETS')))
        return err('Only the channel’s owner and its account editors can change it.', 'forbidden');
      if (v.input.allowPersonalEmotes !== undefined && v.input.allowPersonalEmotes !== null && !held.includes('CHANNELS'))
        return err('Only the channel’s owner and its account editors with the channel settings permission can change its settings.', 'forbidden');
      if (v.input.activeSetId) {
        const target = findSet(v.input.activeSetId);
        if (!target || target.kind !== 'CHANNEL' || target.owner?.id !== c.account.id)
          return err('The active set must be one of the channel owner’s channel sets.', 'invalid_active_set');
        if (!coversSet(c.account.id, target.id)) return err('You can only show sets you work on. Ask the owner to add this one to your sets.', 'forbidden');
      }
      if (v.input.activeSetId !== undefined) c.activeSetId = v.input.activeSetId || null;
      if (v.input.allowPersonalEmotes !== undefined) c.allowPersonalEmotes = v.input.allowPersonalEmotes;
      return { data: { updateChannelSettings: { activeSetId: c.activeSetId, allowPersonalEmotes: c.allowPersonalEmotes } } };
    }
    // The profile (your profile is your dashboard): channels and sets, roles, editors.
    default:
      return null;
  }
}
