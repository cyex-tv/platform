# cyex.tv platform

The backend and website for cyex.tv, an emote platform for Twitch and Kick.

| Path | What |
|---|---|
| `apps/api` | The API at `api.cyex.tv/v1`: GraphQL at `/v1/gql` (and subscriptions at `/v1/gql/ws`), plus a small REST part (Rust, axum, async-graphql) |
| `apps/events` | The event WebSocket at `/v1/events` (Rust) |
| `apps/worker` | Image processing (Rust) |
| `apps/web` | The cyex.tv website (SvelteKit, from Phase 6) |
| `crates/shared` | Config, errors, logging, permissions, API types |
| `crates/db` | Postgres pool and migrations |
| `schema/schema.graphql` | The GraphQL schema, generated from the code |
| `openapi/openapi.json` | The REST part, generated from the code |

Status: Phase 1 (scaffold). See [PLAN.md](PLAN.md), [PROGRESS.md](PROGRESS.md) and the decisions in [docs/adr](docs/adr).

## Develop

You need Rust (stable, the `rust-toolchain.toml` picks it) and Docker.

Start Postgres (and MinIO, used from Phase 3):

```bash
docker compose -f dev/docker-compose.yaml up -d
```

Copy the example config:

```bash
cp .env.example .env
```

Run the API (it runs migrations on start), then check it:

```bash
cargo run -p cyex-tv-api
```

```bash
curl localhost:8080/health/ready
```

Open `http://localhost:8080/v1/gql` in a browser for GraphiQL.

The checks CI runs:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

```bash
cargo deny check
```

After changing the API, regenerate both descriptions and commit them (CI checks they match):

```bash
cargo run -q -p cyex-tv-api --bin schema > schema/schema.graphql
```

```bash
cargo run -q -p cyex-tv-api --bin openapi > openapi/openapi.json
```

## Licence

[AGPL-3.0](LICENSE). Clients that only use the public API are not affected by it.

Not affiliated with, endorsed by, or sponsored by Twitch or Kick.
