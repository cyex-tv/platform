# PLAN.md review (history)

> **This is a historical record.** It is the review of the first draft of the plan, and the decisions as they were made, in order. Several were later replaced: cyex.tv became fully separate from cyex.app, the API became GraphQL-first, and live updates moved to WebSockets. **`PLAN.md` and `docs/adr/` are the current truth.** Where they differ from this file, they win.

Date: 2026/11/16. A critical read of PLAN.md before Phase 0. It covers facts that change the plan, gaps, and proposed changes.

Sources: the cyex-skills repo, a shallow clone of `cyex-app/cyex-deck`, the public GitHub API, and the 7TV, BTTV and FFZ repos and public APIs.

---

## 1. Facts that change the plan

| # | Finding | Impact |
|---|---|---|
| F1 | The `cyex-tv` org now exists. It was created today at 10:05 UTC and has 0 public repos. | Open decision 1 and the org note at the end of PLAN.md are out of date. |
| F2 | `gh` is not installed on this machine (in either Bash or PowerShell). | The repo procedure in Phase 0 cannot run as written. It needs `winget install GitHub.cli` and then `gh auth login`, done by you. |
| F3 | cyex deck is **Tauri 2 + Svelte 5**. It already has a `cyex-emotes` crate (~7.700 lines) with 7TV, BTTV and FFZ providers. That crate covers WS sockets, zero-width emotes, FFZ modifiers, personal and special sets, unlisted and NSFW flags, renames, and fixtures and fuzz tests. | Phase 7 is mostly "add a `cyex` provider to an existing crate", not a new client crate. Open decision 2 is answered: Svelte, in a webview. Animated WebP and AVIF play natively in `<img>`, so there is no "frame data" renderer to build. |
| F4 | cyex already runs a backend at **`api.cyex.app`**, with accounts, sessions, badges (`/badges/v1/...`), a store, and supporter/founder items. The deck signs in to it. | The plan creates a second user system and a second domain (`api.cyex.tv`). This is the biggest open question: see Q1. |
| F5 | 7TV's current website is **SvelteKit**, inside its Rust monorepo. The old Vue site is legacy. Only the 7TV *extension* is Vue. | ADR 0006's reason ("matches 7TV's Website repo") is wrong. The deck is Svelte, and the cyex-docs skill assumes Svelte projects. |
| F6 | Licences: the 7TV Extension is **Apache 2.0 + Commons Clause**. The 7TV monorepo is multi-licensed and requires a CLA. BTTV is a **restrictive custom licence** (no redistribution, not OSI). FFZ's client is Apache-2.0. | ADR 0007 should say we copy no code from 7TV or BTTV. Protocols and public API shapes may be studied. FFZ code may be reused with its notice kept. "Matches SevenTV" is not a reason for any ADR. |
| F7 | On a **GitHub Free** org, branch protection, rulesets and secret scanning are **not available for private repos**. They need GitHub Team, or Secret Protection for scanning. | Phase 0 step 4 fails on private repos unless the org is paid. See Q4. |
| F8 | 7TV uses MongoDB, Typesense and NATS, plus ClickHouse for analytics. It uses S3 (Wasabi) and a Rust image processor (libavif, libwebp, gifski, ffmpeg). Its events are SSE **and** WS with session resume, and each event is `{op, t, d}` with dispatch types like `emote_set.update`. Upload limits: 7 MB, 1000×1000, 1000 frames. Every upload is human-approved. | These are useful reference points. Our stack (Postgres plus a job table plus LISTEN/NOTIFY) is simpler and fits v1 scale. |

---

## 2. Gaps and problems, by phase

### Strategy (not in the plan at all)
- **Distribution is the real problem.** Nobody sees a cyex emote unless their client shows it. Chatterino, the 7TV extension, BTTV and FFZ will not show cyex emotes unless asked to. The plan treats the extension as Phase 8 of 9.
  - Proposal: make the deck the first client right after Phase 3. It already has the provider shape, so it is cheap.
  - Proposal: publish a **stable, documented public read API** early, using the cyex-docs layout, so third-party clients can adopt it.
  - Proposal: decide whether the extension also shows 7TV/BTTV/FFZ emotes, or coexists with their extensions. Two extensions rewriting the same chat DOM fight each other.
