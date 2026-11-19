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

## Amendment (owner feedback, 2026/11/19)
- **Set scope:** a grant covers every set of the owner (`account_editors.all_sets`, the default; sets made later are included) or only the sets listed in `account_editor_sets` (at most 200, each one of the owner's; deleting a set takes it off every list). Every set check (`edit_grant`, `may_view`, `editorSets`, the editable count in set views, and the active set in `updateChannelSettings`) asks for the flag *and* the scope. An editor limited to some sets can't create or copy sets for the owner (they couldn't work on the new one).
- **No widening:** a manager limited to some sets only grants lists inside their own (never "every set"), and only changes or removes editors whose scope is inside theirs, as with flags.
- **New flags** (additive; stored names in brackets):

  | Flag | Allows |
  |---|---|
  | `channels` (`channels`) | change the owner's channels' active set and settings. `sets` keeps allowing this too, so existing grants mean what they meant, and granting `sets` needs `channels` as well |
  | `emote_upload`, `emote_manage`, `emote_transfer` | stored now; checked once uploading, managing and transferring for another account exist |
  | `billing_payment`, `billing_subscription` | stored now; checked once subscriptions and billing exist |

  Grants are listed as stored: a grant of `SETS` is listed as `["SETS"]`, not with `CHANNELS` added.
- **Limit:** 50 editors per account (was 20); over it is `409 too_many_editors` with a sentence saying to remove one.
- **API (additive):** `AccountEditor.allSets`/`setIds`, `EditedAccount.allSets`/`setIds`, `AddAccountEditorInput.setIds`, `updateAccountEditor(setIds:)` (empty: every set; left out: unchanged), and `AccountProfile.editors`: who edits an account, names only, for anyone (never their flags or sets). The audit rows of grants carry `all_sets` and `set_ids` before and after.
- **Website:** the Editors page finds people by name (`searchAccounts`, no platform picker), and adding or changing an editor opens one permissions dialog (grouped flags, "Everything", and a "Sets they can edit" list defaulting to all sets). A grant applies when saved. The set page no longer manages per-set editors; it links to the Editors page. Per-set editors keep working in the API.
- Tests: `apps/api/tests/sets_editors_flow.rs`.

## Amendment (owner feedback, 2026/11/19): the emote permissions are checked
The owner asked for the emote flags to work now (billing stays stored only). They replace "stored now; checked once …" in the table above.

| Flag | Allows, on the owner's account |
|---|---|
| `emote_upload` | upload an emote the owner owns: `POST /v1/emotes` with `owner_id`. `emotes.uploaded_by` names the editor (migration `20261207000080`), and the owner's public feed shows the upload "by" them once it is approved |
| `emote_manage` | see the owner's emotes in every state (`emote`, `accountEmotes`), and change their name, tags, flags and visibility (`updateEmote`, with the owner's resubmit rule) or delete them (`deleteEmote`) |
| `emote_transfer` | give one of the owner's emotes to another account (`transferEmote`), under the receiver rule below |

- **Every set:** the three count only on a grant covering every set (`has_emote_flag`). Emotes belong to the account, not to a set, so a grant narrowed to some sets is too narrow for them; the website says so on the Editors page and in the permissions dialog.
- **Restrictions:** an uploads hold (`restricted_at`, #87) on the editor or on the owner refuses all three, as it refuses the owner's own changes.
- **Upload limit:** an editor's upload counts against them (20 an hour, their own and everyone else's together) and against the owner (20 an hour received, from anyone).
- **Transfer, the receiver's consent:** the receiver must be the caller or an account where the caller holds `emote_upload` on every set. That account already lets the caller put emotes on it, so nobody can push an emote onto an account that never trusted them, and nobody gets an emote they couldn't have uploaded. The owner follows the same rule. There is no accept step: it was the larger feature (a pending transfer, a notification with Accept, expiry), and the rule above gives the same protection for the cases editors need (moving emotes between accounts they look after). Transfers are 60 an hour per caller; the emote keeps its state, visibility, tags and set memberships (a private emote stays in the sets it was in; from then on only its new owner can add it to more). Both owners get an `emote.transferred` notification unless they did it.
- **Audit:** `emote.uploaded`, `emote.updated`, `emote.deleted` and `emote.transferred`, actor the editor, `"via": "account_editor"` in `after`, subject the owner. An owner's own transfer is audited without `via`; staff edits keep `"via": "emotes.delete_any"` and no subject.
- **API (additive):** `owner_id` on the upload form, `transferEmote(id, toAccountId)`, `accountEmotes` also for the account itself and its managers; no field changed meaning.
- Tests: `apps/api/tests/editor_emotes_flow.rs`.

## Amendment (owner decision, 2026/11/19): the owner must accept an editor's transfer
This replaces "There is no accept step" in the amendment above.
- The owner's own `transferEmote` stays immediate, under the same receiver rule.
- An editor's `transferEmote` makes a pending request (`emote_transfer_requests`, migration `20261207000081`) and returns the emote unchanged. The owner is notified (`emote.transfer_requested`, answerable from the notification) and sees it on the emote's edit page; the emote moves only on `acceptEmoteTransfer`. The owner can `declineEmoteTransfer`, the editor can `cancelEmoteTransfer`, and a request expires after 7 days (marked `expired` the next time anything reads or answers requests). One open request per emote (`409 transfer_pending`).
- Accepting checks the rules as they stand then: the request is still open, the emote still belongs to the owner (else the request is closed as cancelled), the receiver is still the editor or an account where the editor holds `emote_upload` on every set, and nobody involved (owner, editor, receiver) is held or gone.
- When the emote moves any other way (the owner's own transfer, or another request accepted), open requests for it are cancelled.
- Every step is audited with the owner as subject: `emote.transfer_requested` (actor the editor, `"via": "account_editor"`), `emote.transfer_accepted` and `emote.transferred` (actor the owner; `requested_by` in `after`), `emote.transfer_declined`, `emote.transfer_cancelled`, `emote.transfer_expired` (no actor). The other side is notified at each step.
- API (additive): `emoteTransferRequests(emoteId)` (open requests on your emotes and the ones you made), `acceptEmoteTransfer`, `declineEmoteTransfer`, `cancelEmoteTransfer`; `EmoteTransferRequest`, `TransferRequestState`. Accepting, declining and cancelling share the 60-an-hour transfer limit.
