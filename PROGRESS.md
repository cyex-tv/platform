# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Account activity feed (branch `feat/activity`)

ADR 0016. A public activity feed per account, like 7TV's profile "Activity" tab.

- [x] Migration `20261122000001_activity.sql` (additive): `audit_log.subject_account_id` and its index. The owner's own set, channel and cosmetics changes are now audited with the account as subject, and so is the approval of its emotes; staff actions never set it.
- [x] GraphQL (additive): `accountActivity(accountId, before, limit)` returning `ActivityEntry` (kinds `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED`, `ACTIVE_SET_CHANGED`, `COSMETICS_SELECTED`, `EMOTE_UPLOADED`, `SET_CREATED`, `SET_RENAMED`). Public, 1 to 100 per page (default 30). `apps/api/src/activity.rs`. Schema and web types regenerated; the OpenAPI spec is unchanged.
- [x] Privacy: no moderation, reasons or reviewers; no pending, private, rejected, taken-down or deleted emotes; blocked owner gives an empty list; blocked actors dropped; deleted sets dropped.
- [x] Tests: `apps/api/tests/activity.rs` against Postgres in CI (kinds in order, paging, privacy filters, editor attribution, limit clamp). `catalog_flow` and `cosmetics_flow` adjusted for the owner's audit rows.
- [x] Dev mock: `accountActivity` with sample activity for lumen_fox, quietoak and sunnyfern.
- No website page yet (another task).

### GitHub write log for PR 51 (`feat/cosmetics`, merged), added afterwards
```
git push -u origin feat/cosmetics
gh pr create --base main --head feat/cosmetics
gh pr merge 51 --merge
```

### GitHub write log for this PR (`feat/activity`)
```
git push -u origin feat/activity
gh pr create --base main --head feat/activity
gh pr merge <n> --merge   (only after all checks were green and the new database suite ran in the CI log)
```

## Chat cosmetics (branch `feat/cosmetics`)

ADR 0015. cyex.tv's own chat badges and name paints, like 7TV's (its public API docs and v3 types studied; no code taken), with no real cosmetics made yet. Separate from cyex deck supporter badges (ADR 0011).

- [x] Migration `20261121000001_cosmetics.sql` (additive): `cosmetic_images`, `badges`, `paints` (linear/radial gradient or image; angle, repeat, shape, fallback colour, 2-10 stops, up to 4 shadows), `badge_entitlements` and `paint_entitlements` (source grant/purchase/event, reason, granted by), `cosmetic_selections` (foreign keys into the entitlements, so only owned cosmetics, cleared on revoke or delete).
- [x] Permissions `cosmetics.manage` and `cosmetics.grant` (the owner holds both); account editors with `profile` pick for the owner.
- [x] Pictures: `POST /v1/cosmetics/images` (staff), original at `originals/cosmetics/{sha256}`, worker job `cosmetic.transcode` makes WebP and AVIF at 1x/2x/4x (badges 18 px square, paint images 64 px tall) under `cosmetics/{badges,paints}/{sha256}/` (`crates/media` profiles).
- [x] GraphQL (additive): `cosmetics`, `userCosmetics`, `myCosmetics`, `cosmeticImage`, `cosmeticEntitlements`; `createBadge`, `updateBadge`, `deleteBadge`, `createPaint`, `updatePaint`, `deletePaint`, `grantCosmetic`, `revokeCosmetic`, `selectCosmetics`. REST: `GET /v1/cosmetics`, `GET /v1/cosmetics/users` (ETag), `POST /v1/cosmetics/images`. Schema, OpenAPI and web types regenerated.
- [x] Events: topic `cosmetics` with `cosmetic.created`/`updated`/`deleted`; `user.cosmetics_updated` on the account's user topics. Every staff action audited; 300 cosmetics writes an hour per account.
- [x] Tests: `apps/api/tests/cosmetics_flow.rs` and `apps/worker/tests/cosmetics.rs` against Postgres in CI; unit tests of the validation, the media profiles, the worker's image path, topics and event shapes.
- [x] Dev mock: sample catalog (2 SVG badges, 3 paints, all marked sample data), every new query and mutation, the REST routes.
- [ ] Owner: allow `/cosmetics/` in the CDN's WAF rule (`docs/setup/owner-steps.md`).
- [ ] Website pages (another task) and extension and deck support.

## Account editors (branch `feat/account-editors`)

ADR 0014. An account lets others work on all its sets and channels, each with permission flags, like 7TV's editors (studied from 7TV's public help pages; no code taken).

