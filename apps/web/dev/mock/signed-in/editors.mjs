// Mock API: signed in/editors. See ../mock-api.mjs.
import { ME_ID, ME_SUMMARY, OTHERS } from '../config.mjs';
import { CHANNELS } from '../account.mjs';
import { SETS } from '../set-data.mjs';
import { rolesOf } from '../roles.mjs';
import { ACCOUNT_EDITORS, EDITOR_OF, MAX_EDITORS, RETIRED_ERR, RETIRED_ONLY, editorChannels, editorOfOut, editorSets, flagsOn, scopeFrom, withImplied } from '../editors.mjs';
import { STAFF_EDITORS, err, setOut } from '../sets.mjs';

// The signed-in operations of this area, by operation name: null when the operation is not here.
export function signedInEditors(op, body, v, ADMIN) {
  switch (op) {
    case 'ProfileMine':
      return { data: { myChannels: CHANNELS, mySets: SETS.filter((s) => s.owner?.id === ME_ID).map(setOut) } };
    case 'ProfileEdited':
      return { data: { editorChannels: editorChannels(), editorSets: editorSets().map(setOut) } };
    case 'EditorOf':
      return { data: { editorOf: editorOfOut() } };
    case 'AccountEditors': {
      if (!v.accountId || v.accountId === ME_ID) return { data: { accountEditors: ACCOUNT_EDITORS } };
      if (ADMIN) return { data: { accountEditors: (STAFF_EDITORS[v.accountId] ??= []) } }; // editors.manage_any
      if (!flagsOn(v.accountId).includes('EDITORS')) return err('This needs the editors permission on that account.', 'forbidden');
      return { data: { accountEditors: [] } };
    }
    case 'AccountRoles': {
      if (v.accountId !== ME_ID && !ADMIN) return err('This needs users.view.', 'forbidden');
      // The signed-in admin is the owner: the top role (only they see it) and what they were given.
      if (v.accountId === ME_ID && !ADMIN) return { data: { accountRoles: [] } };
      return { data: { accountRoles: rolesOf(v.accountId) } };
    }
    case 'Editors':
      return { data: { accountEditors: ACCOUNT_EDITORS, editorOf: editorOfOut(), mySets: SETS.map((x) => ({ id: x.id, name: x.name, kind: x.kind, owner: x.owner ? { id: x.owner.id } : null })) } };
    case 'AddAccountEditor': {
      const { login, accountId, permissions } = v.input;
      if (v.input.ownerId && v.input.ownerId !== ME_ID) {
        // Someone else's account: staff with editors.manage_any (the mock admin) only.
        if (!ADMIN) return err('Only the account’s owner and its editors with the editors permission can manage its editors.', 'forbidden');
        if (RETIRED_ONLY(permissions)) return RETIRED_ERR();
        if (!permissions?.length) return err('Pick at least one permission.', 'no_permissions');
        const list = (STAFF_EDITORS[v.input.ownerId] ??= []);
        const who = String(login ?? '').trim().replace(/^@/, '').toLowerCase();
        const a = [ME_SUMMARY, ...OTHERS].find((o) => (accountId ? o.id === accountId : o.displayName.toLowerCase() === who));
        if (!a) return err('Not found.', 'not_found');
        if (a.id === v.input.ownerId) return err('The account’s owner can do all of this already.', 'already_owner');
        if (list.some((e) => e.account.id === a.id)) return err('They are an editor already.', 'already_editor');
        const now = new Date().toISOString();
        const ed = { account: a, permissions: withImplied(permissions), allSets: true, setIds: [], grantedBy: ME_SUMMARY, grantedAt: now, updatedAt: now };
        list.push(ed);
        return { data: { addAccountEditor: ed } };
      }
      if (RETIRED_ONLY(permissions)) return RETIRED_ERR();
      if (!permissions?.length) return err('Pick at least one permission.', 'no_permissions');
      if (ACCOUNT_EDITORS.length >= MAX_EDITORS) return err('An account can have at most 50 editors. Remove one to add someone else.', 'too_many_editors');
      const scope = scopeFrom(v.input.setIds);
      if (scope.error) return scope.error;
      const name = String(login ?? '').trim().replace(/^@/, '').toLowerCase();
      if (accountId === ME_ID || (!accountId && name === ME_SUMMARY.displayName.toLowerCase())) return err('The account’s owner can do all of this already.', 'already_owner');
      const a = accountId ? OTHERS.find((o) => o.id === accountId) : OTHERS.find((o) => o.displayName.toLowerCase() === name);
      if (!a) return err('Not found.', 'not_found');
      if (ACCOUNT_EDITORS.some((e) => e.account.id === a.id)) return err('They are an editor already.', 'already_editor');
      const now = new Date().toISOString();
      const ed = { account: a, permissions: withImplied(permissions), ...scope, grantedBy: ME_SUMMARY, grantedAt: now, updatedAt: now };
      ACCOUNT_EDITORS.push(ed);
      return { data: { addAccountEditor: ed } };
    }
    case 'UpdateAccountEditor': {
      if (v.ownerId && v.ownerId !== ME_ID && !ADMIN) return err('Only the account’s owner and its editors with the editors permission can manage its editors.', 'forbidden');
      const ed = (v.ownerId && v.ownerId !== ME_ID ? (STAFF_EDITORS[v.ownerId] ?? []) : ACCOUNT_EDITORS).find((e) => e.account.id === v.accountId);
      if (!ed) return err('Not found.', 'not_found');
      if (RETIRED_ONLY(v.permissions)) return RETIRED_ERR();
      if (!v.permissions?.length) return err('Pick at least one permission.', 'no_permissions');
      if (v.setIds) {
        const scope = scopeFrom(v.setIds);
        if (scope.error) return scope.error;
        Object.assign(ed, scope);
      }
      ed.permissions = withImplied(v.permissions);
      ed.updatedAt = new Date().toISOString();
      return { data: { updateAccountEditor: ed } };
    }
    case 'RemoveAccountEditor': {
      if (v.ownerId && v.ownerId !== ME_ID && v.accountId !== ME_ID && ADMIN) {
        const list = STAFF_EDITORS[v.ownerId] ?? [];
        const i = list.findIndex((e) => e.account.id === v.accountId);
        if (i >= 0) list.splice(i, 1);
        return { data: { removeAccountEditor: i >= 0 } };
      }
      if (v.ownerId && v.ownerId !== ME_ID) {
        const i = EDITOR_OF.findIndex((g) => g.account.id === v.ownerId);
        if (i >= 0 && v.accountId === ME_ID) EDITOR_OF.splice(i, 1);
        return { data: { removeAccountEditor: i >= 0 } };
      }
      const i = ACCOUNT_EDITORS.findIndex((e) => e.account.id === v.accountId);
      if (i >= 0) ACCOUNT_EDITORS.splice(i, 1);
      return { data: { removeAccountEditor: i >= 0 } };
    }
    default:
      return null;
  }
}
