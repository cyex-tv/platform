# Security assessment, November 2026

Scope: `cyex-tv/platform` (api, events, worker, crates, web), `cyex-tv/extension` and `cyex-tv/infra`, at `main` on 2026/11/17, plus the website redesign branch (`web/redesign`, PR 37) for new issues only. Method: code review of every area below, with attack cases written as integration tests that CI runs against Postgres and the S3 stand-in. Nothing was tested against a live host, and no real credentials were used.

Areas: OAuth (Twitch, Kick), sessions, CSRF, CORS; authorization on every query, mutation and REST route; GraphQL limits and sockets; uploads, decoding, the blocklist, storage and the worker; the website (XSS, headers, redirects, framing); the extension (messages, DOM, remote data); deployment (compose, tunnel, secrets, metrics, backups).

## Summary

| Severity | Found | Fixed | Open |
|---|---|---|---|
| Critical | 0 | 0 | 0 |
| High | 2 | 2 | 0 |
| Medium | 14 | 9 (1 partly) | 5, plus 1 in an infra PR for the owner |
| Low | 19 | 3 | 16 |

Fix PRs, each with a regression test and green CI (DB tests ran, not skipped): platform #38, #39, #40, #41, #42, #43; extension #10. Infra #5 is open for the owner.

## High

### H1. Renaming or retagging an approved emote skipped review (fixed, #42)
- Where: `apps/api/src/moderation.rs` `update`.
- Repro: upload `Cute`, wait for approval, then `mutation { updateEmote(id: "E", input: { name: "<anything>", tags: ["<anything>"] }) { state name } }`. It answered `APPROVED`, and the new name and tags showed in search and on the emote page without a reviewer seeing them. Accounts with uploads blocked could still edit.
- Fix: when an owner without `emotes.review` or `emotes.edit_any` changes the name or tags of an approved emote, it goes back to pending in the same transaction (audited as `emote.resubmitted`). Flags alone keep it approved. A restricted owner gets `403 restricted`.
- Test: `apps/api/tests/sec_emote_review.rs`.

### H2. A junior moderator could block a senior admin (fixed, #40)
- Where: `apps/api/src/abuse.rs` `restrict`.
- Repro: a role at position 10 with `users.restrict` runs `mutation { restrictAccount(accountId: "<admin at 100>", blocked: true) { blocked } }`. It succeeded and deleted every session of the admin.
- Fix: `Actor::may_act_on` in `crates/auth/src/rbac.rs`. Acting on another account needs the target's highest role to be below yours (the owner always may). `restrict` and `unassignRole` both check it.
- Test: `apps/api/tests/sec_roles.rs`, plus the rbac unit test.

## Medium

