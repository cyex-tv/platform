# 0014: Account editors

- Status: accepted
- Date: 2026/11/17

## Context
Streamers hand their emotes to a team: moderators who add and rename emotes, a manager who keeps the sets in order. Per-set editors (`set_editors`, Phase 2) cover one set at a time and can't touch channel settings. 7TV, which streamers know, lets a user add editors to their whole account and pick what each one may do (its help centre lists emote set, emote and user permissions, including managing editors and the profile). We want the same idea, written here from scratch (AGENTS.md: no code from 7TV).

## Decision
- An account (the **owner** of that account) can make other accounts **account editors**. A grant covers everything the account owns: all its sets (channel and personal) and all its channels. It is not passed on: the editors of an editor get nothing from it.
- Each grant holds one or more permission flags. Names are constants (`EditorPermission` in `crates/shared`), stored as text in `account_editors.permissions`, never free text:

  | Flag | Allows, on the owner's account |
  |---|---|
  | `emotes` | add, remove, rename (alias) and change zero-width on emotes in any of the owner's sets, the active channel sets included; see the owner's pending emotes in those sets; add the owner's own private emotes to the owner's sets (their own private emotes too, as for per-set editors; nobody else's) |
  | `sets` | create, rename and delete sets, change a set's capacity up to its default (raising above it still needs `sets.edit_any`); set a channel's active set and its settings (allow personal emotes) |
  | `editors` | add, change and remove account editors, and per-set editors on the owner's sets (that also needs `emotes`, since a set editor can change emotes) |
  | `profile` | pick the badge and paint the owner shows, and read what they own (ADR 0015); never grant |

- The owner always has every flag and is never in the table (`account_id <> editor_id`).
- **No escalation:** an editor with `editors` can only grant flags they hold, and can only change or remove editors whose current flags are all flags they hold. Nobody but the owner can grant on the owner's behalf more than they have; nobody can act on the owner.
- **Leaving:** an editor can always remove themselves.
- **Limits:** 20 editors per account, checked under a lock on the owner's account row. Adding, changing and removing editors is limited to 60 an hour per caller (in memory, like the other rate limits; `429 rate_limited`).
- **Blocked and deleted accounts:** a blocked or deleted account can't be added. Existing grants of a blocked editor, and grants on a blocked or deleted owner, are ignored by every check and every read until the block is lifted.
- **Checks:** each set or channel mutation names the flag it needs. A set change is allowed for the set's owner, a per-set editor (unchanged), an account editor with the flag, or a staff permission (unchanged). The check reads the database on every request, so a removed or reduced editor loses access at once.
- **Audit:** every grant, change and removal writes `audit_log` (`account.editor_added`, `account.editor_updated`, `account.editor_removed`, target the owner's account) with before and after. Every set or channel change made through an account grant is audited too, with `"via": "account_editor"`.
- **Events:** set changes made by editors publish the same outbox events as the owner's. Editor grants are private, so they publish nothing.
- **API (additive, ADR 0008):** queries `accountEditors(accountId)` (yours by default, or an account you edit with `editors`), `editorOf`, `editorSets`, `editorChannels`; mutations `addAccountEditor`, `updateAccountEditor`, `removeAccountEditor`; an optional `ownerId` on `CreateSetInput`. Existing fields keep their meaning; `mySets` and `myChannels` are unchanged.
- No invitation step: a grant applies at once, and the editor can leave. 7TV asks the editor to accept first; see the open questions.

## Consequences
- One place (`apps/api/src/editors.rs`) answers "may this caller act on this account with this flag"; the set and channel handlers name the flag.
- Each authorization check costs one more indexed lookup.
- The website needs an Editors page and has to label things the caller manages for someone else with their owner.
- Escalation, IDOR, blocked accounts and immediate loss of access are covered by tests against Postgres in CI (`apps/api/tests/sec_account_editors.rs`).
