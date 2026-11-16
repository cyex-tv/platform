# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Phase 3: Upload pipeline, CDN, approval gate (in progress)

Started 2026/11/16, in an overnight run the owner asked for (work on, merge green PRs, questions at the end).

### How Phase 3 is split
1. Emote reads, edits, deletion and the review API (no image code).
2. The job queue (Postgres, `FOR UPDATE SKIP LOCKED`), the storage layer (S3 API: R2 in production, Versity Gateway locally) and the worker loop.
3. The streamed upload: limits, decoding to validate, hashing, the blocklist, duplicates.
4. Transcoding (WebP and AVIF at 1x-4x, static first frames), after benchmarking the encoders as the plan asks.

### PR 2: job queue, storage and the worker (branch `phase-3/queue`)
- [x] Migration `20261118000002_jobs.sql` (additive): `jobs` with state, attempts, `run_after` and a lease (`locked_until`); inserting one sends `NOTIFY cyex_tv_jobs`.
- [x] `crates/db/src/jobs.rs`: enqueue, claim (`FOR UPDATE SKIP LOCKED`, leased; an expired lease can be claimed again), complete, fail (retry with backoff, or for good), cleanup.
- [x] `apps/worker`: a library with the loop (drain due jobs, then wait for `NOTIFY`, a poll, or shutdown), a time limit per attempt, backoff 10 s doubling to an hour, and an hourly clean-up of expired sessions, unfinished sign-ins, outbox events older than 48 h (ADR 0005) and finished jobs after 7 days. Job kinds come with the pipeline (PR 3 and 4).
- [x] `crates/storage`: an S3 client with Signature V4 for R2 and MinIO (put, get, head, delete, bucket setup), and an in-memory store for tests. `object_store` would have pulled in aws-lc (OpenSSL licence, ADR 0007), so the client is small and written here. Keys per ADR 0003; `Cache-Control: public, max-age=31536000, immutable` for emote files.
- [x] `cyex_tv_db::testing::TestDb` (feature `testing`) for other crates' integration tests.
- [x] CI starts an S3 server (Versity Gateway, the dev compose tag) next to Postgres for the S3 tests. MinIO no longer publishes images, so the dev compose uses Versity Gateway too.

#### Tests
- Unit: SigV4 against AWS's published GET Object example signature, path encoding, secrets out of `Debug`, the memory store, S3 config, backoff, handler kinds.
- `apps/worker/tests/jobs.rs` against Postgres: run, retry with backoff, permanent failure, time limit, unhandled kinds left alone, an expired lease claimed again, 6 workers on 40 jobs with none run twice, `NOTIFY` on insert and the full loop until shutdown, the clean-up.
- `crates/storage/tests/s3.rs` against the S3 server: bucket setup (twice), put with content type and cache control, get, head, delete (twice), a wrong secret refused.

### PR 1: emotes and the review gate (branch `phase-3/review`)
- [x] Migration `20261118000001_emote_review.sql` (additive): `reviewed_by`, `reviewed_at`, `review_reason` on `emotes`.
- [x] An emote that isn't approved is visible to its owner and `emotes.review` holders only; `GET /v1/emotes/{id}` serves approved emotes, with `ETag` and `Cache-Control`.
- [x] `reviewQueue` (paged, `page` and `perPage` added), `approveEmote`, `rejectEmote` (reason required, 1 to 500 characters). Only pending emotes can be reviewed (`not_pending`), so two moderators can't both decide. Both are audited, and every set holding the emote gets `emote.updated`.
- [x] `updateEmote`: the owner, or `emotes.edit_any` (audited with before and after). Tags are lowercased and deduplicated, at most 10 of 1 to 32 letters, digits, `-` or `_`.
- [x] `deleteEmote`: the owner only. The emote leaves every set at once, with a `set.emote_removed` event for each.

#### Tests
- Unit: tag cleaning, reasons.
- `apps/api/tests/moderation_flow.rs` against Postgres: who sees pending emotes (GraphQL and REST), the queue and its paging, approve and reject (permissions, `not_pending`, `bad_reason`, audit with the reason, the stored reason, the event, the public read afterwards), owner edits and their checks, an audited moderator edit, deletion leaving its sets.

