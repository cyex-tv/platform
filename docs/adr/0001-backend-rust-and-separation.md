# 0001: Rust backend, separate from cyex.app

- Status: accepted
- Date: 2026/11/16

## Context
cyex.tv is an emote platform for Twitch and Kick. A cyex backend already exists:
- `cyex-app/cyex-api` (api.cyex.app) is Node/Express with Postgres `deck-db`, Redis and ClickHouse;
- it serves cyex logs, recap, the deck's supporter badges, the store and payments.

Changes to it must never break those. cyex.tv's hot paths are:
- serving set lookups to many clients;
- thousands of open live-update connections;
- image transcoding (WebP, AVIF).

cyex deck, the first client, is Rust, and its `cyex-emotes` crate can share types with the server.

## Decision
- cyex.tv is its own product with its own API at `api.cyex.tv`, its own database, accounts, sessions and OAuth apps.
- Some endpoints duplicate cyex-api's (sign-in, `/me`). That is accepted.
- The backend is three Rust services in one Cargo workspace:
  - `api` (axum, HTTP)
  - `events` (SSE)
  - `worker` (image jobs)
- They share crates for models, config, auth and DB access.
- cyex.tv never connects to cyex-api's Postgres, Redis or ClickHouse, and cyex-api is not changed for cyex.tv.
- The only contact with cyex.app is clients reading the public badge API `api.cyex.app/badges/v1`.

## Consequences
- No change on cyex.tv can break cyex.app, and the reverse holds too.
- A cyex deck user has a separate cyex.tv login. This doesn't matter in v1, because the deck only reads cyex.tv (D19).
- Sign-in, sessions, the blocklist and the audit log are written again in Rust. Code is not shared across the two languages.
- One language (Rust) for the services and the deck client. SvelteKit for the website (0006).
