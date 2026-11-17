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
