# 0008: API contract

- Status: accepted
- Date: 2026/11/16

## Context
Third-party clients will depend on the public read API. A hand-written spec drifts from hand-written handlers.

## Decision
- **Code-first OpenAPI with `utoipa`.** Handlers and types carry the annotations.
- `openapi/openapi.yaml` is generated and committed. CI regenerates it and fails if it differs.
- Versioned under `/v1`. Breaking changes need `/v2`. Within v1 we only add fields and endpoints.
- Errors are `{ "error": "<code>", "message": "<text>" }` with stable codes.
- The public read endpoints (sets, emotes, channels, personal lookup, search, events) need no auth, return CORS `*` without credentials, and send `ETag`.
- Credentialed endpoints allow only `https://cyex.tv`.
- The TypeScript client is generated with `openapi-typescript` for `apps/web` and the extension.
- The deck shares Rust types through the platform's `shared` crate or generated types (decided in Phase 4).

## Consequences
- The spec is always true.
- Annotations add noise to handlers, which is accepted.
