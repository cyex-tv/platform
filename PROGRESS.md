# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Adjustable limits and the pending-emote cap (branch `feat/role-limits`, owner request of 2026/11/20)

- [x] Every rate in `limits::Rates` and the upload counts can be set at start: `CYEX_TV_LIMIT_<NAME>=<count>/<window>` (`600/1m`, `30/1h`, `5/30s`, `1/1d`), whole numbers for `CYEX_TV_LIMIT_UPLOADS_PER_HOUR` and `CYEX_TV_LIMIT_PENDING_EMOTES`. Unset keeps today's value; a bad value stops the api naming the variable. Listed in `.env.example`.
- [x] Per-role limits (migration `20261211000001_role_limits`, `roles.limits` jsonb, additive): 15 per-account limits (`limits::AccountLimit`). An account counts under the most generous override among its assigned roles, else the default; signed out and plain users get the defaults. Kept per account for 30 s (`Limits::load`, run by `MaybeCaller`), dropped on assign, unassign, role delete and limit changes, so the checks never wait on the database. The limiter takes a per-key rate (`Limiter::check_with`).
- [x] GraphQL (additive): `limitDefaults`, `roleLimits(id)`, `setRoleLimits(id, limits)` (needs `roles.manage`, same edit rules as `updateRole`, not the User or top role, audited `role.limits_changed`), `uploadQuota(ownerId)`.
- [x] Pending cap: an upload is refused with `409 too_many_pending` when the owner (the account uploaded for, with upload-as) already has its cap (20) waiting for review, transcoding ones included; checked again under the upload lock, so parallel uploads can't both take the last place.
- [x] Website: Limits section in the role editor (Requests, Uploads, Account actions; defaults as placeholders; tooltips); the upload form shows "n of 20 waiting for review" for the account picked and, at the cap, says why Upload is off. Dev mock answers the new operations (`MOCK_PENDING_MAX` lowers the cap).
- [x] Tests: unit tests in `ratelimit.rs`, `limits.rs`, `admin.rs`, `upload.rs`; `apps/api/tests/role_limits_flow.rs` (Postgres in CI); vitest `role-limits.test.ts`, `role-limits.ui.test.ts`, `quota.test.ts`, `upload-quota.ui.test.ts`.
- [x] Manual (mock, 1440 dark): Limits section on the Moderator role saves `setRoleLimits`; the upload page shows 2 of 3, and after one upload the cap message with Upload off.

## App sign-in for cyex deck (branch `feat/app-sign-in`, ADR 0021, owner decision of 2026/11/19, D19 changed)

Handoff `2026-11-19-03-deck-to-cyextv-app-sign-in` from the deck chat.
- [x] Flow: authorization code + PKCE S256 with a loopback redirect (RFC 8252, any port on `127.0.0.1` or `[::1]`, path `/callback`). `GET /v1/auth/app/authorize` checks the client and redirect (400, never redirected to, when they fail) and sends the browser to the site's `/authorize` consent page; `POST /v1/auth/app/consent` (site cookie and Origin only) answers where to go (code and state, or `access_denied`); `POST /v1/auth/app/token` (form; `authorization_code` or `refresh_token`); `POST /v1/auth/app/revoke` (RFC 7009); `GET /v1/auth/app/clients/{id}`.
- [x] Public client `cyex-deck` in `crates/auth/src/app.rs` (no secret). Scopes `sets.emotes` (`addSetEmote`, `updateSetEmote`, `removeSetEmote`) and `cosmetics.select` (`selectCosmetics`); queries allow-listed (`app::APP_QUERIES`: public reads, own reads, `channelEditableSets`; no staff reads), every other query and mutation `insufficient_scope` (checked per resolved field), uploads refused. Staff grants and the owner's never count for an app token (`permissions_of` gives it only the User role's), so `sets.edit_any` doesn't open other sets to the deck.
- [x] Lifetimes: code 2 minutes, single use (a replay signs out what it gave); access 1 hour, not slid; refresh 60 days, rotated, reuse signs the session out. One `sessions` row per sign-in (migration `20261209005001`: `app_client_id`, `access_expires_at`, `app_auth_codes`, `app_refresh_tokens`), listed once in `mySessions` as the app (`appClientId`), revocable there; refresh tokens go with the session.
- [x] GraphQL (additive): `channelEditableSets(platform, id | login)` with `role`, `canEditActiveSet` and each set's `access` (`OWNER`, `ACCOUNT_EDITOR`, `SET_EDITOR`, `STAFF`), built on the same rights check as `addSetEmote`. `AccountSession.appClientId`.
- [x] Website: `/authorize` consent page (cyex app layer: one card, the app, what it may do, the account with Not you?, Deny and Allow; sign-in buttons when signed out; one error state for bad links). Dev mock: the whole flow. Docs: App sign-in guide on /developers, the five endpoints in the reference.
- [x] Tests: `apps/api/tests/app_sign_in_flow.rs` (Postgres in CI), unit tests in `crates/auth/src/app.rs`, vitest `app-auth.test.ts`.
- [x] Manual (CDP, `.claude/runs/2026-11-20-overnight/screens/app-sign-in/shots.mjs`, 375 and 1440, dark and light): consent, sign-in and invalid states; Allow and Deny go back to the loopback address.

## Batch K1: link previews, keyboard shortcuts, changelog, tile marks (branch `feat/k1-previews-shortcuts`, 2026/11/20)

From the 7TV gap study (`.claude/runs/2026-11-20-overnight/gap-7tv-2.md`), ideas only, in cyex design. Web only; no API change.
- [x] Link previews: `lib/meta.ts` builds Open Graph and Twitter tags for emote (4x still frame, kind, creator, channel count), set (its first 4 emote stills in set order, owner, slots, first names), profile (avatar, platforms, emotes made, channels) and channel pages (avatar, platform, active set and size), in each page's load, so they are in the server's HTML; canonical URL from `PUBLIC_SITE_URL`; `noindex` for unlisted, private, pending and missing. Every other page gets the site card `static/og.png` (1200 × 630, the mark and wordmark on the page ground) and keeps its own title and description. The root layout renders them with `Meta.svelte`.
- [x] Keyboard shortcuts: `?` (outside fields, without modifiers) or Keyboard shortcuts in the footer opens a modal sheet (`ShortcutSheet.svelte`); pages register their keys beside their handlers (`registerShortcuts`, `shell/shortcuts.svelte.ts`): Emotes and Select mode on /search, the review queue and its open upload, the emote page's tabs, /developers' filter, /help's search, plus the site-wide keys and the header search. Not shown on touch-only screens.
- [x] Changelog: `/changelog` on the docs layout, prerendered from `lib/changelog/entries.ts` (dates as 2026/11/20, entries of this week); "What's new" in the account menu and the phone menu, with a dot (and on the avatar) until this device has opened the newest entry (`cyex.changelog.seen` in localStorage); a Changelog link in the footer.
- [x] Tiles: "In set" (accent tick and word) on browse tiles already in the target set, following Add and Remove; your own unlisted and private emotes show an Unlisted or Private mark with the emote page's wording, wherever they appear to you.
- [x] Tests: vitest `meta.test.ts`, `shortcuts.test.ts`, `changelog/entries.test.ts`, `changelog/seen.test.ts`, `tile-marks.test.ts`, loaders now check each page's card; `dev/e2e/link-previews.mjs` fetches the SSR HTML of each page type and checks the tags (not in CI). CDP: `.claude/runs/2026-11-20-overnight/screens/k1/k1.mjs` (sheet open, focus, Escape, focus return, fields, groups per page; dot and seen; marks; 375, 1440, 2560, dark and light; reduced motion).

### GitHub write log for this PR

```
git push -u origin feat/k1-previews-shortcuts
gh pr create --base main --head feat/k1-previews-shortcuts --title "Link previews, keyboard shortcut sheet, changelog, tile marks" --body-file <file>
```

## Security fixes S2 (branch `fix/security-s2`, sign-off review #2 of 2026/11/20)

- [x] Search: `listed_emote!` also hides emotes of an owner blocked right now (`blocked_at` set, no end or an end ahead); an ended block lists them again. Test: `every_search_uses_the_listed_emote_rule` (blocked and expired-block owners).
- [x] Read cache across instances: renaming a set, changing its visibility and deleting it publish the internal `cache.forget` event (topic `internal:cache`, which no client can watch) naming the set's read, the channels showing it and, for the global set, its read; every feeder's tap drops them, and the instance that made the change drops them at once. Test: `set_privacy_rename_and_delete_drop_cached_reads_everywhere` (two instances on one database).
- [x] Announcement: create, update and end publish `cache.forget` for `announcement`, so every instance drops its cached line; the cache has a generation that `clear` bumps, so a read that started before a change can't store the old line. Tests: `announcement_changes_reach_every_instance`, `a_read_from_before_a_clear_is_not_stored`.
- [x] Data export: `preferences` (notification kinds off, pop-ups off, collection shown). Test: `account_hub_flow` export.

### GitHub write log for this PR

```
git push -u origin fix/security-s2
gh pr create --base main --head fix/security-s2 --title "Security fixes S2: blocked owners out of search, cache forgets on every instance, export preferences" --body-file <file>
gh pr merge <n> --merge
```

## Security fixes S1 (branch `fix/security-s1`, ADR 0004, 0010, 0014, 0019, 0020 amendments of 2026/11/20)

From the 2026/11/20 security review (findings A4). Each with a test that fails before and passes after:
- [x] HIGH `quickSearch` showed unlisted emotes and those of accounts pending deletion: one `search::LISTED_EMOTE` fragment for `search` (every sort), `quick` and `trending_rank`. Test `every_search_uses_the_listed_emote_rule` (crates/db/tests/queries.rs: public, unlisted, private, NSFW and a pending-deletion owner through all three).
- [x] HIGH `editors.manage_any` reached the owner's account: staff now get `owner_protected` on the owners' accounts and `not_below_you` on accounts at or above their rank (reads too). Test `staff_editor_management_respects_the_owner_and_seniority` (sec_roles.rs).
- [x] MED the built-in User role could be given a staff permission by any `roles.manage` holder: adding one needs the owner (`owner_only`) unless it is on `USER_ROLE_ALLOWED` (empty). Test `only_the_owner_adds_staff_permissions_to_the_user_role` (builtin_roles_flow.rs).
- [x] MED accepting a transfer re-checks the asking editor's `emote_transfer`; without it the request closes (`reason: editor_permission`). Test `accepting_rechecks_the_editors_transfer_permission` (editor_emotes_flow.rs).
- [x] MED erasure deletes staff notes, blanks restriction reasons and strips `body`, `reason`, `note` from audit rows about the account; `identity_holds` retention documented (ADR 0019, `privacy::erase`). Test `erasure_scrubs_notes_reasons_and_audit_text` (queries.rs).
- [x] LOW upload hourly limit re-checked under a per-account advisory lock (`emotes::lock_upload_quota`); text form fields read chunk by chunk with a cap. Tests `concurrent_uploads_respect_the_hourly_limit`, `over_long_text_fields_are_refused` (upload_flow.rs).
- [x] LOW nobody changes their own holds (`own_account`). Test in `restricting_and_blocking_accounts` (abuse_flow.rs).
- [x] LOW app tokens: `exportMyData` and `revokeOtherSessions` are website only (`website_only`); `signOutEverywhere` with an app token ends only that session. Tests in account_hub_flow.rs.
- [x] LOW activity: an unlisted emote's own feed shows its lines; emotes of pending-deletion owners leave every feed. Test `unlisted_and_leaving_emotes_in_feeds` (emote_activity_flow.rs).
- [x] LOW tags: NFC on both sides, combining marks allowed (`text::normalize_tag`, `text::is_tag_char`; web `rules.ts`). Unit tests in shared text.rs, moderation.rs and rules.test.ts.
- [x] LOW transfer answers are `not_found` to anyone but the two parties. Test in `editor_transfers_wait_for_the_owner_and_go_only_to_trusting_accounts`.
- [x] Code review: erase also clears `emotes.uploaded_by`, deletes `oauth_states` and closes open transfer requests the account is a party to. Tests `every_reference_to_an_account_is_handled_by_erase` (every FK to accounts from information_schema, handled or kept with a reason) and `erasure_clears_uploader_and_closes_its_transfer_requests` (queries.rs).
- [x] Code review: `acceptEmoteTransfer` locks the emote before the request, as `transferEmote` does (no deadlock). Test `accept_and_owner_transfer_at_once_dont_deadlock` (editor_emotes_flow.rs).
- [x] Dev mock: `quickSearch` public only; transfers `not_found` to non-parties and the editor re-check on accept (the seeded requester holds `EMOTE_TRANSFER`).

### GitHub write log for this PR

```
git push -u origin fix/security-s1
gh pr create --base main --head fix/security-s1 --title "Security fixes S1: search visibility, staff editor rank, User role, transfers, erasure, app tokens" --body-file <file>
git push
```

## Scale: hot reads at 100k emotes, 50k accounts, 20k channels (branch `perf/scale-1`)

