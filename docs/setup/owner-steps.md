# Owner steps for Phase 0

These are the things only you can do: accounts, app registrations, DNS. The values are prepared below. Nothing here touches cyex.app or cyex-api.

## 1. GitHub CLI
```powershell
winget install GitHub.cli
```
Open a new terminal so `gh` is on the PATH, then log in:
```powershell
gh auth login
```
Choose GitHub.com, HTTPS, and log in with the browser. The account needs the right to create repos in the `cyex-tv` org. Check it with:
```powershell
gh auth status
```

## 2. Twitch app (new, not the cyex.app one)
At https://dev.twitch.tv/console/apps, choose "Register Your Application":
- Name: `cyex.tv`
- OAuth Redirect URLs (Twitch allows `http` only for `localhost`, so not `127.0.0.1`):
  - `https://api.cyex.tv/v1/auth/twitch/callback`
  - `http://localhost:8080/v1/auth/twitch/callback`

Local sign-in tests open the site through `localhost`, not `127.0.0.1`.
- Category: Website Integration
- Client type: **Confidential**

Then generate a client secret. Keep the client ID and secret in your password manager. They go into the server's `.env` later, never into the repo.

Scopes: none. We only need the user's identity.

## 3. Kick app
Kick requires 2FA on your Kick account before you can create an app. Then go to kick.com → Account Settings → Developer → Create app:
- Name: `cyex.tv`
- Redirect URLs:
  - `https://api.cyex.tv/v1/auth/kick/callback`
  - `http://localhost:8080/v1/auth/kick/callback`
- Scopes: **`user:read` only** ("Read user information"). It would allow reading the email
  address; cyex.tv never reads or stores it.
- Webhooks: **off**. cyex.tv uses no Kick events.

Keep the client ID and secret in your password manager.

Please **read Kick's Developer Terms** on that page. Their text isn't public, so I couldn't check them. Tell me about anything on caching user data, branding or third-party chat tools.

## 4. Cloudflare
- If `cyex.tv` isn't on Cloudflare yet, add the zone, and point the domain's nameservers to the two Cloudflare gives you.
- R2: create a bucket `cyex-tv-cdn`, and under Settings → Custom Domains connect `cdn.cyex.tv`. Leave public access through `r2.dev` **off**.
- DNS for `cyex.tv`, `www.cyex.tv` and `api.cyex.tv` is only needed once there is something to serve (Phase 6 and Phase 9). Don't create tunnel routes yet.
- Leave every cyex.app hostname and rule as it is.

## 5. When done
Mark which of 1-4 are done, and record them in `PROGRESS.md`. Don't paste any secrets into the chat.

# Owner steps for Phase 3: the CDN

The worker writes emote files to the R2 bucket `cyex-tv-cdn` (ADR 0003, ADR 0012). These steps make them public at `cdn.cyex.tv`. Nothing here touches cyex.app.