## Phase 2: Data model, auth, roles (done, merged 2026/11/16 as PRs 4 to 7)

Everything is merged and tested against Postgres in CI. Still open, and the owner's: signing in end to end with real Twitch and Kick test accounts (needs the OAuth apps), and the review choices listed under each PR.

Paused on 2026/11/16 until work moves to a machine where commands can run (owner's call). Resumed on the new machine on 2026/11/16. Rust, `git` and `gh` are set up there; Docker isn't installed yet (the owner installs it), so the database tests ran in CI first.

### PR 4: roles, permissions, the owner and the audit log (branch `phase-2/roles`, merged 2026/11/16 as PR 7)
- [x] `crates/auth/src/rbac.rs`: the ADR 0010 rules as plain functions. No escalation (grant only what you hold), only roles below your highest `position`, the owner can't be demoted by others, and no change may leave nobody but the owner holding `roles.manage`.
- [x] The owner: `CYEX_TV_OWNER_IDS` takes `twitch:<id>`, `kick:<id>` or account IDs. Platform IDs are what's known before the first sign-in; ADR 0010 said account IDs, and both work. The owner holds every permission without a role. The owner's real IDs are in the owner's local `.env`, not in this public repo.
- [x] GraphQL: `roles`, `accountRoles`, `auditLog` (new, `audit.view`, paged by entry ID), `createRole`, `updateRole`, `deleteRole`, `assignRole`, `unassignRole`. Each change writes `audit_log` with before and after in the same transaction.
- [x] Role changes take one advisory lock, so two admins demoting each other at once can't both pass the lock-out check.
- [x] Every permission check (`require`, set rules, `me.permissions`) goes through the caller, so the owner rule applies everywhere.

#### Tests
- Unit (`rbac`): owner list parsing, the owner passing every rule, escalation, below-your-highest, owner protection, lock-out.
- GitHub: `git push -u origin phase-2/roles`, `gh pr create ... --title "Phase 2 PR 4: roles, permissions and audit"`, `gh pr merge 7 -R cyex-tv/platform --merge --subject "Merge pull request #7 from cyex-tv/phase-2/roles"`, after all 6 checks passed with `roles_flow` (5) running against Postgres.
- `apps/api/tests/roles_flow.rs` against Postgres: the owner holds everything, no escalation (permissions and positions, also through `updateRole` and `assignRole`), the owner can't be demoted by others, the last manager stays (unassign, delete, update), every change audited with before and after, paging and filters, `accountRoles` access.

### PR 3: channels, sets, editors and the chat-client reads (branch `phase-2/sets`, merged 2026/11/16 as PR 6)
- [x] `apps/api/src/catalog.rs`: the rules, shared by GraphQL and REST. The global set needs `global_set.edit`; other sets take their owner and editors, and anyone else needs `sets.edit_any`. Deleting a set and managing editors is owner-only (or `sets.edit_any`). A channel's active set can't be deleted (`set_in_use`).
- [x] Pending emotes are shown to their owner, the set's owner and editors, and `emotes.review` holders; everyone else, chat clients included, sees approved emotes only (`apps/api/src/views.rs`).
- [x] Batch reads (`crates/db/src/reads.rs`): a set's members with emotes and files, tags, and account summaries in a few queries, however big the set is.
- [x] Every set change publishes its outbox event in the same transaction (`set.emote_added`, `set.emote_renamed`, `set.emote_removed`, `channel.active_set_changed`), on the set's topic, the topics of channels showing it, and for personal sets the owner's user topics.
- [x] Audit rows: editor grants and removals always; set changes when they rely on a permission rather than ownership.
- [x] Channels are made at sign-in for every identity and removed when it is unlinked. The active set must be one of the channel owner's channel sets; an empty `activeSetId` clears it.
- [x] GraphQL: `channel`, `channelByLogin`, `set`, `globalSet`, `personalSet`, `personalSets`, and the new `mySets` and `myChannels`; mutations `updateChannelSettings`, `createSet`, `updateSet`, `deleteSet`, `addSetEmote`, `updateSetEmote`, `removeSetEmote`, `addSetEditor`, `removeSetEditor`.
- [x] REST: `GET /v1/channels/{platform}/{id}`, `GET /v1/sets/{id}`, `GET /v1/sets/global` with a strong `ETag`, `304` on `If-None-Match`, and `Cache-Control: public, max-age=30, stale-while-revalidate=60`; `POST /v1/personal-sets/lookup` (up to 100 chatters; empty and missing sets left out).
- [x] `CYEX_TV_CDN_URL` (default `https://cdn.cyex.tv`) is where emote `host`s point.

#### Tests
- Unit: name and capacity rules, error codes, time format, file variants, hex.
- `apps/api/tests/catalog_flow.rs` against Postgres: pending emotes in sets and the public read hiding them, ETag round trip and change, active set rules and channel reads, `set_in_use`, member rules (`not_approved`, `emote_private`, `emote_unavailable`, `already_in_set`, `bad_name`), events and their topics, editors (grant, limits, leaving, audit), personal sets (one per account, approved only, capacity 5, the lookup and its limit), the global set with `global_set.edit`, capacity with `sets.edit_any` and its audit, who sees pending emotes.
- `auth_flow.rs`: sign-in makes the channel; unlinking removes it.

#### GitHub write log
```
git push -u origin phase-2/sets
gh pr create -R cyex-tv/platform --base main --head phase-2/sets --title "Phase 2 PR 3: channels, sets, editors and reads" --body-file <file>
gh pr merge 6 -R cyex-tv/platform --merge --subject "Merge pull request #6 from cyex-tv/phase-2/sets"
```
Merged after all 6 checks passed on the first run, with `catalog_flow` (7), `auth_flow` (13) and `queries` (20) seen running against Postgres.

### PR 2: sign-in and sessions (branch `phase-2/auth`, merged 2026/11/16 as PR 5)
- [x] `crates/auth`: Twitch and Kick OAuth (authorization code flow). `state` is bound to a short-lived host-only pre-login cookie (`cyex_tv_oauth`, `Path=/v1/auth`, 10 minutes) and to a stored row that is taken on first use. PKCE S256 for Kick (required there); Twitch gets `state` only, since it doesn't document PKCE.
- [x] Migration `20261117000002_oauth_states.sql` (additive): the sign-in in progress, holding the `state` hash, the PKCE verifier (it never reaches the browser), `return_to` and the account being linked.
- [x] Platform access tokens are used once to read the user, then revoked (best effort) and dropped. Kick's `email` is never read into the model.
- [x] Site sessions: 256-bit random token, sha256 stored, cookie `cyex_tv_session` with `Domain=cyex.tv; Path=/; Secure; HttpOnly; SameSite=Lax`, 30 days sliding (written back at most hourly). A new session on every sign-in and link; the old one is deleted.
- [x] App sessions: `Authorization: Bearer`, kind `app`, 90 days. Accepted, not yet issued (D19).
- [x] CSRF by Origin: a cookie caller's writes (any method but GET, HEAD, OPTIONS, so every GraphQL POST) need `Origin: https://cyex.tv`, or fail with `bad_origin`. Bearer callers are exempt.
- [x] `return_to` must be on the site (paths or full `https://cyex.tv` URLs); anything else is `bad_return_to`.
- [x] REST: `GET /v1/auth/{platform}/login` (`?return_to`, `?link=true`), `GET /v1/auth/{platform}/callback`, `POST /v1/auth/logout`. Errors: `unknown_platform`, `bad_return_to`, `signed_out`, `bad_state`, `declined`, `identity_taken`, `platform_already_linked`, `account_deleted`, `platform_error`, `unavailable` (client not configured).
- [x] GraphQL: `me` and `unlinkIdentity` (never the last identity) are built; `require` checks the caller's role permissions (`forbidden` without one). The owner rule comes with PR 4.
- [x] Config: `CYEX_TV_PUBLIC_URL`, `CYEX_TV_SITE_URL`, `CYEX_TV_COOKIE_DOMAIN`, `CYEX_TV_COOKIE_SECURE`, `CYEX_TV_TWITCH_CLIENT_ID`/`_SECRET`, `CYEX_TV_KICK_CLIENT_ID`/`_SECRET` (see `.env.example`). Without a client, that platform answers 503.
- [ ] End to end with real test accounts (needs the owner's OAuth apps).
- Fixes found by CI: same-millisecond IDs now strictly increase (a PR 1 audit test was flaky); `cargo deny` allows `CDLA-Permissive-2.0` for CA certificate data (ADR 0007); TLS uses ring instead of aws-lc.

#### GitHub write log
2026/11/16, account `sery-yx`, under the owner's OK to merge green PRs:
```
git push -u origin phase-2/auth
gh pr create -R cyex-tv/platform --base main --head phase-2/auth --title "Phase 2 PR 2: sign-in and sessions" --body-file <file>
git push   (three fix commits after CI)
gh pr merge 5 -R cyex-tv/platform --merge --subject "Merge pull request #5 from cyex-tv/phase-2/auth"
```
Merged after all 6 checks passed, with `auth_flow` (13) and `queries` (20) seen running against Postgres in the CI log.

#### Tests
- Unit (`crates/auth`): config defaults and validation, cookie strings against ADR 0009, cookie parsing, `return_to` (open-redirect cases), Origin check, PKCE and hashing, authorize URLs per platform, user checks.
- HTTP without a database (`apps/api/tests/http.rs`): unknown or unconfigured platform, `me` signed out, logout clearing the cookie, cookie writes refused without the site Origin.
- OAuth flows (`apps/api/tests/auth_flow.rs`) against fake Twitch and Kick servers answering with `tests/fixtures` (shaped after the docs; to be replaced with recordings): Twitch end to end, Kick with PKCE checked against the verifier, linking both platforms and session rotation, `identity_taken`, `platform_already_linked`, links needing their own session, `state` bound to the cookie and single-use, wrong platform, expiry, declined, platform errors, `return_to`, logout by Origin, Bearer app sessions, unlinking, deleted accounts, permissions from roles.

### Merging (owner's call, 2026/11/16)
The owner allowed the agent to merge its own PRs into `main` once it is sure they work. Each task still gets a branch and a PR, because CI is the gate; the agent merges with `gh pr merge --merge` after every check is green and the database tests are seen to run, and logs it here. Never a direct push to `main`, never a force push.

### PR 1: migrations and db queries (branch `phase-2/db`, merged 2026/11/16 as PR 4)
- [x] Migration `20261117000001_core.sql`: every Phase 2 table from PLAN.md, with the indexes it lists (`(platform, platform_user_id)`, `(platform, platform_channel_id)`, `lower(name)` and a `pg_trgm` index on emote names, set owner, emote state), plus indexes for logins, sessions, editors, roles, audit and the outbox.
- [x] The outbox insert trigger sends `NOTIFY cyex_tv_events, '<id>'` (ADR 0005).
- [x] Query modules in `crates/db`: `accounts`, `sessions`, `channels`, `emotes`, `files`, `sets`, `roles`, `audit`, `outbox`, `ids`. Each takes `&mut PgConnection`, so the api can put a change, its audit row and its event in one transaction.
- [x] Rule violations are named errors (`DbError`): `identity_taken`, `last_identity`, `set_full`, `name_taken`, `not_approved` and others.
- [x] `crates/shared`: text forms for `EmoteState` and `SetKind`, and Postgres mappings for the enums behind a `sqlx` feature.
- [x] Merged by the owner on 2026/11/16. The choices below are still open for the owner's review.

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
- The MinIO image is gone from Docker Hub (found 2026/11/16); dev and CI use Versity Gateway `v1.10.0` instead. Check it on first `up`.

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
