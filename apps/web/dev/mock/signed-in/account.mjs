// Mock API: signed in/account. See ../mock-api.mjs.
import { ME_ID } from '../config.mjs';
import { DISPLAY_PLATFORM, IDENTITIES, MY_ROLES, NOTES, goBy, myIdentities, setDisplayPlatform } from '../account.mjs';
import { ROLES } from '../roles.mjs';
import { PERMISSIONS } from '../editors.mjs';
import { err } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInAccount(op, body, v, ADMIN) {
  switch (op) {
    case 'Me':
      return { data: { me: { id: ME_ID, createdAt: '2026-11-10T12:00:00.000Z', permissions: ADMIN ? PERMISSIONS : ROLES.find((r) => r.builtIn === 'USER').permissions, identities: myIdentities(), displayPlatform: DISPLAY_PLATFORM, roles: MY_ROLES(ADMIN) } } };
    case 'SetDisplayPlatform': {
      if (!IDENTITIES.some((i) => i.platform === v.platform)) return err('That platform isn’t linked to your account.', 'not_found');
      setDisplayPlatform(v.platform);
      goBy();
      return { data: { setDisplayPlatform: { id: ME_ID, displayPlatform: DISPLAY_PLATFORM } } };
    }
    case 'MyRoles':
      return { data: { me: { id: ME_ID, roles: MY_ROLES(ADMIN) } } };
    case 'Unlink': {
      if (IDENTITIES.length < 2) return err('The last identity can’t be removed.', 'last_identity');
      const at = IDENTITIES.findIndex((i) => i.platform === v.platform);
      if (at < 0) return err('That platform isn’t linked to your account.', 'not_found');
      IDENTITIES.splice(at, 1);
      if (DISPLAY_PLATFORM === v.platform) setDisplayPlatform(null);
      goBy();
      return { data: { unlinkIdentity: { id: ME_ID } } };
    }
    case 'MyNotifications':
      // The API also sends transferState for old emote.transfer_requested notes; every request
      // was withdrawn, so it reads CANCELLED (the site no longer asks for it).
      return { data: { myNotifications: NOTES.slice(0, 100).map((n) => (n.kind === 'emote.transfer_requested' ? { ...n, transferState: 'CANCELLED' } : n)) } };
    default:
      return null;
  }
}
