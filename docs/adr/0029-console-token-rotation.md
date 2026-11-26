# 0029: Console tokens rotate

- Status: accepted (owner's decision, 2026/11/26)
- Date: 2026/11/26
- Amends: 0028 (console tokens)

## Context
A console token (ADR 0028) lasted 90 days and ended only when it was signed out itself, by "sign out everywhere", by making a new one, deletion or blocking. The owner decided (2026/11/26) that it should be short-lived and tied to signing out:

- it expires, and so rotates, every 3 days;
- it also rotates whenever the person signs out of a single session or of all of them;
- once rotated, the old token must not work for anything.

## Decision
- **3 days**, from when it is made, never slid (`ttl(SessionKind::Console)`). Migration `20261226000101_console_token_rotation.sql` cuts the live ones to 3 days from when they were made and adds `sessions_console_three_days`, a CHECK that no console session outlives 3 days, so a longer one can't be written by mistake.
- **Signing out rotates it.** Every way a person signs a session out also ends the account's console token, in the same transaction:
  - `POST /v1/auth/logout` (any session: a browser, an app, the token itself);
  - `revokeSession` (when it signs a session out; an unknown ID changes nothing);
  - `signOutEverywhere` (the website's ended every session already; with a Bearer token it now ends the console token too) and `revokeOtherSessions` (already did);
  - `POST /v1/auth/app/revoke` (an app signing itself out, RFC 7009).
- **Rotation is lazy.** The API keeps only the token's sha256 (ADR 0009, 0028), so it cannot hand out a token it made on its own. Rotating ends the old one; the next one is made when the person asks (`cyextv.token()`, `createConsoleToken`, or **New token** on the profile's Account tab). The rate limit (5 an hour) is unchanged and rotation doesn't count against it.
- **The old token works for nothing.** A console token is accepted only by `Auth::caller`, which looks its session up on every request (REST and `POST /v1/gql`). An ended or expired session isn't found, so the token is nobody. Neither socket takes a caller: `/v1/gql/ws` attaches none (subscriptions are public topics) and the event WebSocket (`/v1/events`, ADR 0005) has no sign-in, so no open socket runs as the token and none needs closing.
- **Seeing it.** A new query, `myConsoleToken`, answers the live token's session (`id`, `createdAt`, `expiresAt`, as `mySessions` lists it) or null, never the token. The profile's Account tab has an **API token** section: the current token's expiry (`2026/11/26, 14:05`) or "None", and a button that makes a new one and shows it there with Copy, held only by the page until it is left.

Unchanged: one per account, the account's own rights (staff roles count, the owner's all-permissions don't), website-only minting, the audit row, no notifications.

## Alternatives
- **Making the next token on rotation and keeping it to show later:** the API would have to store the token itself (or a key to derive it), which ADR 0009 and 0028 rule out for every session. Lazy rotation gives the same result: the old one is dead, and a new one is one click away.
- **Rotating on a fixed 3-day calendar:** tokens made late in a window would last minutes. 3 days from when it is made is simpler and predictable.

## Consequences
- A leaked console token works for at most 3 days, and less once its owner signs out anywhere.
- Scripts that held a token for weeks need a new one every 3 days, and after any sign-out.
