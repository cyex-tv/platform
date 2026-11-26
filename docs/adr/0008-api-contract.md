# 0008: API contract

- Status: accepted (revised 2026/11/16: GraphQL-first)
- Date: 2026/11/16

## Context
The website, admin tools, the deck and third-party clients all read and write the same data:
- nested data, such as a channel → its active set → its emotes → their files and owners;
- different views of it: a picker needs little, while an admin page needs everything.

The owner wants the API to be GraphQL wherever that works, with private parts closed off. Some things don't fit GraphQL well:
- OAuth redirects;
- streamed file uploads with size limits;
- health checks;
- the simple, edge-cacheable GETs that chat clients poll.

A hand-written spec also drifts from the code.

## Decision
**GraphQL is the main API, at `https://api.cyex.tv/v1/gql`.**
- Built with `async-graphql`:
  - `POST` runs queries and mutations;
  - `GET` with a query (or a persisted-query hash) runs queries only, cacheable when signed out (ADR 0032);
  - `GET` in a browser opens GraphiQL;
  - subscriptions use `wss://…/v1/gql/ws` (ADR 0005).
- It covers accounts and identities, channels, sets and editors, emotes, search, personal sets, moderation, roles and permissions, and later cosmetics.
- **One public schema; private parts need auth.** Admin and moderation fields and mutations need a session **and** the named permission (ADR 0010). Each resolver checks it first, through one helper, so it's visible in review. Introspection stays on, so developers can explore the public schema. Admin fields show up in the schema, but answer `signed_out` or `forbidden` to anyone without the permission.
- **Limits:** query depth at most 12 and complexity at most 1.000 in v1, plus the rate limits per IP and account from Phase 7. Persisted queries over `GET`: ADR 0032.
- **Errors:** GraphQL errors carry `extensions.code` with the same stable codes as REST (`not_implemented`, `signed_out`, `forbidden`, `not_found`, `set_full`, …).
- **Naming:** GraphQL fields are camelCase and enum values SCREAMING_SNAKE, as is usual.

**REST, only where it fits better, under `/v1`:**
- Auth:
  - `GET /v1/auth/{platform}/login`
  - `GET /v1/auth/{platform}/callback`
  - `POST /v1/auth/logout`
- Upload: `POST /v1/emotes` (multipart, streamed, size-limited).
- **Error codes and limit headers:** the stable error codes are listed once, in `crates/shared/src/error_codes.rs`, published as `components.schemas.ErrorCode` in the OpenAPI document and as a table on the developer docs; a test fails when the source uses an unlisted code. Every `429` sends `Retry-After`; the public reads and `POST /v1/gql` also send `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset`.
- **The chat-client read API** (public, no auth, `ETag` + `Cache-Control`, cacheable at Cloudflare):
  - `GET /v1/channels/{platform}/{id}`: the channel and its active set ID;
  - `GET /v1/sets/{id}`
  - `GET /v1/sets/global`
  - `GET /v1/emotes/{id}`
  - `POST /v1/personal-sets/lookup`
  - `POST /v1/sets/lookup`, `POST /v1/channels/lookup` and `GET /v1/sets/{id}?since=` (ADR 0032)
  
  These return the same shapes as GraphQL, but in snake_case JSON. This is what a chat client polls, so it must stay simple and stable.
- Health: `GET /health`, `GET /health/ready`.

**Generated, committed and checked**
- `schema/schema.graphql`: the GraphQL SDL, from `cargo run -q -p cyex-tv-api --bin schema`.
- `openapi/openapi.json`: the REST part, from `cargo run -q -p cyex-tv-api --bin openapi`. It is code-first with `utoipa`, and `utoipa-axum` builds the router and the spec from the same handlers. It is JSON because utoipa's YAML output needs `serde_yaml`, which is unmaintained.
- CI regenerates both and fails if either differs from what's committed.

**Versioning**
- Everything is under `/v1`. Within v1 we only add fields, types and endpoints.
- GraphQL fields are deprecated with `@deprecated` before removal, and only removed in v2.
- Every answer carries `X-Cyex-Api-Version` (semver). `GET /v1/meta` and the GraphQL `meta` query give the version, the build, where the limits are published and the list of deprecations (id, what, since, sunset, replacement). A request that uses a deprecated input gets `Deprecation`, `Sunset` and `Link: </v1/meta>; rel="deprecation"` headers, and a GraphQL answer `extensions.deprecations` (first user: ULID ids, ADR 0027).

**Clients**
- The website and the extension generate TypeScript types from `schema.graphql` (GraphQL Code Generator).
- Rust clients (the deck) share the types from `crates/shared`, where the GraphQL derives sit behind the `graphql` feature.

## Consequences
- Clients fetch exactly what a view needs in one round trip, and admin tools grow without new endpoints.
- Two descriptions (SDL and OpenAPI) to keep current. Both are generated and drift-checked, so this costs nothing by hand.
- GraphQL responses over `POST` don't cache at the edge. That's why the chat-client reads stay REST (queries over `GET` can, ADR 0032).
