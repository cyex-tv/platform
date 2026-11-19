# 0020: Emote visibility: public, unlisted, private

- Status: accepted
- Date: 2026/11/19

## Context
Emotes had one flag, `private`: only the owner can add it to sets, and search leaves it out. Sets have had three states since ADR 0017. The owner asked (answers of 2026/11/19) for an unlisted emote too: one people can share by link and add to their sets, which doesn't show in search, browse or the creator's page.

## Decision
- Migration `20261207000002_emote_unlisted.sql` adds `emotes.unlisted boolean NOT NULL DEFAULT false` and a CHECK that an emote is never both unlisted and private. Existing emotes keep what they were.
- The API shows `Emote.visibility`: `PUBLIC`, `UNLISTED` or `PRIVATE` (enum `EmoteVisibility`). `flags.private` stays, true exactly for `PRIVATE`.
- Writing it: `updateEmote(input: { visibility })` and the upload form field `visibility` (`public`, `unlisted`, `private`). The old `private` flag keeps working for clients that only know it: `private: true` makes the emote private; `private: false` makes a private emote public and leaves an unlisted one unlisted. Both at once must agree (`bad_visibility`, or `bad_form` on the upload).
- Unlisted: `emote(id)` opens it for anyone (with an "Unlisted" mark on the website), anyone can add it to their sets and clone it with a set, and its channels and its own activity show on its page. `searchEmotes` (search, browse and the creator's page, which is search by owner) and account activity feeds leave it out for everyone; its owner sees it in `myEmotes`.
- Private keeps its meaning: only its owner (and the owner's account editors with `emotes`) can add it to sets, and it is left out of the same listings.

## Consequences
- An unlisted emote that sits in a public set still shows there, as a private one does: visibility is about being found, not about chat.
- Activity lines are filtered by the emote's visibility now, not when the line was written, as for sets (ADR 0017).
- Moderation is unchanged: every new emote is reviewed, whatever its visibility.
