# 0016: Public account activity feed

- Status: accepted
- Date: 2026/11/22

## Context
7TV's profile has an "Activity" tab: who added, removed or renamed an emote in a set, which paint was picked, with times. We want the same for cyex.tv accounts, public and read-only, without exposing moderation or anything private. The audit log (ADR 0010) already records changes, but only those that grant access or lean on a permission; an owner's own edits were not written.

## Decision
- **Source:** `audit_log`. New column `subject_account_id` (nullable, indexed `(subject_account_id, id DESC)`): the account whose feed a row belongs to. Only the owner's own changes, the changes made through an account editor grant (ADR 0014, `"via": "account_editor"`) and the approval of the account's emotes set it. Staff acting on a permission (`sets.edit_any`, reviews, takedowns, grants) and per-set editors never set it, so they cannot reach the feed.
- **New audit rows:** the owner's own `set.created`, `set.renamed`, `set.emote_added`, `set.emote_removed`, `set.emote_updated` (now with the previous name), `channel.settings_changed` (when something changed) and `cosmetics.selected`. Rows made before this change have no subject and don't show.
- **API (additive, ADR 0008):** `accountActivity(accountId: String!, before: String, limit: Int): [ActivityEntry!]!`, public, no session, under the usual GraphQL rate limits. `limit` 1 to 100 (default 30), paged by entry ID like `auditLog`. `ActivityEntry { id kind actor set emote fromName toName badge paint createdAt }`; kinds `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED`, `ACTIVE_SET_CHANGED`, `COSMETICS_SELECTED`, `EMOTE_UPLOADED`, `SET_CREATED`, `SET_RENAMED`. Badge and paint changes carry from/to ids and names.
- **Privacy:** an unknown, deleted or blocked owner has an empty feed; entries by a blocked or deleted actor are dropped. A line about an emote shows only while the emote is approved, not private and not deleted (a pending emote's add appears once it is approved). Lines about deleted sets, and moderation actions, reasons and reviewers never show. An emote's approval appears once, as `EMOTE_UPLOADED` by the owner. Sets have no privacy setting, so every set of a public account is public.
- **Paging:** the SQL filter can't see emote state, so the api reads batches of 100 candidate rows (at most 10 rounds) until the page is full.

## Consequences
- More audit rows (one per owner edit); the feed query is one indexed read plus a few batch lookups.
- Entries show the current names of sets and cosmetics, and the emote's name at the time of the change.

## Amendment 2026/11/19: the staff account dashboard
The staff account page (`/admin/users/<id>`) shows "recent activity" from the audit log by target (`auditLog(targetKind: "account", targetId)`, `audit.view`), not from this feed: staff need the moderation lines this feed leaves out (restrictions, role changes, grants, notes). The public feed stays as decided above; nothing staff do on the dashboard is written to it, and no feed entry carries a reason, a note or a restriction. The new audit actions from the same amendment of ADR 0010 (`account.restriction_placed`, `account.restriction_lifted`, `account.note_added`, `account.note_deleted`) never set `subject_account_id`.
