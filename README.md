# cyex.tv platform

The backend and website for cyex.tv, an emote platform for Twitch and Kick.

- `apps/api`: HTTP API at `api.cyex.tv/v1` (Rust)
- `apps/events`: live updates over SSE (Rust)
- `apps/worker`: image processing (Rust)
- `apps/web`: the cyex.tv website (SvelteKit)

Status: planning. See [PLAN.md](PLAN.md), [PROGRESS.md](PROGRESS.md) and the decisions in [docs/adr](docs/adr).

Licence: [AGPL-3.0](LICENSE). Clients that only use the public API are not affected by it.

Not affiliated with, endorsed by, or sponsored by Twitch or Kick.
