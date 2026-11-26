# 0030: Cursor (keyset) paging for long lists

- Status: accepted (owner's decision, 2026/11/26)
- Date: 2026/11/26

## Context
Most lists page with `page`/`perPage` (`OFFSET`). That has two faults on lists that grow without bound:

- Rows added or removed between two requests shift every later page: a reader sees a row twice or never. Browse sorted `NEW`, a busy review queue, an infinite list on the search page all hit this.
- `OFFSET n` makes Postgres read and drop `n` rows, so deep pages get slower. `MAX_PAGE` (1000, PR #176) caps the damage by refusing deeper pages, which also means nothing past page 1000 can be read.

The activity feeds and the audit log already page by a cursor (`before`, an entry ID; `ActivityPage.next`). The API review (2026-11-26, item 7 and section 3) asked for the same on the other lists, without breaking anyone.

## Decision
- **Keyset paging, additive.** Each list's order is a fixed tuple of keys ending in a unique ID (`cyex_tv_db::keyset::Key`). A page ends at its last row's key values; the next page reads the rows after that tuple (`WHERE (k1, k2, id) > ($1, $2, $3) ORDER BY k1, k2, id LIMIT n + 1`, the extra row only saying that another page follows). The order is the same as the numbered pages', so both ways show the same rows in the same order; a test walks every emote search both ways and compares.
- **Shape, matching what the schema already has.** Lists keep their page objects. They gain `next: String` (null on the last page, as `ActivityPage.next`), and their fields gain `after: String`. `page` and `perPage` work exactly as before; with `after`, `page` is ignored, the `perPage` caps are unchanged, there is no deepest page, and `total` is still counted. No Relay connection types: the schema has none and `items`/`next` is what clients already read.
- **Lists with cursors now:** `searchEmotes` (every sort and filter, browse included), `searchAccounts`, `setEmotes`, `myEmotes`, `accountEmotes`, `reviewQueue`, and REST `GET /v1/sets/{id}/emotes` (new: a set's approved emotes a page at a time, `per_page` and `after`, with `next` in the body and `Link: <...>; rel="next"`). The activity feeds and the audit log keep their `before` cursors.
- **Cursors are opaque and checked, not signed.** A cursor is base64url of a small JSON body (layout version, a hash of the list's name, order and filters, the key values) plus 8 bytes of SHA-256 over it. A cursor that was cut, edited or made up, or one from another list (other text, filters, order, set or account), is the stable error `bad_cursor` (400), never a silent wrong page. It isn't signed with a secret because it grants nothing: it only says where to start, and every read applies the caller's access rules and filters again. Clients must not read or build cursors; the layout may change (the version makes an old one `bad_cursor`).
- **Keys never NULL, all in one direction where it matters.** `NULLS LAST` orders become an "is missing" flag key plus `coalesce`. Where an order mixes directions on numbers (`popularity DESC, id ASC`), the key is negated (`-popularity`) so the whole tuple goes one way and "after" is one row comparison an index reads as a range. Text orders that mix directions (the review queue by name or uploader, a small table) bound the first key and break ties with `OR` terms.
- **Indexes** (additive migrations, `CREATE INDEX CONCURRENTLY`): `emotes_keyset_popular ((-popularity), id)` and `emotes_keyset_top ((-popularity), lower(name), id)` for browse by most used and `TOP` (same partial predicate as the browse indexes), `emotes_owner_newest (owner_account_id, created_at DESC, id DESC)` for an account's own emotes. `NEW` uses `emotes_browse_new`, set members `emote_set_members_set_newest`. The scale test checks, from `EXPLAIN`, that a page after a deep cursor reads its index in order with no sort.
- **Website.** The infinite and "Show more" lists that read pages one after another (search page, Add emotes, a creator's uploads, an account's uploads in admin) go on by `next`. The search page still restores `n=` pages by number on load (a bounded few), then continues by cursor. The UI is unchanged. The dev mock answers `next` (an offset it alone reads) and `bad_cursor`.

## Alternatives
- **Relay connections (`edges`, `pageInfo { endCursor hasNextPage }`):** the standard shape, but a second shape beside `items`/`next` that every client would have to learn, and new types for each list. Matching `ActivityPage` keeps one convention.
- **HMAC-signed cursors:** would need a new deployment secret and rotation plan for no gain, since a forged cursor can only start a list somewhere else the caller may read anyway.
- **Offset inside the cursor:** opaque, but keeps both faults (shifting pages, slow deep reads).

## Consequences
- Clients can page through any of these lists to the end, cheaply, without duplicates or gaps while it changes. Rows whose sort key changes during the walk (an emote's popularity after the nightly refresh, a rename under `TOP`) can still move across the position; that is inherent to any order by a changing value.
- Other lists (favourites, entitlements, orders, code batches, reports) keep numbered pages for now; they are admin tables with totals or small per-account lists. They can gain `after`/`next` the same way.
- Every keyed list must name its keys in one place (`*_keys()` in `crates/db`), next to the `ORDER BY` it mirrors.