Owner: "make sure everything is already laid out for a higher scale deployment like thousands of emotes and users". Audit with file:line and the proposals: `.claude/runs/2026-11-19-feedback/scale-audit.md`.
- [x] Migrations `20261208000901`..`904` (additive, `CREATE INDEX CONCURRENTLY`, one per file, `-- no-transaction`): set members by emote and newest add (channel counts index-only, `emoteChannels` in order), set members by set and newest add (card previews), searchable emotes by TOP and by NEW order.
- [x] `search`: no `count(*) OVER ()` outside TRENDING (counted apart, skipped on a short page), name candidates through the trigram and tag indexes, the tag check skipped when no tag is asked, empty-text RELEVANCE in index order. Same answers (tested against the old query); `quickSearch` now leaves unlisted emotes out, as documented.
- [x] `newest_members` reads each set's newest members from the index (LATERAL ... LIMIT) instead of numbering all of them; `emote_activity_rows` reads its two kinds of row from their own indexes, a page each; outbox pruning deletes in batches of 10 000.
- [x] API: `read_cache` shares the chat-client reads (`/v1/sets/{id}`, `/v1/sets/global`, `/v1/channels/{platform}/{id}`, `/v1/emotes/{id}`, `/v1/cosmetics`) per instance: one build for everyone asking at once, kept up to `CYEX_TV_READ_CACHE_MS` (default 5000, 0 off), dropped when the instance's feeder sees an event about it (`Hub::tap`, additive). Answers, headers and status codes unchanged.
- [x] Website: a public set draws 120 emotes at a time with Load more.
- [x] Extension (cyex-tv/extension PR 33, merged): personal emote lookups ask for every queued chatter, 100 a request (a busy chat lost everyone past 100 for 10 minutes).
- [x] Tests: `crates/db/tests/scale.rs` (seeded volumes; Postgres' own counters show the browse, search, preview, emote channels and emote activity reads use their indexes; the rewritten reads answer exactly what the old ones did; batched pruning); `read_cache` unit tests; `catalog_flow::cached_reads_follow_events` with a live feeder. CI's `load` job times each read at full scale, old against new (`scale -- --ignored --nocapture`).
### GitHub write log for this PR

```
git push -u origin perf/scale-1
gh pr create --base main --head perf/scale-1 --title "Scale: hot reads, indexes and a shared chat-read cache" --body-file <file>
gh pr create --repo cyex-tv/extension --base main --head fix/personal-lookup-batches --title "Personal emote lookups: every queued chatter, 100 a request" --body-file <file>
gh pr merge 33 --repo cyex-tv/extension --merge
## API coverage suite (branch `test/api-coverage`)

Goal: every endpoint exists and works as the website calls it. Tests only, in one new test binary `apps/api/tests/api_coverage/` (Postgres in CI).
- [x] `contract`: reads every GraphQL operation from `apps/web/src/lib/queries.ts` and `paint-studio/queries.ts` (with the fragments each uses), and runs all of them through the real router as the site sends them (cookie and `Origin`) with seeded data: an operation or field the API doesn't have fails, and so does any error except the two listed (checkout and refund need Paddle). An operation added to the site and not run by the test fails it. Also the site's REST calls: emote upload, cosmetic picture upload (and the same bytes again), the Twitch and Kick sign-in redirects, sign-out.
- [x] `authz`: every root field from introspection, called signed out (all but the public reads answer `signed_out`) and, for staff fields, by an account without roles (`forbidden`).
- [x] `live`: one `addSetEmote` reaches both a `/v1/gql/ws` subscriber and a `/v1/events` client (events service as a dev-dependency), each fed from the same outbox.
- [x] `validation`: input rules no other test sent a bad value to (badge, paint, shop item, code batch, grant reason, lookups over 100, review search, reject reason, note body, tags, restriction end, editor sets, role name), and nothing changed.
- [x] Inventory with before and after counts: `.claude/runs/2026-11-20-overnight/api-coverage.md`.

### GitHub write log for this PR

```
git push -u origin test/api-coverage
gh pr create --base main --head test/api-coverage --title "Tests: API coverage suite (website contract, auth sweep, both sockets, input rules)" --body-file <file>
```

## Cosmetics for real: badge upload and studio paints (branch `fix/cosmetics-real`)

Owner (round K): "uploading a badge doesnt even work and it doesn't even make a request to the api" and "paints are still broken". Reproduced in headless Edge on main (ab834a6) against the dev mock, through the UI.
- [x] Badge upload. Causes found: "Make badge" pressed while the picture was still uploading or processing refused locally ("Add a picture and wait until it is ready") and sent no createBadge; picking the same file again after a failure sent nothing (the file input kept its value); the mock swapped every uploaded picture for the sample star, so the new badge never showed what was uploaded. Now: the dialog waits for a picture on its way and then saves; a fresh picture field per open; the input is cleared after each pick; drop and paste work; type, 5 MB and the API's size rules (square, 72 px; paints 64 px tall) are checked before sending and say why; the wait is 2 minutes and stops at once on signed out, forbidden or bad_origin with that message (`lib/cosmetics/upload.ts`).
- [x] Studio uploads (from the API-contract audit): `POST /v1/cosmetics/images` answers in the REST shape, and bytes the API already has come back ready at once; the studio took that as the GraphQL picture and picked the wrong file. It now always reads the picture back through `cosmeticImage`. Same-file re-pick fixed in the studio's file inputs too.
- [x] Paints. The website drew every kind, but in the mock the studio's render never reached it: the mock served the sample star for every paint picture. The mock now keeps the uploaded bytes and serves them at `/cosmetics/uploads/<id>/<n>x[_static].webp` (no transcoding; the same file for every size), answers uploads in the real REST shape, gives the same picture for the same bytes, and checks the API's sizes. One renderer: the studio's previews and CSS export now call the site's `paintStyle` (with the local picture as `imageUrl`), and the studio preview shows the site's readability floor on each ground. CSP: `worker-src 'self' blob:` (the studio's render worker was blocked in dev and fell back to the page).
- [x] Profile Cosmetics: the grid / list switch is gone. Badge and paint admin pages read their permissions reactively.
- [x] Extension (cyex-tv/extension PR 34): picture paints get the same flat under-layer, so a failed or see-through picture leaves the chat name readable.
- [x] Tests: `lib/cosmetics/upload.test.ts` (checks, the wait, the studio's read-back against the REST answer), `lib/cosmetics/paint.test.ts` (every paint kind: drawn, and the studio draws exactly what the site draws; local previews; floor per ground). Browser: `apps/web/dev/e2e/cosmetics.mjs` (27 checks, real UI, both themes; on main 11 of them fail).

### GitHub write log for this PR

```
git push -u origin fix/cosmetics-real
gh pr create --base main --head fix/cosmetics-real --title "Cosmetics: badge upload and studio paints work end to end" --body-file <file>
```

## Emote transfers: the owner must accept (branch `fix/transfer-accept`, ADR 0014 amendment)

Owner decision after #100: an editor's transfer waits for the owner.
- [x] API (additive): an editor's `transferEmote` makes a pending request (migration `20261207000081`, one open per emote, 7 days); `emoteTransferRequests`, `acceptEmoteTransfer` (rules checked again), `declineEmoteTransfer`, `cancelEmoteTransfer`. The owner's own transfer stays immediate and closes any open request. Every step audited and notified.
- [x] Website: the emote's edit page lists open requests (Accept and Decline for the owner, Cancel for the editor) and says the owner has to accept; the `emote.transfer_requested` notification has Accept and Decline; sentences for every transfer notification. Dev mock: one seeded request on the mock account.
- [x] Tests: `editor_emotes_flow.rs` (request, then the owner accepts and it moves; decline, cancel and expiry leave it; neither the editor nor the receiver can accept; holds refuse at accept time; one open request; the owner's immediate transfer closes the open one); vitest `account.test.ts`.

### GitHub write log for this PR

```
git push -u origin fix/transfer-accept
gh pr create --base main --head fix/transfer-accept --title "Emote transfers: the owner accepts an editor's request" --body-file <file>
```

## Account editors' emote permissions (branch `feat/editor-emote-perms`, ADR 0014 amendment of 2026/11/19)

The owner asked for `EMOTE_UPLOAD`, `EMOTE_MANAGE` and `EMOTE_TRANSFER` to work (billing flags stay stored only).
- [x] API (additive): `owner_id` on `POST /v1/emotes` (upload as an account you edit; `emotes.uploaded_by`, migration `20261207000080`); `updateEmote`, `deleteEmote`, `emote` and `accountEmotes` for editors with `EMOTE_MANAGE`; `transferEmote(id, toAccountId)`. All three need a grant on every set; an uploads hold on the editor or the owner refuses them; uploads count against both (20 an hour each side). Audited with the editor as actor, `"via": "account_editor"`, subject the owner; the owner's feed shows an editor's upload "by" them.
- [x] Transfer decision: no accept step. The receiver must be the caller or an account where the caller holds `EMOTE_UPLOAD` on every set (it already trusts them with uploads); the owner follows the same rule; both owners are notified; 60 an hour.
- [x] Website: Upload as picker in the upload form and dialog (you by default, `?for=` preselects); Edit on `/emotes/[id]` and a working edit page (details, delete, Give to another account) for managers; the managed account's Uploaded emotes in every state on its profile; "You edit" rows link Emotes and Upload and say when a set limit switches the emote permissions off; the permissions dialog drops "Saved now; used once…" for Emotes and notes that they need All sets.
- [x] Tests: `apps/api/tests/editor_emotes_flow.rs` (Postgres in CI: allow, no flag, wrong account, set-limited, restricted editor and owner, audit rows, the feed line, both upload limits, transfer receivers and notifications); vitest `editors.test.ts`, `api.test.ts`. Dev mock: OTHERS[0] grants upload, manage and transfer.
- [x] PR 100; CI's test job ran `editor_emotes_flow` against Postgres (4 passed).
- [x] Manual (CDP, `screens/editor-emotes/check.mjs`, 1440 and 375): Edit shows on a managed account's emote and not on another's; the edit page has Details, Give and Delete; Upload as lists you and the managed account, `?for=` picks it; You edit shows Emotes and Upload; the dialog has no "Saved now" note.

### GitHub write log for this PR

```
git push -u origin feat/editor-emote-perms
gh pr create --base main --head feat/editor-emote-perms --title "Account editors: emote upload, manage and transfer" --body-file <file>
gh pr merge 100 --merge
```

## Final pass of the 2026/11/19 feedback round (branch `fix/final-pass`)

Known fixes, each with a test that fails before and passes after:
- [x] Toasts stack above bottom bars: the /search select bar and the emote page's phone action bar register with `use:bottomBar` (`lib/tool/bottom-bars.svelte.ts`); the toast stacks sit above whichever reaches into the bottom of the screen. Vitest `bottom-bars.test.ts`; CDP `screens/final/toast-lift.mjs` (1440 and 375 select bar, 375 emote bar: failed before, passes now).
- [x] Sideways scroll at 375 on /admin/cosmetics/owners: not reproducible on main (the empty toast stacks fit the viewport). The sweep checks every route at 320, 375, 768, 1440 and 2560 signed out, signed in and as admin: no page scrolls sideways.
- [x] The stray "animated" label on a tile: the hover/focus type mark drawn over the picture in `EmoteTile`; removed (the marks stay in the link's accessible name). The unused `EmoteCard` is gone. Vitest `emote-tile.test.ts`.
- [x] The emote edit page shows the plain size row; the Dark / Light / Checker / Chat switch (`PreviewStage`) is deleted. Vitest `no-preview-grounds.test.ts`.
- [x] Profile band facts read with real spaces ("Twitch: lumen_fox (opens in a new tab)", "Main on lumen_fox (Twitch)", "Member since 2026/11/10"); they looked right but read run together. Vitest `header-facts.test.ts`.
- [x] Sessions: Last seen as `2026/11/17 23:40`, relative on hover. CDP `screens/final/fixes.mjs`.
- [x] One TagInput (there was only one component), now with one cleaning rule: /search used its own (spaces to `_`, only a-z, 0-9, `_`), so a tag like `pepe-the-frog` or `猫ダンス` could never be searched for. The browse filters and the URL use `lib/rules` `cleanTag`, and the API's search accepts every tag an upload may carry (1 to 32 letters, digits, `-`, `_`; additive, it only accepts more). Vitest `tag-input.single.test.ts`; Rust unit test in `crates/db/src/search.rs`.

Harmony sweep (`screens/final/sweep.mjs`, contact sheets in `screens/final/sheets/`): every route at 375 and 1440, dark and light, plus 2560 dark for the main pages. Fixed: date and time on the admin account panels, holders, reports and shop orders; roles positions in cyex numbers; one button size in the profile band; the channel page's count line; the cosmetics chat sample's colon; the help answer on cost (now mentions the shop and codes); a Svelte dev warning in the admin names lookup. Checked in `screens/final/harmony-fixes.mjs`. Infra: cyex-tv/infra #8 passes PUBLIC_API_URL and PUBLIC_CDN_URL to the web image build.

### GitHub write log for this PR

```
git push -u origin fix/final-pass
gh pr create --base main --head fix/final-pass --title "Final pass: toasts, tiles, edit page, profile facts, sessions, tags, harmony" --body-file <file>
gh pr merge 99 --merge
```

## Admin: review queue, owners lookup, studio library previews (branch `feat/admin-queue`, owner feedback 2026/11/19 section J)
- Review queue (/admin/review): tabs Public / Unlisted and private / Resolved with counts, a search by emote name or uploader (login or display name), sort (upload date, review date on Resolved, uploader, name) and order, 50 dense cards a page from the server (thousands pending cost the same as fifty; cards use `content-visibility`). A card: the moving picture, name, uploader, flags (visibility, NSFW, zero-width, wide), tag count, upload date and time, and its verdicts: approve, approve as unlisted, approve for its owner only, reject with a reason (ready reasons one click away). The detail dialog: "n / total" with previous and next across pages, kind, name, uploader, visibility, state, flags, tags, ID, the picture at 1x-4x with dimensions and file size, Activity (the emote's audit trail, with date and time, plus its upload) and Notes (reviewers' notes). Keyboard: arrows move, Enter opens, A approves, U unlisted, P owner only, R rejects, Esc closes; after a verdict the next upload is up (in the grid and in the dialog). Merge into an existing emote is left out: the API has no emote merge.
- API (additive): `reviewQueue(filter: ReviewQueueFilter)` (tab, search, sort, descending; no filter = the old behaviour) and `ReviewQueue.counts`; `approveEmote(visibility:)` sets public, unlisted or private in the same step (audited with the visibility, told to the owner); `emoteNotes`, `addEmoteNote`, `deleteEmoteNote` (`emotes.review`, audited `emote.note_added` / `emote.note_deleted`); `emoteHistory` (the emote's audit entries, `emotes.review`). Migration `20261207000075_emote_notes.sql` (table `emote_notes`, index for the resolved tab).
- Owners (/admin/cosmetics/owners) is now a lookup: pick or search a badge, paint or role on the left; the right shows its holders (`entitlementHolders`) held now or ever, with how they got it, who gave it and when, and when and by whom it ended; search, sortable columns, 25 a page, Export CSV (spreadsheet formulas defused), Revoke on held badges and paints. An account field opens the entitlement graph (everything it owns); each row links there too. Shared: `lib/admin/HoldersTable.svelte`, labels and sorting in `lib/admin/graph.ts` (also used by the graph).
- Paint studio: the add-a-layer and add-an-effect dialog has a sidebar of the studio's categories (All, then each with its count) and every option shows a preview: the option painted on a sample name (an effect over a sample gradient), drawn a few at a time and kept for the session; hovering or focusing a card plays it (not under reduced motion).
- Shared: `NoteList.svelte` (account and emote notes), `ReasonDialog` presets, icons `eye` and `refresh`; dev mock (`MOCK_PENDING` for a busy queue, filters, notes, history, holders for catalog items).
- Tests: Postgres `apps/api/tests/review_queue_flow.rs` (tabs, counts, search, sorts, paging, resolved, approve as unlisted/private, audit visibility, notes with author-only delete, history, permission and input errors); unit test for the filter defaults; vitest `queue.test.ts` (history lines, sizes, grid keys), `graph.test.ts` (holders filter, sort, CSV), `studio.test.ts` (every library option renders a preview).
- Manual check (2026/11/19): headless Edge over CDP against the dev mock with 3000 pending, at 1440, 2560 and 375, dark and light: queue, limited tab, detail, reject reasons, owners lookup, studio library (layers, materials, effects); keyboard flow in the queue (arrows, A on a card, Enter, arrows in the dialog, A in the dialog opens the next, R opens the reason, Esc twice). Screens in `.claude/runs/2026-11-19-feedback/screens/j-admin/`.

### GitHub write log for this PR

```
git push -u origin feat/admin-queue
gh pr create --base main --head feat/admin-queue --title "Admin: review queue, owners lookup, studio library previews" --body-file <file>
gh pr merge 98 --merge
```

## Emotes tab: no rail, one toolbar, tile menu, multi-select up to 100 (branch `feat/browse-select`)

Owner feedback 2026/11/19, section J (browse). Web only: no API change (bulk add already sends one `addSetEmote` per emote; 100 of them stay well inside the per-account GraphQL limit of 600 a minute, and there is no other cap to raise).

- [x] Layout: the filter rail is gone (Emotes and Users tabs both use `ColumnLayout`). One toolbar: sort Top / Trending (its Day / 7 days / 30 days next to it) / New (Relevance with a query), the search field, Filters in a popover (a sheet on phones; Type, Flags, Tags as before, with Reset), a Select switch, and the target set button (signed in only). One large grid, no size options.
- [x] Target set (`browse/target.svelte.ts`, `TargetSet.svelte`): your sets and sets you edit with EMOTES; starts as your channel's active set, a pick is remembered on the device.
- [x] Tile menu (`TileMenu.svelte`): right-click, the context-menu key or Shift+F10, or the ⋯ button (shows on keyboard focus and hover, always on touch). Add to / Remove from the target set, Open in new tab, Copy emote link, Select. The browser's own menu never shows on tiles. Arrow keys, Home, End; Escape returns focus to the tile.
- [x] Multi-select (`browse/selection.ts`, pure; `BrowseGrid.svelte`): click or Ctrl/⌘+click toggles, Shift+click a range from the last toggled tile, Space / Shift+Space on the focused tile, a drag rectangle (mouse or pen; from an empty spot it turns Select on) with auto-scroll near the window's edges, Ctrl/⌘+A picks what is shown, Escape clears (again: leaves Select). Touch: long-press turns Select on with that tile (in Select: a range). Cap 100: anything over it is refused with a note in the bar. Bar: "n / 100 selected", Add n to the target set (progress and a summary), Select all shown, Clear, Done. Picked tiles get an accent hairline and tick and a bold name (one CSS rule added to the shared `EmoteTile`).
- [x] Kept: URL state (`select=1` and every filter), the trending `#n` ranks, the empty, error and loading states.
- Tests: vitest `selection.test.ts` (toggle, range both ways, cap and refused counts, Select all, drag rectangle base plus hits, hit-test incl. gaps and touching edges, drag threshold, edge auto-scroll speed).
- Manual check (2026/11/19): headless Edge against the dev mock, `.claude/runs/2026-11-19-feedback/screens/j-browse/check.mjs` and `touch.mjs`: no rail; context menu opens with our items and stays inside the window at 375, 768, 1440 and 2560, dark and light; Add and Remove from the menu; Escape closes it and refocuses the tile; Select in the URL; click plus Ctrl+click (2), Shift range (7); Escape clears; drag rectangle picks the tiles it touches and the click after it doesn't navigate; Ctrl+A over 285 shown picks 100; one more click is refused with the note; Space toggles; bulk add summary; long-press on touch turns Select on without navigating, a tap then toggles; the context-menu key opens the menu with focus in it; Tab reaches the ⋯; no sideways scroll at any width; rank labels; empty state; Filters popover. Screens in `.claude/runs/2026-11-19-feedback/screens/j-browse/`.

### GitHub write log for this PR

```
git push -u origin feat/browse-select
gh pr create --base main --head feat/browse-select --title "Emotes tab: one toolbar, tile menu, multi-select up to 100" --body-file <file>
git merge origin/main   (PROGRESS.md resolved)
gh pr merge 97 --merge
```

## Emote page, round 3: one card, tabs with pagers, dated activity (branch `feat/emote-page-3`)

Owner feedback 2026/11/19, section H. Additive API; no migration; nothing was deployed.

- [x] One main card: the creator and upload date on top, the name, its flags (Animated, Zero-width, Wide, NSFW, Unlisted or Private, each with a tooltip) and tags (links to the tag search), the four sizes on one baseline with scale and pixel size, one line of facts (channels, added in 7 days, formats, file size at the biggest scale, review), and the actions together: Add to set (sets on your own channels first, marked "On your channel"), Edit (owner, or `emotes.edit_any`), More (Copy link, Copy name, Download, Report). Trending #n and Pause sit top right.
- [x] The preview background switch (Dark, Light, Checker, Chat) and the side panel are gone from the emote page; the review block's Report moved into More. (The edit page `/me/emotes/[id]` still uses `PreviewStage`, untouched.)
- [x] A second card with tabs Channels (count), Activity, More by the creator, and a pager in the tab bar ("1–24 of 82", previous, next). Channels page 24 at a time through `emoteChannels(offset:)`; Activity pages by cursor (pages kept, Previous needs no request).
- [x] Activity rows: icon, a plain sentence, who did it, and the date and time (`2026/11/19 14:05`, how long ago on hover). Notifications show the date and time the same way.
- [x] Phones: the card stacks, sizes wrap, the actions stay in the fixed bottom bar (Add to set, Edit, More); reduced motion starts on still frames.
- API: `emoteChannels(id, limit, offset)`, `offset` 0 to 10000 (past it: empty). Postgres test in `apps/api/tests/search_flow.rs`; dev mock and codegen updated.
- Tests: vitest `emote-page.test.ts` (flags, pager range), `pager.test.ts` (cursor trail).
- Checked in the running app (vite dev + mock, headless Edge): 375, 768, 1440, 2560, dark and light, signed out and signed in (Add to set, Edit, More, Download panel), keyboard (tab arrows), pager next page, reduced motion. No horizontal scroll. Screens in `.claude/runs/2026-11-19-feedback/screens/h/` (`before-after-1440-dark.png`).
- Left alone (agent D's files): the profile activity rows (dated by #95) and the sessions list (`apps/web/src/lib/account/SessionsPanel.svelte`, "Last seen" still relative, exact time on hover).

## Profile dashboard, activity filters, settings (branch `feat/profile-dashboard`)

Owner feedback 2026/11/19, sections D and I (profile parts). Additive API; one new migration; nothing was deployed.

- [x] Your profile's dashboard: Overview (`/users/<id>?s=overview`, first rail row for the owner; the account menu's name row opens it): tiles for Channels (and the set each shows), Notifications, Recent uploads with their state, Recent activity, Editors, Cosmetics (what you show, how many you own) and Emote sets, each linking to its tab. Each tile loads and fails on its own.
- [x] Activity tab: filter bar (Everything / Emotes / Sets / Cosmetics / Editors; By: anyone, the owner or one of its editors; the cyex logs timeframe All time / Day / Month with a "Jump to date" / "Jump to month" popover that refuses future dates). Filters live in the address (`?k=`, `?by=`, `?d=` / `?m=`). Rows show date and time (2026/11/19 14:05) and how long ago, everywhere (the emote page's Activity too). On an account's own feed its own lines drop "by <name>"; editor lines read "Added X as an editor", "Removed X as an editor", "X stopped being an editor".
- [x] API: `accountActivityPage(filter: ActivityFilter)` with `groups` (`EMOTES`, `SETS`, `COSMETICS`, `EDITORS`), `actorId`, `since`, `until` (RFC 3339, `bad_time`). New kinds `EDITOR_ADDED`, `EDITOR_REMOVED` and `ActivityEntry.editor` (names only, never permissions); editor lines show only when a filter asks for `EDITORS`, so the unfiltered feed is unchanged. Editor add/remove audit rows now name the owner as their subject (an editor's own change is marked `via: account_editor`); older editor rows don't show.
- [x] Display name: migration `20261207000052_display_platform.sql` (`accounts.display_platform`); `setDisplayPlatform(platform)` (`not_found` when not linked), `Me.displayPlatform`; the chosen identity is listed first and gives the account's name and picture everywhere (summaries, profiles, search results); search still matches every login; unlinking that platform goes back to Twitch first.
- [x] Sign out everywhere: `signOutEverywhere` ends every session, this one too, and clears the cookie (Settings → Account → Security). The header menu's Sign out stays this session only.
- [x] Connections: Disconnect with more than one linked; the last never goes (`last_identity`, the website says why). Restricted, ticket-blocked or blocked accounts still can't unlink (identity holds, #83); an account in its deletion window has only the restore session, so it can't either (#89).
- [x] Settings (account menu → Settings): rail Account / Appearance / Editors (the existing /me/editors page) / Billing (orders). Account: display name choice, connections, security, sessions, data, delete. Appearance: theme and reduced motion (System / On / Off, per device in `cyexMotion`): app.html sets `data-motion` before first paint and makes `matchMedia('(prefers-reduced-motion: …)')` answer by the setting, so every script follows it; `data-motion="reduce"` turns transitions and animations off in CSS. `/me/account` and `/me/settings` redirect to Settings → Account.
- [x] Profile look: the identity moved out of a 7TV-style centred side card into a left-aligned band across the page (paint-ringed avatar, painted name as h1, roles as flags right after it, a facts line: connections, channel sets, editors by name, member since). The rail is navigation only and runs the full height of the left column. Copy link is gone from profiles; the permissions panel is gone from the account page. Others' profiles show their listed sets, public uploads and editors by name.
- [x] /help: the index has no scroll bar of its own on wide windows; it runs the full column height with the search pinned.
- Tests: `apps/api/tests/profile_dashboard_flow.rs` (Postgres, CI: last connection, restricted unlink, display name and search by both logins, sign out everywhere with cookie and app token, activity groups / actor / time / bad input / editor lines), unit tests in `activity.rs`; vitest `lib/profile/timeframe.test.ts`, `lib/account/settings.test.ts`, `activity.test.ts` (editor lines, filter, Overview row).
- Manual check (2026/11/19): headless Edge against the dev mock (MOCK_KICK=1) at 375, 1440 and 2560, dark and light: overview, activity with filters and the date popover (future date refused), another person's profile, settings account (display name switch updated the header and band), appearance, /me/account redirect, help index; screens in the 2026-11-19-feedback run folder, screens/d.

### GitHub write log for this PR

```
git push -u origin feat/profile-dashboard
gh pr create --base main --head feat/profile-dashboard --title "Profile dashboard, activity filters and settings" --body-file <file>
git merge origin/main   (four times; PROGRESS.md, ActivityRows.svelte and editors.rs resolved; the display platform migration renumbered to 000052)
gh pr merge 95 --merge
```

## Paint render fix and global emote/editor permissions (branch `fix/paints-and-global-perms`, ADR 0010 amendment part 3)
- Owner feedback I (2026/11/19): "paints seem to not render properly on the website" (an image paint, "Icy Wave", showed as an empty radio in the cosmetics picker) and "perms to edit someones emotes and editors globally for a role".
- Paint cause: a painted name is transparent text (`-webkit-text-fill-color: transparent`) showing its background through `background-clip: text`. For an image paint the only background was the picture, so whenever the picture didn't load (blocked by the CSP's `img-src`, missing on the CDN, still on its way) the name drew nothing. A local stack serves pictures from `CYEX_TV_CDN_URL` (the S3 server), which the CSP never allowed; the studio previews render locally, so they looked fine. Fix: `paintStyle` (and the studio's `paintStyleText`) put a flat layer in the text colour under the picture; `PUBLIC_CDN_URL` adds the pictures' origin to `img-src` (svelte.config.js, Dockerfile build arg, .env.example).
- Permissions (additive): `emotes.delete_any` (delete anyone's emote, audited `emote.deleted` with `via`) and `editors.manage_any` (list, add, change, remove anyone's editors with every flag, audited with `via`); `emotes.edit_any` already covered name, tags, flags and visibility and is unchanged. Migration `20261207000051_global_emote_editor_perms.sql` gives both to the built-in Admin role. `accountEmotes` also takes `emotes.edit_any` / `emotes.delete_any`.
- Web: role editor labels, tooltips and groups (`lib/admin/permissions.ts`); admin account page Editors panel (`lib/admin/AccountEditorsPanel.svelte`); emote page shows the edit form and delete to holders; dev mock updated.
- Tests: vitest `paint.test.ts` (fails before the fix: no layer under the picture), `studio.test.ts`; Postgres `apps/api/tests/global_perms_flow.rs` (allow and deny for each, audit rows, owner's own actions unmarked).
- Manual check (2026/11/19): headless Edge against the dev mock, `.claude/runs/2026-11-17-overnight/screens/i-paint/paint-visible.mjs` clones the image-painted name with a CSP-blocked and a 404 picture and counts visible pixels: 0.0% before, 29% after, dark and light. Editors panel at 1440 dark and 375 light.

### GitHub write log for this PR

```
git push -u origin fix/paints-and-global-perms
gh pr create --base main --head fix/paints-and-global-perms --title "Paint render fix and global emote/editor permissions" --body-file <file>
gh pr merge 94 --merge
```

## Emote browse: animated everywhere, one grid, flags and tags filters, trending (branch `feat/emote-browse`)

Owner feedback 2026/11/19, section G. Additive API; nothing was deployed.

- [x] Animated emotes play everywhere without hover (tiles, cards, the set page's table, profile set thumbs, activity rows, small thumbs) through one `EmotePicture`: a `<picture>` whose reduced-motion source is the still frame, lazy and `decoding=async`.
- [x] One grid: the grid, compact and table switch on /search is gone (`view=` is ignored now). The table stays only on the public set page, which is a real list (columns for creator, type, channels, date). Tiles in a row share its height (checked in the running app on /search, profiles and set pages).
- [x] One filter model, all in the API (totals are real now): Type (All, Animated, Still), Flags as toggle chips (Zero-width, Wide, NSFW: each narrows to emotes with that flag), Tags as a `TagInput` (Enter adds a chip, Backspace or × removes) plus the results' common tags to add in one tap. `searchEmotes` gains `flags: [EmoteFlag!]`, `tags: [String!]` (all must match, 10 at most) and `exact`; `EmoteFlags.wide` (1x file at least 1.75 times as wide as tall, derived).
- [x] Trending: sort `TRENDING` with `window: DAY | WEEK | MONTH`, ranked by net distinct channels that added the emote in the window. Migration `20261207000050_trending.sql` (`emote_channel_adds`, hourly buckets), filled by the worker's hourly run from the outbox's `set.emote_added`/`set.emote_removed` events (their `channel:*` topics) before the outbox is pruned. Tiles show `#1`, `#2` … in the Trending view; the emote page shows "Trending #n" and "Added in the last 7 days" for the top 100 of the last 7 days (`emoteTrending(id, window)`).
- [x] Dev mock: flags, tags, exact, NSFW emotes, `wide`, trending windows and `emoteTrending`.
- Tests: `apps/api/tests/browse_flow.rs` (flags, tags, exact, trending windows, `emoteTrending`), `crates/db/tests/queries.rs` (rollup: distinct channels per hour, idempotent), unit tests for tag cleaning; vitest `tag-input.test.ts` (rules and markup) and `params.test.ts` (filter and trending URL state, API variables).
- Manual check: /search, Trending (3 windows), filters, the filter sheet, emote page at 375, 768, 1440 and 2560 in both themes; keyboard (tag field Enter/Backspace/paste/×, flag chips with Space, window with arrow keys); reduced motion shows the still frames (screens in the run folder, `2026-11-19-feedback/screens/g`).

## Front page, round 2: a real 3D hero and the zig-zag (branch `feat/front-page-2`)

Owner feedback 2026/11/19, part I ("not 3d enough", "animation too subtle", "stacked more vertically ... zig zag"). Web only; nothing was deployed.

- [x] Hero (`apps/web/src/lib/home/logo-scene.ts`): the three bars are now deep frames (2.6 grid units, a bar is 6 tall) with a rounded 4-step bevel, seen from above and to the side, under a key, a rim and a studio environment map, so tops, sides and fronts read apart. The lit bar stands 2.4 units in front, its front face glowing more than its sides, with glow around it and light inside its opening; its light spills onto the other two.
- [x] Motion (`logo-motion.ts`, pure and tested): the still pose, one full turn (3.2 s, eased), then a sway of about ±24°, a float and the lit bar breathing in and out; scrolling the hero away turns it further and brings the lit bar forward. Nothing follows the pointer. Draws every frame up to 60 fps (was 30), only while on screen and the tab is visible.
- [x] Still image: `static/home/logo-3d-{dark,light}.webp` (920 px, the still pose rendered by the scene; remake with `window.__logoSnap()` in dev) replaces the flat SVG as first paint, fallback and reduced-motion frame. `LogoMark.svelte` is gone.
- [x] Reduced motion: the system setting or a site-level `data-motion="reduce"` on `<html>` (`motionReduced()` in `capable.ts`), watched live: the scene stops and the still image comes back.
- [x] The ways in are stacked one under the other, picture and text trading sides each row, each with a heading and one or two sentences; extension browser menu and latest deck download kept; one column below 720 px. Rows reveal once on scroll (`reveal.ts`), not under reduced motion.
- Tests: `home.test.ts` (still pose at start, the full turn ends where it began, no jumps between frames, sway range, scroll response).
- Checked in the running app (vite dev + mock, headless Edge): 375, 768, 1440, 2560, dark and light, reduced motion (system and site attribute switched live). Frame sample 1440 dark, 5 s: 56.6 fps, p50 17.6 ms, p95 18.4 ms, max 20 ms, CLS 0. Screens in `.claude/runs/2026-11-19-feedback/screens/i-front/`.

### GitHub write log for this PR
2026/11/19, account `sery-yx`.

```
git push -u origin feat/front-page-2
gh pr create --base main --head feat/front-page-2 --title "Front page: a deeper 3D hero and the ways in as a zig-zag" --body-file <file>
gh pr merge 93 --merge
```

## Upload v3 (branch `feat/upload-v3`, owner feedback 2026/11/19 section C)

Additive API; no migration; nothing was deployed.

- [x] Upload is a dialog that opens from anywhere (`lib/upload`): the root layout holds one `UploadDialog`, and every same-site link to `/me/upload` or `/upload` opens it on a plain click (Ctrl/⌘-click, middle click and new tabs still open the page). Its queue stays while you browse; it can't close while files are being sent. `/me/upload` renders the same `UploadForm` as a page for old links; `/upload` redirects there. Phones: the dialog fills the screen.
- [x] Tags are `TagInput` chips (Enter or a comma adds one, Backspace on an empty field removes the last, × removes any, pasting a list works, repeats and the 11th are refused with a note), cleaned to what the API takes (`cleanTag`: lower case, spaces as `-`, letters, digits, `-` and `_`, 32 characters). Used on upload and on the emote edit page. `TagInput.svelte` and `tag-input.ts` are byte for byte the ones in the browse PR (#91), so the two merge cleanly; /search's filters are left to that PR.
- [x] More file types and 8 MB: JPEG is decoded now (the `image` crate's pure-Rust `jpeg` feature, zune-jpeg); APNG, GIF and animated WebP already were; AVIF is still refused (decoding needs dav1d). Emote files may be 8 MB (`cyex_tv_media::Limits::emote()`, used by the API's upload and by the worker's transcode, so the two agree); cosmetic pictures stay at 5 MB. Over the limit the API answers 413 `too_large`, "The file is over 8 MB, the most an upload can be.", while the file is read; the website names the size before uploading ("It’s 9,3 MB; at most 8 MB.").
- [x] Flags at upload: visibility, zero-width and NSFW (as before) plus a Wide line worked out from the picture with the same 1.75:1 rule as `EmoteFlags.wide` in #91 (derived, not set by the uploader). Zero-width in chat: the extension already stacks a cyex.tv set member with `zero_width: true` over the emote before it (`src/chat/tokenize.ts`, `tests/providers.test.ts`), so no extension change.
- [x] A required "I accept the emote guidelines" box, linking to the new `/guidelines` page in a new tab (our own text: your own work, no hate, sexual content only behind NSFW and never explicit, no personal information, no impersonation, nothing dangerous or flashing, readable at chat size, honest names and tags). Footer and the Help answer link it.
- Tests: Postgres `apps/api/tests/upload_flow.rs` (`files_up_to_8_mb_and_jpeg_are_taken`: 6 MB and exactly 8 MB taken, 8 MB + 1 refused with the message, JPEG stored as image/jpeg; `the_upload_dialog_flags_arrive_together`; the oversized case at 8 MB), `crates/media` (`a_jpeg_is_a_still_image`, `emotes_may_be_up_to_8_mb`), unit tests in `apps/api/src/upload.rs`; vitest `lib/rules.test.ts` (types, 8 MB, `cleanTag`, `isWide`), `lib/components/tag-input.test.ts`, `lib/upload/upload.test.ts` (which clicks open the dialog, the guidelines box). Checked in the running app (dev mock) at 375 and 1440 in dark and light.

### GitHub write log for this PR

```
git push -u origin feat/upload-v3
gh pr create --base main --head feat/upload-v3 --title "Upload v3: upload dialog, tag chips, JPEG and 8 MB, guidelines" --body-file <file>
gh pr merge <n> --merge
```

## Admin user dashboard and entitlement graph (branch `feat/admin-dashboard`, ADR 0010 amendment part 2)
- Owner feedback F (2026/11/19, later additions): "an entitlement graph like 7tv has" and "a large admin dashboard" per user.
- API (additive): migration `20261207000022_staff_notes.sql` (staff notes, an index for the graph's audit reads); `staffNotes`, `addStaffNote`, `deleteStaffNote` (users.notes, audited), `accountSessions` (users.sessions), `accountSubscriptions` (empty until subscriptions exist), `entitlementGraph`, `entitlementHolders` (entitlements.view or cosmetics.grant); `orders(accountId)` also for billing.view. ADR 0010 and ADR 0016 amended.
- Tests (Postgres, run in CI): `apps/api/tests/staff_dashboard_flow.rs` (notes, sessions, subscriptions, billing.view's reach, the graph's sources, revokes and the reverse, the top role hidden). Vitest `src/lib/admin/graph.test.ts` (branches, held-only, by holder, counts).
- Web: `/admin/entitlements` (pick an account or a badge, paint or role; a tree from the root through each source to what came from it, with a list view and a held-only switch); the account page is a dashboard in groups (Moderation: restrictions, staff notes, sessions; Roles and cosmetics with the graph; Billing: subscriptions and orders; Content: uploads, sets, reports; Activity) with jump links; the admin rail and overview list the graph; the orders page takes `?account=`.
- Manual check (2026/11/19): headless Edge against the dev mock at 1440 and 375, dark and light, keyboard; screens in .claude/runs/2026-11-19-feedback/screens/f.

### GitHub write log for this PR

```
git push -u origin feat/admin-dashboard
gh pr create --base main --head feat/admin-dashboard --title "Admin: user dashboard and entitlement graph" --body-file <file>
gh pr merge <n> --merge
```

## Owner's answers of 2026/11/19 (branch `feat/owner-answers`)

Additive API; two new migrations; nothing was deployed.

- [x] Account deletion has a 30-day undo window (ADR 0019, amends 0009). `deleteAccount` hides the account at once (migration `20261207000040_deletion_window.sql`: `accounts.erase_after`; `deleted_at` is set too, so every public read hides it), ends its sessions, clears the cookie and tells chat clients its channels, personal set and cosmetics went; audited `account.deletion_scheduled`. Its emotes leave `emote`, search, `emoteChannels` and emote activity; its sets leave `set(id)`; others can't add its emotes. Signing in within 30 days lands on `/restore` with a session only `pendingDeletion` and `restoreAccount` accept; restoring brings everything back (audited `account.restored`). The worker's hourly clean-up erases accounts past the window (`privacy::erase_account`, the old erase and file purge, audited `account.deleted`). Restricted or blocked accounts still can't delete; a block during the window refuses sign-in; a restriction survives restore and, through the identity hold, the erase.
- [x] Emote visibility `PUBLIC` / `UNLISTED` / `PRIVATE` (ADR 0020, migration `20261207000041_emote_unlisted.sql`). `Emote.visibility`, `updateEmote(input: { visibility })`, the upload field `visibility`; `private` keeps working for old clients. Unlisted emotes open by link and can be added by anyone; search, browse, the creator's page and activity feeds leave them out.
- [x] `me.roles`: every role you hold with `public`, so your own profile marks the hidden ones; `accountProfile.roles` is unchanged.
- [x] Website: Account → Delete account copy, the Help answers (deletion, unlisted/private), the `/restore` page, the Public / Unlisted / Private control on upload and edit, the "Unlisted" mark on the emote page, hidden roles on your own profile ("Hidden" with an eye-off icon), and "Clone to my sets" makes a listed copy again. Dev mock follows all of it.
- Tests: `apps/api/tests/auth_flow.rs` (restore within 30 days, past the window, blocked, restriction kept through restore and erase), `apps/api/tests/account_hub_flow.rs` (hidden at once, then erased), `apps/api/tests/visibility_flow.rs` (unlisted, own roles), `apps/worker/tests/erase.rs` (only due accounts erased), unit tests in `crates/shared` (visibility rules), web `visibility.test.ts`, `api.test.ts`, `faq.test.ts`.

## Sets and editors (branch `feat/sets-editors`, owner feedback 2026/11/19 section E)

Additive API, new migrations only; nothing was deployed. ADR 0014 and 0017 amended.

- [x] Default sets: a new account gets a listed channel set named after its display name at sign-up (`sets::create_default`, from the sign-in flow); migration `20261207000011_default_sets.sql` gives one to every live account without a channel set of that name.
- [x] Set page: "Use on my channel" lists the owner's channels (yours, or theirs when you hold `CHANNELS`/`SETS`) and sets the active set through `updateChannelSettings`; the line under the title names the channels showing it.
- [x] Set page "Add emotes": one search field, searched on Enter only; your matching uploads first (pending too, except for a personal set), then everyone's; a list with its own scroll and an Add button per row; empty and error states.
- [x] The per-set editors field is gone from the set page; a card links to the Editors page.
- [x] Editors page: find people by name (`searchAccounts`), Add opens the permissions dialog (groups: Emote sets, Emotes, Profile, Channels, Billing and subscription, Editors; "Everything"; "Sets they can edit", default all sets). Change opens the same dialog. 50 editors per account (`too_many_editors`).
- [x] API: new flags `CHANNELS`, `EMOTE_UPLOAD`, `EMOTE_MANAGE`, `EMOTE_TRANSFER`, `BILLING_PAYMENT`, `BILLING_SUBSCRIPTION` (migration `20261207000010_sets_editors.sql`); set scope (`allSets`, `setIds`) on grants, enforced in every set mutation, private-set reads, `editorSets` and the active set; limited managers can't widen scopes; `AccountProfile.editors` (names only). Audit rows carry the scope.
- Tests: `apps/api/tests/sets_editors_flow.rs` (scope can/can't, limited managers, new flags, 50-editor cap, public editors, backfill), `auth_flow.rs` (default set at sign-up, none on re-sign-in or link), `sec_account_editors.rs` (rate limit), unit tests in `editors.rs`; vitest `lib/tool/editors.test.ts` (dialog permission model), `lib/tool/add-emotes.test.ts`.

### GitHub write log for this PR

```
git push -u origin feat/sets-editors
gh pr create --base main --head feat/sets-editors --title "Sets and editors: default sets, use on channel, editor permissions dialog" --body-file <file>
gh pr merge <n> --merge
```

## Header search, nav, notifications and mark (branch `feat/header-search`)

Owner feedback 2026/11/19, section A. Additive API; nothing was deployed.

- [x] The Ctrl K command palette is gone. The header field is the search: typing drops a list under it (ARIA combobox: ↑ ↓, Enter, Escape, Tab) with the top 5 emotes and top 3 people from the new `quickSearch(query)`; Enter with no row picked opens `/search?q=`. Requests wait 150ms and a newer one aborts the older. `/` outside a field focuses it (a page field marked `data-page-search` keeps it). Phones: the search button turns the header row into the field, no modal.
- [x] `quickSearch` ranks names in tiers: exact, exact ignoring case, prefix, word or initials prefix (`point` and `pp` find `PepePoint`), substring, then near misses (pg_trgm similarity, 3+ characters); most used (channel count) first inside a tier. Migration `20261207000030_quick_search.sql` adds the stored `emotes.name_words` / `name_initials` columns and their indexes. 240 a minute per account or IP (`rate_limited`).
- [x] Focusing the empty field lists `myEditableChannels`: your channels, then those of accounts you edit, then those whose sets you edit, A to Z inside a role, each channel once under its highest role. Signed out: a hint line.
- [x] Header links signed in: Emotes, Store (/shop), Subscription (only with an active subscription: `hasSubscription()` is false until subscriptions exist; placeholder `/me/subscription`), Admin for staff. Signed out unchanged. Under 1080px the links keep their icons. The account menu no longer repeats Admin.
- [x] Notifications: the bell is in the header on every width (a sheet on phones), and new ones pop up as toasts under it (NotificationToasts; only ones newer than the page's first count).
- [x] The mark is the owner's logo: three outlined bars, the middle one offset and lit with a soft glow (an SVG filter, left out under 20px and with more contrast or forced colours). Header, footer, eyebrows; favicon.svg / -dark / -light (the tab icon follows the site theme), favicon.png and apple-touch-icon.png.
- [x] Account menu: the name row at the top opens your profile dashboard (`/users/<id>`; the separate Profile row is gone). Sign out ends this session only (`POST /v1/auth/logout`).
- Tests: `apps/api/tests/quick_search_flow.rs` and `crates/db/tests/queries.rs` (Postgres, CI): tier order, popularity tie-break, camelCase and initials, typo, hidden emotes, rate limit, editable channel roles and order. Vitest: `lib/shell/nav.test.ts` (nav visibility, role sort), `lib/shell/search.test.ts`, `lib/account/toasts.test.ts`.
- Manual check: 375, 768, 1440 and 2560, dark and light, keyboard (screens in the 2026-11-19-feedback run folder, screens/a).

### GitHub write log for this PR

```
git push -u origin feat/header-search
gh pr create --base main --head feat/header-search --title "Header search, nav, notifications and the new mark" --body-file <file>
git merge origin/main   (twice; PROGRESS.md and gql.rs imports resolved)
gh pr merge 86 --merge
```

## Admin roles and moderation (branch `feat/admin-roles`, ADR 0010 amendment 2026/11/19)
- Owner feedback F (2026/11/19): built-in roles, finer permissions, timed restrictions, the admin rail's height.
- API (additive): migrations `20261207000020_default_roles.sql` (roles.system; User, Admin and the owners' top role) and `20261207000021_timed_restrictions.sql` (`*_until` ends, the tickets hold, `account_restrictions` history, identity holds carry both). `Role` gains `builtIn`, `editable`, `assignable`; `AccountStatus`/`AdminAccount` gain `ticketsBlocked` and the three ends; new `placeRestriction`, `liftRestriction`, `accountRestrictions`. Ten new permissions (`roles.assign`, `reports.manage`, `users.restrict_uploads`, `users.restrict_tickets`, `users.block`, `users.sessions`, `users.notes`, `entitlements.view`, `billing.view`, `subscriptions.manage`); each broad permission keeps working. Rules: nobody edits a role they hold (`own_role`), built-in roles (`built_in_role`), the top role is `not_found` to everyone but the owners, custom positions 1 to 999 999 (`bad_position`). Reports refuse `tickets_blocked`. Audit rows: `account.restriction_placed`, `account.restriction_lifted`.
- Tests (Postgres, run in CI): `apps/api/tests/builtin_roles_flow.rs` (top role hidden and untouchable, User role is everyone's, own role, granting at or above, roles.assign), `abuse_flow.rs` (`timed_restrictions_end_on_their_own`, `a_ticket_ban_stops_reports_until_lifted`), `crates/db/tests/queries.rs` (`timed_holds_end_and_follow_the_person`); existing role tests updated for the seeded roles. Unit: rbac `nobody_edits_a_role_they_hold`, positions, restriction ends; vitest `src/lib/admin/permissions.test.ts` (every API permission labelled, group checkbox states).
- Web: the role editor groups permissions (Emotes, Sets, Users, Moderation, Tickets and reports, Cosmetics, Shop and billing, Roles, System) with plain names, a tooltip on hover and focus, and a group checkbox (indeterminate when partial); roles list marks built-in and your own roles and says why one can't be edited. The account page's restrictions panel places uploads, reports or full blocks for 1 hour, 1 day, 7 days, 30 days, until a date, or for good, with a reason, shows ends, lifts them and lists the history. SideRail (shared): no scroll inside the rail any more; a rail taller than the window scrolls with the page; new `fill` prop (the admin rail uses it) runs the full height of the left side.
- Manual check (2026/11/19): headless Edge against the dev mock at 1440 and 375, dark and light, keyboard focus on the permission tooltips; screens in .claude/runs/2026-11-19-feedback/screens/f.

### GitHub write log for this PR

```
git push -u origin feat/admin-roles
gh pr create --base main --head feat/admin-roles --title "Admin: built-in roles, finer permissions, timed restrictions" --body-file <file>
gh pr merge <n> --merge
```

## Front page: 3D logo hero and four boxes (branch `feat/front-page`)

Owner feedback 2026/11/19, part B. Web only; nothing was deployed.

- [x] Hero: the cyex.tv logo (three open bars, the middle one offset and lit) as a three.js object (`apps/web/src/lib/home/logo-scene.ts`, three 0.187.0 pinned). It turns slowly on its own and tilts back as the hero scrolls away; never follows the pointer. The flat SVG (`LogoMark.svelte`) paints first and stays for reduced motion, no WebGL, data saver, under 4 GB memory or under 4 cores. three.js is its own chunk, asked for only after the load event and an idle moment (`LogoHero.svelte`); it draws at 30 fps and only while the hero is on screen.
- [x] Below the hero: no emotes any more (no strip, cluster or chat previews), and one row of four boxes, each with a small picture in the logo's language (`ShowcaseArt.svelte`): the extension (a menu of Chrome, Edge, Firefox, Opera, Brave; URLs in `apps/web/src/lib/home/stores.ts`, `/download#<browser>` until the store listings exist), cyex deck (direct download of the newest installer), the API and cosmetics.
- [x] cyex deck download works like cyex.app/deck: the newest `cyex-deck_<v>_x64-setup.exe` of `github.com/cyex-app/cyex-deck` releases (pre-releases counted, drafts skipped, only that repo's download URLs accepted), read from GitHub's release list in the browser after load at most once an hour (`cyexDeckRelease` in localStorage), falling back to a pinned installer (`deck-release.ts`, move it with each release). CSP `connect-src` gained `https://api.github.com` for that one request.
- [x] /download has anchors for each browser (`#chrome`, `#edge`, `#firefox`, `#opera`, `#brave`).
- Tests: `apps/web/src/lib/home/home.test.ts` (release picking, installer URL checks, version order, kept answer, browser detection, the 3D gate).
- Checked in the running app (vite preview + mock): 375, 768, 1440, 2560, dark and light, reduced motion, Edge with WebGL off, the menu by mouse and keyboard; LCP is the hero text before three.js is asked for, CLS 0 from the page.

### GitHub write log for this PR

```
git push -u origin feat/front-page
gh pr create --base main --head feat/front-page --title "web: front page with a 3D logo hero and four boxes" --body-file <file>
gh pr merge <n> --merge
```

## Admin v2 (branch `feat/admin-v2`)
- API (additive): `auditLog` takes `actorId`, `action`, `since`, `until` besides target and paging; new `adminAccount` (users.view or users.restrict), `accountEmotes` (users.view, emotes.review or emotes.takedown) and `accountReports` (moderators). Postgres tests: `staff_look_up_an_account` (apps/api/tests/abuse_flow.rs) and the audit filter cases in crates/db/tests/queries.rs. Not built: sessions on the account page (no sessions API on main yet).
- Web: `/admin/users/[accountId]` (profile, restrictions, roles, cosmetics, uploads, reports about and by, latest audit entries), `/admin/cosmetics/owners` (who owns what), audit log filters with links to targets and a Changes/JSON toggle, review queue keyboard flow (J, K, A, R) with a 1x/2x/4x comparison, reports grouped by emote.
- Manual check (2026/11/18): headless Edge against the dev mock at 1440 and 375, dark and light, plus the keyboard flow (J, K, R, A) in the review queue; screens in .claude/runs/2026-11-17-overnight/screens/g9.

## Sign-off fixes: API (branch `fix/signoff-api`)

The second security review and the sign-off code review (overnight run 2026-11-17). Additive; nothing was deployed.

- [x] `unlinkIdentity` refuses `restricted` (403) while the account is restricted or blocked. Migration `20261206000001_identity_holds.sql`: a trigger keeps a hold (platform ID + restriction) for every identity that leaves a restricted or blocked account; sign-in and linking with it copy the restriction or block onto the account and use the hold up.
- [x] Linking takes the account row `FOR SHARE` and needs it live (a link racing a deletion answers `account_deleted`).
- [x] Shop: replaced and expired checkouts get their Paddle transaction cancelled (`PATCH /transactions/{id}`; the api on replace, the worker's hourly sweep for the rest); a failed cancel is audited (`shop.transaction_cancel_failed`) and only then is a late payment still taken. The Paddle client moved to `crates/paddle` so the worker can use it.
- [x] Shop: `checkoutUrl` only when it is https on cyex.tv, buy.paddle.com or sandbox-buy.paddle.com. Adjustments for an order that can still take payment answer 503. Orders keep the badges and paints they sold (migration `20261206000002_shop_signoff.sql`), fulfil from them, and are replaced when the item's bundle changed; nothing left to grant is paid and audited `item_empty`. Staff may retry a refund stuck as requested after 10 minutes; the worker puts ones older than an hour back to paid (`shop.refund_request_reverted`). A staff refund of an order with a transaction needs Paddle (`refund_unavailable`). Gift recipients see a gift only once paid, without money fields or the transaction ID (orders list, `order`, export); `gift` is stored at checkout (`is_gift`).
- [x] Webhook timestamps near i64's ends are `stale_signature`, not an overflow. Upload decoder errors stay in the logs; the client gets one sentence.
- [x] Account deletion queues `files.delete` for each file only its deleted emotes used (migration `20261206000003_files_purged.sql`, `files.purged_at`); the worker deletes under the file's lock unless it is in use again; an upload of the same bytes stores and transcodes it again. Deleted emote rows lose their owner.
- [x] Activity lines of uploads no longer need the approver's account; `next` is null after a short batch; `searchAccounts` past the last page is empty with the right total; edits on sets the caller can't view answer `not_found`; `slotsUsed` counts visible members for non-editors; `sets.edit_any` goes before the editor rule for visibility.
- Tests: `apps/api/tests/{auth_flow,shop_flow,account_hub_flow,upload_flow,backend_reads_flow,account_search_flow,public_sets_flow}.rs`, `apps/worker/tests/jobs.rs`, `crates/db/tests/queries.rs`, unit tests in `shop.rs`, `upload.rs`, `crates/paddle`.

### GitHub write log for this PR

```
git push -u origin fix/signoff-api
gh pr create --base main --head fix/signoff-api --title "api: sign-off security and correctness fixes" --body-file <file>
gh pr merge <n> --merge
```

## Design harmony pass (branch `polish/harmony`)

Contact sheets of every route at 1440 and 375, dark and light, read as one product (overnight run 2026-11-17). Web only; nothing was deployed.

- [x] One page top: every page title sits at the same height under the header (shop, download and redeem were 44px lower, help and the API docs 14px lower; phones too).
- [x] One page-title metric (size, line height, tracking) and one line under it (15px, 8px gap) for PageHead, the admin `.ui-head` pages and the admin overview.
- [x] Whole-page states (not-found, error pages, the signed-in pages' error) no longer pick up the global `.page` column class, which inset them by the page padding a second time.
- [x] The shop's load error is the one Problem pattern; the shop and redeem messages use problem.ts's words for offline and unknown failures (test).
- [x] One Copy link button (`CopyLink.svelte`): profile, channel and set pages say "Copy link", then "Copied". The channel page said "Copy set link" but copied the channel's address.
- [x] The signed-in pages' rail is the public profile's rail (Activity row, Copy link), so it no longer shifts between /me/* and /users/<id>.
- [x] Labelled tool buttons press like the site's buttons (scale), not by dimming.
- Manual check: every route re-shot at 375 and 1440 in both themes (contact sheets in the run folder, screens/polish); svelte-check 0/0, vitest, build.

### GitHub write log for this PR

```
git push -u origin polish/harmony
gh pr create --base main --head polish/harmony --title "web: design harmony pass" --body-file <file>
gh pr merge <n> --merge
```

## Shop hardening (branch `fix/shop-hardening`)

The backend audit of the shop (overnight run 2026-11-17, findings B1-B3, M1-M3, L1-L3). Additive; nothing was deployed.

- [x] Migration `20261202000001_shop_hardening.sql`: status `refund_requested`; `shop_orders.checkout_url`, `failed_reason`, `refund_requested_at`, `refund_rejected_at`; `shop_order_entitlements` (every cosmetic an order covers, backfilled from `order_id`); one pending order per buyer, item and recipient (partial unique index); `paddle_events (received_at)` index.
- [x] Refunds take back only what no other paid order, redeemed code or staff grant covers (B1).
- [x] Payments that give nothing are audited: `shop.order_paid_unfulfilled` (cancelled or failed order, deleted or gone recipient, transaction not saved), `shop.order_paid_no_grant` (owned already). Never granted to a deleted account. A transaction ID that can't be saved is retried once, then named in `shop.checkout_failed` (B2).
- [x] Checkout re-reads the item under its lock and uses that row's price (B3).
- [x] A repeated checkout hands out the same open transaction (`checkout_busy` while one starts); a late payment after the hold is taken (`shop.order_oversold` when it sells one too many) (M1).
- [x] Staff refunds: `refund_requested` committed, Paddle asked without the lock, then finished; `already_refunded`; a rejected refund flags the order (`Order.refundRejectedAt`, staff only) and audits `shop.refund_rejected` (M2).
- [x] Notifications `cosmetic.granted` / `cosmetic.revoked` with `source` (`purchase`, `code`) and `order_id`; the export lists orders and code redemptions (M3).
- [x] Redeem sorts its grants, needs a known client IP (`unknown_client`); webhook body limit 64 KB; `paddlePriceId` only for `shop.manage` (L1).
- [x] Worker clean-up: pending orders older than 24 h fail (`expired`, audited `shop.order_expired`), Paddle events older than 90 days go (L2).
- [x] `TestDb` panics in CI (`CI=true`) without `CYEX_TV_TEST_DATABASE_URL` (L3).
- [x] Tests: `apps/api/tests/shop_flow.rs` (B1-B3, M1-M3, L1, and three `tokio::join!` races), `apps/worker/tests/jobs.rs` (expiry and retention), unit tests in `shop.rs`, `codes.rs`, `testing.rs`.

## Cosmetics shop and redeem codes (branch `feat/shop`)

ADR 0016. A shop for cyex.tv's chat cosmetics and redeem codes, like 7TV's Store and Redeem (studied from what 7TV shows publicly; no code taken). Payments through cyex.tv's **own** Paddle Billing account, never cyex.app's; no card data reaches cyex.tv.

- [x] Migration `20261201000001_shop.sql` (additive): `shop_items` (+ `shop_item_cosmetics` bundles), `shop_orders`, `paddle_events`, `code_batches` (+ `code_batch_cosmetics`), `redeem_codes` (sha256 only), `code_redemptions`; entitlements gain source `code` and `order_id`/`code_id`.
- [x] Permissions `shop.manage`, `shop.orders`, `codes.manage` (the owner holds all).
- [x] Paddle (`apps/api/src/paddle.rs`): env `CYEX_TV_PADDLE_ENV`, `_API_KEY`, `_CLIENT_TOKEN`, `_WEBHOOK_SECRET`; keys of the wrong environment refused at start. Checkout creates a Paddle transaction; fulfilment only from the signed webhook `POST /v1/shop/paddle/webhook` (HMAC, 5-minute window, `event_id` recorded once, idempotent by transaction); refunds and chargebacks revoke exactly the order's entitlements and clear the selection.
- [x] GraphQL (additive): `shopItems`, `shopItem`, `myOrders`, `order`, `orders`, `codeBatches`; `createShopItem`, `updateShopItem`, `deleteShopItem`, `startCheckout`, `refundOrder`, `redeemCode`, `createCodeBatch`, `disableCodeBatch`. Schema, OpenAPI and web types regenerated.
- [x] Codes: `PREFIX-XXXX-XXXX-XXXX-XXXX` (80 bits), shown once; constant-time compare; 30 tries an hour, lockout after 5 wrong per account / 20 per IP in 15 minutes.
- [x] Every staff action, fulfilment, refund and redeem audited.
- [x] Tests: `apps/api/tests/shop_flow.rs` against Postgres in CI with a fake Paddle server; unit tests of the checks, signature, config, code format and lockout.
- [x] Dev mock: sample items, immediate fake checkout, sample orders, code batches with the known code `SAMPLE-CODE-0001`.
- [x] Infra: `env/api.env.example` gets the four names, empty (cyex-tv/infra PR 7, merged).
- [ ] Owner: Paddle account, products, keys and webhook (`docs/setup/owner-steps.md`, "Paddle for the cyex.tv shop").
- [ ] Website pages (another task).
- Open questions (defaults taken): `.claude/runs/2026-11-17-shop/questions.md`.

#### GitHub write log
- `gh pr create -R cyex-tv/infra --base main --head env/paddle`, `gh pr merge 7 -R cyex-tv/infra --merge`.
- `gh pr create --base main --head feat/shop`.

## Shop and redeem pages on the website (branch `feat/shop-web`)

The website side of ADR 0016, on the shop API (PR 58). Nothing changes in the API.

- [x] `/shop`: items as cards, each drawn on your own name (a made-up one signed out) in the dark and the light chat ground and as a chat line; price in the cyex number format, copies left, sale end, "Owned". An item opens in a dialog at `/shop?item=<id>` (shareable; signed out, "Sign in to buy" comes back to it): every paint and badge (chat size and 2x), buy for yourself or give to a Twitch or Kick login.
- [x] Checkout: `startCheckout`, then Paddle.js loaded only at that moment (`Environment.set`, `Initialize({ token })`, `Checkout.open({ transactionId })`; `checkoutUrl` when the script can't load), then `order(id)` polled until PAID, REFUNDED or FAILED. The dialog closes when the payment opens (a modal would cover Paddle's window); the page shows a calm state: waiting, confirming, still confirming, window closed (open it again), paid (link to equip it) or failed. Every API error code has its own sentence.
- [x] `/redeem`: one field (case, spaces and dashes ignored), shows what the code gave with a link to equip it; the lockout says in how many minutes to try again.
- [x] Orders: a panel in `/me/account` (item, state, amount paid, date, gift from or to). The refund reason is never shown to buyers.
- [x] Admin, under a "Shop" group of the admin rail with counts: Shop items (list with state, create and edit with a cosmetics picker, price, currency, Paddle price ID, sale window, copies, sort order, visibility; soft delete), Orders (filter by state, item, account; detail; refund with a reason), Codes (batches; a new batch shows its codes once with Copy all and Download .txt; turn a batch off). Each page needs its own permission (`shop.manage`, `shop.orders`, `codes.manage`).
- [x] Entry points: Cosmetics ("Get more in the shop"), the footer ("Shop"). The header is unchanged.
- [x] CSP (`apps/web/svelte.config.js`): `script-src` + `https://cdn.paddle.com`; `style-src-elem` + `https://cdn.paddle.com` and `https://sandbox-cdn.paddle.com`; new `frame-src 'self' https://buy.paddle.com https://sandbox-buy.paddle.com`. Found by loading Paddle.js and opening a checkout under the policy in both environments; Paddle's optional ProfitWell script stays blocked.
- [x] Tests: `src/lib/shop/shop.test.ts` (prices, price fields, redeem codes, every error map, lockout minutes, order states, item states, code files). Dev mock: no recipient means yourself, staff permissions, `MOCK_PADDLE=1` leaves orders pending like the real checkout.
- [ ] Not checked: a real Paddle sandbox payment (needs the owner's sandbox account and token).

## Account hub: sessions, notifications, export, deletion (branch `feat/account-hub`)

Gap list G6. API only; `/me/account` (Connections, Sessions, Data) and `/me/inbox` with the header bell come next.

- [x] Migration `20261130000003_account_hub.sql` (additive): `sessions.id` (a public handle; existing sessions get a random one) and `sessions.user_agent` (cut to 512 characters, saved at sign-in), and an index for unread notifications.
- [x] Sessions: `mySessions` (`AccountSession { id, kind: SITE | APP, device, createdAt, lastSeenAt, expiresAt, current }`; `device` is a short name like `Firefox on Windows`, never the raw user agent, never a token or hash), `revokeSession(id)` (yours only: another account's ID is `false`; revoking the current website session also clears its cookie), `revokeOtherSessions` (count).
- [x] Notifications: `unreadNotificationCount`, `markNotificationsRead(ids)` (optional, up to 100, others' IDs ignored; no `ids` marks all, as before). New kinds made in the same transaction as their change: `emote.approved`, `emote.rejected` (with the reason), `editor.added`, `editor.changed`, `editor.removed` (for the editor, and for the owner when someone else acted; payload has both IDs and names, the actor and the permissions), `cosmetic.granted`, `cosmetic.revoked` (name and reason). Orders wait for the shop.
- [x] `exportMyData`: one JSON document (`format`, `version`, account, identities, sessions, channels, emotes in every state, sets with emotes, set editors, account editors both ways, cosmetics, roles, reports filed, notifications, `orders: []`). Only yours, 5 an hour, audited (`account.exported`, no content).
- [x] `deleteAccount(confirm)`: `confirm` is one of your logins; website (cookie) sessions only (`website_only`); the owner is protected (`owner_protected`); restricted or blocked accounts can't (`restricted`), since signing in again would make a fresh, unrestricted account (security review). At once: sessions, identities, channels (chat clients get `channel.active_set_changed` to none), sets (`personal.updated` for the personal set), editor grants both ways, cosmetics (`user.cosmetics_updated` to none), roles and notifications go; reports filed stay without the reporter; approved emotes in other people's sets stay with no owner ("deleted user"); every other emote is deleted (others' sets get `set.emote_removed`). The account row stays, marked deleted. The response clears the cookie. Audited (`account.deleted` with counts, no names). Signing in again later makes a new account.
- [x] Tests: `apps/api/tests/account_hub_flow.rs` against Postgres in CI (sessions only yours and never tokens, IDOR on revoke, revoke others, notifications made for reviews, editor grants and cosmetics, unread count and marking only yours, export complete and only yours, its limit and audit row, deletion confirm, website only, owner protected, what stays and what goes, invisible afterwards, audit row, a new account on sign-in); unit tests of the device names and the confirm check.
- [x] Dev mock: four sample sessions, the new notification kinds, `mySessions`, `revokeSession`, `revokeOtherSessions`, `exportMyData`, `deleteAccount` (signs the mock out, keeps its data), `unreadNotificationCount`, `markNotificationsRead(ids)`.
- [ ] Website pages (another task). Privacy page (PR #33) should say deletion is self-serve and immediate once this ships.

#### GitHub write log
```
git push -u origin feat/account-hub
gh pr create --base main --head feat/account-hub

## Public profiles and account cosmetics (branch `feat/public-profile`)

For the website's public user pages (like 7TV's) and painted names everywhere. API only.

- [x] Migration `20261130000005_public_roles.sql` (additive): `roles.public` (default false), so staff-only roles stay hidden unless someone with `roles.manage` marks a role public. `Role.public`, `CreateRoleInput.public`, `UpdateRoleInput.public`; the change is in the `role.created` / `role.updated` audit snapshots.
- [x] GraphQL (additive): `accountProfile(accountId): AccountProfile` (`account`, `identities: [PublicIdentity]` with platform, login and display name only, `roles: [PublicRole { id name }]` (public roles, most senior first), `activeSets: [ActiveChannelSet { platform login setId setName }]`, `createdAt`); null for unknown, blocked or deleted accounts. Listed sets stay in `accountSets`.
- [x] `accountCosmetics(accountIds): [AccountCosmetics { accountId badge paint }]`: what up to 100 accounts show, with the badge and paint themselves, by cyex.tv account ID; only live accounts showing something; `too_many` over 100.
- [x] Tests: `apps/api/tests/profile_flow.rs` against Postgres in CI (no platform IDs, only public roles, roles.manage needed and audited, active set shown, blocked and deleted have no profile; cosmetics only for live accounts showing something, duplicates once, the 100 limit).
- [x] Dev mock: `accountProfile` for the mock's people (Moderator and Artist marked public), `accountCosmetics` from the sample picks; new fields in one operation are answered together.
- [ ] Website: role chips, "Active emotes", painted names (another task); a "public" checkbox in the roles editor.

#### GitHub write log
```
git push -u origin feat/public-profile
gh pr create --base main --head feat/public-profile

## Emote activity (branch `feat/emote-activity`)

Gap list G2 (API side): an emote page's Activity tab, from the account activity store (ADR 0016).

- [x] Migration `20261130000004_emote_activity.sql` (additive): an index on the emote ID in activity rows' JSON.
- [x] GraphQL (additive): `emoteActivity(emoteId, before, limit): [ActivityEntry!]!`. Kinds `EMOTE_UPLOADED` (its approval, shown as by the owner), `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED`, each built exactly as on the set owner's own feed: listed sets only (ADR 0017), no moderation, reviewers or reasons, nothing by or about blocked or deleted accounts. Empty unless the emote is approved, not private and not deleted. Paged by entry ID like `accountActivity`; `limit` 1 to 100, default 30.
- [x] Tests: `apps/api/tests/emote_activity_flow.rs` against Postgres in CI (pending emotes have no feed, kinds in order, owner shown for the upload, no reviewer, unlisted and private sets hidden, paging, blocked accounts' lines gone, private emote empty).
- [x] Dev mock: `emoteActivity` for any public emote (the sample feed lines about it plus an upload and a few set changes).
- [ ] Website: the Activity tab on `/emotes/[id]` (another task).

#### GitHub write log
```
git push -u origin feat/emote-activity
gh pr create --base main --head feat/emote-activity
```

## Upload v2 and emote settings (branch `feat/upload-v2`)

Gap list G5.

- [x] API (additive): `POST /v1/emotes` takes `zero_width`, `private` and `nsfw` form fields (`true`/`false`, `1`/`0`, `on`/`off`, empty is off; anything else is `bad_form`). Without them nothing changes. OpenAPI regenerated; the GraphQL schema is unchanged.
- [x] Tests: `upload::tests::flags_parse_like_a_form` (unit) and `upload_flow::flags_can_be_set_at_upload` against Postgres in CI (each flag alone, all on, explicit off, left out; stored rows match).
- [x] `/me/upload`: up to 20 files at once (the hourly limit); the same file can't be queued twice. Each file: name from the file name until typed, tags, Visibility (Public / Private), Zero-width, NSFW (asks first), a preview at real sizes on dark, light and in chat lines (zero-width drawn over a stand-in). Checked in the browser first (type, 5 MB, 1000 × 1000 px), then sent one by one with a progress bar (XHR); refusals in words (`blocked`, `too_large`, `rate_limited`, ...), and the batch stops at `rate_limited`. One file opens its page as before; a batch stays with Open links. "Use the first one's tags and flags for all".
- [x] `/me/emotes/[id]`: the same Visibility and flag controls (`EmoteFlags`), NSFW asks first. "Replace file" skipped: the API can't swap an emote's file.
- [x] Dev mock: the upload reads the flags and tags; the names `blocked`, `ratelimited` and `broken` answer as the API's refusals.
- Manual check: headless Edge at 375, 768, 1440, 2560, dark and light, with the dev mock: queue with good, too-big and wrong-type files, a batch with one refusal, NSFW confirmation by keyboard (focus goes to Mark NSFW and back to the switch), the edit page saving Private + zero-width + NSFW and showing them after a reload (`.claude/runs/2026-11-17-overnight/screens/g8g5`).

## User search (branch `feat/account-search`)

Gap list G10: find people by their Twitch or Kick name. API only; the `/search` Users tab comes next.

- [x] Migration `20261130000002_account_search.sql` (additive): trigram indexes on `lower(login)` and `lower(display_name)` of identities.
- [x] GraphQL (additive): `searchAccounts(query, page, perPage)` returning `AccountResults { items: [AccountHit { account: AccountSummary, identities: [PublicIdentity { platform, login, displayName }] }], total, page, perPage }`. Exact login first, then login and display-name prefixes, then similar names; a leading `@` is ignored; `%` and `_` mean themselves. Public fields only (no platform IDs). Deleted and blocked accounts are never listed. `perPage` 1 to 50, default 20.
- [x] 60 searches a minute per account, or per IP when signed out (the limits middleware now hands the client IP to GraphQL resolvers).
- [x] Tests: `apps/api/tests/account_search_flow.rs` against Postgres in CI (ranking, both platforms in one hit, no IDs, paging, wildcards, blocked and deleted never listed signed in or out, per-account and per-IP limits).
- [x] Dev mock: `searchAccounts` over the mock's people (quietoak has a Kick login too).
- [ ] Website: the Users tab on `/search` and people in the palette (another task).

#### GitHub write log
```
git push -u origin feat/account-search
gh pr create --base main --head feat/account-search
```

## Help, download and the cosmetics gallery (branch `web/help-download`)

- [x] `/help`: 34 short answers in eight groups (getting started, the extension, uploading, sets and channels, editors, cosmetics, account and privacy, moderation and takedowns) on the docs layout. The filter narrows the index and the answers together (every word, question or answer), `/` focuses it, Escape clears it, `?q=` is kept in the address. Questions are data in `lib/help/faq.ts` (vitest: groups, unique anchors, every site link points at a route).
- [x] `/download`: the extension for Chrome and Firefox as "early testing, coming to the stores", the test build (clone, `npm ci`, `npm run build`, then Load unpacked or Load Temporary Add-on), cyex deck "coming", a line for chat clients and bots. Your browser's card first.
- [x] `/cosmetics`: every badge and paint from the public `cosmetics` query, badges with a chat-line sample, paints drawn live; signed in, "Paint name / Your name" draws your name in each paint. No ownership anywhere.
- [x] Footer: Download, Cosmetics, Help (Terms, Privacy, Contact stay out until PR #33). Palette: Help, Download, Cosmetics pages. `initDocs` takes `filter: false` for pages that filter themselves.
- Manual check: 375, 768, 1440, 2560, dark and light, keyboard (`/`, typing, Escape, index jump), with the dev mock (`.claude/runs/2026-11-17-overnight/screens/g8g5`).

## Paint studio (branch `feat/paint-studio`, ADR 0018)
- `apps/web/src/lib/paint-studio/`: the staff editor for name paints, built on the owner's cyex paints tool. Gradient paints (kind, angle or shape, repeat, 2 to 10 stops, up to 4 shadows, fallback colour) with the API's limits checked live; animated paints from the tool's engine (layers, effects, scene elements, presets) rendered and encoded in a Web Worker as WebP, APNG or GIF; previews on sample names on both grounds and in Twitch- and Kick-style chat; import of cyex.tv JSON, integer-colour paint JSON and tool project files, export as cyex.tv JSON, integer JSON, CSS, picture and project; Save to cyex.tv (createPaint / updatePaint; animated: render, upload, wait for ready, IMAGE paint); Open an existing paint.
- Provenance: the tool's gradient CSS builder and integer-colour formatter were ports of the 7TV extension; neither came over (the gradient side is new code, the formatter was removed from the engine). The engine's custom-code items (`new Function`) were removed.
- Admin: `/admin/cosmetics/paints/studio` (new; `?id=` still opens one) and `/admin/cosmetics/paints/<id>/studio` (edit; the Paints list's Edit links there), `cosmetics.manage`; after the first save the address becomes the paint's own (shallow routing) and the catalog cache is dropped. Previews go through `lib/cosmetics/paint.ts`. Dev page `/dev/paint-studio` (404 in production). Tests: `src/lib/paint-studio/studio.test.ts` (colours, limits, CSS, import/export, API mapping, engine render and GIF).
- Manual check (2026/11/17): headless Edge against the dev mock at 1440 and 375, dark and light: a gradient paint saved, an animated paint rendered, uploaded, processed and saved as an IMAGE paint, an existing paint reopened (Open and `?id=`), integer JSON imported with its corrections listed, reduced motion pauses animated previews with a Play button; in the admin pages New paint → save → address moves to the paint, a second save updates it, Edit from the list opens it, an animated paint saves and lists. Screens in `.claude/runs/2026-11-17-web-v4/screens/studio/`.

## Public emote sets (branch `feat/public-sets`)

Gap list G1 (`.claude/runs/2026-11-17-overnight/gap-7tv.md`): sets you can share, list and copy, like 7TV's set pages (its public help pages and v3 schema studied; no code taken). API only; the pages come next.

- [x] Migration `20261130000001_set_visibility.sql` (additive): `emote_sets.visibility` (`listed`, `unlisted`, `private`). Existing rows keep what they could do: channel sets and the global set start listed, personal sets unlisted, nothing private. CHECKs: personal sets never private (chat shows them to everyone), the global set always listed.
- [x] ADR 0017 (set visibility). The activity feed (ADR 0016) now shows lines about listed sets only.
- [x] Rule: anyone opens listed and unlisted sets; a private set only its owner, per-set editors, the owner's account editors and `sets.edit_any`, or anyone while a channel shows it. `set(id)` and `GET /v1/sets/{id}` answer null / 404 otherwise, as for a set that doesn't exist.
- [x] GraphQL (additive): `SetVisibility`; `EmoteSet.visibility`, `listed`, `slotsUsed`, `createdAt`; `CreateSetInput.visibility`, `UpdateSetInput.visibility` (owner or account editors with `SETS`; per-set editors can't); `accountSets(accountId, preview)` (listed sets as `SetSummary` cards: facts, `active`, newest `preview` emotes; empty for unknown, blocked or deleted accounts); `cloneSet(id, name, ownerId, visibility)` returning `ClonedSet { set, copied, skipped }`; event type `set.created` (listed sets only, on the owner's `user:` topics). Schema, OpenAPI and web types regenerated.
- [x] Clone rules: only sets the caller may open; for yourself or an account you edit with `SETS` (audited as `set.created` with `source_id`, `copied` and `skipped`, and `via: account_editor` through a grant, so copies show in the activity feed); emotes go in as `addSetEmote` would (capacity 1000, approved only plus your own pending, others' private emotes skipped, names and zero-width kept); 30 clones an hour per account (`rate_limited`).
- [x] Tests: `apps/api/tests/public_sets_flow.rs` against Postgres in CI (defaults, private never readable by others over GraphQL and REST, active-set exception, account editors, `accountSets` of blocked and deleted accounts, clone of private and pending emotes, capacity with 1005 emotes, IDOR on `ownerId`, rate limit, audit row, `set.created`); unit tests of the visibility names.
- [x] Dev mock: sets carry visibility; nora_byte has listed, private and personal sample sets; `accountSets`, `cloneSet`, `createSet`/`updateSet` visibility and the private rule on `set(id)`.
- [ ] Website: `/sets/[id]`, profile set cards, clone and visibility controls (another task).

#### GitHub write log
```
git push -u origin feat/public-sets
gh pr create --base main --head feat/public-sets
git push origin feat/public-sets   (merges of origin/main, a test fix)
gh pr merge 53 --merge   (all checks green; public_sets_flow ran against Postgres in CI)
```

## Website v4: the profile is the dashboard, cosmetics, admin cosmetics (branch `web/v4`)

7TV's signed-in structure, in the cyex design (one shell, one accent, selection = bold text + accent icon).

- [x] `/users/<id>` is the one profile page. Everyone: Uploaded emotes. The owner and the account's editors also get Active emotes, Emote sets (cards: two rows of emotes, capacity meter, active set marked, New set), Cosmetics (owner, or editors with PROFILE) and Channels (owner). Rail head: avatar ringed in the paint, name in the paint with the badge, role chips, collapsible Connections and Editors; "Admin ›" pinned for staff. Phones: profile card on top, sections as a tab strip.
- [x] `/me`, `/me/sets` and `/me/cosmetics` redirect to the profile sections; the other `/me` pages keep their routes and show the profile rail with their row marked.
- [x] Account menu: Profile, Your emotes, Cosmetics, Account, Admin, Sign out; its head shows the name in the paint, the badge and the highest role.
- [x] Paint renderer `lib/cosmetics/paint.ts` (strict colour check, clamped values, safe image urls, at most 4 shadows; vitest), `PaintedName`, `BadgeIcon`, a batched `userCosmetics` cache; paints on the profile, the account menu, channel pages, admin Users and admin lists.
- [x] Cosmetics picker: badge and paint radio groups with None, search, grid/list and a "Paint name / Your name" preview; saves with `selectCosmetics` (with `ownerId` for edited accounts); empty states.
- [x] Admin rail: Overview; Tickets (Review, Reports with counts); Cosmetics (Paints, Badges); People; System. Overview: welcome line and count cards. Paints: list with preview, owners, Give, Delete, Edit and New paint go to the paint studio placeholder (`/admin/cosmetics/paints/studio`, to be replaced by the ported paint tool). Badges: list, New/Edit dialog with picture upload and ready polling, Give, owners with Revoke. Users: owned cosmetics, Give with a reason, Revoke.
- [x] Activity (public, ADR 0016): `accountActivity` through the adapter in `lib/profile/activity.ts`, rows with icon, sentence (emote picture, set, paints in the paint), who and how long ago, paged by 30.
- Manual check: screenshots at 375 and 1440, dark and light, with the dev mock (`.claude/runs/2026-11-17-web-v4/screens`). Open questions: `.claude/runs/2026-11-17-web-v4/questions.md`.
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
- [x] Browse parity (`feat/search-browse-parity`, web only): `/search` grid or table view (`view=`, remembered per device; Channels and Added headers sort like Top and New), exact-name match (`exact=1`, client side: the API has no arg for it), opt-in multi-select (`select=1`, Space toggles the focused tile) with a sticky bar that adds the picked emotes to one of your sets with the existing `addSetEmote`, one after another, respecting capacity. Trending is not offered: the API has no honest way to back it yet (no per-week adds). Creators' paints in the table come from the shared cosmetics cache, so only your own show (the API serves paints by platform id only). Checked with the mock at 375, 1440 and 2560, dark and light, keyboard; screens in `.claude/runs/2026-11-17-overnight/screens/g4/`.

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
- [x] App sessions: `Authorization: Bearer`, kind `app`, 90 days. Accepted, not yet issued (D19). Issued since ADR 0021 through app sign-in (1 hour access, 60-day rotating refresh).
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

## Hero shadows (branch `feat/hero-shadows`)

Owner feedback 2026/11/19: "the landing page hero looks better now but it needs shadows to look more realistic". Web only; nothing was deployed.

- [x] Real shadows between the bars: the key light is now a spot (it falls off across the mark, so faces read with depth) casting a soft PCF shadow map (1536 px, wide noisy kernel; three 0.187 folded the old "soft" mode into `PCFShadowMap`); every bar casts and receives.
- [x] Ground: a transparent shadow-catcher floor under the mark (tilted with the logo's pitch, faded out toward its edges so no floor edge shows), a soft contact shadow that loosens and lightens as the mark floats up and follows the lit bar coming forward, and a faint pool of the lit bar's light.
- [x] Both themes: dark has deep shadows and an accent pool added as light; light has lighter, cooler, softer ones (lower shadow intensity, wider kernel).
- [x] Still images `static/home/logo-3d-{dark,light}.webp` remade with `window.__logoSnap()`; reduced motion shows them. Lazy load, off-screen pause and CLS unchanged (CLS 0).
- Dev-only probes (stripped from production): `?logoperf` times each draw, `window.__logoBench(n)` the mean GPU-synced draw.
- Frame cost (headless Edge on a fast GPU, `__logoBench`, mean of 5 x 200 draws): about 0.04 ms before, 0.07 ms after at 1x; with CPU throttle 4x about 0.2-1.0 ms before, 0.3-0.9 ms after. The 5 s rAF sample is vsync-less here (360 Hz), p95 2.9 ms, 0 frames over 20 ms, CLS 0, so it can only show the headroom: the added work is one more depth pass over three meshes and three small transparent quads.
- Screens in `.claude/runs/2026-11-19-feedback/screens/hero-shadows/`.

### GitHub write log for this PR
2026/11/19, account `sery-yx`.

```
git push -u origin feat/hero-shadows
gh pr create --base main --head feat/hero-shadows --title "Hero: real soft shadows, a ground shadow and light falloff" --body-file <file>
gh pr merge <n> --merge
```

## 2026/11/20 Test gaps (test/gaps)
- Web: vitest now has two projects, `unit` (node) and `ui` (jsdom, `*.ui.test.ts`, with @testing-library/svelte and user-event; jsdom shims in `src/lib/test/dom-setup.ts`). New tests: loader, inbox, hooks.server headers, the CSP in svelte.config.js, the public pages' load functions, docs behaviour, Popover, TagInput, Toasts, SelectBar, HeaderSearch keyboard. `npm run test:coverage` prints a coverage summary (CI uses it, no threshold). `npm run test:e2e` runs dev/e2e/cosmetics.mjs, which needs the dev mock, vite dev and a headless browser on a debug port (setup in the file's header); it is not in CI.
- Rust: sleeps used as synchronisation in apps/api/tests are polling helpers with a deadline (`common::wait_until`, `wait_for_feeders`); `hot_reads_use_their_indexes` runs VACUUM ANALYZE and asserts that an index of the table was used.
## Set editor paging, Add emotes paging, transfer notification state (2026/11/20)
- API (additive): `setEmotes(id, query, page, perPage)` returns a page of a set's emotes, oldest first, with `total`; the visibility rule of `set`; `query` filters by name in the set or own name; `perPage` 1 to 200, default 120. `Notification.transferState` shows where a transfer request stands now (pending, accepted, declined, cancelled, expired); null for other kinds. Postgres tests: `set_emotes_pages_filters_and_keeps_visibility`, `transfer_notifications_show_where_the_request_stands`.
- Web: `/me/sets/[id]` reads 120 emotes at a time with Load more, Filter asks the API (whole set), ids and names for Add come as slim pages. Add emotes has Load more with "Showing 40 of N". A transfer-request notification whose request was answered, withdrawn or expired elsewhere shows that, not Accept and Decline.
- Dev mock: `MOCK_SET_SIZE=1000` fills the Main set.
- At 1.000 emotes (headless Edge, dev server, warm): first render about 600 ms before, 320 ms after; JS heap 95 MB before, 20 MB after; 1000 tiles before, 120 after.
- Screens in `.claude/runs/2026-11-20-overnight/screens/f-u2b/`.
## Announcements, notification preferences, public collection (2026/11/20, K3)
- API (additive): `activeAnnouncement` (public, shared per instance for the read-cache time), `announcements`, `createAnnouncement`, `updateAnnouncement`, `endAnnouncement` (need the new `site.announce`, which the built-in Admin role gets; audited as `announcement.create/update/end`). One line (1 to 200 characters), kind `INFO`/`MAINTENANCE`/`CAMPAIGN`, optional link (`https://` or a path here), link words, approved emote or badge, start and end. `myNotificationPrefs` / `setNotificationPrefs`: per kind `notify` (off: `notifications::create` stores nothing) and `toast`; hand-over requests and takedowns are required. `accountCollection(accountId)` / `setCollectionPublic(public)`: owned badges and paints with `ownedSince` and `source` only, private by default; `AccountProfile.collectionPublic`. New tables `announcements`, `account_prefs` (erased with the account). Postgres tests in `apps/api/tests/site_flow.rs`; coverage sweep and contract updated.
- Web: the announcement bar under the header on every page (closed per device and per announcement); `/admin/announcements` (System group); Settings → Notifications; filter chips on `/me/notifications`; pop-ups skip kinds whose pop-up is off; the profile's Collection tab (owner always, others when shown) with the switch there and in Settings → Account; `site.announce` in the role editor (System). Changelog entry.
- Dev mock: all of the above; `MOCK_ANNOUNCE=0` starts without an announcement; `/mock/notify?kind=…` respects turned-off kinds.
- Screens in `.claude/runs/2026-11-20-overnight/screens/k3/`.
- GitHub write log (2026/11/20, account `sery-yx`): `git push -u origin feat/k3-banner-prefs-collection`; `gh pr create --base main --head feat/k3-banner-prefs-collection --title "Site announcements, notification preferences and a public collection" --body-file <file>` (#125); `gh pr merge 125 --merge` once green.
## Hide for me and favourites (feat/k2-hide-favourites, 2026/11/20)
- API (additive, ADR 0022): `hiddenEmotes`, `hiddenCreators`, `myFavourites(query, page, perPage)`, `favouriteEmoteIds`; `hideEmote`/`unhideEmote`, `hideCreator`/`unhideCreator`, `favouriteEmote`/`unfavouriteEmote`. Private to the caller (no account argument), caps 1000 / 200 / 1000 (`limit_reached`), only emotes the caller may see, never yourself as a creator (`own_account`), 120 changes a minute per account (`personal_lists`). In the data export, erased with the account. Migration `20261208000920_personal_lists.sql`. Postgres tests: `personal_lists_flow` (allow/deny, caps, privacy, rate, erase) and the contract run.
- Web: tile menu Favourite / Hide for me / Hide everything by <creator> (with Undo); emote page heart beside More, More → Hide for me / Hide everything by <creator> (Show again once hidden); hidden emotes drop out of browse, search, trending, the header search and Add emotes for you only (client-side filter, see the ADR); Settings → Hidden lists both with Show again; profile tab Favourites (owner only) with a name filter, `/me/favourites` goes there; favourites come first, with a heart, in the header search and the set editor's Add emotes.
- Extension: unchanged. It has no cyex.tv sign-in, so its device-only hidden list stays as it is.
- Dev mock answers all of it.
- GitHub write log:
```
git push -u origin feat/k2-hide-favourites
gh pr create --repo cyex-tv/platform --base main --head feat/k2-hide-favourites --title "Hide for me and favourites (personal lists)" --body-file <file>
gh pr merge 126 --merge
```
## Review queue and bulk admin (feat/k4-review-bulk, 2026/11/20)
- API (additive): `reviewEmotes(ids, verdict, visibility, reason)` approves or rejects up to 100 pending emotes in one transaction, one audit row and one notice each, emotes that aren't pending listed in `failed`; `dismissReports(ids, note)` the same for open reports (takedowns stay one by one); `emoteDuplicates(emoteId)` lists emotes from the same file (sha256) and with close names (pg_trgm, 0.5 and up), for `emotes.review`; `emotesWithSameFile(sha256)` lists public approved emotes made from that file, signed in, for the upload form; `reviewReasons` / `setReviewReasons(reasons)` keep the ready reject reasons (table `review_reasons`, seeded with the six the web carried; new permission `review.settings`, given to the built-in Admin role; audited `review.reasons_changed`); `accountsByLogin(logins)` matches up to 100 pasted logins (`name`, `@name`, `twitch:name`, `kick:name`) to accounts, or says `not_found`, `ambiguous` or `bad_login`. Index `emotes_file_id`. Postgres tests: `apps/api/tests/review_batch_flow.rs`.
- Web: /admin/review counts what is left, S skips (on to the next, no verdict) on the grid and in the detail dialog, cards can be ticked (X, Shift+X or Shift+click for a range, Ctrl/Cmd+A for the page, Esc clears, cap 100) and a bar approves (with the 5 s undo), approves as unlisted or for their owners only, or rejects with one reason; the detail dialog lists possible duplicates; the reject dialog offers the reasons from the API, edited on /admin/review/reasons. /admin/reports ticks rows and dismisses all their reports with one note. /admin/grants ("Give to many") checks a pasted list of logins and gives one badge, paint or role to each match, row by row. The upload form says "Looks like <emote> by <owner>" when the file is already a public emote, with a link; it never blocks. Changelog entry.
- Checked in the running app (dev mock on 9301, vite 5701, headless Edge) at 1440, 2560 and 375, dark and light, with the keys; screens in `.claude/runs/2026-11-20-overnight/screens/k4/`.
- Review fixes before merge: `accountsByLogin` takes from the account search limit; saving the reasons holds an advisory lock (two saves take turns, Postgres test); the same-file check uses the shared listed-emote rule (`search::LISTED_EMOTE`); approvals waiting out their undo stay off the review page across reloads (cards and counts), checked with a CDP run (approve X and Y, undo X, refresh: Y stays hidden, counts down by one).
- GitHub write log (2026/11/20, account `sery-yx`): `git push -u origin feat/k4-review-bulk`; `gh pr create --base main --head feat/k4-review-bulk --title "Review queue: bulk verdicts, skip, duplicates, editable reasons; bulk dismissals; give to many; same-file note on upload" --body-file <file>`; `gh pr merge <n> --merge`.
## Moderator emote flags and requeue on undo (feat/mod-flags-sessions, 2026/11/20)
- API (additive, ADR 0024): permission `emotes.flags` ("Edit emote flags (incl. hidden ones)", Admin gets it by migration); `setEmoteFlags(id, input: { visibility, nsfw, zeroWidth })` for holders (and `emotes.edit_any`), any state, audited as `emote.flags_changed`, owner told on a visibility change (`emote.visibility_changed`, always on); `accountEmotes` readable with `emotes.flags`. `requeueEmotes(ids)` (`emotes.review`, up to 100, audited `emote.requeued`) sets the new `emotes.requeued_at`; the queue's upload-time order reads `coalesce(requeued_at, created_at)`; a verdict clears it.
- Web: an "Emote flags" panel for holders on the emote page, the review detail (an "Emote flags" tab for resolved uploads) and the admin user page's uploads (Flags per row); role editor label and tooltip in the Emotes group; Undo in the review queue puts the card(s) at the end and calls `requeueEmotes`; history lines for both actions.
- Tests: Postgres `apps/api/tests/mod_flags_flow.rs` (allow/deny/audit/notice, requeue order for two reviewers, verdict clears it); vitest `toEnd` and history lines.
- Dev mock: `SetEmoteFlags`, `RequeueEmotes`, the mock admin holds `emotes.flags`.
## Retire unscoped app sessions (feat/retire-unscoped-sessions, 2026/11/20)
- Owner decision (2026/11/20). Migration `20261210005001_retire_unscoped_app_sessions.sql` signs out `app` sessions without a client and adds CHECK `sessions_app_has_client`; `Auth::caller` ignores a Bearer token whose session names no client. The website cookie session is unaffected. ADR 0021 amended.
- Tests: Postgres `unscoped_app_sessions_are_retired` (crates/db, runs the migration's own SQL) and `unscoped_bearer_token_is_signed_out` (apps/api); the API tests that used an unscoped app token as a full-rights caller now use a site session (cookie plus `Origin`), and the deck-session tests make real app sign-in sessions (`app_auth::create_session`).

## Idempotency keys (feat/idempotency-counts, 2026/11/20)
- API (additive, ADR 0023): optional `idempotencyKey` (UUID) on `startCheckout`, `redeemCode`, `approveEmote` and `reviewEmotes`. Table `idempotency_keys` (account, operation, key, sha256 of the request, answer), 24 h, purged by the worker's clean-up, erased with the account. Repeat with the same key and request: the stored answer; still running: `idempotency_in_progress`; another request: `idempotency_key_reused`; a failed request gives its key back; `bad_idempotency_key` when it isn't a UUID.
- Web: one key per purchase (item + recipient) and per code, reused when the same action is tried again, dropped once a request answers. Timeout sentences now say trying again is safe. Dev mock answers keyed repeats the same way.
- Postgres tests: `shop_flow::a_checkout_retried_with_its_key_starts_one_order` (two at once, one order; after payment the retry answers the first checkout; another item with the key refused; per account; failure gives the key back; expiry), `shop_flow::a_redeem_retried_with_its_key_redeems_once` (one redemption, conflict, no plain code stored), `review_batch_flow::verdicts_retried_with_their_key_answer_the_first_outcome`. Vitest: the checkout key is reused after a failed start and new after one that answered.
- Checked in the running app (mock 9341, vite 5741, headless Edge 9841): Undo puts the card last and keeps it last after a reload (`RequeueEmotes` sent); the review detail's Emote flags tab saves (`SetEmoteFlags`), updates Visibility and the history; the emote page panel at 375, 768, 1440 and 2560 in dark and light, keyboard focus, no sideways scroll; the admin user page's Flags row. Screens in `.claude/runs/2026-11-20-overnight/screens/w2/`.
## Stored channel counts (feat/channel-counts, 2026/11/20)
- Scale audit P1, owner's call: `emotes.channel_count`, at most about a minute old. Migrations `20261210005001_emote_channel_count.sql` (column with a constant default, no table rewrite; the queue table `emote_channel_count_queue`; triggers on `emote_set_members` (statement-level, transition tables), `channels` and `accounts` queue every emote whose count may have changed) and `20261210002002_emote_channel_count_backfill.sql` (no-transaction, 2000 emotes per committed batch). No new index.
- Worker: every minute `channel_counts::refresh_queued` recounts the queued emotes and removes only the queue rows it read; the hourly clean-up runs `channel_counts::reconcile_all` (every emote, 2000 per statement; covers timed blocks running out, which no write announces).
- Reads: `reads::channel_counts` reads the stored count (primary key); the live count is `channel_counts::live`, the worker's source of truth. API shape unchanged.
- Tests (Postgres, CI): `crates/db/tests/channel_counts.rs` (adds, removes, set switches, blocks, deletes, set and channel removal, stored equals live; reconcile repairs what no write announced; long queues in rounds), `search_flow` refreshes as the worker would, `scale::hot_reads_use_their_indexes` checks the stored read and the refresh read no table whole. Timings in the `load` job (PR labelled `scale`).
