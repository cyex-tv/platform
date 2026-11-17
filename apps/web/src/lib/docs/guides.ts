// The guide sections of /developers, as data: the page renders them in order, and the index's
// Notes group lists them. Inline `code` and [links](#anchor) are written as in the API spec.
import { EX, paragraphs, type Segment } from './example';
import { baseUrl } from './rest';

export type Block =
  | { t: 'lede'; text: Segment[][] }
  | { t: 'p'; text: Segment[][] }
  | { t: 'ol' | 'ul'; items: Segment[][][] }
  | { t: 'base'; rows: [string, string][] }
  | { t: 'fmt'; rows: [string, Segment[][]][]; plain?: boolean }
  | { t: 'note'; text: Segment[][] }
  | { t: 'pre'; code: string };

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
      note('Tokens for apps (cyex deck, the extension) are planned and not in v1. Until then, signed-in features live on cyex.tv.'),
    ],
  },
  {
    id: 'errors',
    title: 'Errors and caching',
    search: 'error codes etag 304 cache if-none-match',
    blocks: [
      p('REST errors are `{ "error": "not_found", "message": "No emote with that ID." }`. Match on `error`, a stable code; `message` is for people. GraphQL errors carry the same codes in `extensions.code`, and an answer can hold data and errors together.'),
      p('The reads send an `ETag`. Send it back as `If-None-Match` and you get `304` with no body while nothing changed: poll that way and a channel costs almost nothing.'),
      { t: 'pre', code: `curl -s -i '${baseUrl}/v1/sets/${EX.setId}' \\\n  -H 'if-none-match: "3f9a1c"'\n\nHTTP/2 304\netag: "3f9a1c"` },
      p('GraphQL queries are limited to a depth of 12 and a complexity of 1.000.'),
    ],
  },
  {
    id: 'rate-limits',
    title: 'Rate limits',
    search: 'limits 429 retry-after throttle',
    blocks: [
      p('Limits are per IP, and on GraphQL also per account. Past one you get `429` with the code `rate_limited` and a `Retry-After` header in seconds. Wait that long, then go on.'),
      table(
        ['Reads', '1.200 a minute per IP. Most of them are answered by the cache.'],
        ['GraphQL', '300 a minute per IP, and 600 a minute per account.'],
        ['Sign-in', '30 a minute per IP.'],
        ['Uploads', '30 an hour per IP, and 20 an hour per account.'],
        ['Reports', '10 an hour per account, one open report per emote.'],
        ['Event socket', '60 connections a minute per IP; 10 frames a second and 16 KB a frame from a client.'],
      ),
    ],
  },
  {
    id: 'emotes-and-files',
    title: 'Emotes and files',
    search: 'emote object cdn files webp avif static picture image host',
    blocks: [
      p('An emote has an `id` (store this: names change), a `name`, `flags` (`zero_width`, `nsfw`, `private`), and a `host` with a list of `files`. Public reads only return approved emotes; a pending one shows to its owner alone.'),
      p('Each file is at `<host>/<name>`:'),
      { t: 'pre', code: `${EX.host}/2x.webp` },
      fmt(
        ['1x.webp … 4x.webp', 'Scales 1 to 4. Pick by line height times the device pixel ratio.'],
        ['1x.avif … 4x.avif', 'Still emotes also come as AVIF.'],
        ['1x_static.webp', 'Animated emotes: the first frame, for reduced motion or a paused chat.'],
      ),
      p('The host is the hash of the picture, so a file never changes once it exists. Cache files for good.'),
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
