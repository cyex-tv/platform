// Mock API: signed in/roles. See ../mock-api.mjs.
import { ME_ID } from '../config.mjs';
import { MINE } from '../emotes.mjs';
import { PENDING } from '../review.mjs';
import { ACCOUNT_ROLES, LIMIT_DEFAULTS, ROLES, ROLE_LIMITS, decisionsFor, limitMax, limitOf, limitsOut, pendingOf, roleOut } from '../roles.mjs';
import { PERMISSIONS } from '../editors.mjs';
import { emoteGrant, err } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
// The API's role colour rule (`colour` in admin.rs): #rrggbb in any case, stored lower case; '' clears (update only).
const COLOR = /^#[0-9a-f]{6}$/i;
const badColor = () => err('A role colour is a hex colour like #3b82f6.', 'bad_color');
export function signedInRoles(op, body, v, ADMIN) {
  switch (op) {
    case 'Roles':
      return { data: { roles: ROLES.map(roleOut), permissions: PERMISSIONS } };
    case 'AccountPermissions':
      if (v.accountId !== ME_ID && !ADMIN) return err('This needs users.view.', 'forbidden');
      return { data: { accountPermissions: decisionsFor(v.accountId) } };
    case 'AllRoles':
      return { data: { roles: ROLES.map(roleOut) } };
    case 'CreateRole': {
      if (v.input.color != null && !COLOR.test(v.input.color.trim())) return badColor();
      if (!(v.input.position >= 1 && v.input.position < 1000000)) return err('Positions are 1 to 999999: above the User role, below the top role.', 'bad_position');
      if (ROLES.some((x) => x.name.toLowerCase() === v.input.name.toLowerCase())) return err('A role with that name exists.', 'role_name_taken');
      const overlap = (v.input.denies ?? []).some((k) => v.input.permissions.includes(k));
      if (overlap) return err('A permission can’t be both allowed and denied.', 'bad_permissions');
      const r = { id: 'ROLE' + (ROLES.length + 1), ...v.input, description: v.input.description ?? null, denies: v.input.denies ?? [],
        invisible: v.input.invisible ?? false, color: v.input.color?.trim().toLowerCase() ?? null, public: v.input.invisible ? false : (v.input.public ?? false), builtIn: null };
      ROLES.push(r);
      ROLES.sort((a, b) => b.position - a.position);
      return { data: { createRole: roleOut(r) } };
    }
    case 'UpdateRole': {
      const r = ROLES.find((x) => x.id === v.id);
      if (v.input.color != null && v.input.color.trim() !== '' && !COLOR.test(v.input.color.trim())) return badColor();
      if (!r) return err('Not found.', 'not_found');
      if (r.builtIn === 'ROOT') return err('This is a built-in role; it can’t be changed this way.', 'built_in_role');
      if (!roleOut(r).editable) return err('You can’t edit a role you hold. Someone above you has to.', 'own_role');
      if (r.builtIn === 'USER' && v.input.position != null && v.input.position !== 0) return err('This is a built-in role; it can’t be changed this way.', 'built_in_role');
      const allows = v.input.permissions ?? r.permissions;
      const denies = v.input.denies ?? r.denies ?? [];
      if (denies.some((k) => allows.includes(k))) return err('A permission can’t be both allowed and denied.', 'bad_permissions');
      Object.assign(r, Object.fromEntries(Object.entries(v.input).filter(([k, x]) => x != null && k !== 'color')));
      if (v.input.color != null) r.color = v.input.color.trim() === '' ? null : v.input.color.trim().toLowerCase();
      if (r.invisible) r.public = false;
      if (r.description === '') r.description = null;
      ROLES.sort((a, b) => b.position - a.position);
      return { data: { updateRole: roleOut(r) } };
    }
    case 'LimitDefaults':
      if (!ADMIN) return err('This needs roles.manage.', 'forbidden');
      return { data: { limitDefaults: limitsOut() } };
    case 'RoleLimits': {
      if (!ADMIN) return err('This needs roles.manage.', 'forbidden');
      if (!ROLES.some((x) => x.id === v.id)) return err('Not found.', 'not_found');
      return { data: { roleLimits: limitsOut(ROLE_LIMITS[v.id]) } };
    }
    case 'SetRoleLimits': {
      if (!ADMIN) return err('This needs roles.manage.', 'forbidden');
      const r = ROLES.find((x) => x.id === v.id);
      if (!r) return err('Not found.', 'not_found');
      if (r.builtIn === 'ROOT') return err('This is a built-in role; it can’t be changed this way.', 'built_in_role');
      if (r.builtIn === 'USER') return err('Everyone holds the User role, so its limits are the defaults the API starts with.', 'built_in_role');
      if (!roleOut(r).editable) return err('You can’t edit a role you hold. Someone above you has to.', 'own_role');
      const next = {};
      for (const l of v.limits ?? []) {
        if (l.key.trim().startsWith('emote_max_')) return err(`${l.key.trim()} can’t be set: emote limits are fixed and the same for everyone (at most 1000×1000 px, 8 MB, 1000 frames, 50 fps, 4:1).`, 'bad_limit');
        if (!LIMIT_DEFAULTS.some((d) => d[0] === l.key.trim())) return err(`"${l.key}" is not a limit.`, 'unknown_limit');
        if (l.value == null) continue;
        const most = limitMax(l.key.trim());
        if (!(Number.isInteger(l.value) && l.value >= 1 && l.value <= most)) return err(`${l.key.trim()} is 1 to ${most}; leave it empty to keep the default.`, 'bad_limit');
        next[l.key.trim()] = l.value;
      }
      ROLE_LIMITS[r.id] = next;
      return { data: { setRoleLimits: limitsOut(next) } };
    }
    case 'UploadQuota': {
      const owner = v.ownerId || ME_ID;
      if (owner !== ME_ID && !emoteGrant(owner, 'EMOTE_UPLOAD'))
        return err('You can see another account’s uploads only as its editor with the upload permission on every set.', 'forbidden');
      const hour = Date.now() - 3600_000;
      return { data: { uploadQuota: { accountId: owner, pending: pendingOf(owner), pendingMax: limitOf(owner, 'pending_emotes'),
        uploadsThisHour: [...MINE, ...PENDING].filter((e) => e.owner?.id === owner && Date.parse(e.createdAt) > hour).length,
        uploadsPerHour: limitOf(owner, 'uploads_per_hour'),
        maxBytes: 8 * 1024 * 1024, maxSide: 1000, maxFrames: 1000, maxFps: 50, maxAspect: 4 } } };
    }
    case 'DeleteRole': {
      const r = ROLES.find((x) => x.id === v.id);
      if (r?.builtIn) return err('This is a built-in role; it can’t be changed this way.', 'built_in_role');
      ROLES.splice(ROLES.findIndex((x) => x.id === v.id), 1);
      return { data: { deleteRole: true } };
    }
    case 'AssignRole': {
      const r = ROLES.find((x) => x.id === v.roleId);
      if (!r || r.builtIn === 'ROOT' || r.builtIn === 'USER') return err('This is a built-in role; it can’t be changed this way.', 'built_in_role');
      (ACCOUNT_ROLES[v.accountId] ??= []).push(v.roleId);
      return { data: { assignRole: true } };
    }
    case 'UnassignRole':
      ACCOUNT_ROLES[v.accountId] = (ACCOUNT_ROLES[v.accountId] ?? []).filter((id) => id !== v.roleId);
      return { data: { unassignRole: true } };
    default:
      return null;
  }
}
