# cyex.tv platform

The backend and website for cyex.tv, an emote platform for Twitch and Kick.

| Path | What |
|---|---|
| `apps/api` | HTTP API at `api.cyex.tv/v1` (Rust, axum) |
| `apps/events` | Live updates over SSE (Rust) |
| `apps/worker` | Image processing (Rust) |
| `apps/web` | The cyex.tv website (SvelteKit, from Phase 6) |
| `crates/shared` | Config, errors, logging, permissions, API types |
| `crates/db` | Postgres pool and migrations |
| `openapi/openapi.json` | The API contract, generated from the code |

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

After changing an endpoint, regenerate the spec and commit it:

```bash
cargo run -q -p cyex-tv-api --bin openapi > openapi/openapi.json
```

## Licence

[AGPL-3.0](LICENSE). Clients that only use the public API are not affected by it.

Not affiliated with, endorsed by, or sponsored by Twitch or Kick.
