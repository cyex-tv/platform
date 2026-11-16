# 0005: Live updates

- Status: accepted (revised 2026/11/16: WebSocket and GraphQL subscriptions replace SSE)
- Date: 2026/11/16

## Context
Clients must see set changes within a second. They are:
- the deck and the extension, which watch many channels at once and change their subscriptions as the user moves between chats;
- the website;
- third-party chat clients.

Browsers allow only 6 HTTP/1.1 connections per origin, so one connection per channel doesn't work. A connection must carry many subscriptions, and they must be changeable without reconnecting. The first version of this ADR chose SSE, which can't change subscriptions on an open connection. The owner asked for WebSockets.

The API is GraphQL-first (ADR 0008), so GraphQL clients expect subscriptions.

## Decision
Two transports over **one event model**:

**1. The event WebSocket, `wss://api.cyex.tv/v1/events`** (events service)
- This is the main transport for chat clients: the deck, the extension, Chatterino-style clients.
- It is a small JSON protocol, so a client needs no GraphQL library. Every frame is `{ "op": "<name>", "d": { … } }`:

| Direction | `op` | `d` |
|---|---|---|
| server → client | `hello` | `session_id`, `heartbeat_interval` (ms), `max_subscriptions` |
| client → server | `subscribe` | `topics: ["channel:twitch:<id>", "channel:kick:<id>", "set:<id>", "user:<platform>:<id>"]` |
| client → server | `unsubscribe` | `topics: […]` |
| server → client | `ack` | `op`, `topics`, and the `error` code per topic that was refused |
| server → client | `dispatch` | `id` (event ID), `type`, `data` |
| both | `heartbeat` | `{}`. The server sends one every 20 s; a client that misses 3 should reconnect. |
| client → server | `resume` | `session_id`, `last_event_id`. Replays missed events for the old topics. |
| server → client | `reconnect` | The server is restarting, so reconnect and resume. |
| server → client | `error` | `code`, `message`, then the connection closes. |

- Limits: 500 topics per connection. A client may send at most 10 frames per second, and a frame is at most 16 KB.

**2. GraphQL subscriptions, `wss://api.cyex.tv/v1/gql/ws`** (api service)
- These are for the website and apps that already speak GraphQL.
- Protocol: `graphql-transport-ws` (the `graphql-ws` library).
- One field: `subscription { events(topics: [String!]!) { id type … } }`. It uses the same topics and event types as the event WebSocket.

**The event model, shared by both**
- Types:
  - `set.emote_added`
  - `set.emote_removed`
  - `set.emote_renamed`
  - `channel.active_set_changed`
  - `personal.updated`
  - `emote.updated` (approved, taken down, renamed)
  - `resync` (the client should refetch)
  
  `cosmetic.*` is reserved (ADR 0011).
- Every event has a monotonically increasing `id` from the outbox.
- **Source:** every write inserts into `events_outbox` in the same transaction as the change. `NOTIFY` wakes both services, so the API's write path never calls a socket directly.
- **Resume:** missed events replay from the outbox for 24 h. Anything older gets `resync`. The outbox is cleaned after 48 h.
- **Personal emotes:** `personal.updated` goes to the `channel:` topics where that user chatted in the last 30 min, and to `user:` topics.

## Consequences
- Chat clients get changeable subscriptions on one connection, and resume.
- GraphQL clients get the idiomatic way.
- There are two socket endpoints to run and load-test (Phase 5). The event model and the outbox are written once and used by both.
- WebSockets through Cloudflare need the tunnel to allow upgrades, and idle timeouts above the heartbeat interval.
