# 0021: App sign-in for desktop apps

- Status: accepted (pending the owner's review)
- Date: 2026/11/19
- Amends: 0009 (app sessions), PLAN.md D19 (the owner decided on 2026/11/19 that cyex deck signs in to cyex.tv)

## Context
ADR 0009 kept `app` sessions (Bearer) in the model but left how an app gets one "designed when needed". cyex deck now needs one: emote add, rename and remove commands in chat, and a cosmetic picker. It is a desktop app: it can't keep a secret, it can open the system browser, and it already signs in to Twitch and 7TV through the browser with a loopback redirect. The website already signs people in with Twitch and Kick; an app should reuse that, not ask for a platform token itself.

## Decision
**Flow:** OAuth 2.0 authorization code with PKCE for a public client (RFC 6749, RFC 7636) and a loopback redirect (RFC 8252), following RFC 9700 where it applies.
1. The app makes a verifier (43 to 128 characters), its S256 challenge and a `state`, listens on `http://127.0.0.1:<any free port>/callback` and opens `GET /v1/auth/app/authorize?response_type=code&client_id=…&redirect_uri=…&code_challenge=…&code_challenge_method=S256&state=…` in the browser.
2. The API checks the client and the redirect first. An unknown client or a redirect that isn't loopback answers 400 and is never redirected to. Other mistakes go back to the redirect as `error=invalid_request` with `state`. A good request goes on (303) to `https://cyex.tv/authorize` with the same parameters.
3. The consent page signs the person in with Twitch or Kick if needed (the usual site sign-in, coming back to the page), names the app and what it may do, and shows Allow and Deny. The answer is `POST /v1/auth/app/consent`, with the site cookie and `Origin` only (never an app token), which checks everything again and answers where to send the browser: the redirect with `code` and `state`, or `error=access_denied` and `state`. Consent isn't remembered: each sign-in asks.
4. The app trades `code`, `code_verifier`, `redirect_uri` and `client_id` at `POST /v1/auth/app/token` (form-encoded) for an access token and a refresh token. Refresh with `grant_type=refresh_token`. `POST /v1/auth/app/revoke` (RFC 7009) or `POST /v1/auth/logout` with the Bearer token signs it out.

**Redirects:** `http` to `127.0.0.1` or `[::1]` only, any port (RFC 8252 7.3), the registered path (`/callback`), no user info, query or fragment. `localhost` is refused (RFC 8252 8.3: it can resolve elsewhere). Custom schemes and https are not offered; a client that needs one gets it registered with a note here.

**Clients:** registered in code (`crates/auth/src/app.rs`), public, no secret. One so far:

| client_id | Name | Redirect | Scopes |
|---|---|---|---|
| `cyex-deck` | cyex deck | `http://127.0.0.1:<port>/callback`, `http://[::1]:<port>/callback` | `sets.emotes`, `cosmetics.select` |

**Scopes:** an app session runs only the GraphQL queries `app::APP_QUERIES` lists (public reads; its own account, sets, emotes, channels, lists and cosmetics; `channelEditableSets`; no staff read) and the mutations its scopes name; anything else answers `insufficient_scope`, checked per resolved top-level field so aliases and fragments change nothing. REST uploads are refused. Staff grants never count for an app session (amendment of 2026/11/20, security review S2): it holds only the built-in User role's permissions and is never the owner, so the rights checks behind the allowed mutations are the account's own (set owner, account editors with `EMOTES` covering the set, set editors, set capacity), not `sets.edit_any`.
- `sets.emotes` ("Manage emotes in sets you can edit"): `addSetEmote`, `updateSetEmote`, `removeSetEmote`.
- `cosmetics.select` ("Pick your cosmetics"): `selectCosmetics`.

**Lifetimes and storage:**
- Code: 256 random bits, stored as sha256, 2 minutes, single use. A wrong verifier, client or redirect uses it up. A code presented again signs out the session it gave (RFC 6749 4.1.2).
- Access token: 256 random bits (the session token, as for every session), stored as sha256, **1 hour**, not slid by use.
- Refresh token: 256 random bits, stored as sha256, **60 days**, rotated on every refresh; each refresh starts the 60 days again. Presenting a used refresh token signs the whole session out (reuse detection, RFC 9700 4.14.2). Used tokens are remembered for 7 days for that check.
- One `sessions` row per sign-in (`kind = 'app'`, `app_client_id`, `access_expires_at`): a refresh rotates the token on the same row, so the session keeps its public ID and `mySessions` lists it once, named after the app, with the refresh window as `expiresAt`. Signing the session out in Settings, "sign out everywhere", account deletion and blocking end its refresh tokens too (they hang off the session, `ON DELETE CASCADE`; a refresh also checks the account is live).
- Limits: `/v1/auth/app/*` is under the sign-in limit (30 a minute per IP); the app's GraphQL calls count like any account's.

## Alternatives
- **Device code (RFC 8628):** works without a local listener, but the person types a code and polls; the deck already runs a loopback listener for Twitch and 7TV, so the loopback flow is one click less. It can be added later for clients without a browser.
- **Longer access tokens (the 90 days of ADR 0009):** simpler, but a leaked token would work for months. One hour with rotating refresh limits the damage and lets reuse detection notice a copy.
- **Full access for app tokens:** less code, but a desktop app's token could then delete the account or sign everyone out. Scopes keep it to what the consent page says.

## Consequences
- The deck can sign in without ever seeing a Twitch or Kick token, and the person can sign it out from the website.
- New apps are a code change (a `CLIENTS` entry and a line in this ADR), not a self-service registration.
- Only the mutations listed above work with an app token; widening a scope or adding one changes the consent text and needs a note here.
- App sessions made before this (unscoped, with no client) are retired: see the amendment below.

## Amendment: unscoped app sessions retired (2026/11/20)
The owner decided on 2026/11/20 to retire the `app` sessions made before this flow: they had no client and no scopes, so their Bearer token carried the account's full rights (staff grants included).
- Migration `20261209006001_retire_unscoped_app_sessions.sql` signs every `app` session without an `app_client_id` out (their refresh tokens go by CASCADE) and adds `CHECK (kind <> 'app' OR app_client_id IS NOT NULL)`, so none can be made again. `sessions::create` makes site sessions; an app session comes only from `/v1/auth/app/token`.
- `Auth::caller` answers signed out for a Bearer token whose session names no client, as a second line behind the CHECK.
- The website's cookie session is unaffected. The only Bearer callers left are app sign-ins, with their scopes.
- Tests that used an unscoped app session as a convenient full-rights token now use a site session (cookie plus the site's `Origin`).