| # | Finding | Where | Status |
|---|---|---|---|
| M1 | Bidi controls (U+202A-202E, U+2066-2069), zero-width and other invisible characters were accepted in emote names, aliases, set and role names, reasons and descriptions; names spoofed in the UI, `KEKW` + U+200B dodged `name_taken`, and NUL in a reason became an `internal` error | `catalog.rs` `set_name`/`member_name`, `moderation.rs` `reason`, `abuse.rs` `text`, `admin.rs` | Fixed, #41: one check in `crates/shared/src/text.rs`; refused on write (reasons allow `\n`), stripped from platform display names at sign-in. Test `sec_text.rs` |
| M2 | The GraphQL socket ran any query, outside the per-IP and per-account GraphQL limits | `apps/api/src/lib.rs` `/v1/gql/ws` | Fixed, #43: the socket runs subscriptions only (`subscriptions_only`). Test `sec_ws.rs` |
| M3 | No cap on subscriptions per GraphQL socket | `apps/api/src/gql.rs` | Fixed, #43: 8 per connection (`too_many_subscriptions`) |
| M4 | `/v1/events` `resume` could be repeated to grow topics past 500 and ran a 5000-row replay before the too-old check | `apps/events/src/lib.rs` | Fixed, #43: one successful resume per connection, `too_many_topics` over the cap, too-old checked first. Test `apps/events/tests/sec_resume.rs` |
| M5 | Unreviewed emote names went out on public event topics (`set.emote_added`, `set.emote_renamed`) | `catalog.rs` `publish_member_event`, `moderation.rs` | Fixed, #42: events carrying a name go out only for approved emotes |
| M6 | The dHash blocklist hashed only the first frame: a benign first frame hid a blocked image in frame 2 | `apps/api/src/upload.rs`, `crates/media/src/lib.rs` | Fixed, #39: every frame is hashed and checked in one query. Test `sec_media.rs` |
| M7 | A takedown while transcoding was queued or running left public CDN objects that were never purged | `apps/worker/src/purge.rs`, `transcode.rs` | Fixed, #39: purge deletes every possible variant name; transcode re-checks the blocklist after each write and cleans up. Test `apps/worker/tests/sec_purge.rs` |
| M8 | No security headers on the website: it could be framed by any site | `apps/web` (no `hooks.server.ts`, no `kit.csp`) | Fixed, #38 and `sec/web-csp-bidi`: `X-Frame-Options: DENY`, `frame-ancestors 'none'`, `nosniff`, `Referrer-Policy`, `Permissions-Policy`, HSTS on https; a script CSP via `kit.csp` (nonces, hashes on prerendered pages). Owner: the Cloudflare rules for prerendered pages (`docs/setup/owner-steps.md`) |
| M9 | Database backups went to R2 unencrypted | infra `backups/backup.sh` | PR infra #5 (owner merges): `age` encryption when `CYEXTV_BACKUP_AGE_RECIPIENT` is set |
| M10 | Aliases multiply expensive resolvers: complexity counts each field as 1, so one POST can run ~300 searches | `gql.rs` `limit_complexity(1_000)` | Open: weight list fields by `perPage`, lower the limit, cap root fields per operation |
| M11 | dHash ignores alpha and is not flip, invert or crop robust; a flat image hashes to 0 | `crates/media/src/lib.rs` `dhash` | Open: composite on a fixed background, hash flipped variants, refuse degenerate hashes. Changes stored hashes, so needs a migration plan |
| M12 | GIF sub-frames larger than the canvas are fully decoded; the pixel budget counts canvas pixels | `crates/media/src/lib.rs` `decode_frames` | Open: refuse frame descriptors larger than the canvas, cooperative deadline in the frame callback |
| M13 | The worker's 120 s timeout can't stop a blocking transcode, so retries stack up | `apps/worker/src/lib.rs`, `transcode.rs` | Open: deadline inside the encode, permanent failure after 1-2 timeouts, a semaphore on encodes, container memory limit |
| M14 | The owner can clear an NSFW flag a moderator set | `moderation.rs` `update` | Open: store who set the flag (`nsfw_locked`) |

## Low

