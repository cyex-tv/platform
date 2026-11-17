# 0013: How clients get the API's types

- Status: accepted (pending the owner's review)
- Date: 2026/11/17

## Context
PLAN.md Phase 4 asks how the deck (the first client) shares the platform's response types: "either through a small published crate or generated types". The facts that decide it:
- The platform is AGPL-3.0 (ADR 0007). `cyex-tv-shared` is part of it.
- `cyex-app/cyex-deck` is public but has no licence (all rights reserved). The extension and third-party chat clients have their own licences.
- Linking an AGPL crate into the deck would put the deck under the AGPL. ADR 0007 already says clients that only *call* the API are unaffected; that only holds if they don't *link* platform code.
- The contract is already published and checked in CI: `openapi/openapi.json` (REST) and `schema/schema.graphql` (GraphQL). Neither can drift from the code (ADR 0008).

## Decision
- **No platform crate is linked into clients.** Clients describe what they read themselves, from the published contract.
- **Rust clients (the deck):** small hand-written `serde` structs for the few REST reads they use (`GET /v1/channels/{platform}/{id}`, `GET /v1/sets/{id}`, `GET /v1/sets/global`, `POST /v1/personal-sets/lookup`) and the event WebSocket frames. Unknown fields are ignored, so additive changes never break them. Each client keeps fixtures copied from real platform output and tests its mapping against them.
- **TypeScript clients (the website, the extension):** types generated from `schema.graphql` with GraphQL Code Generator (PLAN.md Phase 6), and from `openapi.json` where they use REST.
- **The platform keeps the contract stable:** additive changes only within v1; anything breaking is a v2 path. CI's contract job already fails when the descriptions and the code disagree.
- If a permissively licensed client library is ever wanted, it gets its own repo and licence, written against the contract, not copied from the platform (ADR 0007's "own ADR" case).

## Consequences
- The deck's licence stays its owner's choice.
- A field a client needs must exist in the contract; a client can't reach into platform internals.
- A few structs are written twice (platform and client). They are small, and the fixtures catch mismatches.
