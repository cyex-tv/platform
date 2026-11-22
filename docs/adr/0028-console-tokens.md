# 0028: Console tokens (your API token)

- Status: accepted (pending the owner's review)
- Date: 2026/11/22
- Amends: 0009 (sessions), 0021 (Bearer tokens are app sign-ins only)

## Context
The owner asked (2026/11/22) that people can export their authorization from the browser console, the way some emote sites let you run `copy(localStorage.getItem('…-token'))`. It helps for a GraphQL playground, for scripts, and for merging accounts later if someone signs in with the other platform by mistake and makes a second profile.

On cyex.tv the website's session is an `HttpOnly` cookie (ADR 0009). Putting it in `localStorage` would let any script that ever runs on the page (an injected one included) read and keep it. That stays ruled out.

## Decision
A **console token**: a new personal Bearer token, made on request, never the website's own session.

- `POST /v1/auth/token/export` (REST, under `/v1/auth/*`) and the GraphQL mutation `createConsoleToken` make one and answer `{ token, token_type: "Bearer", expires_at, session_id }` (`tokenType`, `expiresAt`, `sessionId` in GraphQL), with `Cache-Control: no-store`.
- Only the website's own cookie session may call them, with `Origin: https://cyex.tv` (the usual cookie check). A Bearer token (an app's or a console token) gets `website_only`, so a token can't make more of itself.
- A `console` session kind (`sessions.kind`, migration `20261213009001_console_tokens.sql`): 256 random bits, stored as sha256 like every session, **90 days from when it is made, never slid** by use.
- **One per account**: making one signs the previous one out in the same transaction; a partial unique index (`sessions_one_console`) refuses a second one made at the same moment.
- Listed in `mySessions` as kind `CONSOLE`, device `Console token`. It ends with `revokeSession`, "sign out everywhere" (the website's), `POST /v1/auth/logout` with the token, account deletion and blocking, like any session. With the token itself, `signOutEverywhere` ends only the token (as for app tokens).
- **Rights:** the account's own. Its roles count, staff roles included, so a moderator can use the review queries from a playground. It is never the owner (`CYEX_TV_OWNER_IDS`'s all-permissions don't carry over), and the website-only actions stay website only: deleting the account, the data copy, signing other sessions out, consenting to an app, making another token. No scope list: it is meant for the person themselves.
- **Limits:** 5 an hour per account (`CYEX_TV_LIMIT_CONSOLE_TOKENS`, not per role), plus the `/v1/auth/*` limit per IP. Its GraphQL calls count like any account's.
- **Audit:** `account.console_token_created` on the account, with the new session's ID and expiry and the replaced one's ID. Never the token.
- **Website:** signed in, the page defines `window.cyextv.token()`, an async function that calls the endpoint and returns the token, so `copy(await cyextv.token())` works in the console. It prints one hint per page load ("cyex.tv: copy(await cyextv.token()) copies a personal API token. Anyone with it can act as you; revoke it in your account."). Signed out, `cyextv.token` is removed. The token is never written to `localStorage`, `sessionStorage` or a cookie.

## Alternatives
- **The session cookie in `localStorage`:** what was asked for literally, but every script on the page could take the session. Refused.
- **Reading the cookie on request:** the cookie is `HttpOnly`, and an endpoint that echoes it would turn any XSS into a session theft with no way to revoke just that copy. A separate token can be signed out alone.
- **Scoped tokens chosen by the person:** more control, more UI. Can be added later as named personal tokens; the console token covers the playground and the account-merge case now.
- **No expiry:** convenient, but a forgotten token would work for ever. 90 days, and making a new one is one line in the console.

## Consequences
- A token that leaks can act as the person for up to 90 days, short of the website-only actions. The hint says so; "Sign out everywhere" on the website ends it, and `mySessions` / `revokeSession` show and end it through the API (the website has no sessions list since #160).
- Bearer tokens are now app sign-ins (ADR 0021) or console tokens. `Auth::caller` takes a Bearer token only for those two kinds; the cookie only for `site`.
- A future account merge can ask for both accounts' proof with a console token from each.
