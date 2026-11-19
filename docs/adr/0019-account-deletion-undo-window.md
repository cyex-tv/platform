# 0019: Account deletion with a 30-day undo window

- Status: accepted (amends 0009)
- Date: 2026/11/19

## Context
`deleteAccount` erased an account at once: identities, channels, sets, grants, cosmetics, roles and notifications went, emotes were deleted or disowned, and the files were queued for deletion. A mistake, a stolen session or a change of heart could not be undone. The owner decided (answers of 2026/11/19) that deletion gets a 30-day undo window instead, like most platforms offer. The data export stays a signed-in feature: after deleting you are signed out, so it isn't offered.

## Decision
- **Hide at once.** `deleteAccount` keeps its checks (cookie session only, the owner protected, restricted or blocked accounts refused, a login to confirm, 3 tries an hour) and then sets `accounts.deleted_at = now()` and the new `accounts.erase_after = now() + 30 days` (migration `20261207000030_deletion_window.sql`, additive). Its sessions are deleted and the response clears the cookie. Chat clients hear that its channels show no set, its personal set changed and its cosmetics went (the events the erase sent before). Audited `account.deletion_scheduled`.
- **Hidden means read as deleted.** Every public read already treats `deleted_at` as gone (profile, `account`, `accountSets`, `searchAccounts`, channels, summaries, cosmetics, editors, activity). On top: `emote(id)`, `searchEmotes`, `emoteChannels` and `emoteActivityPage` hide emotes whose owner is pending, `set(id)` hides its sets from everyone but `sets.edit_any`, personal-set lookups leave it out, and others can't add its emotes to sets (`emote_unavailable`). Its emotes that others already have in their sets keep working there, shown as by a deleted user, as they would after the erase.
- **Restore by signing in.** Signing in with one of its identities inside the window succeeds but sends the browser to `/restore` with a site session that `caller` never accepts (`sessions::find_active` requires `deleted_at IS NULL`). Only `pendingDeletion` (when it was asked for, when it will be erased, the name and avatar) and `restoreAccount` read that session (`Auth::restorable_caller`), and `POST /v1/auth/logout` ends it. `restoreAccount` clears both columns, tells chat clients the channels, personal set and cosmetics are back, and audits `account.restored`; the same cookie is then a normal session. A blocked account is refused at sign-in as before; one that staff restricted during the window is restored still restricted.
- **Erase after 30 days.** The worker's hourly clean-up erases accounts whose `erase_after` has passed (`privacy::erase_account`, at most 50 per run, each in its own transaction): exactly what `deleteAccount` did at once before, including the `files.delete` jobs from the sign-off fixes, then `erase_after` is cleared and `account.deleted` is audited with the counts. Deleting the identities fires the identity-hold trigger, so a restriction added during the window still follows the person to a new account.
- **After the window** the restore session stops working and signing in is `account_deleted` until the worker has erased the account; after that, signing in makes a new account, as before.

## Consequences
- The API is additive: `deleteAccount` keeps its arguments and answer; `pendingDeletion`, `restoreAccount`, `PendingDeletion` and the column are new. Clients that showed "deleted at once" need their copy changed (the website did).
- For 30 days the data stays in the database. The privacy page's "its data is removed" now means within 30 days, and the privacy text (legal PR) should say so.
- A row with `deleted_at` set and `erase_after` NULL is erased for good; one with both set is pending. A CHECK keeps `erase_after` from being set on a live account.
- The erase now runs without a person waiting on it: a failure is logged and retried at the next clean-up.
