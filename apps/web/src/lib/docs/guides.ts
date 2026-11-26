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
    id: 'api-keys',
    title: 'API keys',
    search: 'developer key bot read-only bearer cyex_live usage revoke rate limit',
    blocks: [
      lede('A bot or tool that reads a lot can use a developer API key: a read-only key with a rate limit of its own, instead of sharing the per-IP limit with everyone at the same address.'),
      ol(
        'Sign in on cyex.tv, open your profile’s Account tab and find Developers.',
        'Name a key (one per bot or tool, so you can revoke it alone) and make it. Copy it then: it is shown once.',
        'Send it as `Authorization: Bearer cyex_live_…` to the REST reads and to `POST /v1/gql` queries.',
      ),
      { t: 'pre', code: `curl -s '${baseUrl}/v1/sets/global' \\n  -H 'authorization: Bearer cyex_live_<key>'` },
      table(
        ['Rights', 'Public reads only. A request with a key runs signed out: it never acts as you, holds none of your roles, and the cookie is ignored. Mutations, uploads, sign-in and the sockets answer `api_key_read_only`.'],
        ['Rate limit', '3.000 a minute per key, and 3.000 a minute for all of an account’s keys together, in place of the per-IP read and GraphQL limits. `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset` describe the tighter of the two.'],
        ['How many', 'Up to 10 keys per account. Made on the website only, never with a token.'],
        ['Usage', 'The Account tab shows each key’s prefix, when it was made and last used, and its requests per day for 30 days ([myApiKeys](#gql-query-myApiKeys)).'],
        ['Revoking', 'Revoke stops a key within 30 seconds ([revokeApiKey](#gql-mutation-revokeApiKey)). Deleting the account and a staff block end its keys too. Signing out does not: keys are for bots that run without you.'],
      ),
      note('A key that was revoked or never existed answers `401 api_key_invalid`. Keep keys on your server; a leaked one can only read what anyone can, but it spends your allowance.'),
    ],
  },
  {
    id: 'webhooks',
    title: 'Webhooks',
    search: 'webhook push http callback events channel signature hmac retry delivery secret',
    blocks: [
      lede('A webhook sends a channel’s events to your server as HTTP POSTs, so a bot doesn’t need to hold the event socket open. The channel’s owner, and its account editors with the channel settings permission, add them on the Channels page.'),
      table(
        ['URL', 'An `https://` URL on a public address, on the https port (443). Private, local and reserved addresses are refused when you save it and again before every delivery. Redirects are never followed. Only you and whoever made the webhook see the whole URL; editors see its host.'],
        ['Events', '`set.emote_added`, `set.emote_removed`, `set.emote_renamed`, `channel.active_set_changed` and `emote.updated` for the channel’s active set, and `cosmetic.created`, `cosmetic.updated`, `cosmetic.deleted` for the cosmetics catalog.'],
        ['How many', '5 per channel.'],
        ['Retries', 'Answer with a 2xx within 10 seconds. Anything else is tried again with backoff (10 s, 20 s, 40 s, …), 8 times in all. The webhook turns off after 5 deliveries in a row fail every try, or when at least 20 and over 80% of a day’s deliveries failed; the channel’s owner is told (once a day at most). Turn it on again when your server is fixed. With 100 deliveries waiting, new ones are dropped and logged.'],
        ['Send test', 'Sends one `webhook.test` delivery at once. The log keeps the latest 50 deliveries.'],
      ),
      p('Every delivery is a JSON body like this, with the headers `X-Cyex-Signature`, `X-Cyex-Event` and `X-Cyex-Delivery`:'),
      { t: 'pre', code: `{\n  "delivery_id": "1842",\n  "webhook_id": "01JDK3Z8Q4M9V2N6R7T5W1X0YA",\n  "channel": { "platform": "twitch", "id": "${EX.twitchId}", "login": "${EX.login}" },\n  "event": {\n    "id": "90211",\n    "type": "set.emote_added",\n    "created_at": "2026-11-26T14:05:00.000+00:00",\n    "data": { "set_id": "${EX.setId}", "emote_id": "${EX.emoteId}", "name": "KEKW" }\n  }\n}` },
      p('`event.data` is what the [event socket](#live-events) sends for the same event. Delivery is at least once and in no set order: use `delivery_id` to drop repeats, and `event.id` to order them. Manage webhooks with [createWebhook](#gql-mutation-createWebhook), [updateWebhook](#gql-mutation-updateWebhook), [testWebhook](#gql-mutation-testWebhook) and [webhookDeliveries](#gql-query-webhookDeliveries).'),
    ],
  },
  {
    id: 'webhook-signatures',
    title: 'Checking webhook signatures',
    search: 'signature verify hmac sha256 secret x-cyex-signature timestamp replay node python rust',
    blocks: [
      p('Each webhook has a secret (`cyexhook_…`), shown once when you add it or make a new one. Every delivery is signed with it: `X-Cyex-Signature: t=<unix seconds>,v1=<signature>`, where the signature is the hex HMAC-SHA256 of the timestamp, a dot and the raw body, under the secret. For 24 hours after you make a new secret, the header carries a second `v1=` signed with the old one: accept the delivery when any `v1` matches.'),
      ol(
        'Read the raw body as bytes, before any JSON parsing.',
        'Split the header on `,` and take `t` and `v1`.',
        'Compute HMAC-SHA256 of `<t>.<body>` with the secret, as hex, and compare it to `v1` in constant time.',
        'Refuse it when `t` is more than 5 minutes from your clock, so an old delivery can’t be replayed.',
      ),
      { t: 'pre', code: `// Node.js\nimport { createHmac, timingSafeEqual } from 'node:crypto';\n\nexport function verify(secret, header, rawBody, tolerance = 300) {\n  const parts = header.split(',').map((p) => p.trim().split('='));\n  const t = Number(parts.find(([k]) => k === 't')?.[1]);\n  if (!Number.isFinite(t) || Math.abs(Date.now() / 1000 - t) > tolerance) return false;\n  const expected = Buffer.from(createHmac('sha256', secret).update(\`\${t}.\`).update(rawBody).digest('hex'), 'utf8');\n  return parts.filter(([k]) => k === 'v1').some(([, v]) => {\n    const given = Buffer.from(v ?? '', 'utf8');\n    return given.length === expected.length && timingSafeEqual(given, expected);\n  });\n}` },
      { t: 'pre', code: `# Python\nimport hashlib, hmac, time\n\ndef verify(secret: str, header: str, raw_body: bytes, tolerance: int = 300) -> bool:\n    parts = [p.strip().split("=", 1) for p in header.split(",") if "=" in p]\n    t = int(next((v for k, v in parts if k == "t"), "0"))\n    if abs(time.time() - t) > tolerance:\n        return False\n    expected = hmac.new(secret.encode(), f"{t}.".encode() + raw_body, hashlib.sha256).hexdigest()\n    return any(hmac.compare_digest(expected, v) for k, v in parts if k == "v1")` },
      p('To check your code: with the secret `key`, the timestamp `1` and the body `{}`, the signature is `1ba6b8171186efc613e8bcc0cbdab2748f24984d7c5a84faa2637afa0e40d224`.'),
      note('Keep the secret on your server. If it leaks, make a new one on the Channels page ([rotateWebhookSecret](#gql-mutation-rotateWebhookSecret)); the old one stops signing after 24 hours.'),
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
      p('Limits are per IP, and on GraphQL also per account; a request with an [API key](#api-keys) counts under the key’s own limit instead. Past one you get `429` with the code `rate_limited` and a `Retry-After` header in seconds. Wait that long, then go on. Every `429` has `Retry-After`, the event socket and the open-socket cap included.'),
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
      p('Every upload and field limit is one public list: the GraphQL `limits` query, or `GET /v1/limits` over REST. Per kind (`emote`, `badge`, `paint`) it gives `maxBytes`, `maxSide`, `maxFrames`, `maxTotalPixels` and `maxDurationMs` (null: no cap), the smallest side and widest shape a picture may have (`maxAspect`, width over height), `maxFps` (emotes only) and the tallest file the CDN serves. It also lists the name and tag lengths and the per-IP request rates. Read it instead of copying numbers. The emote numbers are fixed rules, the same for everyone; the badge and paint numbers are what the operator configured.'),
      p('Signed in, the GraphQL `limits.account` lists your own per-account limits (`uploads_per_hour`, `pending_emotes`, `set_capacity_max` and the rest) with the `default` and what applies to you, `effective`, after your roles’ overrides. `uploadQuota` adds the emote rules next to `pendingMax` and `uploadsPerHour`: `maxBytes`, `maxSide`, `maxFrames`, `maxFps` and `maxAspect`. They are the same for every account.'),
      p('A role can also change the file limits of the people who hold it for badge and paint pictures: `badge_max_bytes`, `badge_max_side`, `badge_max_frames`, `paint_max_bytes` and `paint_max_side`, listed in `limits.account` with the rest, up to a ceiling the operator sets (16 MB; 4096 px for a badge, 6144 px for a paint, by default). Emote limits can’t be set per role or by the operator: `setRoleLimits` answers `bad_limit` to an `emote_max_*` key.'),
      note('Emote rules, fixed for everyone: at most 1000 × 1000 px, 8 MB (8 × 1024 × 1024 bytes), 1000 frames, 50 fps and 4:1 wide (width at most 4 × height; a tall picture has no ratio rule). A GIF frame shown for under 20 ms (a 0 or 10 ms delay) is shown for 100 ms, as browsers do, so the rate never goes over 50 fps and such a GIF is accepted. Wider than 4:1 is refused with `bad_dimensions`. The CDN serves an emote at most 128 px tall.'),
      p('Both upload routes, `POST /v1/emotes` and `POST /v1/cosmetics/images`, answer `202` with the same four fields next to the object itself, so one client handles both: `id`, `state`, `files` and `limits_used`. `state` is the emote’s review state (`pending` first) or, for a badge or paint picture, `processing` until the worker has made its files and then `ready`. `files` is empty until then. `limits_used` says what the file was checked against (`kind`, `max_bytes`, `max_side`, `max_frames`): for an emote the fixed rules, for a badge or paint your own numbers after your roles. They share their errors too: `bad_form`, `signed_out`, `forbidden`, `restricted`, `too_large`, `unsupported_format`, `invalid_image`, `rate_limited` and `unavailable`; an emote can also be `too_many_pending`, `blocked`, `bad_name` or `bad_dimensions` (wider than 4:1), a picture `bad_kind` or `bad_dimensions`.'),
      { t: 'pre', code: `HTTP/2 202\n\n{ "id": "${EX.emoteId}", "state": "pending", "files": [],\n  "limits_used": { "kind": "emote", "max_bytes": 8388608, "max_side": 1000, "max_frames": 1000 },\n  "name": "OMEGALUL", … }` },
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
      p('One rule sets the page size of every list, GraphQL or REST: `perPage` (`per_page` over REST, `limit` for the activity feeds and the audit log) is 1 to 100, and 50 when you leave it out. More than the maximum is cut to it, and 0 is 1. Five lists always had other numbers and keep them, so nothing you read changed. Each field says its own numbers in its description.'),
      table(
        ['Every other list', '`perPage` 1 to 100, default 50: `searchEmotes`, `myEmotes`, `accountEmotes`, `reviewQueue`, `reportQueue`, `myFavourites`, `myOrders`, `orders`, `codeBatches`, `cosmeticEntitlements` and `auditLog`.'],
        ['`searchAccounts`', '`perPage` 1 to 50, default 20.'],
        ['`setEmotes`, `GET /v1/sets/{id}/emotes`', '`perPage` 1 to 200, default 120: chat clients read whole sets.'],
        ['`accountActivity`, `emoteActivity` and their `…Page` forms', '`limit` 1 to 100, default 30.'],
        ['`emoteChannels`', '`limit` 1 to 50, default 8.'],
        ['`accountSets` `preview`', 'Up to 50 emotes a card, default 12.'],
      ),
      p('Over REST, [a set’s emotes, a page at a time](#rest-get_set_emotes) answers `next` too, and a `Link` header with the next page’s address:'),
      { t: 'pre', code: `curl -s -i '${baseUrl}/v1/sets/${EX.setId}/emotes?per_page=120'\n\nHTTP/2 200\nlink: </v1/sets/${EX.setId}/emotes?per_page=120&after=eyJ2Ijox…>; rel="next"` },
    ],
  },
  {
    id: 'limit-errors',
    title: 'Limit errors',
    search: 'limit errors 409 413 422 details current set_full too_many_pending limit_reached too_many status',
    blocks: [
      p('When something is refused because a limit is reached, the answer says which numbers, so you can show them. Over REST the error body has `details` with `limit` (what is allowed) and `current` (what you have or sent); in GraphQL they are `extensions.limit` and `extensions.current`. They are only there on the limit errors below.'),
      { t: 'pre', code: `HTTP/2 409\n\n{ "error": "too_many_pending",\n  "message": "You have 20 emotes waiting for review. Upload more once some are reviewed.",\n  "details": { "limit": 20, "current": 20 } }` },
      p('Which status a limit error has follows one rule. A limit the account or set has **reached** is `409`: the request is fine, the state is full. A cap on **one request’s size** is `400`, and a file over its byte cap is `413`. These existed before the rule, so they keep their status. At a `409` limit `current` equals `limit`; for a request cap it is what you sent; for `too_large` over the byte cap it is the file’s size. A side or frame cap names its numbers in the message instead.'),
      table(
        ['`set_full`, `too_many_pending`, `too_many_editors`, `limit_reached`', '`409`: the set, the review queue, the editors or the list is full.'],
        ['`capacity_over_limit`', '`400`: a set capacity above what the owner’s roles allow. It was `400` from the start and stays, so clients that match it keep working.'],
        ['`too_many`, `bad_batch`, `too_many_tags`, `too_many_reasons`, `too_many_sets`', '`400`: more items in one request than allowed.'],
        ['`too_large`', '`413`: a file over its byte, side or frame cap.'],
        ['`too_many_topics`, `too_many_subscriptions`, `too_many_fields`', 'GraphQL only (so HTTP `200` with the code in `extensions.code`): topics, subscriptions or root fields over the cap.'],
        ['`rate_limited`', '`429` with `Retry-After`; see [Rate limits](#rate-limits). No `details`.'],
      ),
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
        ['file_scale_non_emote', 'The `scale` of a badge or paint picture file: 1, 2 and 4 for the chat sizes, but 7, 14, 28 and 8 for the larger ones, which is no scale of anything. Read `width` instead; it is the size of every file. Emote files keep `scale` (1 to 4) and are not affected. Sunset 2027/11/26.'],
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
        ['1x.avif … 4x.avif', 'Still emotes also come as AVIF. Animated ones too (an animated AVIF, `is_static` false), at each size where it is no larger than the animated WebP; a size without one has only WebP, so read the `files` list rather than assuming.'],
        ['1x_static.webp', 'Animated emotes: the first frame, for reduced motion or a paused chat.'],
      ),
      p('The host is the hash of the picture, so a file never changes once it exists. The CDN sends them with `Cache-Control: public, max-age=31536000, immutable`: cache files for good. The API’s own reads (sets, channels, emote objects) are the opposite: they change, so they carry an `ETag` and a short `max-age`, set per route; see [Caching](#caching).'),
      note('A zero-width emote draws over the emote before it, not beside it.'),
    ],
  },
  {
    id: 'caching',
    title: 'Caching',
    search: 'cache caching max-age etag if-none-match stale-while-revalidate stale-if-error vary cache-control private',
    blocks: [
      p('Every public read sends an `ETag`: ask again with `If-None-Match` and an unchanged answer is a cheap `304`. `Cache-Control` says how long you may go without asking, and differs by route, because they change at different speeds. `stale-while-revalidate` lets a cache serve the old copy while it fetches a new one, and `stale-if-error` for that long when the API is failing.'),
      table(
        ['[Get a channel](#rest-get_channel)', '`max-age=10`, `stale-while-revalidate=30`, `stale-if-error=300`. The active set can change at any time, so this is the short one.'],
        ['[Get a set](#rest-get_set), [a set’s emotes](#rest-get_set_emotes)', '`max-age=30`, `stale-while-revalidate=60`, `stale-if-error=600`.'],
        ['[The global set](#rest-get_global_set)', '`max-age=300`, `stale-while-revalidate=600`, `stale-if-error=86400`: staff change it rarely.'],
        ['[Get an emote](#rest-get_emote)', '`max-age=30`, `stale-while-revalidate=60`, `stale-if-error=86400`.'],
        ['[Limits](#rest-get_limits)', '`max-age=300`, `stale-while-revalidate=600`, `stale-if-error=86400`: configuration.'],
        ['[The cosmetics catalog](#rest-get_cosmetics)', '`max-age=60`, `stale-while-revalidate=120`, `stale-if-error=3600`.'],
        ['[Chatters’ cosmetics](#rest-get_user_cosmetics)', '`max-age=30`, `stale-while-revalidate=60`, `stale-if-error=600`.'],
      ),
      p('These answers are `public` and `Vary: Accept-Encoding`: they are the same for everyone. The one answer that depends on who asks is the cosmetics catalog, since staff also see badges and paints that are not out yet. It varies by `Cookie` and `Authorization`, an answer to a signed-in request is `private` (only that browser may keep it, never a shared cache or the CDN), and staff get `private, no-cache`. Send no cookie when you only read it, and you get the shared copy.'),
      note('Live changes arrive on [the event socket](#ws-events) sooner than any `max-age`; use it instead of polling when you can.'),
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
