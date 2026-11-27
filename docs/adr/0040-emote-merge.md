# 0040: Merging emotes, and downloads in GIF, WebP or AVIF

- Status: accepted (owner request of 2026/11/27)
- Date: 2026/11/27
- Builds on: 0035 (set versions and delta sync), 0038 (personal set slots), 0039 (emote approvals)

## Context
The same picture often exists as several emotes (re-uploads, an old and a better version). Channels each picked one, so a duplicate can't simply be deleted: deleting takes it out of every set, and chat loses the emote. The owner wants staff to merge a duplicate into the emote that should stay, with every set that used it keeping a working emote, old links and clients resolving, and nothing left behind that channels can't remove. The owner also wants the emote page's More menu to download a chosen format and size, and Copy name to go.

## Decision

**Who and how**
- A new permission, `emotes.merge` ("Merge emotes"), given to the built-in Admin role by the migration (`20261231000201_emote_merge.sql`). Anyone else gets `403 forbidden`, the emote's owner included.
- GraphQL (additive): `emoteMergePreview(id, intoId)` → `EmoteMergePreview { source, target, followed, moves, collapses }` changes nothing; `mergeEmote(id, intoId)` → `EmoteMergeResult { sourceId, target, moved, collapsed }`. `MergeCollapse { setId, setName, keptName, droppedName }`. No REST mutation: staff tools are GraphQL.
- Website: the emote page's More menu has **Merge into another emote…** for `emotes.merge` holders. A dialog takes the target's link or ID, shows both emotes side by side with what will happen (memberships that move, sets that already hold the target with both names, the rules below), and merges on confirm, then opens the target's page.

**What a merge does** (one transaction)
1. The target given is followed if it was merged itself (`followed`), to the final emote.
2. Refused: the source as target, also after following (`400 merge_into_self`); a target that is deleted, pending, rejected or taken down (`409 merge_target_unavailable`); a source merged already (`409 already_merged`); an unknown or deleted source (`404 not_found`). Any source state is allowed (a pending duplicate can go into the approved original).
3. Every set membership of the source (channel, personal, global, any set) names the target instead, **keeping its name in the set** and its zero-width flag. A set that already holds the target keeps the target's membership with the target's name and drops the source's; the answer lists those sets (`collapsed`, with both names, so staff see where chat now types a different name).
4. Name, tags and files are the target's; nothing of the source is copied. Favourites and "Hide for me" of the source move to the target (once per account).
5. The source is **retired**: `deleted_at` is set and `merged_into` names the target (`merged_at`, `merged_by`). Being deleted, it is out of search, browse, sets and every write path with no new checks; a CHECK keeps `merged_into` only on deleted rows. Emotes merged into the source earlier are pointed at the new target in the same transaction, so a chain is never more than one step (reads still follow up to 8 steps, as a guard).
6. Both emotes' channel counts are recounted in the transaction (the count triggers also queue them for the worker).
7. Audited as `emote.merged` on both emotes (`after`: both IDs and names, `moved`, `collapsed`, `"via": "emotes.merge"`); each owner gets the notification `emote.merged` (`role`: `source`, `target`, or `both` when one account owns both; not sent to the one who merged). Each affected set gets `set.emote_removed` for the source (by ID only: the source is retired by then) and, where it moved, `set.emote_added` for the target with its name (personal sets: `personal.updated`, as always).

**Locks** (matching ADR 0035's triggers)
- Both emotes are locked first, in ID order, `FOR NO KEY UPDATE`: that waits for any other change of either emote but not for a member insert, which takes `FOR KEY SHARE` on its emote while it holds its set's lock.
- Then every set holding the source, in ID order, `FOR NO KEY UPDATE`: the order and lock the set-version triggers take once #203 (review fixes) lands, taken up front so the triggers find them held. Until then main's triggers ask `FOR UPDATE` on those rows, a stronger lock than held; the rare deadlock that upgrade can meet is retried (below).
- Postgres can still pick a merge as a deadlock victim against some other ordering; `40P01` is retried, at most three tries in all, and a target merged between the lookup and the lock is followed on the next try.

**Delta sync**
- Moving memberships is one `UPDATE emote_set_members SET emote_id = target`: the existing statement trigger logs it as the source `removed` and the target `added` in each set, one version per set. A collapse is a `DELETE`: the source `removed`. Retiring the source bumps nothing more (it is in no set by then).
- So `setChanges(since)` / `GET /v1/sets/{id}?since=` answer the source in `removedIds` and the target in `added` (with its kept name) for a moved membership, and the source in `removedIds` alone for a collapse. Clients need nothing new.

**Old IDs**
- REST `GET /v1/emotes/{source}` answers `301 Moved Permanently` with `Location: /v1/emotes/{target}` (`Cache-Control: public, max-age=300`). It is asked only when the normal read misses, so live reads cost nothing more, and only when the target is visible signed out.
- GraphQL `emote(id: source)` and `emotes(ids)` answer the target's emote under the asked `id`, with the new field `Emote.mergedInto` (the final target's ID), by the target's visibility rule. Old clients that ignore it still show the right picture; new ones (the website) go to the target. `mergedInto` is null on every other answer and absent from REST bodies.
- The website's emote page redirects (301 on the server, a replaced history entry in the browser) from a merged ID to the target.

**Download chooser**
- Copy name is gone from the More menu (Copy link stays).
- Download asks the **format** first (GIF, WebP, AVIF), then animated or still frame (animated emotes only), then the **size** (1x to 4x that exist, with pixel sizes). Files save as `name_2x.webp`, `name_2x.avif`, `name_2x.gif`, and `name_2x_still.webp` for a still frame.
- What exists: the CDN has WebP and AVIF at 1x to 4x (animated WebP, animated AVIF where it is smaller, still frames) and **no GIF**; the uploaded original is private storage (ADR 0003) and emotes make no `original` file (ADR 0012), so "original size" is not offered. Serving originals publicly would expose uploads that were never meant to be public files, and adding GIF to the transcoder would grow every emote's storage for a rare download.
- **GIF is made in the browser**: the WebP of the chosen size and kind is fetched from the CDN (CORS, no credentials), decoded and re-encoded as GIF by the emote editor's worker (gifenc, already used to export GIFs). GIF has one-bit transparency, so soft edges turn hard; the chooser says so. If the fetch or the conversion fails, it says so and suggests WebP. WebP and AVIF download the CDN file as it is.

## Alternatives
- **Keep the source live as an alias row** (a second name for the target): every read, search and set write would need to know about aliases. Retiring it as deleted reuses every existing rule.
- **Hard-delete the source and rewrite references**: loses the audit trail and breaks old links outright.
- **Answer GraphQL with the target's own ID**: clients caching by ID would see an answer that doesn't match what they asked for, with no field saying why; keeping the asked ID plus `mergedInto` is explicit.
- **Copy the source's set names onto the target**: names are per set already; the target keeps its own name and tags, as the owner asked.
- **GIF on the CDN**: more storage and transcode time for every emote, for a rare download.

## Consequences
- Duplicates can be cleaned up without breaking any channel's chat; channels remove the target like any emote afterwards.
- A merge can't be undone (the source's memberships are gone into the target's). The confirm screen says so.
- Deleted emotes now include merged ones; anything counting deleted emotes (the upload quota counts them) counts merged ones too, as before the merge.