| # | Finding | Status |
|---|---|---|
| L1 | `unassignRole` ignored the target's seniority (`admin.rs`) | Fixed, #40 |
| L2 | The sign-in `back` check let `//host` and `/\host` through (the API refused them, so an error page, not a redirect) (`apps/web/src/lib/api.ts`) | Fixed, #38 (`safeBack`) |
| L3 | Extension: page-controlled channel and user ids went into provider URLs (`..` survives `encodeURIComponent`) | Fixed, extension #10 (numeric ids only) |
| L4 | A cross-site GET to `/v1/auth/<platform>/login` switches a signed-in user to whatever account their already-authorized identity maps to (nuisance; never into the attacker's account) | Open: refuse or confirm when a cookie session exists and `link` is false |
| L5 | `CF-Connecting-IP` is trusted from any peer when `CYEX_TV_TRUST_CLOUDFLARE` is on | Safe as deployed (ports bound to 127.0.0.1); open as hardening: honour it only from private peers |
| L6 | Report limit (10/h) is check-then-insert and can be raced | Open: advisory lock per account |
| L7 | Upload limit (20/h per account) can be raced the same way; failed uploads aren't counted | Open |
| L8 | Blocked accounts' channels, personal sets and emotes are still served (`reads.rs`, `sets.rs`, `search.rs`) | Open: filter `blocked_at` |
| L9 | Reviewers can approve their own emotes | Open |
| L10 | Unbounded `page` offsets; no cap on sets per account or editors per set | Open |
| L11 | Introspection and GraphiQL on in production (by design, `gql.rs`) | Accepted |
| L12 | The original is stored before the transaction commits and orphaned on rollback | Open |
| L13 | Frame delays: huge delays saturate timestamps and fail every transcode attempt; 20-99 ms strobing is allowed | Open: clamp delays and total length |
| L14 | Upload form text fields are read whole before the length check; repeated `file` fields accepted | Open (bounded by the upload body limit: the largest role ceiling plus 64 KB, ADR 0031) |
| L15 | Re-uploading bytes whose transcode failed reuses the failed file and stays pending forever | Open |
| L16 | Takedown blocks the sha256 and the first-frame dHash only; a re-encode with a new first frame passes unless another frame matches | Open (follow-up to M6) |
| L17 | Containers lack `read_only`, `cap_drop` and `no-new-privileges`; images use floating tags | Open (infra) |
| L18 | Compose builds `web` with the wrong context (`apps/web` vs the Dockerfile's repo-root paths): a deploy failure, not a vulnerability | Open (infra) |
| L19 | Extension: emote and badge image hosts aren't allowlisted (viewer IP leak to a hostile host) | Open |

## Checked and found sound
- OAuth state: 256-bit, hashed, single use, 10-minute expiry, bound to a host-only cookie and the platform; PKCE S256 for Kick; fixed `redirect_uri`; no redirects followed to providers.
- `return_to`: exact origin compare; `//`, backslashes, control characters, userinfo, other schemes and ports refused.
- Account linking: link target stored server-side; an identity on another account answers `identity_taken`; identities keyed on numeric platform ids, never logins.
- Sessions: 256-bit tokens stored as sha256; new token on every sign-in; logout deletes the row; blocking deletes all sessions; cookie `HttpOnly; Secure; SameSite=Lax`.
- CSRF: every non-GET cookie request needs `Origin: https://cyex.tv`, checked before the body is read, for every content type. The GraphQL socket never attaches a caller, so cross-site socket hijacking has nothing to use.
- CORS: credentials only for the exact site origin; no reflection, `null` or suffix matching.
- Authorization: set edit and owner grants re-read on every call; editors can't manage editors or delete sets; channel settings owner-only; role rules (`may_grant`, `may_manage`, owner protection, last manager) under an advisory lock.
- Visibility: pending, rejected, private and deleted emotes hidden from the public on every read path; no emails or tokens in the schema; provider ids only on `me`.
- GraphQL: array batching refused; depth 12; DB errors become `internal` with details only in logs. SQL is all bound parameters; LIKE escaping is correct.
- Uploads: body limit before buffering; sniffed formats only (PNG, GIF, WebP; no SVG); header dimensions and `max_alloc` checked before decoding; Content-Type from the sniffed format; originals never public; object keys are server-side hashes only.
- Website: no `{@html}` with API data (the redesign's `Icon.svelte` reads a constant map); API URLs only in `<img src>`; `/dev/components` 404s in production; the mock API isn't in the image.
- Extension: `storage` and `activeTab` only; host permissions limited to the provider APIs; `onMessage` checks the sender; fetcher https-only on an allowlist; names inserted as text; paint CSS through CSSOM with `url()` limited to the 7TV CDN; no `postMessage`, `eval` or remote code.
- Infra: published ports on 127.0.0.1 only; Postgres and metrics on the private network; tunnel routes three hostnames with a 404 catch-all; no secrets in the repo; services run as non-root users.

## For the owner
- Merge infra #5 and set `CYEXTV_BACKUP_AGE_RECIPIENT` on the host (keep the private key off it).
- After the redesign merges: add `kit.csp` (script hashes for the inline theme script, `connect-src` api and wss, `img-src` cdn.cyex.tv plus the Twitch and Kick avatar CDNs), and set the same headers at Cloudflare for prerendered pages.
- UI-level (website, not edited here because of the redesign): render user-chosen names (emote names, aliases, display names, set names, reasons) in an isolated bidi context (`<bdi>` or `unicode-bidi: isolate; dir="auto"`), so any old rows or platform display names can't reorder the text around them.
- Medium items M10 to M14 and the lows are open; M11 changes stored hashes and needs a decision.
