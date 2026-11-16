# Kick API notes

Researched 2026/11/16 from docs.kick.com. "Unverified" means it was not found in an official source.

## OAuth
- OAuth 2.1 on `https://id.kick.com`:
  - `GET /oauth/authorize`
  - `POST /oauth/token` (code, refresh, app token)
  - `POST /oauth/revoke`
  - `POST /oauth/token/introspect`
  - Source: https://docs.kick.com/getting-started/generating-tokens-oauth2-flow
- PKCE S256 is required. Authorize takes `response_type=code`, `client_id`, `redirect_uri` (an exact match), `scope`, `code_challenge`, `code_challenge_method`, `state`. The client secret is sent at the token exchange and on refresh.
- Refresh returns a new access token and a new refresh token. Token lifetimes are not documented, so read `expires_in`.
- Scopes: https://docs.kick.com/getting-started/scopes
  - `user:read` is enough for identity.
  - Others: `channel:read`, `channel:write`, `chat:write`, `events:subscribe`, `moderation:ban`, `moderation:chat_message:manage`, rewards, `streamkey:read`, `kicks:read`, ads.
- Current user: `GET https://api.kick.com/public/v1/users` with no params returns `user_id` (int64), `name`, `profile_picture`, `email`. Source: https://docs.kick.com/apis/users
- App setup: https://docs.kick.com/getting-started/kick-apps-setup
  - you need a Kick account with 2FA;
  - create the app under Account Settings, Developer tab, which gives a client ID and secret;
  - register the redirect URL there;
  - there is no approval step. Optional verification (by email to developers@kick.com) gives a badge and higher limits.

## IDs and lookups
- Channels are keyed by `broadcaster_user_id`, the broadcaster's `user_id`. Source: https://docs.kick.com/events/event-types.md
- `GET /public/v1/channels` takes up to 50 `broadcaster_user_id` values **or** up to 50 `slug` values (not mixed). Source: https://docs.kick.com/apis/channels
- `GET /public/v1/users?id=…` (many allowed).
- Unverified: whether `user_id` stays the same across username changes.

## Limits
- No published rate limit numbers (unverified). Handle 429 with backoff.
- Event subscriptions are capped at 10.000 per type per app. Unverified apps get 1.000 for `chat.message.sent`.

## Chat (for the extension and the deck)
- The official chat events are **webhooks only** (`chat.message.sent`, scope `events:subscribe`). They go to a server and don't suit a browser extension.
  - Message `content` has inline `[emote:<id>:<name>]` tokens and an `emotes[]` list.
- Unofficial, in community use: a Pusher websocket on `chatrooms.{chatroom_id}.v2`. The chatroom ID comes from the internal `kick.com/api/v2/channels/{slug}` (`chatroom.id`), which is behind Cloudflare and can change without notice. Unverified.
- The kick.com site is reportedly React / Next.js (from job postings; unverified). The 7TV extension lists Kick support (https://addons.mozilla.org/addon/2857945/).

## Developer terms
- These are linked only from the app setup page, so the owner should read them in the Developer tab before launch.
- Our stance: store only `user_id`, `name` and `profile_picture`, never `email`, and keep no platform tokens after sign-in.