- **Global emotes.** Is there a cyex global set? It is not in the data model.

### Phase 0
- Tasks 2 and 3 are yours, not the agent's. The agent must not create accounts or register apps. Mark them as user-owned.
- The Twitch app needs a second redirect for local dev: `http://localhost:<port>/auth/twitch/callback`.
- The plan lists both a separate `infra` repo and `platform/infra/docker-compose.yaml`. Pick one. Proposal: local dev compose goes in `platform`, and the `infra` repo holds Cloudflare, deploy and backups.
- ADR 0003: use **Cloudflare R2** for storage. It has no egress fees and sits on the same Cloudflare account as the CDN. This is cheaper than a generic S3 bucket behind Cloudflare.
- ADR 0004: start with **Postgres `pg_trgm` + full-text search**. Emote search is short-name prefix and fuzzy matching, which Postgres handles well at v1 scale. That is one less service to run, back up and sync. Move to Meilisearch or Typesense once there is load. The sync job from Phase 4 task 1 then goes away for v1.
- ADR 0005: SSE is fine, but see the connection-count issue under Phase 5.
- Add **ADR 0008, OpenAPI workflow**. Proposal: code-first with `utoipa`, the spec generated, a CI check that the committed `openapi.yaml` equals the generated one, and the TS client generated with `openapi-typescript`. A hand-written spec plus hand-written handlers drifts, and the check "OpenAPI matches the implementation" cannot be automated.

### Phase 1
- Fine as is. Add `cargo deny` for licences and advisories, since ADR 0007 needs something that enforces it.

### Phase 2: data model
- **Key everything by Twitch user ID, not login.** Logins change. `GET /channels/{login}` and `/events?channel={login}` should take an ID, with a login lookup as a convenience.
- `channels` duplicates `users`: a Twitch channel *is* a Twitch user. Proposal: make `active_set_id` a column on `users`, or keep a 1:1 `channels` row keyed by the same ID.
- Missing:
  - **Editors/grants** table. Phase 2 task 4 says "or have been granted", but nothing stores grants.
  - **Set capacity**.
  - **Emote flags** spelled out: zero-width, private, listed/unlisted, NSFW. The deck model already has all of these.
  - **Tags**. Search uses them, but no table stores them.
  - **Sessions**.
  - **Usage/popularity** source. Search ranks by it, but nothing records it.
  - **Global set**.
- `emote_versions`: does v1 need re-uploads that replace an emote under the same ID? If not, drop it and keep one file per emote. That makes v1 simpler.
- Auth: if Q1 answers "reuse cyex accounts", Phase 2's OAuth work shrinks to verifying the existing session.

### Phase 3: uploads
- **512 KB is too small.** Typical animated GIFs are 1-5 MB. 7TV allows 7 MB. Proposal: 5 MB in, plus limits on frames, dimensions (1000×1000) and total decoded pixels, to stop decompression bombs.
- Inputs: PNG, APNG, GIF, WebP, AVIF. No video in v1.
- Outputs: WebP and AVIF at 1x-4x. Also add **a static first-frame variant** for animated emotes. The cyex design rules require a reduced-motion branch, and 7TV and BTTV ship one too. Consider GIF output for older clients (7TV still serves it).
- Decoding untrusted images: the worker gets only the credentials it needs (job table and bucket), no outbound network beyond them, and memory and time limits per job.
- AVIF encoding is slow (ravif/rav1e). The planned benchmark is right. Encode WebP first and mark the emote ready, then add AVIF after.

### Phase 4: web
- Framework: see Q2.
- Styling: use the cyex token files (plain CSS, from `cyex-app` and `cyex-design`), not Tailwind. The emote site is an operated tool, so `cyex-app` applies at web-tool density. Its landing page and the API docs use `cyex-web` and `cyex-docs`.

### Phase 5: events
- **Connection count.** `GET /events?channel={login}` means one connection per channel. Browsers cap HTTP/1.1 at 6 connections per origin, and the deck watches many channels at once. The endpoint needs **many subscriptions on one connection** (`?channels=id1,id2,...`, or subscribe/unsubscribe over WS). HTTP/2 on Cloudflare helps, but the API should not depend on it.
- Events should name the set ID as well as the channel, so a channel switching its active set is its own event (`channel.set_changed`).
- Resume via `Last-Event-ID` needs a replay buffer (an events table with a retention window). Say how long it keeps events.

