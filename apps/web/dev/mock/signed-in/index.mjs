// Mock API: signed in/index. See ../mock-api.mjs.
import { signedInAccount } from './account.mjs';
import { signedInEmotes } from './emotes.mjs';
import { signedInSets } from './sets.mjs';
import { signedInChannels } from './channels.mjs';
import { signedInEditors } from './editors.mjs';
import { signedInReview } from './review.mjs';
import { signedInAdmin } from './admin.mjs';
import { signedInRoles } from './roles.mjs';
import { signedInDevelopers } from './developers.mjs';

export function signedIn(body, v, ADMIN) {
  const op = (body.query.match(/(?:query|mutation)\s+(\w+)/) ?? [])[1];
  for (const area of [signedInAccount, signedInEmotes, signedInSets, signedInChannels, signedInEditors, signedInReview, signedInAdmin, signedInRoles, signedInDevelopers]) {
    const out = area(op, body, v, ADMIN);
    if (out !== null && out !== undefined) return out;
  }
  return null;
}
