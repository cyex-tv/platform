# 0005: Live updates

- Status: accepted
- Date: 2026/11/16

## Context
Clients (deck, extension, website) must see set changes within a second. The deck and the extension watch many channels at once. Browsers allow 6 HTTP/1.1 connections per origin, so one connection per channel doesn't work. 7TV offers SSE and WebSocket with resume.

## Decision
- **SSE** at `GET api.cyex.tv/v1/events`, with **many subscriptions on one connection**:
  - `?sub=channel:twitch:123&sub=channel:kick:456&sub=set:<id>`, up to 100 per connection;
  - changing the list means reconnecting with a new query and the `Last-Event-ID`.
- Event types:
  - `set.emote_added`
  - `set.emote_removed`
  - `set.emote_renamed`
  - `channel.active_set_changed`
  - `personal.updated`
  - `emote.updated`
  - `resync`
- `cosmetic.*` is reserved.
- Every event has an `id` from the outbox sequence, a `type`, and JSON `data` with the IDs involved.
- **Source:** writes insert into `events_outbox` in the same transaction as the change. `NOTIFY` wakes the events service, so the API never calls the events service.
- **Resume:** `Last-Event-ID` replays from the outbox for 24 h. Anything older gets `resync`, and the client refetches.
- **Heartbeat:** a comment line every 20 s.
- WebSocket is added only if clients need to change subscriptions often without reconnecting.

## Consequences
- Plain HTTP, easy to cache-bypass and debug, and works through Cloudflare.
- The outbox needs a cleanup job (delete events older than 48 h).