- [x] Flags: `emotes` (members, aliases, the active sets too), `sets` (create, rename, delete, capacity up to the default, a channel's active set and settings), `editors` (manage editors within your own flags), `profile` (stored for cosmetics). The owner holds all of them.
- [x] Migration `20261120000001_account_editors.sql` (additive), `crates/db/src/editors.rs`, `apps/api/src/editors.rs`; set and channel checks in `catalog` name the flag they need. Per-set editors are unchanged.
- [x] GraphQL (additive): `accountEditors`, `editorOf`, `editorSets`, `editorChannels`, `addAccountEditor`, `updateAccountEditor`, `removeAccountEditor`, `CreateSetInput.ownerId`. `schema/schema.graphql` regenerated; the OpenAPI spec is unchanged (REST has no editor routes).
- [x] Rules: no escalation (grant only flags you hold, change only editors within your flags), the owner can't be added or removed, editors can leave, 20 editors per account, 60 editor changes an hour per caller, blocked or deleted accounts can't be added and their grants count for nothing, every change audited (and every set or channel change made through a grant, with `"via": "account_editor"`).
- [x] Tests: `apps/api/tests/sec_account_editors.rs` against Postgres in CI (each flag, IDOR, escalation, immediate loss of access, blocked accounts, limits); unit tests in `apps/api/src/editors.rs` and `apps/web/src/lib/tool/editors.test.ts`.
- [x] Website: `/me/editors` (rail under Settings): your editors with their permissions, add by Twitch or Kick login with checkboxes, change, remove; "You edit" with links to their sets and channels, and leave. `/me/sets` and `/me/channels` show what you manage for others, grouped and labelled with the owner; a new set can be made for an account you edit with `sets`. The set page shows what you may do there. The dev mock has sample editors and accounts you edit.
- [ ] Manual: the pages against the mock in the browser at phone and desktop widths, both themes.
- Open questions (defaults taken): an accept step like 7TV's invitations, notifications when a grant changes, whether editors may add the owner's private emotes.

## Security assessment 2026-11 (done 2026/11/17; open items listed)

Report: `docs/security/assessment-2026-11.md`. 0 critical, 2 high, 14 medium, 19 low. Both highs and 9 mediums fixed, each with a regression test that ran in CI against Postgres: platform PRs 38 (web headers, `back` check), 39 (blocklist on every frame, full purge), 40 (target seniority), 41 (bidi and invisible characters refused on write), 42 (re-review renamed emotes, no pending names on public topics), 43 (GraphQL socket subscriptions only, caps, bounded resume); extension PR 10 (numeric ids). Infra PR 5 (age-encrypted backups) waits for the owner. The owner extended merge rights to these security PRs.

- [x] Website follow-ups (branch `sec/web-csp-bidi`): a script CSP via `kit.csp` (nonces on rendered pages, hashes on prerendered ones; zero violations signed out, signed in and as staff in headless Edge against the mock), user-chosen and platform names rendered in `<bdi>` (`Name.svelte`) or wrapped in isolates (`iso()`) in strings; owner steps for the Cloudflare headers on prerendered pages in `docs/setup/owner-steps.md`.

#### GitHub write log
- `gh pr create --base main` for each fix branch (`sec/web-headers`, `sec/media`, `sec/role-seniority`, `sec/text-sanitize`, `sec/emote-edit-review`, `sec/gql-ws`), `-R cyex-tv/extension` for `sec/extension`, `-R cyex-tv/infra` for `sec/infra`.
- `gh pr merge <n> --merge` for 40, 41, 42, 39, 43, 38; `gh pr merge 10 -R cyex-tv/extension --merge`.

## Phase 8: Browser extension (merged, in `cyex-tv/extension`)

Six stacked PRs there, each with CI (licences, types, unit tests, Chrome and Firefox builds), and the store listing draft. The owner extended merge rights to the extension on 2026/11/17; all seven were merged that day, in order, and main's CI passed after the last one.

#### GitHub write log
- `gh pr merge 2 -R cyex-tv/extension --merge`, then for 3 to 7: `gh pr edit <n> -R cyex-tv/extension --base main` and `gh pr merge <n> -R cyex-tv/extension --merge`; then `gh pr merge 8 -R cyex-tv/extension --merge`.

- [x] PR 1 (#2): Manifest V3 for Chrome and Firefox from one codebase (Vite); emote providers for cyex.tv, 7TV, BTTV and FFZ over their public APIs; the emote index and message tokenizer (whole words, links and code untouched, zero-width and FFZ modifiers); settings; a background fetcher limited to the providers' API hosts, cached and revalidated with ETags.
- [x] PR 2 (#3): Twitch and Kick site adapters (all selectors in one module per site); a page-world script that copies sender and channel IDs from the sites' React props onto `data-` attributes; rendering with srcset, overlays, stills and hover animation, tooltips, copy as typed; personal emotes in batches; per-channel opt-out; providers left to their own extensions by default. Checked against live Twitch (148 of 148 messages tagged, BTTV and FFZ emotes drawn) and Kick (28 of 28 message bodies found).
- [x] PR 3 (#4): the popup with every setting, in the cyex-app tool style.
- [x] PR 4 (#5): badges: cyex deck supporters (`api.cyex.app/badges/v1` as its spec says; the background reaches cyex.app only under `/badges/v1/`), BTTV and FFZ. Real APIs' shapes checked.
- [x] PR 5 (#6): live updates for cyex.tv and 7TV channel sets (reload on change, reconnect with backoff). 7TV's EventAPI handshake checked live.
- [x] PR 6 (#7): 7TV badges and name paints from 7TV's EventAPI (`cosmetic.*`, `entitlement.*` per channel; gradients, 7TV-hosted pictures, shadows), with no presence posted for the viewer. Checked live: 7TV acknowledged both subscriptions and sent definitions and entitlements in the expected shapes.
- [ ] The manual checklist (`docs/test-checklist.md`) with the installed build in Chrome and Firefox, on three Twitch and two Kick channels.
- [ ] Owner: store listings (Chrome Web Store, Firefox Add-ons); the agent prepares text and screenshots.

## Phase 9: Operations and launch (started)

### PR 1: service images (branch `phase-9/images`)
- [x] `Dockerfile`: one release build of the workspace, then one image per service (`--target api`, `events`, `worker`) on `debian:bookworm-slim`, running as a non-root user, with health checks for api and events. Migrations are compiled into the api. No secrets in the images.
- [x] CI's `images` job builds all three and checks that each binary starts and asks for its config.
- [x] The production compose, tunnel hostnames, backups and restore are in `cyex-tv/infra` (PR 2, merged by the owner 2026/11/17).

### PR 2: metrics (branch `phase-9/metrics`)
- [x] `cyex_tv_shared::metrics`: counters, histograms and gauges in the Prometheus text format, written here (no new dependency). Each service serves `GET /metrics` on `CYEX_TV_METRICS_BIND` when set, on its own listener, so it never sits behind a public route.
- [x] api and events: `cyex_tv_http_requests_total` and `cyex_tv_http_request_duration_seconds` by method, matched route (`/v1/emotes/{id}`, never the raw path) and status, 429s included; live-update subscribers and topics; the database pool.
- [x] worker: `cyex_tv_jobs_finished_total` and `cyex_tv_job_duration_seconds` by kind and outcome; `cyex_tv_jobs` (not done, by kind and state: due, waiting, running, failed), read at scrape time; `cyex_tv_metrics_db_up`.
- [x] Merged 2026/11/17 as PR 30 (all checks green, the database tests seen running).
- [ ] Alerts: `cyex-tv/infra` PR 3 (a cron check posting to a webhook), for the owner.

### PR 3: the website's image (branch `phase-9/web-image`)
- [x] `apps/web/Dockerfile`: SvelteKit's Node build as the `node` user, configured at run time (`PUBLIC_API_URL`, `ORIGIN`, `PORT`); the health check loads `/developers`, which needs no API call.
- [x] CI's `images` job builds it and checks that `/developers` and `/` answer with the API unreachable, and that it doesn't run as root.
- [x] Merged 2026/11/17 as PR 31 (all checks green, the image started and served pages in CI).
- [x] The `web` service and the `www` redirect: `cyex-tv/infra` PR 3, for the owner.

### PR 4: the public API docs (branch `phase-9/api-docs`)
- [x] `/developers/reference`, prerendered, in the cyex-docs layout (sticky filterable index, endpoint blocks beside a sample panel with curl and JavaScript tabs and copy buttons) on the site's tokens, both themes, reduced motion, no external requests of its own.
- [x] REST: generated at build time from `openapi/openapi.json` (`apps/web/src/lib/docs/rest.ts`): every path and method, parameters, request body, responses, samples and example responses built from the schemas. GraphQL: generated from `schema/schema.graphql` with the `graphql` package (`graphql.ts`): queries, mutations, subscriptions, arguments, return types, the types they use, and curl and fetch samples. The event socket guide follows ADR 0005; its event types come from the SDL's `EventType`.
- [x] The web image is now built from the repository root (`docker build -f apps/web/Dockerfile .`, with `apps/web/Dockerfile.dockerignore`) so the build can read `openapi/` and `schema/`; CI's `images` job uses the new command.
- [x] Test: `apps/web/src/lib/docs/docs.test.ts` (every path and method in `openapi.json` is listed, samples, GraphQL operations and types).
- [ ] Manual: the image build and `/developers/reference` in CI's `images` job (Docker isn't available here); scroll and filter checked in the browser at 1440 px.

### Owner setup (2026/11/17)
- [x] Twitch and Kick OAuth apps; Cloudflare R2, `cdn.cyex.tv`, its WAF and cache rules, the storage and purge tokens (details in `docs/setup/owner-steps.md`). The CDN's rules were checked from outside.
- [ ] The env files on the server, the deploy (`cyex-tv/infra` `docs/deploy.md`) and the tunnel hostnames.
- [ ] The abuse@cyex.tv and privacy@cyex.tv mailboxes (the pages already name them).

#### Tests
- Unit: the text format (escaping, buckets, grouping), gauges, the optional bind.
- `apps/api/tests/http.rs`: requests counted per matched route and status, unrouted paths making no series, the scrape's content type, the gauges.
- `apps/worker/tests/jobs.rs` against Postgres: outcomes and durations counted per kind, the queue gauges before and after.

## Phase 4: cyex deck integration (in progress)

### The deck PR (branch `cyex-tv/provider` in `cyex-app/cyex-deck`, local only)
- [x] Provider `CyexTv`: the global set and a channel's active set from `api.cyex.tv/v1`, loaded and retried like 7TV, BTTV and FFZ; its own switch (`chat.cyextvEmotes`), the emote menu tab, cards linking cyex.tv, recordings. Types are the deck's own (ADR 0013), tested against fixtures in the platform's output shape.
- [x] Checked: deck clippy clean; deck Rust and UI tests pass apart from four that read `docs/`, which isn't in that repo (they fail on a fresh clone of main too); `svelte-check` 0 errors, 0 warnings.
- [ ] Pushed and opened as a PR: waits for the owner's OK (pushing outside `cyex-tv` needs it).
- [ ] Live updates from `/v1/events` and personal emotes per chatter (next deck PR). Kick waits for Kick support in the deck.

Work in `cyex-app/cyex-deck` goes on a feature branch with a PR the owner merges (the agent doesn't merge outside `cyex-tv/platform`).

### ADR 0013: how clients get the API's types (branch `phase-4/client-types`)
- [x] No platform crate is linked into clients: the platform is AGPL-3.0 and the deck has its own licence. Rust clients write small `serde` structs for the reads they use, tested against fixtures of real output; TypeScript clients generate types from `schema.graphql` and `openapi.json`. The contract stays additive within v1.

## Phase 7: Moderation, abuse and limits (in progress)

### PR 4: a test hang (branch `phase-7/test-shutdown`)
- [x] `restricting_and_blocking_accounts` hung for 20 minutes once in CI (PR 21's run, 2026/11/17). `abuse_flow`'s test world closed the pool while the event feeder still held its listener connection, and closing a pool waits for every connection; `live_flow` and the socket tests already stop the feeder first. `abuse_flow` now does the same (`World::close`).
- [x] CI's `test` job has `timeout-minutes: 20`, so a hang fails instead of holding a runner for six hours.

### PR 3: reading an account's restrictions (branch `phase-7/account-status`)
- [x] `accountStatus(accountId)`: whether uploads or sign-in are blocked, for `users.view` or `users.restrict`. Additive; the admin users page needs it. Tested in `restricting_and_blocking_accounts` (both permissions read it, anyone else is refused).

### PR 2: reports, takedowns and restrictions (branch `phase-7/moderation`)
- [x] Migration `20261118000006_moderation.sql` (additive): `reports` (one open report per emote and reporter), `notifications`, and `accounts.blocked_at`.
- [x] Reports: `reportEmote(id, reason)` for any emote the caller can see, at most 10 an hour per account. `reportQueue` and `resolveReport(id, DISMISS | TAKE_DOWN, note)` for `emotes.review` or `emotes.takedown`; taking down needs `emotes.takedown`. Audited.
- [x] Takedown (`takedownEmote` or a report's `TAKE_DOWN`), in one transaction: every emote made from the same file is taken down and leaves every set (`emote.updated`, then `set.emote_removed`, to the sets' channels and users), the sha256 and dHash go on the blocklist, owners get a notification with the reason, open reports on them close as actioned, and a `files.delete` job is queued. Audited.
- [x] The worker's `files.delete`: deletes the variants and the original from storage, then purges their CDN URLs through Cloudflare's API (`CYEX_TV_CLOUDFLARE_ZONE_ID`, `CYEX_TV_CLOUDFLARE_PURGE_TOKEN`) in batches of 30; a refused purge retries. Without a token it deletes and warns.
- [x] Restrictions: `restrictAccount(accountId, uploadsBlocked, blocked)` with `users.restrict`; a block ends the account's sessions and refuses sign-in (`account_blocked`); the owner is protected. Audited.
- [x] `myNotifications(unreadOnly)` and `markNotificationsRead`.
- [x] `docs/takedown.md`: the process, the public contact, what a takedown does.

#### Tests
- Unit: reasons, report states.
- `apps/api/tests/abuse_flow.rs` against Postgres: report and dismiss (duplicates, empty reasons, unseen pending emotes, the queue's permission, dismiss vs take-down permission, the audit row); the hourly report limit; a takedown through a report removing both uploads of the image from their sets, **reaching a live subscriber within a second** (`emote.updated` and `set.emote_removed`), blocking the image (a re-upload is `blocked`), queueing the file deletion, notifying both owners, and refusing a second takedown; restricting uploads, blocking an account (signed out at once), the owner protected.
- `apps/worker/tests/purge.rs` with a fake Cloudflare API: objects deleted and the right URLs purged with the token; a refused purge retries without forgetting the files; no token still deletes.

### PR 1: rate limits (branch `phase-7/limits`)
- [x] `cyex_tv_shared::ratelimit`: a token bucket per key, in memory (per instance; v1 runs one of each service), swept when it grows. The client IP is the peer address, or `CF-Connecting-IP` when `CYEX_TV_TRUST_CLOUDFLARE=true` (never trusted otherwise, since anyone can send it).
- [x] The api, per IP: sign-in 30 a minute, GraphQL 300 a minute, uploads 30 an hour, public reads 1200 a minute, opening the GraphQL socket 60 a minute; per signed-in account: GraphQL 600 a minute (plus the 20 uploads an hour from Phase 3). Health checks are never limited.
- [x] The events service: 60 new connections a minute per IP.
- [x] Over a limit: `429 rate_limited` with `Retry-After`.

#### Tests
- Unit: the bucket (burst, refill, separate keys), the client IP with and without trusting Cloudflare, which route falls under which limit, `Retry-After`.
- `apps/api/tests/http.rs` (no database): the sign-in limit hit and its `Retry-After`, health never limited, Cloudflare's header ignored unless trusted, and per-client buckets behind Cloudflare.
- `apps/events/tests/socket.rs`: the third connection in a minute from one IP refused with 429 at the handshake.

## Phase 6: Search and the website (in progress)

### PR 7: add and report from an emote's page (branch `phase-6/emote-actions`)
- [x] On an approved emote's public page: signed in, add it to one of your sets or report it (a reason of up to 500 characters); signed out, a sign-in link that comes back to the emote. Checked in a browser against the mock, signed in and out.

### PR 6: the website, admin pages (branch `phase-6/web-admin`, stacked on PR 5 and Phase 7 PR 3)
- [x] `/admin`, shown to accounts holding any moderation or admin permission, each part only with the permission it needs: an overview with the queue counts; the review queue (approve, reject with a reason); reports (dismiss with a note, take down with a reason); users (find by Twitch or Kick login, block uploads or the account, give and take roles); roles (create, edit with the permissions you hold, delete); the audit log (filter by kind and ID, before and after, older pages); the global set (the set page, without delete).
- [x] The mock gives the signed-in account every permission after `/mock/admin`.
- [x] Checked in a browser against the mock: every action above, 1280 and 375 wide (no sideways scroll), and an account without permissions refused.

### PR 5: the website, signed-in pages (branch `phase-6/web-me`, stacked on PR 3 and PR 4)
- [x] A `(tool)` route group in the cyex-app web-tool style (IBM Plex, warm grey, square cards, flat sticky header with uppercase links): its tokens are a copy of cyex-app's, set on `.tool` so the public pages keep the cyex-web look. Browser-only (`ssr = false`): the session cookie belongs to the API's domain. Signed out goes to `/signin?back=…`.
- [x] Your emotes (every state, paged), upload (drag and drop or pick, previews at three sizes, name and tags checked like the API before sending, `POST /v1/emotes`), an emote's page (details, zero-width, unlisted, NSFW, add to a set, delete; picks up the files once the worker has made them).
- [x] Sets: yours and the ones you edit, a new channel or personal set, a set's page (filter, rename an emote in the set, zero-width, remove; add from search or your uploads, with full sets and name clashes shown; rename; editors added by Twitch or Kick login and removed; delete, or leave as an editor).
- [x] Channels (active set, personal emotes) and account (linked accounts with link and unlink, notifications, permissions, sign out).
- [x] `dev/mock-api.mjs` signs in with a cookie and keeps the signed-in data in memory.
- [x] Checked in a browser against the mock: every flow above, 1280 and 375 wide (no sideways scroll), dark and light, and the public pages unchanged after visiting the signed-in ones. Unit tests for the name, tag and file rules.

### PR 3: the website, public pages (branch `phase-6/web`, stacked on PR 2)
- [x] `apps/web`: SvelteKit 2 (Svelte 5, TypeScript, `adapter-node`), the cyex web design (cyex-design + cyex-web tokens copied into `src/tokens.css`; Geist; dark by default with the light theme as a peer, switched before first paint; reduced motion honoured). Versions are pinned and at least two weeks old.
- [x] GraphQL types generated from `schema/schema.graphql` with GraphQL Code Generator (string documents, no GraphQL runtime in the browser); CI fails when they are stale.
- [x] Pages: home (hero, the global set, what it's for, developers band), search (name and tag, all, animated or still, paging), an emote's page (preview, 1x to 4x, facts), a channel's page (its active set), sign in (Twitch or Kick, and what is stored), developers (the public reads, personal lookup, live updates), 404 and errors.
- [x] `apps/web/dev/mock-api.mjs`: a stand-in API for looking at the site without the stack (development only).
- [x] Checked in a browser against the mock: desktop 1280 and phone 375, dark and light, the phone menu, search with filters, an emote, a channel, an unknown channel (404).
- [x] Signed-in pages and admin pages: PR 5 and PR 6.
- [ ] Playwright end-to-end tests: they need test OAuth apps (the owner's) to sign in.

### PR 2: CORS for the website and browser clients (branch `phase-6/cors`)
- [x] `apps/api/src/cors.rs`: the site (`CYEX_TV_SITE_URL`) gets credentialed CORS for everything; any other origin only the public reads (channels, sets, emotes, the personal-set lookup) with `*` and no credentials; nothing else. Preflights answer before the rate limits; `ETag` and `Retry-After` are exposed.
- [x] Tests (`apps/api/tests/http.rs`, no database): the site's preflight and requests, public reads from another origin, GraphQL, uploads and sign-in refused to other origins.

### PR 1: search (branch `phase-6/search`, merged 2026/11/17 as PR 15)
- [x] `crates/db::search` behind the `EmoteSearch` trait (ADR 0004), on Postgres: names by substring and `pg_trgm` similarity, tags by prefix; only approved, listed (not private), not deleted emotes, and NSFW only with `nsfw: true` (new, additive). Ranked: exact name, name prefix, similarity, popularity. `%` and `_` are taken literally.
- [x] Migration `20261118000005_popularity.sql` (additive): `emotes.popularity`, the number of channels whose active set holds the emote, refreshed in the worker's hourly run; indexes for searchable emotes and tag prefixes.
- [x] `searchEmotes(query, animated, owner, nsfw, page, perPage)`: 1 to 100 per page, default 50. With that, every GraphQL field in the contract is built.
- [x] Browse data for the redesigned site (additive, `api/browse-data`): `searchEmotes(sort: RELEVANCE | TOP | NEW)`, `Emote.channelCount` (one grouped query per list), `Query.account(id)`, `Query.emoteChannels(id, limit)` (1 to 50, default 8); migration `20261119000001_channels_active_set.sql` (index); the mock API and the generated web types follow. There is no REST search endpoint, so REST only gets the optional `channel_count` on the emote shape. Tests in `search_flow.rs`.
- [ ] Website redesign (`web/redesign`): new nav with Ctrl K palette, emote cards that play on hover, home (emote cluster, Top / New strip, chat previews, steps, install cards), browse with sort, filters and load more, emote page (preview stage, facts, used in, review, more from the creator), channel and new creator pages, signed-in pages brought to cyex-app. Research and plan in `.claude/runs/2026-11-17-web-redesign/`. Checked with the mock at 375 to 2560, dark and light; reduced motion only through headless rendering; not yet on a real phone.

#### Tests
- Unit: LIKE escaping.
- `apps/api/tests/search_flow.rs` against Postgres: rank order (exact, prefixes, tag-only last), pending, private, deleted and unasked NSFW emotes never found, the animated and owner filters, an empty query by popularity, paging and the page-size cap, literal wildcards; popularity counted from active sets, and a second refresh changing nothing.

### PR 4: your own emotes and a set's editors (branch `phase-6/my-reads`)
- [x] `myEmotes(page, perPage)`: the caller's emotes in every state (pending, rejected, taken down), newest first, as an `EmotePage`; `setEditors(setId)` for the set's owner, its editors and `sets.edit_any`. Additive; the signed-in pages need both.
- [x] Tests in `apps/api/tests/catalog_flow.rs`: paging and order, only your own, signed out refused; editors seen by owner and editor, refused to a stranger.

## Phase 5: Live updates (done, merged 2026/11/17 as PRs 12 to 14 and 27)

Taken before Phase 4 in the overnight run: Phase 4 (the deck, in `cyex-app/cyex-deck`) can't meet its "Done when" until the owner's CDN and OAuth steps are done, and its step 2 waits for Phase 5 anyway. See the run's questions.

### PR 4: the GraphQL socket's load run (branch `phase-5/gql-load`)
- [x] `apps/api/tests/gql_load.rs` (ignored; CI's `load` job runs it after the event socket's): 2000 `graphql-transport-ws` subscriptions to `events` on `/v1/gql/ws`, ten topics each, 300 events at 30 a second.
- [x] CI run on 2026/11/17: 2.000 of 2.000 subscribed in 7,8 s; 6.000 of 6.000 deliveries; write to delivery p50 1,2 ms, p95 2,8 ms, p99 7,7 ms, max 9,1 ms. The same run's event-socket test: 15.000 of 15.000, p95 18,9 ms (both share one runner).

### PR 3: the load test (branch `phase-5/load`)
- [x] `apps/events/tests/load.rs`, run by CI's `load` job in a release build (`cargo test --release -p cyex-tv-events --test load -- --ignored --nocapture`).
- Result, CI run on 2026/11/17 (GitHub `ubuntu-latest`, client and server in one process, Postgres in a service container):
  - 5000 of 5000 connections subscribed, ten topics each over 1000 topics, in 12.3 s; 0 failed.
  - 300 events written at 30 a second; 15000 of 15000 deliveries arrived.
  - Write to delivery: **p50 1.6 ms, p95 2.5 ms, p99 6.6 ms, max 85.3 ms** (target: p95 under 1 s).
- [ ] The same load on the GraphQL socket. It runs the same hub and feeder, so it is expected to match; a second run is still owed.

### PR 2: the GraphQL subscription (branch `phase-5/gql`)
- [x] `subscription { events(topics) }` on `/v1/gql/ws` (`graphql-transport-ws`), fed by its own hub and feeder in the api, so it gets the same events as `/v1/events` from the same outbox. Topics are checked (`invalid_topic`, `too_many_topics`, 500 at most). A lagging subscriber gets a `RESYNC` event.
- [x] Changes to personal sets go out as `personal.updated` (ADR 0005), on the set's topic and the owner's `user:` topics.

#### Tests
- `apps/api/tests/live_flow.rs` with a real server, a `graphql-transport-ws` client and Postgres: an `addSetEmote` mutation reaches a subscriber on another connection in under a second (measured), a personal set change arrives as `PERSONAL_UPDATED` on the user topic, a bad topic is refused.

### PR 1: the hub and the event WebSocket (branch `phase-5/events`, merged 2026/11/16 as PR 12)
- [x] `crates/hub`: topics and subscribers, each event handed to a subscriber once even when it matches several of its topics. A subscriber whose queue (1024) fills is marked lagged and gets `resync`, so a slow client never holds up the rest.
- [x] The feeder reads `events_outbox` on `NOTIFY cyex_tv_events` and every second. It keeps a watermark, so an event whose transaction commits after a higher ID is visible is still delivered; a missing ID (a rolled-back transaction) holds it back for at most 5 s.
- [x] `apps/events`: `/v1/events` with the ADR 0005 protocol: `hello`, `subscribe` and `unsubscribe` with `ack` (refused topics listed with their code), `dispatch`, `heartbeat` both ways (closed after 60 s of silence), `resume` with replay from the outbox or `resync` (gap over 24 h, older than the outbox keeps, or over 5000 events), `reconnect` on shutdown, `error` and close on a broken limit (500 topics, 10 frames a second, 16 KB a frame).
- [x] Migration `20261118000004_event_sessions.sql` (additive): a session's topics, kept 24 h, so `resume` works on another instance and after a restart. Saved on every subscribe and unsubscribe; an open connection marks its session in use every 10 minutes. The worker's clean-up removes old ones.

#### Tests
- Unit: the hub (once per subscriber, unsubscribing, dropping, lagging), topic names, the frame bucket, both event shapes.
- `crates/hub/tests/feeder.rs` against Postgres: a commit out of ID order is delivered once the lower ID commits, and a rolled-back ID is passed after the wait.
- `apps/events/tests/socket.rs` with real servers and a real WebSocket client: subscribe with refusals and delivery in under a second (measured and printed), other topics and unsubscribed ones not delivered, resume replaying exactly the missed events once and then live ones, an unknown session, a gap too old (`resync`), resume on a fresh instance after `reconnect` and a restart, the topic, rate, format and size limits.

## Phase 3: Upload pipeline, CDN, approval gate (done, merged 2026/11/16 as PRs 8 to 11; the CDN itself waits for the owner's R2 steps)

Started 2026/11/16, in an overnight run the owner asked for (work on, merge green PRs, questions at the end).

### How Phase 3 is split
1. Emote reads, edits, deletion and the review API (no image code).
2. The job queue (Postgres, `FOR UPDATE SKIP LOCKED`), the storage layer (S3 API: R2 in production, Versity Gateway locally) and the worker loop.
3. The streamed upload: limits, decoding to validate, hashing, the blocklist, duplicates.
4. Transcoding (WebP and AVIF at 1x-4x, static first frames), after benchmarking the encoders as the plan asks.

### Merges (agent, under the owner's OK)
`gh pr merge <n> -R cyex-tv/platform --merge --subject "Merge pull request #<n> from cyex-tv/<branch>"` for PR 8 (`phase-3/review`), PR 9 (`phase-3/queue`), PR 10 (`phase-3/upload`) and PR 11 (`phase-3/transcode`), each after all 6 checks passed with its new database suites seen running in the CI log. Fixes CI found on the way: a stale `not_implemented` expectation, the MinIO image being gone (now Versity Gateway), a schema check on a test session, a fixture whose dHash really was close to the blocked one, and the `NCSA` licence for `libfuzzer-sys`.

### PR 4: transcoding (branch `phase-3/transcode`)
- [x] ADR 0012 (image encoding), after a benchmark: WebP through libwebp (`webp-animation`, still and animated, quality 90), AVIF through `ravif` (quality 80, speed 8) for still images and first frames. No animated AVIF yet: `ravif` can't write it, and libavif needs CMake and a C AV1 encoder (the owner's call, see questions).
- [x] `crates/media::encode`: sizes 32, 64, 96 and 128 px tall, aspect kept up to 4:1, never scaled up; Lanczos3. The original is decoded frame by frame and each frame goes straight into four streaming WebP encoders, so an animation is never held in memory whole. Frames under 20 ms show for 100 ms, as browsers do.
- [x] The `emote.transcode` job in the worker: reads the original, writes `emotes/{sha256}/{name}` with `Cache-Control: public, max-age=31536000, immutable` and the right type, records the WebP files first (the emote is usable), then the AVIF files. A missing or broken original fails the job for good; missing storage retries.
- [x] Owner steps for the CDN (`docs/setup/owner-steps.md`, Phase 3): the custom domain, a WAF rule so only `emotes/` is served, the cache rule, CORS, and the R2 token for the env.

#### Tests
- Unit (`crates/media`): the size rules, an animated WebP upload read, a still image's eight files (each WebP decoded back at the right size, each AVIF checked), animations from GIF, APNG and animated WebP staying animated (frame count decoded back) with their static WebP and AVIF first frames, the limits holding while transcoding.
- `apps/worker/tests/transcode.rs` against Postgres: a still upload's eight files with type, size and cache header, an animated one's twelve, a missing original and a broken one failing for good, a missing store retrying.

### Phase 3 status against "Done when"
- A fixture upload gives 1x-4x WebP, AVIF and static files under `emotes/{sha256}/`: yes, in the tests with the in-memory store and the S3 client against a real S3 server. On the real CDN: after the owner's R2 steps.
- Every rejection case has a test: yes (SVG, video, AVIF for now, corrupt, oversized, too wide, pixel bomb, too many frames, too much work, blocklisted exact and near copy, rate limit, restricted, signed out).
- Pending emotes stay out of public reads: yes (sets, `GET /v1/emotes/{id}`, GraphQL).

### PR 3: the upload (branch `phase-3/upload`)
- [x] `crates/media`: the format by magic bytes; full decoding under the limits (5 MB, 1000×1000 px, 1000 frames, and 100 million pixels over all frames), with the canvas size checked from the header before anything is decoded; the 64-bit dHash. PNG, APNG, GIF and WebP (still and animated). AVIF is recognised and refused for now (decoding it needs dav1d, a C library); SVG and video are refused.
- [x] Migration `20261118000003_blocklist.sql` (additive): `blocked_images` (exact sha256, or a dHash within 4 bits), and an index for the upload rate limit.
- [x] `POST /v1/emotes` (multipart: `name`, `tags` comma-separated, `file`): signed in, not restricted, at most 20 uploads an hour (the owner is exempt). The body is capped just over 5 MB and the file stops being read at 5 MB. Decoding runs off the request threads. A blocked image is `blocked`; a file seen before is reused (same object, same URLs, no second job).
- [x] In one transaction: the file row, the original in storage (`originals/{sha256}`, no public cache header), the `emote.transcode` job, and the `pending` emote with its tags. If storage fails, nothing is kept.
- [x] The api reads `CYEX_TV_S3_*` too; without storage, uploads answer 503.

#### Tests
- Unit (`crates/media`, with every fixture made in code): sniffing, a static PNG, an APNG, animated and still GIFs, a static WebP, SVG, video and AVIF refused, corrupt PNG, GIF and WebP, the file-size, side, frame and total-pixel limits, a 50000×50000 pixel bomb with a valid header checksum, dHash on near copies.
- `apps/api/tests/upload_flow.rs` against Postgres: a PNG becomes a pending emote with its original stored and one job; APNG, GIF and WebP; a duplicate reuses the file; SVG, video, corrupt, oversized and too-wide files refused with nothing kept; bad forms, signed out, restricted; exact and near-copy blocklist hits; the hourly limit per account; no storage.

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
