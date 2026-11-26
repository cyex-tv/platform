# cyex.tv: master plan

Scope: **cyex.tv**, an emote platform for Twitch and Kick, in the style of 7TV, BTTV and FFZ, with room for chat cosmetics (badges, name paints) later.
- Clients: the website at cyex.tv, cyex deck, and a browser extension.
- cyex.tv is its own product with its own API. It is separate from cyex.app (cyex-api, the deck's supporter badges and store, logs).

The work is built in phases. Each phase ends in a PAUSE, a compact and a checkpoint. This plan lives in the repo, so a fresh session can always resume from it.

Version 3, 2026/11/16:
- Version 2 put cyex.tv's API inside cyex.app and shared its accounts. Version 3 separates the two completely.
- The reasons for earlier changes are in `PLAN-REVIEW.md`.

---

## Decisions already made

| # | Decision |
|---|---|
| D1 | **Separate from cyex.app.** cyex.tv has its own API, database, accounts and sessions. Some endpoints will look like cyex-api's (sign-in, `/me`, sessions). That duplication is accepted for a clean split. cyex.tv does not read or write cyex-api's database, and cyex-api is not changed for cyex.tv. |
| D2 | **Backend:** Rust services `api`, `events` and `worker`. |
| D3 | **Hosts:** the website at `cyex.tv` (`www` redirects), the API and events at `api.cyex.tv/v1/...`, files at `cdn.cyex.tv`. Same-site, so first-party cookies work. |
| D4 | **Database:** its own Postgres 17 database and container (`tv-db`) with its own credentials, backups and network. It may run on the same host as cyex-api, but shares nothing with `deck-db`. |
| D5 | **Storage:** Cloudflare R2, served through `cdn.cyex.tv`. |
| D6 | **Search:** Postgres `pg_trgm` + full-text search in v1. Meilisearch or Typesense later, only when load needs it. |
| D7 | **Live updates:** a WebSocket event protocol (`/v1/events`) for chat clients, and GraphQL subscriptions (`/v1/gql/ws`) for GraphQL clients. One event model and outbox behind both. Many subscriptions per connection, changeable without reconnecting, with resume. No SSE (revised 2026/11/16, ADR 0005). |
| D7b | **API shape:** GraphQL-first at `/v1/gql` for reads, writes, moderation and admin. Admin parts need a session and a permission, and introspection stays on. REST only for OAuth, uploads, health and a small, cacheable chat-client read API (revised 2026/11/16, ADR 0008). |
| D8 | **Web:** SvelteKit + TypeScript, inside the platform monorepo. Styling uses the cyex token CSS (`cyex-design` + `cyex-app`, `cyex-web` for the landing page, `cyex-docs` for API docs). No Tailwind. |
| D9 | **Platforms:** Twitch **and** Kick in v1. A cyex.tv account has one or more linked identities, `(platform, platform_user_id)`, and never uses logins as keys. |
| D10 | **Uploads:** anyone signed in can upload. A new emote is **unlisted until approved**. It works in the uploader's own channel right away, but is not in search or other people's pickers. |
| D11 | **Moderation:** a team from day one, with real roles. Admins create roles, choose a role's permissions, and assign roles. Every change is audited. |
| D12 | **Global set:** yes, a cyex.tv global set curated by admins. Editing it is a permission. |
| D13 | **Extension:** the providers each have their own on/off switch, and it works on Twitch and Kick.<br>Emotes: cyex.tv, 7TV, BTTV, FFZ.<br>Badges and paints: cyex deck supporter badges (the public `api.cyex.app/badges/v1` read API, used as is), 7TV badges and paints, BTTV and FFZ badges, and cyex.tv cosmetics once they exist. |
| D14 | **Repos:** public from the start, in the `cyex-tv` org (it exists, created 2026/11/16). |
| D15 | **cyex deck:** gets a new `cyex.tv` emote provider in its existing `cyex-emotes` crate, in `cyex-app/cyex-deck`. It becomes the first client, right after uploads work. The deck's own supporter badges stay exactly as they are; they are a separate thing. |
| D16 | **Borrowed code:** none from 7TV or BTTV, because their licences forbid it (Commons Clause, and a restrictive custom licence). Their public protocols and API shapes may be studied. FFZ code (Apache-2.0) may be reused with its notice kept. |
| D17 | **Cosmetics are two separate things.**<br>**cyex deck supporter badges** belong to cyex.app. They are bought in the deck's store, served by cyex-api, and have nothing to do with the emote platform.<br>**cyex.tv chat cosmetics** (badges and name paints) are a possible later feature of cyex.tv itself. They get their own phase and their own data, and never mix with the deck's. |
| D18 | **Licence:** AGPL-3.0 for all three cyex-tv repos. Clients that only call the public API are not affected. |
| D19 | **Deck in v1:** it shows cyex.tv emotes only. No cyex.tv sign-in and no editing in the deck yet.<br>**Changed 2026/11/19 (owner):** the deck gets cyex.tv sign-in (app sign-in, ADR 0021) for emote commands and a cosmetic picker. |
| D20 | **Set capacity:** 1.000 emotes by default. Admins can change it per set. |
| D21 | **Personal sets in v1:** each account gets one small personal set (default 5 slots, as in 7TV), usable in any channel. Only **approved** emotes can go in it, because they show in other people's chats. Channel owners can turn off other people's personal emotes in their channel. |

---

## 0. Working rules (copied into AGENTS.md in each repo)

- The source of truth is `PLAN.md` (this file) and `PROGRESS.md` (the checkpoint log). Update `PROGRESS.md` at the end of every task.
- One task = one branch = one PR. Commits stay small and reviewable. You (the owner) review and merge; the agent never merges.
- Architecture decisions go in `docs/adr/NNNN-title.md`. Never decide silently.
- Never push to `main`. Use branches and PRs.
- No code is written before its phase's "Decisions needed" are answered.
- Tests come with features. No feature is "done" without a test, or a manual check written in PROGRESS.md.
- Commits are authored by the owner's GitHub account (`sery-yx`, with its noreply address). No co-author trailers, and no tool or AI attribution in code, docs, commits or PRs.
- Secrets never go into the repo. Use `.env.example` and a secrets manager. The repos are public, so check this twice.
- Accounts, app registrations, DNS, billing and store listings are done by the owner. The agent prepares the exact values and steps.

### Separation rules (hard limits)

cyex-api (cyex.app) serves cyex logs, recap, deck badges, the store and Paddle payments. cyex.tv must never be able to break it.
1. **No changes to `cyex-app/cyex-api`** for cyex.tv work. If something seems to need one, stop and ask the owner.
2. cyex.tv never connects to `deck-db`, cyex-api's Redis or its ClickHouse. It has its own database, credentials and network.
3. cyex.tv's only contact with cyex.app is the extension (and the deck, as today) **reading** the public badge API `api.cyex.app/badges/v1`, following its spec (cache, `If-None-Match`, refresh no more than every 30 minutes).
4. Deploying cyex.tv adds new containers, hostnames and tunnel routes only. It never edits cyex-api's containers, compose files, env or Cloudflare rules.
5. Code that looks like cyex-api's is rewritten in Rust for cyex.tv, not shared. Examples are sign-in, sessions, the image blocklist and the audit log.

---

## Phase 0: Setup and decisions

Goal: tools, repos, external accounts and open decisions settled before any code exists.

### Tasks
1. **Owner:** install and log in to the GitHub CLI.
   ```powershell
   winget install GitHub.cli
   ```
   ```powershell
   gh auth login
   ```
2. **Owner:** register cyex.tv's **own** OAuth apps, using the redirect URIs and scopes in `docs/setup/owner-steps.md`:
   - a Twitch app and a Kick app;
   - redirects `https://api.cyex.tv/v1/auth/{twitch|kick}/callback` and `http://localhost:<port>/v1/auth/{twitch|kick}/callback`.
   
   Don't reuse the cyex.app Twitch app.
3. **Owner:** Cloudflare setup.
   - Add the `cyex.tv` zone.
   - DNS for `cyex.tv`, `www.cyex.tv` (redirecting to `cyex.tv`), `api.cyex.tv` and `cdn.cyex.tv`.
   - An R2 bucket bound to `cdn.cyex.tv`.
   - New tunnel hostnames for `cyex.tv` and `api.cyex.tv`. Leave every cyex.app hostname as it is.
4. **Agent:** create the repos with the scoped procedure below.
5. **Agent:** write the ADRs:
   - 0001 Backend: Rust services, and the separation from cyex.app (D1, D2).
   - 0002 Database: its own Postgres, migrations with sqlx, pool limits, backups (D4).
   - 0003 Storage: R2 + `cdn.cyex.tv`, key layout, cache rules (D5).
   - 0004 Search: Postgres first, and the trigger for moving to a search engine (D6).
   - 0005 Live updates: the event WebSocket and GraphQL subscriptions, one event model, resume window, heartbeat (D7).
   - 0006 Frontend: SvelteKit + cyex tokens (D8).
   - 0007 Licence: AGPL-3.0 (D18), plus the borrowed-code rule (D16) and `cargo deny` / licence checks in CI (dependencies must be AGPL-compatible).
   - 0008 API contract: GraphQL-first with `async-graphql` plus a small REST part with `utoipa`; the SDL and the OpenAPI spec are both generated, committed and checked in CI.
   - 0009 Accounts and identities: one cyex.tv account with linked Twitch and Kick identities, sign-in, linking and unlinking, sessions (site cookie, app Bearer).
   - 0010 Roles and permissions: the RBAC model (see Phase 2).
   - 0011 Cosmetics boundary: cyex deck badges vs cyex.tv cosmetics (D17), and the namespaces and IDs that keep them apart in clients.

### Decisions needed before Phase 1
- **Q3, Kick API:** confirm the current Kick OAuth endpoints and scopes, whether user and channel IDs are stable, and the rate limits. The agent researches this; the owner confirms.

### Repo creation (scoped)
Allowlist (the only repos created in this phase):
- `cyex-tv/platform`: the Rust services, the SvelteKit web app, shared crates, the OpenAPI spec, and local dev compose
- `cyex-tv/extension`: the browser extension
- `cyex-tv/infra`: Cloudflare config, deploy compose for the production host, backups, runbooks

Work in existing repos happens **only on a feature branch with a PR**, and only here:
- `cyex-app/cyex-deck`: the cyex.tv provider (Phase 4).

`cyex-app/cyex-api` is not touched (separation rule 1).

Procedure
1. Print the org, the exact list above and `gh auth status`. Stop and ask for confirmation.
2. For each name, run `gh repo view cyex-tv/<name>`. If it exists, stop and report. Never adopt or overwrite an existing repo.
3. Create it:
   ```bash
   gh repo create cyex-tv/<name> --public --description "<text>"
   ```
   It is public (D14), but only after ADR 0007 has picked the licence, so the first push contains `LICENSE`.
4. Default branch `main`. Set up branch protection on `main` (PR required, no force push, no deletion), and turn on secret scanning and push protection. Nothing else.
5. Push only the initial scaffold to `setup/initial` and open the PR. Never push to `main` directly.

Hard limits (in AGENTS.md as well)
- Only the allowlisted repo names. Any other write to the org is refused: other repos, teams, members, org settings, Actions secrets, billing, webhooks.
- No `gh repo delete`, no `gh repo edit --visibility`, no `gh api` calls that change org settings.
- No force pushes, no history rewrites.
- Commits authored by the owner's account only, with no attribution trailers.
- Every write action is logged in PROGRESS.md with the exact command.

Check afterwards: `gh repo list cyex-tv --limit 50` shows only these three repos.

Deliverable: the tools are installed, the three repos exist with a licence, ADRs 0001-0011 are committed, the cyex.tv OAuth apps and Cloudflare zone are set up, and Q3 is answered.

PAUSE. Run the compact below before Phase 1.

---

## Phase 1: Monorepo scaffold, CI, API contract

Goal: an empty but working repo with CI and a reviewed API contract.

Repo layout (`cyex-tv/platform`):
```
platform/
  apps/
    api/            Rust, axum: HTTP API at api.cyex.tv/v1
    events/         Rust: the event WebSocket
    worker/         Rust: image pipeline jobs
    web/            SvelteKit: cyex.tv, the website and admin pages
  crates/
    shared/         models, error types, config, permission names
    db/             sqlx migrations and queries
    auth/           Twitch and Kick OAuth, sessions, account + roles
  schema/
    schema.graphql  the GraphQL SDL, generated from the code, checked in CI
  openapi/
    openapi.json    the REST part, generated from the code (utoipa), checked in CI
  dev/
    docker-compose.yaml   local postgres, minio (R2 stand-in)
  docs/adr/
  PLAN.md
  PROGRESS.md
  AGENTS.md
```

Tasks
1. Cargo workspace, rustfmt, clippy with `-D warnings`, `cargo deny` (licences and advisories).
2. Dev compose with Postgres and MinIO.
3. GitHub Actions: fmt, clippy, test, build, `cargo deny`, the drift checks for the GraphQL SDL and the OpenAPI spec, and later the web build and lint.
4. Draft the API in code: a GraphQL schema with stub resolvers, plus the REST part as utoipa stubs. Generate `schema.graphql` and `openapi.json`. It covers:
   - auth
   - accounts (`/me`, identities)
   - channels
   - emote sets
   - emotes
   - search
   - uploads
   - moderation
   - roles
   - events

   Review the spec before any handler is written. The public read endpoints are the stable contract third-party clients will use. Version them and keep them small. Leave a `/v1/cosmetics` namespace reserved (D17).
5. Health endpoint, config from env, structured logging, a DB pool limit and a statement timeout.

Done when:
- `docker compose up` works;
- CI is green;
- the GraphQL and REST drafts are reviewed and committed;
- health returns 200.

PAUSE. Compact before Phase 2.

---

## Phase 2: Data model, auth, roles

Goal: a user signs in to cyex.tv with Twitch or Kick, links the other one, manages their sets, and admins manage roles.

Data model (final shape in ADRs 0009 and 0010):
- `accounts`: id, created_at, restricted_at, deleted_at.
- `identities`: account_id, platform, platform_user_id, login, display_name, avatar, linked_at. Unique on (platform, platform_user_id).
- `sessions`: id_hash, account_id, kind (site | app), created_at, expires_at, last_seen_at. Tokens are stored as sha256 only.
- `channels`: account_id, platform, platform_channel_id, active_set_id.
  - One row per platform channel. One account can have a Twitch and a Kick channel.
- `emote_sets`: id, owner_account_id, name, kind (channel | personal | global), capacity, created_at.
  - Capacity defaults: channel 1.000 (D20), personal 5 (D21). One personal set per account.
- `channel_settings`: channel_id, allow_personal_emotes (default true).
- `emotes`: id, owner_account_id, name, flags, state, created_at, deleted_at.
  - Flags: zero_width, private, nsfw, animated. (2026/11/22: NSFW emotes aren't allowed, so nsfw is retired; creators set zero-width and private only, unlisted is for staff. ADR 0020 amendment.)
  - State: pending | approved | rejected | taken_down.
- `emote_tags`: emote_id, tag.
- `emote_set_members`: set_id, emote_id, alias, flags, added_by, added_at.
- `set_editors`: set_id, account_id, granted_by, granted_at.
- `files`: id, sha256, dhash, mime, width, height, frame_count, variants_json, storage_key.
- `roles`: id, name, description, created_by, created_at.
- `role_permissions`: role_id, permission.
- `account_roles`: account_id, role_id, granted_by, granted_at.
- `audit_log`: id, actor_account_id, action, target_kind, target_id, before_json, after_json, created_at.
- `events_outbox`: id, channel keys, set_id, type, payload, created_at.

Permissions are named and namespaced, defined in `crates/shared`:
- `emotes.review`
- `emotes.takedown`
- `emotes.edit_any`
- `sets.edit_any`
- `global_set.edit`
- `users.view`
- `users.restrict`
- `roles.manage`
- `audit.view`

Cosmetics permissions are added in the cosmetics phase.

The owner account always has every permission and can't be locked out. A role can't grant `roles.manage` unless the person granting it holds it too.

Tasks
1. sqlx migrations, with indexes on:
   - `(platform, platform_user_id)`
   - `(platform, platform_channel_id)`
   - lower(emote name), plus a `pg_trgm` index
   - set owner
   - emote state
2. `crates/auth`:
   - Twitch and Kick OAuth with PKCE and `state`;
   - sign-in, linking a second platform, unlinking (never the last identity);
   - a site session cookie (`Domain=cyex.tv; Secure; HttpOnly; SameSite=Lax`);
   - an app Bearer session for the deck and other apps, through a device-style or loopback redirect login;
   - sign-out, and expiry and rotation;
   - CSRF by origin for cookie callers, allowing only `https://cyex.tv`.
3. Endpoints:
   - `/v1/auth/*`
   - `GET /v1/me`
   - `GET /v1/channels/{platform}/{id}` (plus a login lookup)
   - CRUD for `/v1/sets`
   - set membership (a personal set accepts only approved emotes)
   - `GET /v1/users/{platform}/{id}/personal`, plus a batch lookup for many chatters at once (clients look up who in chat has personal emotes)
   - set editors
   - `/v1/roles`
   - role assignment
4. Authorization: a user edits only sets they own or were made editor of. Everything else goes through permissions. Every write that changes roles, grants or moderation state writes `audit_log`.
5. Tests:
   - unit tests for every permission rule, including the owner lockout guard;
   - integration tests against a real Postgres;
   - OAuth flows against recorded fixtures.

Done when:
- sign-in works end to end with a test Twitch account and a test Kick account;
- linking both to one account works;
- the permission tests pass;
- the GraphQL SDL and OpenAPI match the code, as checked by CI.

PAUSE. Compact before Phase 3.

---

## Phase 3: Upload pipeline, CDN, approval gate

Goal: upload an image and get back a validated, transcoded emote, served from the CDN, that is pending approval.

Pipeline
1. **Accept** the upload, streamed:
   - limits: ≤ 8 MB in, ≤ 1000×1000 px, ≤ 1000 frames, and a cap on total decoded pixels (to stop decompression bombs);
   - inputs: PNG, APNG, GIF, WebP, AVIF;
   - no video and no SVG in v1.
2. **Validate** by actually decoding it, not by the extension. Reject unknown codecs.
3. **Hash.** Check the sha256 and the dHash against cyex.tv's blocklist. A duplicate upload reuses the existing file.
4. **Transcode** (worker):
   - WebP at 1x/2x/3x/4x first, and mark the emote usable;
   - then AVIF at 1x-4x;
   - animated emotes also get a static first-frame variant, for reduced motion and "animations off".
5. **Store** under `emotes/{sha256}/{variant}` in R2. The files are immutable, cached for a long time by Cloudflare, and served at `cdn.cyex.tv`. The `cosmetics/` prefix is reserved for later.
6. **State.** A new emote is `pending`:
   - it is usable in the uploader's own sets right away;
   - it is invisible to search and to other people until a holder of `emotes.review` approves it (D10).

Tasks
1. Upload endpoint with streaming and limits, and rate limits per account.
2. Worker queue: a Postgres job table with `FOR UPDATE SKIP LOCKED`.
3. Image processing. Benchmark `image` + `ravif` against libavif and libwebp bindings before choosing. The worker gets only the bucket and job-table credentials, plus memory and time limits per job. Write the pipeline so it can be reused for cosmetic badge art later.
4. R2 layout, Cloudflare cache rules, and CORS for `cdn.cyex.tv`.
5. A minimal review API: list pending, approve, reject with a reason. All of it is audited.
6. Tests with fixture images:
   - static PNG
   - APNG
   - animated GIF
   - animated WebP
   - corrupt file
   - oversized file
   - pixel bomb
   - SVG rejection
   - duplicate
   - blocklisted hash

Done when:
- a fixture upload gives working 1x-4x URLs (WebP, AVIF, static) on the CDN;
- every rejection case has a test;
- pending emotes stay out of public reads.

PAUSE. Compact before Phase 4.

---

## Phase 4: cyex deck integration (first client)

Goal: cyex deck shows cyex.tv emotes next to 7TV, BTTV and FFZ, for Twitch and Kick channels.

Work goes on a feature branch in `cyex-app/cyex-deck`.
- The deck is Tauri 2 + Svelte 5.
- `crates/emotes` already has the 7TV, BTTV and FFZ providers, zero-width, personal sets, flags and fixtures.
- Its cyex supporter badges (`crates/emotes/src/badges/cyex.rs`, `api.cyex.app`) are **not** touched (D17).

Tasks
1. Add a `cyex.tv` emote provider: `crates/emotes/src/cyextv/` (api, events), mapped into the existing `Emote` and `EmoteSet`.
   - Name it so it can't be confused with the cyex supporter badges: provider `CyexTv`, label "cyex.tv".
   - Share the response types with the platform, either through a small published crate or generated types (decided in an ADR).
2. Fetch the global set and the channel's active set, then subscribe to events (once Phase 5 lands; poll with `If-None-Match` until then).
3. Images: the webview loads WebP and AVIF straight from `cdn.cyex.tv`. Check `app/src/sys/cache.rs` first, and only add a disk cache if the existing one doesn't cover emotes.
4. A settings toggle for the cyex.tv provider, matching the existing provider toggles.
5. v1 was read-only in the deck (D19). Since 2026/11/19 the deck signs in through app sign-in (ADR 0021) for emote add, rename and remove commands and the cosmetic picker.
6. Personal emotes (D21): map them to the deck's existing `Scope::Personal`. They apply only to that user's messages, and respect the channel's `allow_personal_emotes`.
7. Tests: mapping fixtures, name lookup with cyex.tv alongside the other providers, personal emotes only on their owner's messages, and the offline fallback to the last known set.

Done when:
- the deck shows cyex.tv emotes on a test Twitch channel and a test Kick channel;
- the toggle works;
- the deck's supporter badges behave exactly as before;
- the tests pass.

PAUSE. Compact before Phase 5.

---

## Phase 5: Live updates

Goal: when a set changes, every connected client sees it within a second.

Tasks
1. `apps/events`: the event WebSocket at `wss://api.cyex.tv/v1/events` (protocol in ADR 0005), with **many subscriptions on one connection**, changeable without reconnecting.
   - Topics: `channel:<platform>:<id>`, `set:<id>`, `user:<platform>:<id>`.
   - Also `subscription { events(topics) }` on the api's GraphQL socket `/v1/gql/ws`, fed by the same outbox.
   - Event types:
     - `set.emote_added`
     - `set.emote_removed`
     - `set.emote_renamed`
     - `channel.active_set_changed`
     - `personal.updated` (sent to the channels where that user was recently seen chatting, so clients refresh their personal emotes)
     - `emote.updated` (approved, taken down, renamed)
   - The `cosmetic.*` namespace is reserved.
2. Source of events: the `events_outbox` table plus `LISTEN/NOTIFY` as a wake-up. There is no direct coupling from the API to the events service.
3. Resume with `resume` (session ID + last event ID) from the outbox, within a 24 h replay window. Send a heartbeat every 20 s. Older gaps get a `resync` event.
4. Load test on both sockets: 5.000 open connections, each with 10 subscriptions; p95 under 1 s from write to delivery.
5. The deck switches from polling to events.

Done when:
- a set change reaches a test client in under a second;
- resume works across a restart;
- the load test is recorded in PROGRESS.md.

PAUSE. Compact before Phase 6.

---

## Phase 6: Search and the website

Goal: browse, search and manage sets and moderation on cyex.tv.

Tasks
1. Search endpoint on Postgres (`pg_trgm` for names, full text for tags):
   - paging;
   - filters: animated, owner, channel, platform;
   - ranking by how many channels use an emote (computed from `emote_set_members` on a schedule). No data from cyex.app's ClickHouse.
2. `apps/web` (SvelteKit, TypeScript, cyex tokens) at `cyex.tv`.
3. Public pages:
   - home (`cyex-web`)
   - search
   - emote page
   - channel page
   - API docs (`cyex-docs`)
4. Signed-in pages (`cyex-app`, web-tool density):
   - my emotes
   - my sets and set editors
   - my personal set
   - channel settings (allow personal emotes)
   - upload
   - linked accounts (Twitch, Kick)
5. Admin pages:
   - review queue
   - takedowns
   - users
   - **roles**: create a role, choose its permissions, assign it to people, see who has what
   - the audit log
   - the global set editor
6. GraphQL for the website, with types generated from `schema.graphql` (GraphQL Code Generator). Never hand-write types for what the schema describes.
7. Playwright e2e tests:
   - sign in with Twitch and with Kick (test apps)
   - link both
   - upload, then approve as a second account with `emotes.review`
   - create a set and add the emote
   - search finds it
   - create a role and assign it
8. A `#uitest` pass across viewports and both themes before the phase is called done.

Done when:
- search finds approved emotes and not pending ones;
- the full upload → review → set → search flow works in the UI;
- role management works;
- the e2e tests pass.

PAUSE. Compact before Phase 7.

---

## Phase 7: Moderation, abuse and limits

Goal: the platform can run in public without being abused.

Tasks
1. Rate limits per IP and per account, on upload, search, auth and write endpoints.
2. Report flow: users report an emote with a reason, it goes into a queue, and a moderator acts on it. Everything is audited.
3. Takedown:
   - remove the emote;
   - block its sha256 and dHash;
   - push `emote.updated` so clients drop it;
   - notify the owner.

   Document it in `docs/takedown.md`, with a public takedown contact.
4. User restrictions: block uploads, or block an account, gated by `users.restrict`.
5. Legal pages for cyex.tv, checked against Swiss law (revFADP privacy notice, contact page) and the platform rules for Twitch and Kick.
6. Tests for the limits and for report → takedown → client removal.

Done when:
- a report can be filed and resolved end to end;
- a takedown reaches a connected client;
- the rate limits are tested.

PAUSE. Compact before Phase 8.

---

## Phase 8: Browser extension

Goal: emotes, badges and name paints from every supported provider in Twitch and Kick chat in the browser, each provider switchable.

Tasks
1. `cyex-tv/extension`: Manifest V3, TypeScript, Vite. One codebase for Chrome and Firefox (service worker vs background-script differences handled in one place).
2. Site adapters, one module per site: `sites/twitch/` and `sites/kick/`.
   - All selectors and any framework internals (Twitch chat is React) are behind that module, so a site change is a one-module fix.
3. Providers, through their **public APIs only** (no code copied, per D16), each switchable on its own:
   - **Emotes:** cyex.tv, 7TV, BTTV, FFZ.
   - **Badges and paints:**
     - cyex deck supporter badges (read `api.cyex.app/badges/v1/users` and `/badges/v1/badges` as their spec says: one snapshot, `If-None-Match`, every 30 minutes, nothing per message);
     - 7TV badges and paints;
     - BTTV and FFZ badges.
   - Keep cosmetics providers in their own module next to emote providers, so cyex.tv cosmetics can be added later without touching the emote code.
   - The live sockets follow the same pattern as the deck's.
4. Replace emote names in chat messages with images. cyex.tv personal emotes apply only to their owner's messages, using the batch lookup. Respect word boundaries and skip code and links. Handle zero-width emotes and FFZ modifiers. Render badges and paints with the user's name.
5. **Coexistence:** detect when the 7TV, BTTV or FFZ extensions are installed. In that case, turn off our rendering for that provider by default, so two extensions don't rewrite the same message.
6. Popup and settings:
   - per-provider toggles, with emotes and cosmetics listed separately;
   - per-channel enable;
   - animations on, off, or on hover (using the static variants);
   - a tooltip with the emote's name, provider and owner.
7. Manual test checklist in `docs/test-checklist.md`, run in Chrome and Firefox, on Twitch and Kick.
8. **Owner:** submit to the Chrome Web Store and Firefox Add-ons. The agent prepares the listing text and screenshots. Store review can take days to weeks.

Done when:
- the extension shows all providers on at least three real Twitch channels and two Kick channels;
- cyex deck badges show for a supporter;
- the toggles work;
- the checklist is complete;
- the store listings are submitted.

PAUSE. Compact before Phase 9.

---

## Phase 9: Operations and launch

Goal: production runs, is monitored, and can be restored.

Tasks
1. Deploy, documented in `cyex-tv/infra`:
   - containers for api, events, worker, web and `tv-db`, on their own Docker network;
   - new `cyex.tv` and `api.cyex.tv` tunnel hostnames;
   - nothing of cyex-api's is changed.
2. Backups:
   - a nightly `tv-db` dump, kept off the server as well;
   - R2 object versioning or a second copy;
   - a **tested restore**.
3. Monitoring:
   - error tracking and request metrics;
   - alerts on the events service, the worker queue depth and the R2 error rate;
   - if cyex-api shares the host, a resource limit per container so neither can starve the other.
4. A status page, or at least a documented incident process.
5. Launch checklist in `docs/launch.md`:
   - legal pages
   - privacy page
   - takedown contact
   - rate limits on
   - backups verified
   - moderators have roles
   - public API docs published

Done when:
- production is up on cyex.tv;
- a restore has been tested;
- the launch checklist is ticked.

---

## Later: cyex.tv chat cosmetics (not v1)

A possible feature of cyex.tv itself: badges and name paints shown next to a user's name in chat, in the extension and the deck. It is separate from the cyex deck supporter badges (D17).

What v1 leaves ready for it:
- the reserved `/v1/cosmetics` API namespace;
- the `cosmetic.*` event types;
- the `cosmetics/` R2 prefix;
- an image pipeline that is reusable;
- a cosmetics module in the extension.

When it starts, it gets its own planning pass and these decisions:
- how cosmetics are earned or bought (and whether cyex.tv takes payments at all);
- the paint format (gradients, images, shadows; the spec has to render the same in the extension and the deck);
- moderation of user-made art;
- the permissions (`cosmetics.review`, `cosmetics.grant`);
- how clients show cyex.tv cosmetics next to cyex deck badges without confusing them.

---

# Compact commands

Use these at each PAUSE. Run them in the coding agent as written, replacing `N` with the finished phase number.

Compact before the next phase:
```
/compact Phase N is complete. Keep: the repo paths, the ADR file names and their decisions, the current branch and open PR numbers, the contents of PROGRESS.md for Phase N, any failing test names and their error summary, and the exact task list of Phase N+1 from PLAN.md. Drop: command output logs, file contents that are committed and not being edited, resolved errors and failed attempts, and exploration results. Next step: read PLAN.md Phase N+1 and PROGRESS.md, then start Phase N+1 task 1.
```

Before the first phase (kickoff, no compact needed):
```
Read PLAN.md, AGENTS.md and PROGRESS.md. Summarize in 10 lines what is done and what is next. Do not change code yet.
```

Resume in a new session:
```
Read PLAN.md, AGENTS.md, PROGRESS.md and the last 5 commits. Continue from the first unchecked task in PROGRESS.md. Update PROGRESS.md after each task.
```

Emergency compact mid-phase:
```
/compact Keep: current task from PROGRESS.md, files being edited right now with their intended change, the last failing command and its error, decisions made in this session that are not yet in an ADR. Drop everything else.
```

---

# Open decisions

1. **Q3, researched 2026/11/16** (`docs/research/kick-api.md`, ADR 0009). Still open:
   - the owner reads Kick's Developer Terms (not public);
   - Kick doesn't confirm that `user_id` stays the same across username changes, or publish rate limits.

Answered 2026/11/16: licence (D18), deck in v1 (D19), set capacity (D20), personal sets (D21).
