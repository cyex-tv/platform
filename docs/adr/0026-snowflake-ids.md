# 0026: Snowflake IDs for emotes, accounts and sets

Status: accepted (owner, 2026/11/21)

## Context
Emotes, profiles and sets are shared by link: `cyex.tv/emotes/<id>`, `/users/<id>`, `/sets/<id>`.
Their IDs were ULIDs (ADR 0002), 26 characters like `01JD8X3Q5Z…`, the same shape 7TV uses. The
owner wants numbers instead, the way Discord and Twitter write them (`1314628736410763264`): they
read as an ID, sort by time and don't look borrowed.

Every table is keyed by a uuid that holds the ULID, and every join, foreign key, cursor and cache
key uses it. Changing the keys would touch every table and every query for no gain inside.

## Decision
- **Format.** A positive 64-bit integer: 41 bits of milliseconds since the cyex epoch
  `2026-01-01T00:00:00Z` (Unix ms 1767225600000; enough until 2095), 10 bits of instance,
  12 bits of sequence. Written everywhere as a decimal string, never a JSON number (JavaScript
  can't hold 64-bit integers): GraphQL `String`, REST strings, event payload strings, URLs.
- **Who gets one.** Accounts (profiles), emotes and emote sets, the things in shared URLs. Sets
  too, so every shareable link looks the same. Badges, paints, roles, orders, items, code
  batches, images, notifications, audit entries, sessions and everything else keep ULIDs: they are
  not in links people share.
- **Made in Postgres.** `snowflake_id()` (migration `20261212000001`) is the default of a new
  `public_id bigint NOT NULL UNIQUE` column on `accounts`, `emotes` and `emote_sets`. One generator
  for all three tables and all api instances: it keeps the last ID in a sequence (non-transactional,
  so no row lock lives until the caller's commit) behind a short advisory lock, takes
  `clock_timestamp()`, and hands out strictly increasing IDs: 4096 a millisecond, then it borrows
  the next millisecond; a clock that steps back continues from the last ID. IDs are therefore
  unique across the three tables, which the edges rely on. The instance is
  `cyex_tv.snowflake_instance` (0 to 1023, default 0), for a second database should there ever be
  one. `snowflake_time(id)` reads the time back.
- **Backfill.** Existing rows, oldest first across all three tables, get the time part from
  `created_at` (rows from before the epoch get its first millisecond) and a sequence part that
  keeps them unique; a millisecond with more than 4096 rows spills into the next. The generator
  then starts after the highest one.
- **Uuids stay inside; the edges translate.** No query, join or row type changed. Translation
  happens in one place per edge, by field name:
  - GraphQL: the `PublicIds` schema extension (`apps/api/src/public_ids.rs`). Before a request
    runs, it reads every Snowflake-shaped string in the query text and the variables and looks
    them up at once (`public_ids::resolve`). The request runs with those (`public_ids::resolving`,
    a task-local), so `ids::from_text`, and with it `errors::parse_id` and every resolver, takes
    either form. After it, every field named `id`, `ids`, `done` (batch outcomes), `…Id`, `…Ids` or `…By` (`addedBy`, `createdBy`) whose value is the ULID
    of an account, emote or set gets the Snowflake instead, also inside the JSON text of
    `before`, `after` and `payload` (audit entries, notifications) and the account export
    (`exportMyData`). One query per request each way, only when
    there is something to look up.
  - REST: the cacheable reads are translated when their body is built (so the cached body and its
    ETag already carry Snowflakes), uploads and the personal-set lookup when they answer
    (`public_ids::publish`; snake_case `…_id` fields too); IDs in paths and the upload's
    `owner_id` go through `public_ids::canonical`.
  - Events: `outbox::publish` stores the event as clients get it: `set:<snowflake>` topics, the
    payload's IDs by Snowflake, and the set's Snowflake as `payload.set_id` (the `set_id` column
    keeps the uuid for the read caches). Replays need no translation.
- **Transition.** Every input that takes an account, emote or set ID takes the Snowflake or the
  old ULID, until further notice (removing the ULID form will be its own decision, announced in
  the changelog first). Answers always carry Snowflakes. `set:<ULID>` still subscribes, on the
  event socket and on the GraphQL `events` subscription: it is mapped to the Snowflake topic, and
  that client gets the set's events under the name it subscribed with; payloads carry
  Snowflakes either way. Cursors were page numbers or audit-entry IDs (ULIDs), so none changed.
- **Website.** Links and routes use Snowflakes. `/emotes/<ULID>`, `/users/<ULID>` and
  `/sets/<ULID>` answer `308` to the Snowflake URL, query kept (the browser keeps the fragment);
  unknown IDs are not found as before. Stored IDs in the browser (rail state, the target set)
  tolerate the old form.

## Consequences
- Clients that store IDs see new ones in answers: they must treat IDs as opaque strings and
  accept digits. The extension and cyex deck move over in their own changes; old stored ULIDs keep
  working as input meanwhile.
- A Snowflake reveals when the row was made, as the ULID already did.
- Field names matter now: a new field holding an account, emote or set ID must be named `id`,
  `…Id`, `…Ids` or `…By` (REST `…_id`, `…_by`), or it is shown as the ULID. An alias that drops the suffix (`x: id`)
  shows the ULID too. Rows that are gone (an erased account named in the audit log) have no
  Snowflake and stay ULIDs.
- Tests that compare answers with a row's uuid read its Snowflake (`public_ids::public_text`).
- The lookups cost one indexed query per GraphQL request that has IDs; a process-wide cache is
  possible (the mapping never changes) if that ever shows.
