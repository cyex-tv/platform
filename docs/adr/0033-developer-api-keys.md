# 0033: Developer API keys

- Status: accepted (owner approved the developer platform, 2026/11/26; details decided here)
- Date: 2026/11/26
- Amends: 0008 (rate limits), 0028 (Bearer tokens)

## Context
Bots and third-party tools read cyex.tv's public data (channels, sets, emotes, cosmetics). Until now they read signed out, under the per-IP limits (`reads` 1200 a minute, `gql` 300 a minute), or with a console token (ADR 0028, 0029), which acts as the person with every right they have and lasts 3 days. Neither suits a bot: the IP limit is shared with everyone behind the same address, and a console token is far more power than reading needs and rotates too often.

## Decision
- **A developer API key** is `cyex_live_` and 43 random base64url characters (256 bits). It is shown once, when made; only its sha256 is kept (`api_keys.key_hash`), like every session (ADR 0009). The list shows its **prefix** (`cyex_live_` and 8 more characters), name, when it was made and last used, and its requests per UTC day for the last 30 days.
- **Made on the website only:** `createApiKey(name)` with the website's cookie session; a Bearer token (app or console) gets `website_only`, so a 3-day token can't turn itself into a lasting key. At most **10 per account** (`too_many_api_keys`). Listed with `myApiKeys`, revoked with `revokeApiKey`. Making one counts under the per-account `prefs` limit. Made and revoked keys are audited (`account.api_key_created`, `account.api_key_revoked`), never with the key.
- **Sent as `Authorization: Bearer cyex_live_...`.** It is not a session: `Auth::caller` never accepts it (it isn't a session token's shape), so a request with a key runs **signed out**, and the cookie is ignored. A key reads public data and nothing else:
  - the public REST reads (`GET`/`HEAD` under `/v1`, `POST /v1/personal-sets/lookup`) and `POST /v1/gql` queries;
  - any other `/v1` route (uploads, `/v1/auth/*`, sockets) answers `403 api_key_read_only` before its handler;
  - every GraphQL mutation answers `api_key_read_only`, even one a signed-out person could call.
  It never carries the account's roles, staff permissions or the owner's all-permissions.
- **Its own buckets:** a request with a key counts under `api_key` (default **3000 a minute**, `CYEX_TV_LIMIT_API_KEY`) per key, and under `api_key_account` (default the same, `CYEX_TV_LIMIT_API_KEY_ACCOUNT`) for all of the account's keys together, so ten keys don't read ten times as much. They replace the per-IP `reads` and `gql` limits; `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset` describe the tighter of the two.
- **Nothing reaches the database before a limit** (security review, 2026/11/26): the key's bucket is counted by the key's hash before anything else; lookups are cached by hash for 30 s, found or not (negative caching); a cache miss first costs the IP a token of its usual limit, and so does every refused key. A key that isn't valid answers `401 api_key_invalid`. Revoking, a staff block and account deletion forget the account's cached lookups on that instance at once; other instances see it within the 30 s.
- **Usage** is counted in memory per api instance and written every minute (`api_key_usage`, one row per key and day, `last_used_at`); the hourly clean-up drops days past 31.
- **What ends a key:** revoking it (at once on the instance that revoked it, within 30 s on the others), deleting the account (scheduling the erase deletes the keys; a restore doesn't bring them back) and a staff block on the whole account (the block deletes them). While an account is deleted or blocked its keys are refused anyway. **Signing out, one session or everywhere, does not end keys**: they are for bots that run without anyone signed in, and sign-out rotating the console token (ADR 0029) is about tokens that act as the person.

## Alternatives
- **Scoped personal tokens that act as the person** (ADR 0028's "later"): more power than reading needs; a leaked key would then be an account takeover. Read-only keys can leak with little harm.
- **Ending keys on sign-out everywhere:** surprising for bots, and "sign out everywhere" is about where the person is signed in. Revoking a key is one click.
- **Keys by IP allow-list or OAuth client credentials:** more setup for bot authors, not needed for public reads.

## Consequences
- Bots get a predictable limit of their own and a usage view; abuse can be traced to an account and cut off by revoking or blocking.
- Keyed requests skip the edge cache (an `Authorization` header), so the per-key limit is what protects the reads; it is set per deployment.
- Usage can lose up to a minute of counts if an instance dies; counts are a guide, not billing.