## 1. The bucket's custom domain
In Cloudflare → R2 → `cyex-tv-cdn` → Settings → Custom Domains, connect `cdn.cyex.tv` (if it isn't yet). Keep public access through `r2.dev` **off**.

## 2. What may be served
Only `emotes/` is public. `originals/` must never be. In R2 there is no per-prefix public switch, so add a WAF custom rule on `cdn.cyex.tv`:
- Name: `cyex.tv: block originals`
- Expression: `(http.host eq "cdn.cyex.tv" and not starts_with(http.request.uri.path, "/emotes/"))`
- Action: Block

(`cosmetics/` joins the allowed prefixes in the cosmetics phase.)

## 3. Caching
The files carry `Cache-Control: public, max-age=31536000, immutable`, and their keys never change. Add a Cache Rule on `cdn.cyex.tv`:
- When: hostname equals `cdn.cyex.tv`
- Cache eligibility: eligible for cache
- Edge TTL: use the cache-control header from the origin
- Browser TTL: respect origin

## 4. CORS
Chat clients and the extension fetch the files from other origins. In R2 → `cyex-tv-cdn` → Settings → CORS policy:
```json
[
  {
    "AllowedOrigins": ["*"],
    "AllowedMethods": ["GET", "HEAD"],
    "AllowedHeaders": ["*"],
    "MaxAgeSeconds": 86400
  }
]
```

## 5. Keys for the worker and the api
In R2 → Manage API tokens, create an **Account API token** with **Object Read & Write** on `cyex-tv-cdn` only (optionally limited to the server's IPs). The worker and api need its Access Key ID, Secret Access Key and the account's S3 endpoint, not the token value. Put them in `/etc/cyextv/api.env` and `worker.env` (never the repo or a chat):
```
CYEX_TV_S3_ENDPOINT=https://<account id>.r2.cloudflarestorage.com
CYEX_TV_S3_BUCKET=cyex-tv-cdn
CYEX_TV_S3_ACCESS_KEY_ID=<from the token>
CYEX_TV_S3_SECRET_ACCESS_KEY=<from the token>
CYEX_TV_S3_REGION=auto
```

## 6. The purge token for takedowns
Manage Account → Account API Tokens → Create Token, with one permission policy: **Specified
Domains → `cyex.tv`**, **Cache & Performance → Cache → Purge**. Optionally limited to the
server's IPs. Copy the token value at once. The token dialog's "Example usage" box prints the
token in clear text, so never screenshot or share that dialog.

The Zone ID is on the cyex.tv zone's Overview page (right column, "API"). Both go in
`/etc/cyextv/worker.env`:
```
CYEX_TV_CLOUDFLARE_ZONE_ID=<zone id>
CYEX_TV_CLOUDFLARE_PURGE_TOKEN=<token value>
```

## Done (2026/11/17)
- Twitch and Kick apps registered, redirect URLs as above.
- Cloudflare: the cyex.tv zone; R2 bucket `cyex-tv-cdn` (location automatic, Eastern Europe;
  Standard class); custom domain `cdn.cyex.tv` active with minimum TLS 1.2; r2.dev off; CORS;
  the WAF rule and the cache rule; the storage token; the purge token (rolled once after it was
  shown in a screenshot).
- Checked from outside: `cdn.cyex.tv/emotes/…` reaches R2 (404 for a missing file), `/originals/…`
  and other paths answer 403, and CORS sends `Access-Control-Allow-Origin: *`.


# Owner steps 2026/11/17: security headers at Cloudflare for cyex.tv

The website's Node server sets its security headers and a Content-Security-Policy on every page
it renders. Prerendered pages are files the server sends as they are, without that code: today
only `/developers` (the API docs). They carry their script policy in a `<meta>` tag, but a
`<meta>` policy can't say `frame-ancestors`, and the other headers aren't there at all. These two
rules add them at Cloudflare. Nothing here touches cyex.app, api.cyex.tv or cdn.cyex.tv.

Where: the `cyex.tv` zone → Rules → Overview → Create rule → **Response Header Transform Rule**
(older dashboards: Rules → Transform Rules → Modify Response Header → Create rule).

## 1. Rule "cyex.tv security headers"
- If incoming requests match: **Custom filter expression**, Edit expression:
  ```
  (http.host eq "cyex.tv")
  ```
- Then, one row per header, operation **Set static**:

  | Header name | Value |
  |---|---|
  | `X-Frame-Options` | `DENY` |
  | `X-Content-Type-Options` | `nosniff` |
  | `Referrer-Policy` | `strict-origin-when-cross-origin` |
  | `Permissions-Policy` | `camera=(), microphone=(), geolocation=()` |
  | `Strict-Transport-Security` | `max-age=31536000` |

  These are the values the server already sends, so on server-rendered pages nothing changes.
- Deploy.

## 2. Rule "cyex.tv CSP for prerendered pages"
Only for the prerendered paths: **Set** replaces a header, and on every other page it would
replace the server's full policy (with its per-request nonce) with this short one.
- Custom filter expression:
  ```
  (http.host eq "cyex.tv" and http.request.uri.path in {"/developers" "/developers/"})
  ```
- Then, operation **Set static**:

  | Header name | Value |
  |---|---|
  | `Content-Security-Policy` | `frame-ancestors 'none'; base-uri 'self'; object-src 'none'; form-action 'self'` |

  The page's own `<meta>` policy (scripts by hash, styles, images, connections) still applies;
  the browser enforces both.
- Deploy. If a later release prerenders another page, add its path to this expression (the
  build lists them under `apps/web/build/prerendered/`).

## 3. Check
```
curl -sI https://cyex.tv/developers | grep -iE "content-security|x-frame|nosniff|referrer|permissions|strict-transport"
curl -sI https://cyex.tv/ | grep -i content-security
```
The first shows all six headers. The second still shows the long policy with
`script-src 'self' 'sha256-…' … 'nonce-…'` and `frame-ancestors 'none'` at the end, which means
rule 2 didn't catch it.
