# 0032: API consistency pass

- Status: accepted (owner approved, 2026/11/26)
- Date: 2026/11/26
- Amends: 0008 (API contract), 0030 (cursor paging)

## Context
The API review (2026-11-26, section 3, items 3, 5, 9, 10, 12 and 14) found the same things done several ways: page sizes of six different kinds, limit errors with mixed statuses and no numbers, two upload routes with different answers, one `max-age` for every read, and a `scale` field that means something else on badges. Everything here is additive: the extension and the cyex deck read this API.

## Decision
- **One page-size rule** (`apps/api/src/paging.rs`). `perPage` (`per_page` over REST, `limit` for the feeds) is 1 to 100, default 50. The lists that always had other numbers keep them, in one table (`OVERRIDES`): `searchAccounts` 20/50, `setEmotes` and `GET /v1/sets/{id}/emotes` 120/200, the activity feeds 30/100, `emoteChannels` 8/50, the set card `preview` 12/50. A test reads the schema descriptions and fails when one says other numbers.
- **One table of limit errors** (`crates/shared/src/limit_errors.rs`). A limit the account or set has reached is `409` (`set_full`, `too_many_pending`, `too_many_editors`, `limit_reached`); new limit errors use it too. Caps on one request's size stay `400` (`too_many`, `bad_batch`, `too_many_tags`, `too_many_reasons`, `too_many_sets`), a file over its byte cap `413` (`too_large`), `capacity_over_limit` `400` (it shipped so). Every one carries `details { limit, current }` over REST and `extensions.limit` and `extensions.current` in GraphQL; `too_large` only when the bytes are over, since a side or frame cap names its numbers in the message. `rate_limited` is `429` with `Retry-After` and no details.
- **One upload answer.** `POST /v1/emotes` and `POST /v1/cosmetics/images` answer `202` with their object and the fields of `UploadResult`: `id`, `state`, `files`, `limits_used` (`kind`, `max_bytes`, `max_side`, `max_frames`, the uploader's effective numbers). An emote's `state` is its review state; a picture's is `processing`, then `ready`. Both document the same status list.
- **Caching per route** (`ReadCache` in `routes/v1.rs`). Channel 10 s, set and set page 30 s, emote 30 s, cosmetics 60 s, chatters' cosmetics 30 s, limits and global set 300 s, each with `stale-while-revalidate` and `stale-if-error`. All reads `Vary: Accept-Encoding`. The cosmetics catalog also varies by `Cookie` and `Authorization`, a signed-in answer is `private`, a staff answer `private, no-cache`; the shared copy is only for requests that name no account.
- **`width` is the contract of a file.** `EmoteFile.scale` is deprecated in the schema (reason: for badge and paint files; emote files keep 1 to 4) and listed in `/v1/meta` as `file_scale_non_emote`. Nothing is removed in v1.
- **Docs drift.** Figures that the code contradicts were corrected (the security assessment's 5 MB body limit; PLAN.md and the ADRs already said 8 MB).

## Alternatives
- *422 for limit errors:* the request is valid and the state is full, which is what 409 says; 422 would be a third status next to the 400 and 409 clients already match.
- *Moving every existing limit error to 409:* it would break clients that match on the status. The table documents the exceptions instead.
- *A separate `deprecated` type for badge files:* a new GraphQL type breaks typed fragments on `EmoteFile`.

## Consequences
- Clients can show "20 of 20" from any limit error without parsing messages.
- `EmoteFile.scale` shows as deprecated for emote files too in generated clients, though it is not going away for them; the reason text says so.
- Shorter `max-age` on channels means more revalidations (cheap `304`s) and a quicker view of a changed active set.
