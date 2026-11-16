# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Phase 1: Monorepo scaffold, CI, API contract (in progress)

Branch `phase-1/scaffold`, one PR for the whole phase (owner's call). There is one commit per task.

The code was written on a machine where nothing may be run without asking, and the owner chose "write code only". So **nothing has been compiled or run locally yet**, and CI on the PR does the first build.

### Tasks
- [x] 1. Cargo workspace (`apps/api`, `apps/events`, `apps/worker`, `crates/shared`, `crates/db`), with `rustfmt.toml`, `rust-toolchain.toml`, workspace lints, `deny.toml` (AGPL-compatible licence allowlist), and `.gitattributes` (LF).
- [x] 2. `dev/docker-compose.yaml`: Postgres 17 and MinIO, both on 127.0.0.1 only, plus `.env.example`.
- [x] 3. `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, test (with a Postgres service), release build, OpenAPI drift check, cargo-deny.
- [x] 4. The v1 contract is drafted as `utoipa` stubs in `apps/api/src/routes/v1.rs`, covering auth, accounts, channels, sets, personal sets, emotes, search, moderation, roles and events. Every stub answers `501 not_implemented`. The API types are in `crates/shared/src/model.rs`.
- [x] 5. `/health` and `/health/ready` (checks the DB). Config comes from `CYEX_TV_*` env (the DB URL is never logged), with JSON or pretty logs, graceful shutdown, a DB pool limit and a statement timeout set on every connection.

### Not done yet
- [ ] The first CI run is green.
- [ ] `openapi/openapi.json` is committed. It has to be generated first (`cargo run -q -p cyex-tv-api --bin openapi > openapi/openapi.json`), or taken from CI's `openapi` artifact. The drift check fails until it's committed.
- [ ] `Cargo.lock` is committed, from the first build or CI's `cargo-lock` artifact.
- [ ] `docker compose up` is checked by hand, including the MinIO image tag.
- [ ] The owner reviews the API contract before Phase 2 handlers are written.

### Changes from the plan
- The spec is `openapi/openapi.json`, not `.yaml`. The YAML output needs `serde_yaml`, which is unmaintained (ADR 0008 updated).
- `crates/auth` is created in Phase 2, where its code lands, instead of as an empty crate now.
- `apps/web` starts in Phase 6. CI gets the web build and lint then.

### Tests in this phase
- `crates/shared`:
  - config defaults, the required DB URL, named errors for bad values, `Debug` hiding the DB URL;
  - permission names unique and namespaced;
  - platform parsing and JSON round trip.
- `apps/api/tests/http.rs`:
  - `/health` is 200;
  - `/health/ready` is 503 without a DB and 200 with one (CI only);
  - stubs answer 501 `not_implemented`;
  - unknown paths are 404;
  - the spec lists the contract's paths and schemas.

## Phase 0: Setup and decisions (done, except the owner steps below)

### Tasks
- [x] 1. Owner: GitHub CLI installed and logged in as `sery-yx` (scopes `repo`, `read:org`, `workflow`, `gist`), 2026/11/16
- [ ] 2. Owner: register cyex.tv's own Twitch and Kick OAuth apps (values in `docs/setup/owner-steps.md`)
- [ ] 3. Owner: Cloudflare zone `cyex.tv`, DNS, R2 bucket on `cdn.cyex.tv` (tunnel hostnames wait until deploy)
- [x] 4. Created `cyex-tv/platform`, `cyex-tv/extension`, `cyex-tv/infra` with the scoped procedure, after the owner confirmed, 2026/11/16. Checks before creation: all three names were free and the org was empty.
- [x] 5. ADRs 0001-0011 written (`docs/adr/`), 2026/11/16
- [x] Q3: Kick API researched (`docs/research/kick-api.md`, ADR 0009), 2026/11/16. Owner still has to read Kick's Developer Terms.

### Decisions made 2026/11/16
D1-D21 in `PLAN.md`. The licence is AGPL-3.0 (ADR 0007).

### GitHub write log
2026/11/16, account `sery-yx`. All of these ran after the owner's explicit OK.

```
gh repo create cyex-tv/platform  --public --license agpl-3.0 --description "cyex.tv emote platform: Rust API, live events, image worker and the SvelteKit website"
gh repo create cyex-tv/extension --public --license agpl-3.0 --description "cyex.tv browser extension: emotes, badges and paints in Twitch and Kick chat"
gh repo create cyex-tv/infra     --public --license agpl-3.0 --description "cyex.tv deployment, Cloudflare config, backups and runbooks"
```

For each of `platform`, `extension` and `infra`:

```
gh api -X PUT   repos/cyex-tv/<repo>/branches/main/protection --input protection.json
gh api -X PATCH repos/cyex-tv/<repo> --input security.json
git push -u origin setup/initial
gh pr create --base main --head setup/initial
```

- `protection.json`: PR required (0 approvals), `enforce_admins: true`, no force pushes, no deletion.
- `security.json`: secret scanning and push protection on.
- Result checked by reading the settings back: all three match.

Same day, at the owner's request: no tool attribution anywhere. The rules file was renamed to `AGENTS.md`. The one setup commit per repo was re-authored as the owner's account, with no trailers, and force-pushed to the unmerged `setup/initial` branches only. This was a one-time exception the owner approved; `main` was not touched. The PR descriptions were edited.

```
git commit --amend --reset-author
git push --force-with-lease origin setup/initial
gh pr edit 1 --body-file <file>
```

`cyex-tv/infra` PR 1 had already been merged with the old commit, so infra's `main` was rewritten, with the owner's explicit OK:

```
gh api -X DELETE repos/cyex-tv/infra/branches/main/protection
git push --force-with-lease=main:<old main> origin HEAD:main
gh api -X PUT repos/cyex-tv/infra/branches/main/protection --input protection.json
```

infra `main` is now `Initial commit` → `b31fc33`. The protection was read back afterwards and matches the other repos.
