# 0038: Two approvals per emote: public listing and personal use

- Status: accepted (owner decisions of 2026/11/27)
- Date: 2026/11/27
- Amends: 0010 (two permissions), 0014 (transfers), 0020 (who decides the listing), 0022 (favourites on the emote page), 0024 (the queue and the flags panel)

## Context
Until now one review decided everything: an approved emote was public (on search, browse and its creator's page) unless its owner made it private or a moderator unlisted it, and it could go in any set, a personal set included. The owner wants two separate questions answered for every emote: may it be **listed publicly**, and may it be **used as a personal emote** (shown in chat everywhere its owner talks). Uploaders no longer pick listed or unlisted; staff decide the listing in review. Ownership changes move to staff alone.

## Decision

**The model**
- Two columns on `emotes` (migration `20261230000101_emote_approvals.sql`): `listing_review` and `personal_review`, each `pending`, `approved` or `denied` (CHECK), plus `personal_reviewed_by`, `personal_reviewed_at`, `personal_review_reason`. The listing keeps the existing `reviewed_by`, `reviewed_at` and `review_reason`.
- `state` keeps its meaning: whether the emote may be used at all (`pending` until the first listing verdict, `approved`, `rejected`, `taken_down`). `state = 'pending'` exactly when the listing is pending, so the main queue, the pending cap and the requeue keep reading `state`.
- **`unlisted` follows the listing.** A trigger (`emotes_listing_follows_review`) sets `unlisted = NOT private AND listing_review <> 'approved'` on every insert and update, so no code path can list an emote that wasn't approved for listing, and the "never both private and unlisted" CHECK holds. Search and browse need no change: `search::LISTED_EMOTE` already leaves unlisted emotes out.
- API (additive): `Emote.approvals { publicListing, personalUse }` (`ApprovalState`: `PENDING`, `APPROVED`, `DENIED`), in GraphQL and in every REST emote view (`approvals.public_listing`, `approvals.personal_use`). Anyone who may see the emote sees both.

**Uploads**
- Every upload starts with both approvals pending and is in the review queue at once. A public upload is stored unlisted until its listing is approved; the website says "Unlisted until reviewed".
- The owner's toggles are **zero-width** and **private** only. `visibility=public` on the form, and `visibility: PUBLIC` / `private: false` on `updateEmote`, now mean "not private": the emote is listed only once its listing is approved. `unlisted` from anyone without `emotes.flags` or `emotes.edit_any` stays `400 bad_visibility` (the message now says listing is decided in review). Making an emote private and public again never changes its listing approval.
- A renamed or retagged approved emote goes back to review as before (`emote.resubmitted`): its listing is pending again; its personal-use approval stays (it is about the picture).

**Review**
- One new mutation for both queues and the edit dialog: `reviewApprovals(ids, input: { publicListing, personalUse, reason, onlyPending }, idempotencyKey)` → `BatchOutcome`. `publicListing` and `personalUse` are each `APPROVE`, `DENY` or left out; a denial needs a reason (1 to 500 characters, the owner sees it); at least one part (`bad_approvals`). Up to 100 emotes, one transaction, one audit row and one notification per emote.
  - Listing `APPROVE`: `state` approved, listing approved (listed unless private). Listing `DENY`: `state` approved (it can still be used by link and in channel sets) and listing denied (stays unlisted). The first verdict on a pending emote is written as `emote.approved` (on the owner's public activity feed as before) with `public_listing` in `after`; later listing changes are `emote.approvals_changed`.
  - Personal `APPROVE` / `DENY`: only the personal-use columns. Audited as `emote.approvals_changed` (or within the same `emote.approved` row when both are decided at once).
  - `onlyPending` (default `true`, what the queues send): a part that is no longer pending is skipped, and an emote with no pending part left is listed in `failed` as `not_pending`, so two reviewers can't overturn each other. The edit dialog sends `false` to change a decision.
  - Rejected and taken-down emotes can't be reviewed this way (`not_pending`); `rejectEmote` is still how an emote is refused outright, and it now denies both approvals.
  - The owner is told with `emote.approvals_changed` (`{ emote_id, name, public_listing, personal_use, reason }`, can be turned off like `emote.approved`); the first verdict also keeps sending `emote.approved`.
- `approveEmote` and `reviewEmotes` keep working and decide the **listing only**: no `visibility` or `PUBLIC` approves the listing, `UNLISTED` denies it (the old "approve as unlisted"), `PRIVATE` makes the emote private and approves the listing. The personal-use approval then waits in its own queue.
- **Queues.** The main queue (`reviewQueue`, tabs `PENDING`, `PUBLIC`, `LIMITED`, `RESOLVED`) is unchanged in shape: `PUBLIC` is now "pending and not private", `LIMITED` "pending and private". A new tab `PERSONAL` lists emotes whose personal use is pending and that may be used (`state` pending or approved); `counts.personal` counts it. The website's main queue offers **Approve both** (first), approve or deny the listing, approve or deny personal use, and Reject; the personal queue offers approve and deny personal use.
- **Permissions.** Personal-use review is its own permission, `emotes.review_personal` ("Review personal use"), so a community can let trusted helpers decide personal use without the main queue. The migration gives it to every role that allows `emotes.review` (and denies it where `emotes.review` is denied), so no reviewer loses anything. Listing verdicts keep `emotes.review`; Approve both needs both.

**Personal sets**
- A personal set takes only emotes approved for personal use (and approved): anything else is `409 personal_use_not_approved` ("This emote isn't approved for personal use yet." / "… was not approved for personal use."). The website disables Add to personal set with that reason.
- **Existing members that aren't approved are kept**, so nobody's chat changes overnight, and each is flagged (`approvals.personalUse` is pending or denied) on the set page. They sit in the personal queue, and staff decide them there.

**Migration of existing emotes**
- `pending` → both pending. `rejected` and `taken_down` → both denied.
- `approved` and not unlisted (public or private) → listing approved, **personal use approved** (it was publicly approved; the owner's default).
- `approved` and unlisted (staff took it off the listings, or approved it "for personal use only") → listing denied, personal use **pending**, so staff look at it again in the personal queue.

**Edit emote pop-up**
- "Edit emote" on the emote page opens a dialog instead of the old flags panel. The owner, and account editors with Manage, change name, tags, zero-width and private (`updateEmote`). Staff with `emotes.review` / `emotes.review_personal` also approve or deny the listing and personal use there (`reviewApprovals` with `onlyPending: false`), and staff with `emotes.transfer` give the emote to another account found by name or profile link.
- The staff Emote flags panel is gone from the emote page; `setEmoteFlags` stays in the API for scripts. A visibility change through it moves the listing approval with it (`PUBLIC` approves, `UNLISTED` denies), so the two never disagree.
- The emote page's stats grid shows **Public listing** and **Personal use** as two cells in place of the single Review cell. The Favourite button and menu item are gone from the emote page: favouriting isn't a feature (the `favouriteEmote` API and existing lists stay, ADR 0022, for clients that use them).

**Transfers**
- **Only staff move an emote**: `transferEmote` needs the new permission `emotes.transfer` ("Transfer emotes"), given to the built-in Admin role; `emotes.edit_any` doesn't imply it. It moves the emote directly to any active account (not deleted, blocked or restricted), with no request and no accept step, audited as `emote.transferred` (`"via": "emotes.transfer"`), and both the old and the new owner get `emote.transferred`. The owner's own give-away is retired with the request flow: anyone without the permission, the owner included, gets `403 transfer_staff_only`.
- The request flow is gone from the API too: `acceptEmoteTransfer`, `declineEmoteTransfer` and `cancelEmoteTransfer` answer `410 transfer_requests_retired`, `emoteTransferRequests` is always empty, all four stay in the schema deprecated, and `/v1/meta` lists the deprecation `emote_transfer_requests` (sunset 2027-06-01). The audit log keeps every past transfer and request.

## Consequences
- Reviewers decide two things per emote; Approve both keeps the common case one click.
- A personal emote is a stronger promise (it shows in every channel its owner chats in), and now has its own gate.
- Clients reading `state`, `visibility` and `flags` keep working: a new upload simply reads `UNLISTED` until its listing is approved.
- The trigger is the one place the listing rule lives; a future change to it is a migration.
