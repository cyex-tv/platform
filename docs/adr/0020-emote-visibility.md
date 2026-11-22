# 0020: Emote visibility: public, unlisted, private

- Status: accepted
- Date: 2026/11/19

## Context
Emotes had one flag, `private`: only the owner can add it to sets, and search leaves it out. Sets have had three states since ADR 0017. The owner asked (answers of 2026/11/19) for an unlisted emote too: one people can share by link and add to their sets, which doesn't show in search, browse or the creator's page.

## Decision
- Migration `20261207000041_emote_unlisted.sql` adds `emotes.unlisted boolean NOT NULL DEFAULT false` and a CHECK that an emote is never both unlisted and private. Existing emotes keep what they were.
- The API shows `Emote.visibility`: `PUBLIC`, `UNLISTED` or `PRIVATE` (enum `EmoteVisibility`). `flags.private` stays, true exactly for `PRIVATE`.
- Writing it: `updateEmote(input: { visibility })` and the upload form field `visibility` (`public`, `unlisted`, `private`). The old `private` flag keeps working for clients that only know it: `private: true` makes the emote private; `private: false` makes a private emote public and leaves an unlisted one unlisted. Both at once must agree (`bad_visibility`, or `bad_form` on the upload).
- Unlisted: `emote(id)` opens it for anyone (with an "Unlisted" mark on the website), anyone can add it to their sets and clone it with a set, and its channels and its own activity show on its page. `searchEmotes` (search, browse and the creator's page, which is search by owner) and account activity feeds leave it out for everyone; its owner sees it in `myEmotes`.
- Private keeps its meaning: only its owner (and the owner's account editors with `emotes`) can add it to sets, and it is left out of the same listings.

## Consequences
- An unlisted emote that sits in a public set still shows there, as a private one does: visibility is about being found, not about chat.
- Activity lines are filtered by the emote's visibility now, not when the line was written, as for sets (ADR 0017).
- Moderation is unchanged: every new emote is reviewed, whatever its visibility.

## Amendment (security review, 2026/11/20)
- One rule for "listed": `search::LISTED_EMOTE` (approved, not deleted, not private, not unlisted, owner not pending deletion) is the one SQL fragment `searchEmotes` (every sort, trending included), `quickSearch` and an emote's trending rank use, so they can't drift. `quickSearch` had missed unlisted emotes and pending-deletion owners. NSFW stays each caller's own rule.
- Activity: an unlisted emote's own feed shows its lines (as its page is open by link); other feeds still leave it out. Emotes of an account whose deletion is pending leave every feed (ADR 0019).

## Amendment (owner, 2026/11/22): two creator flags, no NSFW, unlisted is for staff
Owner: "nsfw is not an emote flag, nsfw emotes are not allowed. users can only tick to have an emote be overlaying (zero width) or private (only for their own channel)" and "all emotes can only be uploaded with the zero width and or private flag, not unlisted, all other emotes are public by default".
- Creators (owners and their account editors) set two flags: **Overlaying** (`zero_width`) and **Private**. Everything else is public. Upload and `updateEmote` refuse `unlisted` (`400 bad_visibility`) unless the caller holds `emotes.flags` or `emotes.edit_any`; an owner also can't change the visibility of an emote a moderator unlisted (both ways, so private-then-public can't undo it). Staff keep all three states (`setEmoteFlags`, `approveEmote`, `updateEmote` with `emotes.edit_any`): a moderator "removing an emote's public listing" makes it unlisted (ADR 0024).
- **NSFW emotes aren't allowed**, so NSFW is no flag. The `emotes.nsfw` column stays (additive rule) but nothing sets it any more:
  - the upload's `nsfw=true` and `updateEmote`/`setEmoteFlags` with `nsfw: true` are `400 nsfw_not_allowed` with a message saying so; `nsfw=false`/`nsfw: false` is still taken, so old clients keep working;
  - `EmoteFlags.nsfw` is deprecated and always `false`; `searchEmotes(nsfw:)` is deprecated and changes nothing; the `NSFW` value of `EmoteFlag` is deprecated and finds nothing;
  - an emote still flagged from before is **kept off every listing** (`search::LISTED_EMOTE` now includes `NOT e.nsfw`, so search, browse, the creator's page, quick search, trending and the same-file notice all leave it out) until staff save its flags: `setEmoteFlags` clears the old flag (staff looking at it is its review), or they take it down. Its page still opens by link, as an unlisted emote's does. We chose hiding over treating them as normal because an emote someone marked NSFW is likely against the new rule.
- Migration `20261215000001_retire_nsfw_reason.sql` rewords the seeded reject reason "NSFW but not marked" to "NSFW or sexual content (not allowed)" (only the untouched seed row).
- The website: the upload dialog and the emote edit page show the two checkboxes; staff's Edit emote flags keeps Visibility (public, unlisted, private) and Zero-width; the browse NSFW filter, NSFW tile and review marks are gone; the guidelines say NSFW content is not allowed and is removed.