### Phase 6: moderation
- Uploads are visible from Phase 3 and 4 on, but moderation arrives in Phase 6. Proposal: from day one, an emote is **unlisted until approved** (the 7TV, BTTV and FFZ model). It is still usable in the uploader's own channel, but it doesn't appear in search until a moderator approves it. Without this, public search is a liability before Phase 6.
- Legal: a takedown contact and process before anything is public, not only at launch. Also check what Swiss law requires (an Impressum-style contact page and data-protection notes under the revFADP).

### Phase 7: deck
- Rewrite it around F3:
  - add `Provider::Cyex` to `cyex-emotes`
  - map the API into the existing `Emote`/`EmoteSet`
  - connect events
  - add a settings toggle
- The disk cache by sha256 and LRU may already exist in `app/src/sys/cache.rs`. Check that before building one.
- Drop "decide the render approach". It is the webview.

### Phase 8: extension
- Add Firefox MV3 differences (background scripts vs service worker) and store review times, which take days to weeks.
- Twitch chat is React. BTTV reads React fiber internals as well as DOM selectors. Selectors in one module is right, but plan for React-internal access too, behind the same module.

### Process
- "One task = one PR" with branch protection and PR review means every task waits on you. Fine, but say so: you are the reviewer and merger.
- Put PLAN.md, PROGRESS.md and AGENTS.md in the `platform` repo once it exists. Until then they live in this folder.

---

## 3. Decisions (answered 2026/11/16)

1. **Identity:** reuse the existing cyex accounts from `cyex-app/cyex-api` (private; Node 22, Express 5, Postgres 17 `deck-db`, Redis, ClickHouse; self-hosted Docker behind a cloudflared tunnel). It has site cookie sessions and deck Bearer sessions (both stored as sha256), CSRF origin checks, rate limits, an image pipeline (sharp, sha256 + dHash blocklist), a badge review queue, admin permissions and an audit log.
2. **Web framework:** SvelteKit, with the cyex token CSS.
3. **Uploads in v1:** anyone signed in can upload. An emote is unlisted until approved and usable in the uploader's own channel right away.
4. **GitHub:** the repos are public from the start, so ADR 0007 (licence) comes before the first push.
5. **Platforms:** Twitch **and Kick** in v1.
6. **Moderation:** a small team from day one, so admin tools need real **role creation and permission management**. Admins create roles, choose a role's permissions, and assign roles to people, and every change is audited. Today cyex-api has fixed permissions stored as a comma list per admin, plus owners set in an env var. That is not enough.
7. **Extension:** shows **all providers** (cyex, 7TV, BTTV, FFZ), and each can be turned off on its own, like the deck.
8. **Global set:** yes, curated by admins, and editing it is a permission.
9. **Search:** Postgres (`pg_trgm` + full-text) first.
10. **Storage:** Cloudflare R2, served at `cdn.cyex.tv`.
11. **Hosts:** the whole platform is on cyex.tv. The website is `cyex.tv`, the API and events are at `api.cyex.tv/v1`, and the files are on `cdn.cyex.tv`. This replaces the earlier "inside api.cyex.app" answer. Accounts are still shared, through a single sign-on handoff: you sign in on cyex.app, a one-time code is passed back, and cyex.tv sets its own cookie. That avoids cross-site cookies, and it means nothing in cyex.app's routing changes.
12. **Accounts:** linked identities (one cyex account, with Twitch and Kick identities attached), added **without breaking cyex-api** (see "cyex-api safety rules" below).
    - New tables `accounts` and `identities(account_id, platform, platform_user_id)` are added beside the existing ones.
    - Every existing `users` row is backfilled as a Twitch identity.
    - `users`, `sessions`, `app_sessions` and every other existing table, route and response stay exactly as they are. The badges, store, logs and recap code keeps working unchanged.
    - Kick sign-in writes only to the new tables, so badges stay Twitch-only until cyex-api chooses to opt in, if it ever does.
