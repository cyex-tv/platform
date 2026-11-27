# 0038: Personal set slots

- Status: accepted (owner decision, 2026/11/27)
- Date: 2026/11/27
- Builds on: 0010 (roles), 0017 (set capacity), 0031 (role limits), 0016 (shop)

## Context
A personal set (emotes that go with their owner into every channel that allows them) held 5 emotes, an owner could lower it, and an account made one only if it chose to. The owner wants every account to have one with 3 slots, 10 for subscribers, a staff-only way to give one set a different number, aliases, and no silent deletion when someone stops being a subscriber.

**What "subscriber" is in this codebase:** nothing yet. The shop (0016) sells badges and paints bought once through Paddle; `accountSubscriptions` exists for staff and is always empty ("cyex.tv doesn't sell any yet"); there is no subscription table, role or entitlement kind. Roles do exist (0010) and carry per-account limits (0031), which the admin role editor already hints at ("a subscriber role could raise it", ADR 0017).

## Decision
- **Subscriber = holding a role whose `personal_set_slots` role limit is set** (a new `AccountLimit`, default 3, not an environment setting). A "Subscriber" role with `personal_set_slots` 10 is made in the admin role editor; when a subscription product exists, its fulfilment assigns and removes that role (the same way staff do by hand today). Several roles: the most generous number counts, like every role limit. No new table, permission or enum, and nothing in the checkout path changes. A role flag was rejected: the limit already is a flag with a number, the role editor edits it, and it works for gifts, giveaways and staff grants too.
- **Effective capacity of a personal set** = `capacity_override` if set, else the owner's `personal_set_slots` (3 without a role), computed in the database by `personal_set_capacity(owner, override)` and read as `emote_sets.capacity` by every query in `crates/db/src/sets.rs`, so every reader and the "holds at least capacity" check in `add_member` see one number. The stored `capacity` of a personal set is ignored from now on. `EmoteSet` gains `capacityOverride` (null when the roles decide).
- **Only staff change it.** `updateSet(capacity)` on a personal set needs `sets.edit_any` and is `403 forbidden` for the owner, their editors and account editors; with the permission it sets the override (1 to 1.000, never below what the set holds; `bad_capacity` above), audited as `set.capacity_changed` with the capacity and override before and after. `updateSet(resetCapacity: true)` drops the override. `setCapacityRange` is null for everyone else. The cap of 1.000 (not 1.000.000) keeps the 100-chatter personal-set lookup answer small (0035).
- **Losing the role keeps the members.** A set that holds more than its capacity keeps every emote; `addSetEmote` answers `409 set_full` with `details {limit: capacity, current: members held}` until it is under the limit. Removing emotes, renaming, aliasing and reading work as before. Nothing deletes members.
- **Creation.** Sign-up makes the personal set next to the default channel set (`sets::ensure_personal`, idempotent); migration `20261230000201_personal_set_slots.sql` makes one for every live account without one, and keeps the number of sets staff had raised above 5 as an override. `createSet(kind: PERSONAL)` still works if an account deleted theirs. A deleted account (undo window) gets none until it signs in as new.
- **Aliases and channels are unchanged:** a member's name in the set may differ from the emote's (`addSetEmote(name)`), and personal sets work in any channel with `allow_personal_emotes` on; channels can turn them off.
- **Website:** the set editor shows "x of N slots" for a personal set, a note when staff set the number or the set is over its limit, and an upsell line ("Subscribers get 10 slots") only once `SUBSCRIPTION_OFFERED` is true (it is false: nothing is sold). Staff see the override field (the existing capacity form) with "Use the plan's slots" to drop it. The role editor lists "Personal set slots".
- **Another change runs in parallel** (approved-for-personal-use emotes only); this one does not touch that check.

## Consequences
- With no subscription product, nobody is a subscriber: everyone gets 3 unless staff give a role or an override. The owner decides which role a future subscription grants and sets it to 10.
- Every personal-set read runs one small role lookup (`account_roles` by account, indexed) per personal set row; channel sets are unaffected.
- The `personal_set_capacity` default (3) is repeated in SQL and in `sets::PERSONAL_CAPACITY`; a db test checks they agree.
- Accounts whose personal set held 4 or 5 emotes keep them (3 slots now): no new adds until under 3.
- Tests: `apps/api/tests/personal_set_slots_flow.rs` (default 3, subscriber 10, override by permission only, owner can't, full is 409, losing the role, alias) and `crates/db/tests/queries.rs`; web `lib/sets/capacity.test.ts`.
