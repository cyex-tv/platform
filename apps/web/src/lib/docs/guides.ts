// The guide sections of /developers, as data: the page renders them in order, and the index's
// Notes group lists them. Inline `code` and [links](#anchor) are written as in the API spec.
import { EX, paragraphs, type Segment } from './example';
import spec from '../../../../../openapi/openapi.json';
import { baseUrl } from './rest';

export type Block =
  | { t: 'lede'; text: Segment[][] }
  | { t: 'p'; text: Segment[][] }
  | { t: 'ol' | 'ul'; items: Segment[][][] }
  | { t: 'base'; rows: [string, string][] }
  | { t: 'fmt'; rows: [string, Segment[][]][]; plain?: boolean }
  | { t: 'codes'; rows: ErrorCodeRow[] }
  | { t: 'note'; text: Segment[][] }
  | { t: 'pre'; code: string };

export interface ErrorCodeRow {
  code: string;
  group: string;
  meaning: string;
  when: string;
}

/** Every stable error code, from `components.schemas.ErrorCode` of the spec (the one list in the API). */
export const errorCodes: ErrorCodeRow[] = (spec.components.schemas as unknown as Record<string, { 'x-codes'?: ErrorCodeRow[] }>).ErrorCode?.['x-codes'] ?? [];

export interface Guide {
  id: string;
  title: string;
  /** What the filter matches besides the title. */
  search: string;
  blocks: Block[];
}

const ws = baseUrl.replace(/^http/, 'ws');
const SPEC = 'https://github.com/cyex-tv/platform/blob/main';

const lede = (s: string): Block => ({ t: 'lede', text: paragraphs(s) });
const p = (s: string): Block => ({ t: 'p', text: paragraphs(s) });
const note = (s: string): Block => ({ t: 'note', text: paragraphs(s) });
const ol = (...items: string[]): Block => ({ t: 'ol', items: items.map(paragraphs) });
const ul = (...items: string[]): Block => ({ t: 'ul', items: items.map(paragraphs) });
const fmt = (...rows: [string, string][]): Block => ({ t: 'fmt', rows: rows.map(([k, v]) => [k, paragraphs(v)]) });
/** A grid whose left column is a name, not a value. */
const table = (...rows: [string, string][]): Block => ({ ...(fmt(...rows) as Extract<Block, { t: 'fmt' }>), plain: true });

