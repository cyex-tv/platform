# 0017: Set visibility

- Status: accepted
- Date: 2026/11/30

## Context
7TV has public set pages that people share and copy. Until now every cyex.tv set could be read by anyone who had its ID (`set(id)`, `GET /v1/sets/{id}`), and none were listed anywhere. Profiles now show set cards (gap list G1), so owners need to choose what is listed, what is shared by link only, and what nobody else sees.

## Decision
- `emote_sets.visibility`: `listed` (anyone opens it; it shows on the owner's profile), `unlisted` (anyone with its ID opens it), `private` (only the owner, per-set editors, the owner's account editors and `sets.edit_any`).
- A channel's active set is public whatever its visibility: chat shows it to everyone and chat clients fetch it by ID. A private set that stops being active is private again.
- Personal sets can't be private (chat shows them to everyone); the global set is always listed. CHECK constraints back both.
- Defaults: new channel sets `listed`, personal sets `unlisted`. The migration keeps what existing rows could do: nothing becomes private; channel sets and the global set are listed, personal sets unlisted.
- Who changes it: the owner and account editors with `sets`; per-set editors can't.
- Everything that names a set to the public follows it: `accountSets` lists listed sets only, `set.created` is published for listed sets only (every topic is public), and the activity feed (ADR 0016) shows lines about listed sets only.
- `cloneSet` copies any set the caller may open into a new channel set, with `addSetEmote`'s rules for each emote.

## Consequences
- A private set the caller may not see answers exactly like a set that doesn't exist (null, 404), so its existence doesn't leak.
- Making a set unlisted or private also hides its past activity lines, since the feed checks the set's visibility now, not when the line was written.

## Amendment (owner feedback, 2026/11/19): default sets
- Every account has a default set: a listed channel set named after its display name (cut to 100 characters), owned like any other set. The sign-in flow makes it when a sign-in creates the account (`sets::create_default`); linking a second platform doesn't make another.
- Migration `20261207000011_default_sets.sql` gives every live account that has no channel set of that name (any case) one, with a ULID stamped with the account's creation time so it sorts before the account's other sets. Running it again adds nothing.
- It isn't made the channels' active set: the owner picks it with "Use on my channel" on the set page (`updateChannelSettings`), like any other set.
- Tests: `default_sets_are_backfilled_once_for_accounts_without_one` and the sign-in checks in `auth_flow.rs`.

## Amendment (owner decision, 2026/11/22): every set is public; capacity is the owner's to pick
The owner: "emote sets shouldnt be public, private or unlisted for regular users. all emote sets are public" and "emote set capacity should be possible to be adjusted (1-1000 for regular users) in the future subscribers get 2500 slots and as an admin i should be able to change any set to up to a million slots".

**Visibility**
- Every set is public: `createSet`, `cloneSet` and `updateSet` take `LISTED` (or nothing) from anyone, and `UNLISTED` or `PRIVATE` only from staff with `sets.edit_any` (`400 bad_visibility` otherwise). New sets of every kind are `LISTED`, personal sets too.
- Staff keep the ability to take a set off the listings (`UNLISTED`) or hide it (`PRIVATE`), audited as `set.visibility_changed` through `sets.edit_any`: moderation needs a way to hide a reported set without deleting the owner's work. The owner can't list it again; staff can. The rules above for unlisted and private sets apply to those.
- **Migration** `20261215004102_sets_all_public.sql` (data only): every unlisted and private set becomes listed. Private *emotes* are not touched: they stay usable only by their owner, in their owner's sets (ADR 0020).
- The website has no visibility choice anywhere (new set, copy, set editor); a set staff hid shows a small mark on its card.

**Capacity**
- A set's capacity is 1 to the owner's `set_capacity_max` (a role limit, `limits::AccountLimit::SetCapacityMax`, default 1.000, `CYEX_TV_LIMIT_SET_CAPACITY_MAX`; a role can raise it, e.g. a future subscriber role to 2.500) for a channel set, 1 to 5 for a personal set, for the owner, the set's editors and account editors with `sets`. Above that, `400 capacity_over_limit`. A set staff raised can be lowered again by its owner (never raised past what it has).
- Staff with `sets.edit_any` (or `global_set.edit` for the global set) set any set from 1 to 1.000.000, audited through the permission. Over 1.000.000 or 0 is `400 bad_capacity`.
- Never below what the set holds: `409 capacity_too_low`.
- **Migration** `20261215004103_set_capacity_million.sql` widens the check on `emote_sets.capacity` to 1 to 1.000.000.
- **API (additive):** `setCapacityRange(id)` (`min`, `max`, `ownerMax`, `used`, `staff`; null when you can't change it), the role limit key `set_capacity_max` in `limitDefaults`/`roleLimits`/`setRoleLimits`.
- **Website:** a Capacity block in the set editor and "Change capacity" in the set menu (right-click a set card, a row of the browse rail, or the set page's ⋯), both showing the allowed range. The set menu also renames in place, copies the link, sets the set active on each channel you can, opens the set page or editor, and deletes (with a confirmation). The chat preview on the set page and in the set editor is gone.
- Tests: `apps/api/tests/sets_capacity_flow.rs`, `public_sets_flow.rs`, `lib/sets/*.test.ts`.
