# 0009: Accounts and identities

- Status: accepted
- Date: 2026/11/16

## Context
cyex.tv supports Twitch and Kick in v1 (D9), and has its own accounts, separate from cyex.app (0001). A person may stream or chat on both platforms and should have one cyex.tv account. Logins change on both platforms, but numeric IDs don't. The research behind this is in `docs/research/kick-api.md`.

## Decision
**Model**
- `accounts` (id, created_at, restricted_at, deleted_at)
- `identities` (account_id, platform `twitch|kick`, platform_user_id, login, display_name, avatar, linked_at, refreshed_at)
  - unique on (platform, platform_user_id)
- `sessions` (id_hash, account_id, kind `site|app`, created_at, expires_at, last_seen_at)
- We store only: platform user ID, login, display name and avatar URL. We do not store email, even though Kick's `user:read` returns it. We do not keep platform access tokens after sign-in; we only need identity.

**Sign-in**
- Twitch:
  - authorization code flow at `id.twitch.tv`, with no scopes (identity only);
  - user from `GET https://api.twitch.tv/helix/users`;
  - key `id` (numeric string).
- Kick:
  - OAuth 2.1 at `id.kick.com`, authorization code flow;
  - PKCE S256 is **required**, and the client secret is sent at the token exchange;
  - scope `user:read` only;
  - user from `GET https://api.kick.com/public/v1/users` (no params means the caller), giving `user_id` (int64), `name`, `profile_picture`;
  - key `user_id`.
- Both use a `state` bound to a short-lived pre-login cookie, and PKCE wherever the platform supports it.
- The first sign-in with an unknown identity creates an account. A signed-in user can **link** the other platform. Linking an identity that already belongs to another account is refused with `identity_taken`; merging accounts is a manual admin action, audited. **Unlinking** is allowed, except for the last identity.

**Sessions**
- Site sessions:
  - a random 256-bit token, stored as sha256;
  - the cookie is `Domain=cyex.tv; Path=/; Secure; HttpOnly; SameSite=Lax`;
  - 30 days, sliding;
  - rotated on sign-in and privilege change.
- App sessions (Bearer, for future deck and extension sign-in):
  - the same storage;
  - 90 days;
  - created through a loopback or device-style flow (designed when needed; not in v1, D19).
- CSRF: cookie callers must send `Origin: https://cyex.tv` on writes.

**Channels**
- A channel is `(platform, platform_channel_id)`:
  - on Twitch, the broadcaster's user `id`;
  - on Kick, `broadcaster_user_id`, which equals the broadcaster's `user_id`.
- Login and slug are lookup conveniences, refreshed when seen.

## Consequences
- One account across platforms, and no reliance on logins.
- Kick facts that are **not confirmed** in official docs:
  - whether `user_id` stays the same across username changes (we assume it does, because it's the only ID Kick documents);
  - token lifetimes (read `expires_in`);
  - rate limits (none published, so back off on 429).
- The owner must read Kick's Developer Terms in the Developer tab before launch, because their text isn't public.