export const guides: Guide[] = [
  {
    id: 'start-here',
    title: 'Start here',
    search: 'basics intro overview chat client',
    blocks: [
      lede('cyex.tv hosts emotes for Twitch and Kick chat. This API is how chat clients, bots and tools read them.'),
      p('A chat client needs three reads and one socket:'),
      ol(
        `Look up the channel by its platform ID with [Get a channel](#rest-get_channel). It names the channel's active set.`,
        `Fetch that set with [Get a set](#rest-get_set), and [the global set](#rest-get_global_set), which shows in every channel.`,
        `Draw each emote from its files on the CDN, as in [Emotes and files](#emotes-and-files).`,
        `Keep it current on [the event socket](#ws-events) instead of polling.`,
      ),
      note('Reading is public: no key and no sign-in, and CORS is open to every site, so an extension can call it from twitch.tv or kick.com.'),
    ],
  },
  {
    id: 'base-urls',
    title: 'Base URLs',
    search: 'host server version v1 openapi schema',
    blocks: [
      {
        t: 'base',
        rows: [
          ['REST', `${baseUrl}/v1`],
          ['GraphQL', `${baseUrl}/v1/gql`],
          ['Events', `${ws}/v1/events`],
          ['Files', 'https://cdn.cyex.tv'],
        ],
      },
      fmt(
        ['/v1/…', 'REST: the cacheable reads, sign-in and uploads. JSON, fields in `snake_case`.'],
        ['/v1/gql', 'GraphQL: everything else. Fields in `camelCase`, enums in capitals (`TWITCH`).'],
        ['/v1/events', 'The event socket, JSON frames, for chat clients.'],
        ['/v1/gql/ws', 'The same events as GraphQL subscriptions, over `graphql-transport-ws`.'],
      ),
      p('Everything sits under `/v1`. Inside v1 we only add fields, types and endpoints. Anything going away is marked deprecated first and only goes in v2.'),
      p(`This page is built from the machine-readable descriptions: [openapi.json](${SPEC}/openapi/openapi.json) and [schema.graphql](${SPEC}/schema/schema.graphql).`),
    ],
  },
  {
    id: 'ids',
    title: 'IDs',
    search: 'id ids snowflake ulid string number 64-bit store',
    blocks: [
      p(`The IDs of emotes, accounts and sets are Snowflakes: numbers written as strings of digits, like \`${EX.emoteId}\`. They fit in 64 bits, which is more than a JavaScript number holds exactly, so always store and send them as strings. The same goes for every field that points at one (\`owner_id\`, \`set_id\`, \`emote_id\`, \`active_set_id\` and the rest), in REST, GraphQL and the event socket.`),
      p('A Snowflake is not a secret and has no meaning you should depend on, but it does sort by age: a bigger one was made later. Every other kind of ID (badges, paints, roles, orders, images, notifications) is still a 26-character ULID.'),
      note('Emotes, accounts and sets used to have ULIDs. Until 2027/06/01 (see [Versioning and deprecations](#versioning)) every place that takes one of those IDs also takes the old ULID, and `/emotes/<ULID>`, `/users/<ULID>` and `/sets/<ULID>` on the site send you to the new address. Answers always carry the Snowflake, so switch what you store to those, as strings. Event topics are `set:<Snowflake>`; `set:<ULID>` still subscribes.'),
    ],
  },
  {
    id: 'signing-in',
    title: 'Signing in',
    search: 'auth authentication session cookie origin login',
    blocks: [
      p('Public reads need nothing. Acting for an account (uploads, sets, channel settings, moderation) needs a session.'),
      p('A session comes from signing in with Twitch or Kick on cyex.tv. [Start sign-in](#rest-login) sends the browser to the platform; [Finish sign-in](#rest-callback) sets the `cyex_tv_session` cookie. It lasts 30 days and renews while it is used.'),
      p('The cookie is `HttpOnly` and `SameSite=Lax`, and a write that carries it must send `Origin: https://cyex.tv`. That keeps other sites from acting for you. The samples for signed-in calls read your own cookie from `CYEX_TV_SESSION`, so you can try them against your account.'),
      fmt(
        ['signed_out', 'No session, or it ran out. Sign in again.'],
        ['forbidden', 'Signed in, without the permission the call names.'],
        ['bad_origin', 'The cookie came from another site.'],
      ),
      note('Desktop apps get a Bearer token instead of the cookie: see [App sign-in](#app-sign-in).'),
    ],
  },
  {
    id: 'app-sign-in',
    title: 'App sign-in',
    search: 'app desktop deck oauth pkce loopback bearer token refresh client_id scope consent',
    blocks: [
      lede('A desktop app such as cyex deck signs in through the browser and gets a Bearer token. It never sees a Twitch or Kick token, and the person can sign it out from the website.'),
      p('The flow is OAuth 2.0 authorization code with PKCE (S256) and a loopback redirect (RFC 6749, 7636 and 8252). The app is a public client: there is no secret.'),
      ol(
        'Make a random verifier (43 to 128 characters of `A-Z a-z 0-9 - . _ ~`), its challenge `base64url(sha256(verifier))` without padding, and a random `state`. Listen on `http://127.0.0.1:<any free port>/callback`.',
        'Open [Start app sign-in](#rest-authorize) in the browser with `response_type=code`, `client_id`, `redirect_uri`, `code_challenge`, `code_challenge_method=S256` and `state`. The person signs in on cyex.tv if needed and sees what the app asks for, with Allow and Deny.',
        'The browser comes back to your listener with `code` and your `state`, or with `error=access_denied`. Check that `state` is yours.',
        'Trade the code at [Get app tokens](#rest-token) with `grant_type=authorization_code`, `code`, `code_verifier`, the same `redirect_uri` and `client_id`, form-encoded. Send `access_token` as `Authorization: Bearer <token>`.',
        'Before it runs out, refresh at the same endpoint with `grant_type=refresh_token`, `refresh_token` and `client_id`. Keep the new refresh token: each one works once.',
        'To sign out, call [Sign an app out](#rest-revoke) with either token, or [Sign out](#rest-logout) with the Bearer token.',
      ),
      { t: 'pre', code: `curl -s -X POST '${baseUrl}/v1/auth/app/token' \\\n  -d grant_type=authorization_code \\\n  -d client_id=cyex-deck \\\n  -d code=… -d code_verifier=… \\\n  -d redirect_uri=http://127.0.0.1:53682/callback\\n\\n{ "access_token": "…", "token_type": "Bearer", "expires_in": 3600,\\n  "refresh_token": "…", "scope": "sets.emotes cosmetics.select" }` },
      table(
        ['client_id', '`cyex-deck` (cyex deck). Apps are registered in the platform’s code; there is no self-service registration yet.'],
        ['redirect_uri', '`http://127.0.0.1:<port>/callback` or `http://[::1]:<port>/callback`, any port. `localhost`, https and custom schemes are refused.'],
        ['Code', 'Works once, within 2 minutes. A wrong verifier uses it up; using it twice signs out what it gave.'],
        ['Access token', '1 hour. Then the API answers `signed_out` until you refresh.'],
        ['Refresh token', '60 days, starting again at every refresh. Each works once: presenting a used one signs the whole session out, so never retry a refresh with an old token.'],
        ['Signing out', 'Sign out everywhere, on your profile’s Account tab, signs the app out too, with every other session.'],
        ['Limits', 'The sign-in limit (30 a minute per IP) for `/v1/auth/app/*`; GraphQL counts like any account.'],
      ),
      p('Scopes say which mutations the token may run. Queries are limited to public reads, your own account, sets, emotes, channels and cosmetics, and `channelEditableSets`; any other query or mutation answers `insufficient_scope`, and uploads are refused. The usual rights checks still apply: set owner, account editors with `EMOTES`, set editors and set capacity. Staff permissions never count for an app token.'),
      fmt(
        ['sets.emotes', 'Manage emotes in sets you can edit: [addSetEmote](#gql-mutation-addSetEmote), [updateSetEmote](#gql-mutation-updateSetEmote), [removeSetEmote](#gql-mutation-removeSetEmote).'],
        ['cosmetics.select', 'Pick your cosmetics: [selectCosmetics](#gql-mutation-selectCosmetics), with [myCosmetics](#gql-query-myCosmetics) to list what you own.'],
      ),
      p('To know what you may change in a channel before sending a command, ask [channelEditableSets](#gql-query-channelEditableSets) with the platform and the channel’s ID or login: it answers the channel (with `activeSetId`), your role, `canEditActiveSet`, and each set you may change with how (`OWNER`, `ACCOUNT_EDITOR`, `SET_EDITOR`, `STAFF`) and its `capacity` and `slotsUsed`.'),
      fmt(
        ['invalid_grant', 'The code or refresh token is wrong, expired or used. Start signing in again.'],
        ['invalid_client', 'Unknown `client_id`.'],
        ['insufficient_scope', 'The token may not run that query or mutation. Do it on cyex.tv.'],
      ),
    ],
  },
  {
    id: 'api-token',
    title: 'Your API token',
    search: 'token console bearer personal key copy devtools playground script revoke cyextv',
    blocks: [
      lede('To try the API as yourself (a GraphQL playground, a script, a one-off call), take a personal API token from the browser console. It is a Bearer token that acts as you.'),
      ol(
        'Sign in on cyex.tv and open the browser’s developer tools, then the Console.',
        'Run `copy(await cyextv.token())`. The token is now on your clipboard; the page doesn’t keep it anywhere.',
        'Send it as `Authorization: Bearer <token>` to `/v1/gql` and to the REST calls that need a session.',
      ),
      { t: 'pre', code: `curl -s '${baseUrl}/v1/gql' \\\n  -H 'authorization: Bearer <token>' \\\n  -H 'content-type: application/json' \\\n  -d '{"query":"{ me { id displayName } }"}'` },
      table(
        ['Rights', 'Your account’s own: what you can do on the site, with your roles. Owner-only work and the website-only actions (deleting the account, downloading your data, signing other sessions out, making another token) don’t work with it.'],
        ['Lifetime', '3 days from when you made it, however much it is used. Your account’s Account tab shows when it expires and makes a new one.'],
        ['One at a time', 'Making a new token signs the last one out. Five an hour.'],
        ['Revoking', 'Signing out anywhere ends it: Sign out in any browser or app, and Sign out everywhere (your account’s Account tab). So do [Sign out](#rest-logout) with the token, [revokeSession](#gql-mutation-revokeSession) with its `session_id`, and making a new one. [mySessions](#gql-query-mySessions) lists it as `Console token`, and [myConsoleToken](#gql-query-myConsoleToken) shows its expiry.'],
      ),
      p('Behind `cyextv.token()` is [Make a personal API token](#rest-export_token) (or the [createConsoleToken](#gql-mutation-createConsoleToken) mutation), which only the website’s own session can call.'),
      note('Anyone with the token can act as you until it is signed out or runs out. Don’t paste it into sites or chats, and sign it out when you are done. The website’s own session stays in an `HttpOnly` cookie that scripts can’t read; this is the only way to hold a token.'),
    ],
  },
  {
    id: 'errors',
    title: 'Errors and caching',
    search: 'error codes etag 304 cache if-none-match',
    blocks: [
      p('REST errors are `{ "error": "not_found", "message": "No emote with that ID." }`. Match on `error`, a stable code; `message` is for people. GraphQL errors carry the same codes in `extensions.code`, and an answer can hold data and errors together.'),
      p('Every code the API answers with is listed in [Error codes](#error-codes).'),
      p('The reads send an `ETag`. Send it back as `If-None-Match` and you get `304` with no body while nothing changed: poll that way and a channel costs almost nothing.'),
      { t: 'pre', code: `curl -s -i '${baseUrl}/v1/sets/${EX.setId}' \\\n  -H 'if-none-match: "3f9a1c"'\n\nHTTP/2 304\netag: "3f9a1c"` },
      p('GraphQL queries are limited to a depth of 12, a complexity of 1.000 and 50 top-level fields (aliases count). The searches cost 50 each and single lookups such as `emote` or `channel` 10, so ask for many things in one list query, not in many aliases.'),
    ],
  },
  {
    id: 'rate-limits',
    title: 'Rate limits',
    search: 'limits 429 retry-after throttle',
    blocks: [
      p('Limits are per IP, and on GraphQL also per account. Past one you get `429` with the code `rate_limited` and a `Retry-After` header in seconds. Wait that long, then go on. Every `429` has `Retry-After`, the event socket and the open-socket cap included.'),
      p('The public reads and `POST /v1/gql` also say where you stand on every answer, with the `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset` headers: the size of the allowance, the requests left in it, and the seconds until it is full again. On GraphQL, a signed-in call counts under the per-account allowance. Pace yourself with them and you never see a `429`.'),
      table(
        ['Reads', '1.200 a minute per IP. Most of them are answered by the cache.'],
        ['GraphQL', '300 a minute per IP, and 600 a minute per account.'],
        ['Sign-in', '30 a minute per IP, app sign-in and refreshes included.'],
        ['Uploads', '30 an hour per IP, and 20 an hour per account.'],
        ['Reports', '10 an hour per account, one open report per emote.'],
        ['Event socket', '60 connections a minute per IP, 50 open at once; 10 frames a second and 16 KB a frame from a client.'],
        ['GraphQL socket', '60 connections a minute per IP, 50 open at once; 64 KB a message, `connection_init` within 10 s, 8 subscriptions per socket.'],
      ),
    ],
  },
  {
    id: 'upload-limits',
    title: 'Upload limits',
    search: 'limits upload size bytes side frames pixels duration name tags quota effective role badge paint emote',
    blocks: [
      p('Every upload and field limit is one public list: the GraphQL `limits` query, or `GET /v1/limits` over REST. Per kind (`emote`, `badge`, `paint`) it gives `maxBytes`, `maxSide`, `maxFrames`, `maxTotalPixels` and `maxDurationMs` (null: no cap), the smallest side and widest shape a picture may have, and the tallest file the CDN serves. It also lists the name and tag lengths and the per-IP request rates. Read it instead of copying numbers: an operator can change the media limits, and they change here with them.'),
      p('Signed in, the GraphQL `limits.account` lists your own per-account limits (`uploads_per_hour`, `pending_emotes`, `set_capacity_max` and the rest) with the `default` and what applies to you, `effective`, after your roles’ overrides. `uploadQuota` adds the file limits an upload for that account is checked against: `maxBytes`, `maxSide` and `maxFrames`, next to `pendingMax` and `uploadsPerHour`.'),
      note('An emote is at most 1000 × 1000 px whatever the operator sets, and the CDN serves it at most 128 px tall.'),
    ],
  },
  {
    id: 'paging',
    title: 'Paging',
    search: 'paging pagination cursor after next page perpage link rel next bad_cursor',
    blocks: [
      p('Long lists page with a cursor. Every page of `searchEmotes`, `searchAccounts`, `setEmotes`, `myEmotes`, `accountEmotes` and `reviewQueue` carries `next`: pass it back as `after` (with the same filters, order and `perPage`) for the page that follows. `next` is null on the last page. The order stays the one the list documents, and nothing is shown twice or skipped while rows are added or removed between pages, which numbered pages can’t promise.'),
      { t: 'pre', code: `query {\n  searchEmotes(query: "pog", sort: NEW, perPage: 50, after: "eyJ2Ijox…") {\n    total\n    next\n    items { id name }\n  }\n}` },
      p('Cursors are opaque: don’t read or build them, and keep one only as long as you page through that list. A cursor that was changed, made up or comes from another list (other text, filters or order) answers `bad_cursor`; start again from the first page. The page size caps hold as before, and `page` is ignored when `after` is given.'),
      p('`page` and `perPage` still work as they always did. With a cursor there is no deepest page; numbered pages stop at page 1.000.'),
      p('Over REST, [a set’s emotes, a page at a time](#rest-get_set_emotes) answers `next` too, and a `Link` header with the next page’s address:'),
      { t: 'pre', code: `curl -s -i '${baseUrl}/v1/sets/${EX.setId}/emotes?per_page=120'\n\nHTTP/2 200\nlink: </v1/sets/${EX.setId}/emotes?per_page=120&after=eyJ2Ijox…>; rel="next"` },
    ],
  },
  {
    id: 'bulk-reads',
    title: 'Bulk reads',
    search: 'bulk batch many ids lookup emotes sets channels too_many cap',
    blocks: [
      p('Read many emotes, sets or channels in one call instead of one call each. GraphQL `emotes(ids:)` and `sets(ids:)` take emote and set IDs, `channels(platform:, ids:)` broadcaster platform IDs. Over REST, [POST /v1/sets/lookup](#rest-lookup_sets) takes `{ "ids": [...] }` and [POST /v1/channels/lookup](#rest-lookup_channels) takes `{ "platform": "twitch", "ids": [...] }`, like the personal-set lookup.'),
      { t: 'pre', code: `query {\n  sets(ids: ["${EX.setId}", "99908371611660331"]) { id version emotes { name emote { id } } }\n  channels(platform: TWITCH, ids: ["${EX.twitchId}"]) { login activeSetId }\n}` },
      p('Answers keep the order you asked in and list each item once. Unknown items, and items you may not see, are left out rather than null: who sees what is the rule of the single read (`emote`, `set`, `channel`). The REST lookups answer as signed out, approved emotes only, as the other chat-client reads do.'),
      p('At most 100 emotes, 100 channels or 25 sets per call; more is `too_many`. Sets are capped lower because a set can hold 1.000 emotes. A REST lookup counts as one read against your rate limit; in GraphQL `sets` costs what that many `set` lookups would, and `emotes` and `channels` cost one lookup per 10 IDs.'),
      { t: 'pre', code: `curl -s -X POST '${baseUrl}/v1/sets/lookup' \\\n  -H 'content-type: application/json' \\\n  -d '{"ids":["${EX.setId}"]}'` },
    ],
  },
  {
    id: 'delta-sync',
    title: 'Delta sync',
    search: 'delta sync since version changes setChanges resync full etag poll',
    blocks: [
      p('Every set has a `version` that goes up whenever what it shows changes: an emote added, removed or renamed in it, the set renamed, or an emote in it changed (approved, taken down, its files ready). Keep the version with your copy of the set, then ask only for what changed after it: [GET /v1/sets/{id}?since=<version>](#rest-get_set) over REST, `setChanges(id:, since:)` in GraphQL.'),
      { t: 'pre', code: `curl -s '${baseUrl}/v1/sets/${EX.setId}?since=41'\n\n{ "id": "${EX.setId}", "version": 44, "since": 41, "full": false, "set": null,\n  "name": "Main", "added": [ … ], "renamed": [ … ], "updated": [ … ], "removed_ids": ["${EX.emoteId}"] }` },
      p('Apply `added`, `renamed` and `updated` as upserts by emote ID (each is a whole set emote, as the set shows it now), remove the emotes in `removed_ids` (`removedIds` in GraphQL), and keep the new `version`. Applying the same answer twice is harmless. With nothing new the lists are empty and `version` is the one you sent.'),
      p('The change log keeps 7 days, and at most the last 1.000 versions of a set. When `since` is older than that, newer than the set, or too far behind, the answer is `full: true` with the whole set in `set`: replace your copy. `ETag` and `If-None-Match` work on both answers. A delta doesn’t follow an emote’s `channel_count`, its owner’s name or its tags; read the full set now and then if you show those.'),
      note('Live events (`set.emote_added` and the rest) still come first. Use deltas to catch up after a reconnect, or instead of rereading the whole set when its `ETag` changes.'),
    ],
  },
  {
    id: 'persisted-queries',
    title: 'Persisted queries',
    search: 'persisted queries apq automatic get cache cdn sha256 hash persistedquerynotfound post_required',
    blocks: [
      p('GraphQL queries can go over `GET`, so browsers and the CDN can cache them. Send the query in the address (`query`, `variables` and `operationName`, the last two as JSON), or send only its hash as an automatic persisted query: `extensions={"persistedQuery":{"version":1,"sha256Hash":"<sha-256 of the query text, lowercase hex>"}}`.'),
      p('An unknown hash is answered with the error `PersistedQueryNotFound` (code `persisted_query_not_found`): send the same request once more with the `query` text, which stores it, and from then on the hash alone is enough. A hash that isn’t the text’s is `persisted_query_mismatch`. Apollo Client’s persisted-queries link does all this for you. Persisted queries work over `POST` too.'),
      { t: 'pre', code: `curl -s -G '${baseUrl}/v1/gql' \\\n  --data-urlencode 'query={ globalSet { id version } }' \\\n  --data-urlencode 'extensions={"persistedQuery":{"version":1,"sha256Hash":"…"}}'` },
      p('Only queries run over `GET`. A mutation or subscription is refused with `post_required` (`405`, `Allow: POST`). `GET` never reads the session cookie, so a page on another site can’t make a signed-in browser run anything; send a Bearer token if the query needs you signed in.'),
      p('Signed-out answers without errors are `Cache-Control: public, max-age=30` with an `ETag` (send `If-None-Match` for a `304`). Answers with a Bearer token are `private, no-cache`, answers with errors `no-store`, and every answer has `Vary: Authorization`. `GET` counts against the GraphQL rate limit like `POST`.'),
    ],
  },
  {
    id: 'error-codes',
    title: 'Error codes',
    search: 'error codes list table meaning when rate_limited not_found forbidden bad_name',
    blocks: [
      p('The `error` of every error answer, and `extensions.code` in GraphQL, is one of these. Codes are stable: they are never renamed or reused for something else, and new ones may be added, so treat a code you do not know like its HTTP status. The same list is the `ErrorCode` schema of [openapi.json](' + SPEC + '/openapi/openapi.json).'),
      { t: 'codes', rows: errorCodes },
    ],
  },
  {
    id: 'versioning',
    title: 'Versioning and deprecations',
    search: 'version versioning meta build deprecation deprecated sunset header x-cyex-api-version ulid',
    blocks: [
      p('Everything is under `/v1`, and inside v1 we only add. Every answer carries `X-Cyex-Api-Version`, the API’s semver version, and [GET /v1/meta](#rest-get_meta) (GraphQL: the `meta` query) says the same together with the build, where the limits are published and everything that is deprecated right now.'),
      { t: 'pre', code: `curl -s '${baseUrl}/v1/meta'\n\n{ "api_version": "0.1.0", "build": "3ffc9b5d1e20",\n  "limits": { "rest": "/v1/limits", "graphql_field": "limits" },\n  "deprecations": [ { "id": "ulid_id_input", "since": "2026-11-26", "sunset": "2027-06-01", … } ] }` },
      p('Each deprecation has an `id`, what it is (`what`), the day it started (`since`), the day it stops working (`sunset`) and what to use instead (`replacement`). Nothing is removed before its sunset day, and a sunset is announced in the changelog first.'),
      p('A request that uses a deprecated input is told so in its answer: the REST headers `Deprecation: true`, `Sunset: <HTTP date>` and `Link: </v1/meta>; rel="deprecation"` (as in RFC 8594), and in GraphQL the answer’s `extensions.deprecations`, a list of the deprecation ids. Watch for them in your logs and you know what to change before the sunset day.'),
      table(
        ['ulid_id_input', 'An account, emote or set ID sent as a ULID. Sunset 2027/06/01. Send the Snowflake that answers carry (see [IDs](#ids)).'],
      ),
    ],
  },
  {
    id: 'emotes-and-files',
    title: 'Emotes and files',
    search: 'emote object cdn files webp avif static picture image host',
    blocks: [
      p('An emote has an `id` (store this, as a string: names change; see [IDs](#ids)), a `name`, `flags` (`zero_width`, `private`, `wide`; `nsfw` is retired and always false: NSFW emotes aren’t allowed), and a `host` with a list of `files`. Public reads only return approved emotes; a pending one shows to its owner alone.'),
      p('Each file entry carries its full address as `url` (`<host>/<name>`) and its media type as `mime` (`image/webp` or `image/avif`), so a client uses `url` as it is instead of joining `host` and `name`. `host` and `name` stay. The address is:'),
      { t: 'pre', code: `${EX.host}/2x.webp` },
      fmt(
        ['1x.webp … 4x.webp', 'Scales 1 to 4. Pick by line height times the device pixel ratio.'],
        ['1x.avif … 4x.avif', 'Still emotes also come as AVIF.'],
        ['1x_static.webp', 'Animated emotes: the first frame, for reduced motion or a paused chat.'],
      ),
      p('The host is the hash of the picture, so a file never changes once it exists. The CDN sends them with `Cache-Control: public, max-age=31536000, immutable`: cache files for good. The API’s own reads (sets, channels, emote objects) are the opposite: they change, so they carry `max-age=30` and an `ETag`.'),
      note('A zero-width emote draws over the emote before it, not beside it.'),
    ],
  },
  {
    id: 'sets-and-channels',
    title: 'Sets and channels',
    search: 'set channel personal global active capacity',
    blocks: [
      p(`A channel is a platform and the broadcaster's user ID: on Twitch the user \`id\` (for example \`${EX.twitchId}\`), on Kick \`broadcaster_user_id\`. Store the ID; logins change.`),
      p('A channel points at one active set. A set holds up to `capacity` emotes, each under the name it is typed as there, which can differ from the emote\'s own name.'),
      fmt(
        ['CHANNEL', 'A channel\'s emotes. A channel can keep several and switch the active one.'],
        ['GLOBAL', 'The cyex.tv global set, shown in every channel.'],
        ['PERSONAL', 'One per account: emotes its owner can use in any channel that allows them.'],
      ),
      p('So a chat line can use the channel\'s active set, the global set, and its sender\'s personal set where the channel has `allow_personal_emotes` on. Look personal sets up for up to 100 chatters at once with [Look up personal sets](#rest-lookup_personal_sets).'),
    ],
  },
  {
    id: 'live-events',
    title: 'Live events',
    search: 'websocket socket realtime subscribe topics',
    blocks: [
      p('One WebSocket carries every channel you show, so you never poll. Subscribe to topics and changes arrive as they happen.'),
      fmt(
        [`channel:twitch:${EX.twitchId}`, 'The channel\'s active set changed, or an emote in it did.'],
        [`set:${EX.setId}`, 'An emote was added to, removed from or renamed in this set.'],
        [`user:twitch:${EX.twitchId}`, 'This user\'s personal set changed.'],
      ),
      p('After a reconnect, resume with your session and the last event ID you saw: what you missed in the last 24 h is sent again. Frames, limits and a client are under [Event socket](#ws-events); GraphQL clients use [the events subscription](#gql-subscription-events).'),
    ],
  },
  {
    id: 'graphql',
    title: 'GraphQL',
    search: 'graphql gql graphiql introspection query mutation',
    blocks: [
      p(`Send queries and mutations as \`POST ${baseUrl}/v1/gql\` with a JSON body of \`query\` and \`variables\`. Open the same address in a browser for GraphiQL; introspection is on.`),
      p('Use REST for the reads a chat client repeats (they cache and answer `304`), and GraphQL for search, accounts, sets and anything signed in. Operations that need a session or a permission answer `signed_out` or `forbidden` without one; they are marked below.'),
      p(`Subscriptions use \`graphql-transport-ws\` at \`${ws}/v1/gql/ws\`.`),
    ],
  },
];