13. **Backend language:** you left it to me. My choice: **Rust services beside cyex-api**, not inside it.
    - Cloudflare routes `api.cyex.app/emotes/*` to them.
    - They live in the same Postgres instance, in their own `emotes` schema, with their own DB role.
    - That role can only read cyex-api's sessions and accounts tables, so a session from the site or the deck works with no token exchange.
    - Why Rust:
      - the hot paths (CDN misses, thousands of open event connections, AVIF/WebP transcoding through libavif/libwebp) are where Rust pays off;
      - the deck's `cyex-emotes` crate can share the API types directly;
      - cyex-api keeps images as `bytea` in Postgres, which is fine for badges but not for an emote library in R2.
    - What carries over from cyex-api: the upload rules, the sha256 + dHash blocklist approach, and the audit-log shape. These are reimplemented in Rust, with no code copied across languages.

## cyex-api safety rules (your rule, 2026/11/16)

cyex-api also serves cyex logs, recap, deck badges, the store and payments. Nothing in the emote work may break them. These rules go into the platform AGENTS.md:

- **Additive only.**
  - Never rename, drop or change the type of an existing column, table, index, route, env var or response field.
  - New migrations only add tables or nullable columns, and run cleanly on a copy of the production schema.
- **Emote code stays out of cyex-api where possible.** The Rust services own the `emotes` schema. Their DB role can only `SELECT` from cyex-api's `sessions`, `app_sessions`, `users` and the new `identities`. It cannot write to them, and it has no access to payments, purchases or the store.
- **No route overlap.** cyex-api has no `/emotes` path today, which I checked. The Cloudflare rule sends only `api.cyex.app/emotes/*` to the new services, and everything else still goes to cyex-api.
- **Shared database load.** The emote services get their own connection pool limit and statement timeout, so they can't starve cyex-api's connections on `deck-db`.
- **Every cyex-api change is its own PR**, with:
  - cyex-api's tests (`npm run test:deck`) passing;
  - a manual check of the badges and logs endpoints;
  - a rollback note.
  
  It is never bundled with emote work.
- **ClickHouse is read-only.** Its `emoteUsage` table (already used by recap) is a good source for popularity. The emote services only read it.
- When a change to cyex-api can't be made additive, it stops and becomes a question to you.

## 4. Follow-ups these decisions create

- **Roles in one place.** Design the RBAC tables (`roles`, `role_permissions`, `account_roles`) once, in the shared database, with permissions namespaced (`emotes.review`, `emotes.global_set`, `badges.review`, `store.edit`, …). cyex-api's comma-list admins keep working as they are. Moving them onto the shared roles is optional, done later, in its own PR, and only if it can be done without breaking anything.
- **Kick details to check before Phase 2:**
  - Kick's OAuth app registration and scopes;
  - how stable Kick channel and user IDs are;
  - Kick chat's DOM for the extension (kick.com is not React-fiber-based like Twitch);
  - how the deck already handles Kick, if it does.
- **Deploy.** The Rust services join the same Docker host and the `deck_db` network, as new containers behind the existing cloudflared tunnel. The `infra` repo documents it.
- **Phase order.** Phase 0 now includes the additive identity tables in cyex-api. The deck integration moves up, right after uploads, to be the first real client.

## 5. Update 2026/11/16, PLAN.md version 3 (owner's call)

These two decisions **replace** decisions 1, 11, 12 and 13 and the "cyex-api safety rules" above, wherever they conflict.

- **cyex.tv and cyex.app are separate products with separate APIs.**
  - cyex.tv gets its own Rust API at `api.cyex.tv`, its own Postgres (`tv-db`), its own accounts with linked Twitch and Kick identities, its own sessions and its own OAuth apps.
  - Some endpoints duplicate cyex-api's (sign-in, `/me`). That is accepted for a clean split.
  - cyex-api isn't changed at all: no identity tables, no single sign-on, no shared database grants.
- **Cosmetics are split.**
  - The cyex deck supporter badges belong to cyex.app. The extension shows them through the public `api.cyex.app/badges/v1` read API, as one provider among 7TV, BTTV and FFZ.
  - cyex.tv's own chat cosmetics (badges and name paints) are a later phase. v1 only reserves room for them (API namespace, event types, R2 prefix, extension module).
