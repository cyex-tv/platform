# 0034: Channel webhooks

- Status: accepted (owner approved the developer platform, 2026/11/26; details decided here)
- Date: 2026/11/26
- Amends: 0005 (live updates)

## Context
Bots that react to a channel's emotes (announce a new emote in chat, keep a list current) must hold an event socket open today (ADR 0005). A server that only wants a call when something changes is better served by an HTTP push. Sending requests to URLs people type is a server-side request forgery risk, so the guard has to be part of the design.

## Decision
**What a webhook is**
- One channel's subscription: an `https://` URL and the event types it wants. Managed by the channel's owner and the owner's account editors with the `channels` permission (ADR 0014), through GraphQL: `channelWebhooks`, `createWebhook`, `updateWebhook` (URL, events, on/off), `deleteWebhook`, `rotateWebhookSecret`, `testWebhook`, `webhookDeliveries`. Anyone else gets `forbidden` for a channel and `not_found` for a webhook ID. API keys (ADR 0033) can't manage them. At most **5 per channel** (`too_many_webhooks`). Changes are audited on the channel's account (`channel.webhook_*`) and count under the per-account `prefs` limit.
- **Event types** are the ones the outbox already has that belong to a channel: `set.emote_added`, `set.emote_removed`, `set.emote_renamed`, `channel.active_set_changed`, `emote.updated` (on the channel's topic: its active set's changes), and the cosmetics catalog's `cosmetic.created`, `cosmetic.updated`, `cosmetic.deleted` (topic `cosmetics`, the same for every channel). Others (`personal.updated`, `resync`, ...) are refused (`bad_webhook_events`).
- **Secret:** `whsec_` and 43 random characters, shown once when made or rotated. It must be at hand to sign, so it is stored as it is (unlike keys and sessions); anyone who can read the database could sign as us, which they could do anyway.

**How events become deliveries**
- A trigger on `events_outbox` (migration `20261228000501`) inserts a `webhook_deliveries` row and a `webhook.deliver` job for every live webhook whose topic and event types match, in the transaction that wrote the event. No reader has to keep a cursor, nothing is skipped when IDs commit out of order, and nothing is sent twice by a second worker. The row keeps the event as it was, so retries send the same body after the outbox is cleaned.
- **Body** (JSON): `delivery_id`, `webhook_id`, `channel` (`platform`, `id`, `login`) and `event` (`id`, `type`, `created_at`, `data`: the payload the sockets send). Headers: `X-Cyex-Signature: t=<unix seconds>,v1=<hex HMAC-SHA256 of "<t>.<body>">`, `X-Cyex-Event`, `X-Cyex-Delivery`. Receivers should check the signature and that `t` is within 5 minutes, and use `delivery_id` to drop repeats (delivery is at least once, in no guaranteed order).
- **Retries:** a 2xx is delivered; anything else is tried again with the queue's backoff (10 s doubling), **8 attempts** in all (about 21 minutes). A delivery that fails them all counts against the webhook; **5 in a row** turn it off (`disabled_reason = failures`) and send the channel's owner (and whoever made it) a `webhook.disabled` notification. A success starts the count over; turning it on again does too. **Send test** queues one `webhook.test` delivery, sent once even while the webhook is off, never counted.
- **Log:** the latest 50 deliveries per webhook (state, attempts, last status, error, duration).

**SSRF guard (mandatory)**
- Saved URLs must be `https`, without a user name or password, at most 2000 characters, and not a private or local name (`localhost`, `*.local`, `*.internal`, single-label names, ...) or IP literal (`bad_webhook_url`).
- At every delivery the worker resolves the host itself and refuses the delivery if **any** address is not globally routable: private, loopback, link-local (the `169.254.169.254` metadata address), carrier-grade NAT, documentation, benchmarking, multicast, reserved; IPv6 loopback, unique-local (`fd00:ec2::254`), link-local, site-local, multicast, documentation, ORCHID, and every form that carries an IPv4 address (mapped `::ffff:`, compatible, NAT64 `64:ff9b::`, 6to4, Teredo), mapped ones judged by the IPv4 address inside. `cyex_tv_shared::webhooks::is_public_ip` holds the list, with unit tests for each range.
- The request goes only to the addresses checked (pinned, no second lookup: DNS rebinding can't swap them), through no proxy, and **redirects are never followed** (a 3xx is a failure). 5 s to connect, 10 s in all, and at most 64 KB of the answer is read.

## Alternatives
- **A cursor that tails the outbox in the worker:** needs leader election and the gap handling of the hub feeder (ADR 0005); the trigger gets exactly-once enqueueing for free.
- **Following redirects with a re-check per hop:** more code and more ways to get it wrong; receivers can give their final URL.
- **Signing with a key derived from a server secret:** a new secret to deploy to api and worker for little gain.

## Consequences
- Every outbox insert runs one indexed lookup on `webhooks` (`webhooks_live_topic`).
- A slow receiver holds a worker slot for up to 10 s per attempt; the queue's other jobs share the workers.
- Receivers see at-least-once delivery and must be idempotent on `delivery_id`.
