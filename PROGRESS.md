# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Phase 2: Data model, auth, roles (in progress)

Paused on 2026/11/16 until work moves to a machine where commands can run (owner's call). Resumed on the new machine on 2026/11/16. Rust, `git` and `gh` are set up there; Docker isn't installed yet (the owner installs it), so the database tests ran in CI first.

### PR 1: migrations and db queries (branch `phase-2/db`, in review)
- [x] Migration `20261117000001_core.sql`: every Phase 2 table from PLAN.md, with the indexes it lists (`(platform, platform_user_id)`, `(platform, platform_channel_id)`, `lower(name)` and a `pg_trgm` index on emote names, set owner, emote state), plus indexes for logins, sessions, editors, roles, audit and the outbox.
- [x] The outbox insert trigger sends `NOTIFY cyex_tv_events, '<id>'` (ADR 0005).
- [x] Query modules in `crates/db`: `accounts`, `sessions`, `channels`, `emotes`, `files`, `sets`, `roles`, `audit`, `outbox`, `ids`. Each takes `&mut PgConnection`, so the api can put a change, its audit row and its event in one transaction.
- [x] Rule violations are named errors (`DbError`): `identity_taken`, `last_identity`, `set_full`, `name_taken`, `not_approved` and others.
- [x] `crates/shared`: text forms for `EmoteState` and `SetKind`, and Postgres mappings for the enums behind a `sqlx` feature.
- [ ] Owner: review the schema choices below.

#### Choices in this PR (for the owner's review)
1. **Runtime-checked queries, not compile-time ones.** ADR 0002 says "checked queries". The `query!` macros need a live database or a committed `.sqlx` cache on every build, CI's fmt, clippy and build jobs included. The queries are plain `sqlx::query_as` instead, and every query runs in the integration tests against a real Postgres. Switching to the macros later is a mechanical change. If the owner prefers the macros now, ADR 0002 stays as it is and the `.sqlx` cache gets committed. If not, ADR 0002 gets a line saying the tests do the checking.
2. **One identity per platform per account** (`UNIQUE (account_id, platform)`): an account has at most one Twitch and one Kick identity.
3. **Channel settings live on `channels`** (`allow_personal_emotes`), not in a separate `channel_settings` table. They are 1:1.
4. **A set member stores its own name.** It starts as the emote's name and doesn't follow later renames of the emote. That makes `UNIQUE (set_id, name)` a real constraint.
5. **`emotes.file_id`** (nullable) links an emote to its stored file; PLAN.md didn't name the link. Phase 3 fills it.
6. **Set rules are enforced in `sets::add_member`**, under a row lock on the set: capacity, unique names, pending emotes only in their owner's own channel sets, approved only in personal sets, nothing rejected or taken down. Who may edit a set stays the api's check.
7. **Outbox IDs are a bigint sequence**, so clients resume from a number. A sequence can commit out of order under concurrent writes; the Phase 5 readers have to allow for that.
8. The text-valued columns (platform, kind, state) are `text` with a CHECK, not Postgres enums, so adding a value is a plain forward migration.

#### GitHub write log
2026/11/16, account `sery-yx`, under the owner's standing OK for Phase 2 branches:
```
git push -u origin phase-2/db
gh pr create -R cyex-tv/platform --base main --head phase-2/db --title "Phase 2 PR 1: migrations and db queries" --body-file <file>
```
The result is PR 4.

#### Tests
- Unit: ULID text round trip and ordering; topic and event type names; the text forms of the enums match serde.
- Integration (`crates/db/tests/queries.rs`), one fresh migrated database per test:
  - sign-in creates, then reuses and refreshes, an account; six concurrent first sign-ins end on one account;
  - linking and unlinking: `identity_taken`, `platform_already_linked`, `last_identity`;
  - restricting an account; the schema refusing bad rows (non-numeric IDs, whitespace in names, an owned global set);
  - sessions: expiry, the sliding `touch` never shortening, sign-out everywhere but the current session, deleted accounts;
  - channels: upsert, login lookup ignoring case, active set cleared when its set is deleted;
  - emotes: patch, tags, state, review queue, soft delete; files shared by sha256;
  - sets: kinds, one personal set, the global set, membership rules, capacity (also with 12 concurrent adds into 5 slots), editors and `can_edit`, the personal lookup for chatters;
  - roles: permissions union, unknown permissions, names unique ignoring case, highest position, holders of a permission;
  - audit paging and rollback with the change; outbox IDs, topic filter, NOTIFY and cleanup.

### How Phase 2 is split (owner's call)
Four PRs, in this order. The owner merges each before the next one starts:
1. Migrations and db queries (`crates/db`): accounts, identities, sessions, channels, sets, set members, editors, emotes, files, roles, audit log, events outbox.
2. Sign-in and sessions (`crates/auth`, the REST auth routes): Twitch and Kick OAuth with PKCE and `state`, site cookie sessions, linking and unlinking.
3. Channels, sets, set editors and the REST chat-client reads, as GraphQL resolvers and REST handlers.
4. Roles, permissions and the audit log: the escalation and lock-out rules (ADR 0010), and the admin mutations.

### Before PR 2 can be tested end to end (owner)
- [ ] The Twitch and Kick OAuth apps are registered (`docs/setup/owner-steps.md`), with the client IDs and secrets in a local `.env`, never in the repo or a chat.
- [ ] There is a test account on Twitch and one on Kick.
- [ ] The owner account (`CYEX_TV_OWNER_IDS`) is named.
- [ ] Kick's Developer Terms are read, and anything relevant is noted.

### Setup on the new machine
- Rust (rustup; `rust-toolchain.toml` picks stable), Docker, `git`, and `gh` logged in as the owner.
- `gh repo clone cyex-tv/platform` (and `extension` and `infra` when their phases start).
- `docker compose -f dev/docker-compose.yaml up -d`, `cp .env.example .env`, then `cargo test --workspace`.
- Check the MinIO image tag in `dev/docker-compose.yaml` on first `up` (an open item from Phase 1).

## Phase 1: Monorepo scaffold, CI, API contract (done, merged 2026/11/16 as PR 2)

Branch `phase-1/scaffold`, one PR for the whole phase (owner's call). There is one commit per task.

The code was written on a machine where nothing may be run without asking, and the owner chose "write code only". So nothing was compiled or run locally, and **CI on the PR is the verification**. Every check passed on 2026/11/16 (run 43884331989 on `2332969`): fmt, clippy, test (with Postgres), build, contract, deny.

### Tasks
- [x] 1. Cargo workspace (`apps/api`, `apps/events`, `apps/worker`, `crates/shared`, `crates/db`), with `rustfmt.toml`, `rust-toolchain.toml`, workspace lints, `deny.toml` (AGPL-compatible licence allowlist), and `.gitattributes` (LF).
- [x] 2. `dev/docker-compose.yaml`: Postgres 17 and MinIO, both on 127.0.0.1 only, plus `.env.example`.
- [x] 3. `.github/workflows/ci.yml`: fmt, clippy `-D warnings`, test (with a Postgres service), release build, the drift check for the GraphQL SDL and OpenAPI, cargo-deny.
- [x] 4. The v1 contract, reworked after the owner's review (2026/11/16) to be GraphQL-first:
  - GraphQL schema draft in `apps/api/src/gql.rs`: queries, mutations and the `events` subscription. Admin and moderation fields check a permission first and answer `signed_out` for now. `permissions` is public, and introspection is on.
  - REST, only for auth, upload and the cacheable chat-client reads, in `apps/api/src/routes/v1.rs`.
  - The API types are in `crates/shared/src/model.rs`, with the GraphQL derives behind the `graphql` feature.
  - Every stub answers `not_implemented`.
- [x] Live updates reworked to the event WebSocket plus GraphQL subscriptions, with no SSE (ADR 0005). The protocol is written down. The events service still only serves `/health` until Phase 5.
- [x] 5. `/health` and `/health/ready` (checks the DB). Config comes from `CYEX_TV_*` env (the DB URL is never logged), with JSON or pretty logs, graceful shutdown, a DB pool limit and a statement timeout set on every connection.

- [x] CI is green on all six checks.
- [x] `schema/schema.graphql`, `openapi/openapi.json` and `Cargo.lock` are committed, taken from CI's artifacts.
- [x] CI actions moved off Node 20: `actions/checkout` and `actions/upload-artifact` v4 → v7.

### CI history on PR 2
1. First build: build, clippy and tests passed. Three things failed:
   - fmt: hand formatting;
   - deny: `paste` is unmaintained;
   - the spec check: nothing committed yet.
   
   All three were fixed.
2. After the GraphQL rework, async-graphql didn't compile. With `rust-version = "1.85"` the resolver paired async-graphql 7.0 with async-graphql-derive 7.2. The fix was `rust-version = "1.89"` and requiring 7.2 for both.
3. Only the drift check failed. The generated files were committed.
4. All green.

### Not done yet
- [ ] `docker compose up` is checked by hand, including the MinIO image tag. This needs a machine where commands may run.
- [ ] The owner reviews the API contract before Phase 2 handlers are written: the GraphQL schema, the event WebSocket protocol, the REST reads.

### Changes from the plan
- The owner asked for GraphQL-first and WebSockets (ADR 0005 and 0008 revised, D7 and D7b in the plan).
- The REST spec is `openapi/openapi.json`, not `.yaml`. The YAML output needs `serde_yaml`, which is unmaintained.
- `deny.toml` allows RUSTSEC-2024-0436 (`paste`, unmaintained, only through utoipa-axum).
- The minimum Rust version is 1.89, not 1.85 (see CI history).
- `crates/auth` is created in Phase 2, where its code lands, instead of as an empty crate now.
- `apps/web` starts in Phase 6. CI gets the web build and lint then.

### Tests in this phase
- `crates/shared`:
  - config defaults, the required DB URL, named errors for bad values, `Debug` hiding the DB URL;
  - permission names unique and namespaced;
  - platform parsing and JSON round trip.
- `crates/shared`: event types serialize with their dotted names.
- `apps/api/tests/http.rs`:
  - `/health` is 200;
  - `/health/ready` is 503 without a DB and 200 with one (CI only);
  - REST and GraphQL stubs answer `not_implemented`;
  - admin GraphQL fields answer `signed_out`;
  - `permissions` is public;
  - introspection works;
  - GraphiQL opens;
  - unknown paths are 404;
  - the OpenAPI spec holds only the REST part, and the SDL lists the contract.

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
