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
- Expression: `(http.host eq "cdn.cyex.tv" and not starts_with(http.request.uri.path, "/emotes/") and not starts_with(http.request.uri.path, "/cosmetics/"))`
- Action: Block

`/cosmetics/` holds the chat badge and paint files (ADR 0015). Their originals are under `/originals/cosmetics/`, which this rule keeps blocked. If the rule was made with `/emotes/` only, edit its expression to the one above.

## 3. Caching
The files carry `Cache-Control: public, max-age=31536000, immutable`, and their keys never change. Add a Cache Rule on `cdn.cyex.tv`:
- When: hostname equals `cdn.cyex.tv`
- Cache eligibility: eligible for cache
- Edge TTL: use the cache-control header from the origin
- Browser TTL: respect origin

Only the sha256-keyed files (`emotes/`, `cosmetics/badges/`, `cosmetics/paints/`) are set to `immutable` by the worker (`cyex_tv_storage::cache_control_for`). The private `originals/` have no `Cache-Control` and the WAF rule keeps them unreachable. Reads from the API (`api.cyex.tv`) are not content-addressed and keep `max-age=30` with an `ETag`; do not add a long Cache Rule for them.

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

## 7. Media limits (optional)
The upload limits (file size, side, frames, pixels, duration per emote, badge and paint) and the
badge and paint size ladder are `CYEX_TV_LIMIT_*` variables with the numbers the code ships with
as defaults; `.env.example` lists them all. Set the same values in `api.env` and `worker.env`
(the api checks uploads with them, the worker decodes with them), then restart both; either
refuses to start on a value that doesn't read. Nothing needs setting unless a limit should
change. An emote's side can't be set above 1000 px. `GET /v1/limits` and the GraphQL `limits`
query show what a running api enforces, so check them after a change.

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
`/developers` (the API docs), `/help`, `/guidelines`, `/changelog` and `/download`. They carry their script policy in a `<meta>` tag, but a
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
  (http.host eq "cyex.tv" and http.request.uri.path in {"/developers" "/developers/" "/help" "/help/" "/guidelines" "/guidelines/" "/changelog" "/changelog/" "/download" "/download/"})
  ```
- Then, operation **Set static**:

  | Header name | Value |
  |---|---|
  | `Content-Security-Policy` | `frame-ancestors 'none'; base-uri 'self'; object-src 'none'; form-action 'self'` |

  The page's own `<meta>` policy (scripts by hash, styles, images, connections) still applies;
  the browser enforces both.
- Deploy. If a later release prerenders another page, add its path (and the trailing-slash form) to this expression; a test (`src/prerender-doc.test.ts`) fails until you do (the
  build lists them under `apps/web/build/prerendered/`).

## 3. Check
```
curl -sI https://cyex.tv/developers | grep -iE "content-security|x-frame|nosniff|referrer|permissions|strict-transport"
curl -sI https://cyex.tv/ | grep -i content-security
```
The first shows all six headers. The second still shows the long policy with
`script-src 'self' 'sha256-…' … 'nonce-…'` and `frame-ancestors 'none'` at the end, which means
rule 2 didn't catch it.

# Owner steps 2026/11/17: Paddle for the cyex.tv shop

ADR 0016. cyex.tv gets its **own** Paddle account. Don't reuse or change anything in cyex.app's
Paddle setup (its products, keys or webhooks), and nothing here touches cyex-api. Until these
steps are done the shop lists items and checkout says it isn't available yet.

Do it in the **sandbox** first (https://sandbox-vendors.paddle.com), with `CYEX_TV_PADDLE_ENV=sandbox`,
and test with Paddle's test cards. Repeat in the live account (https://vendors.paddle.com) only
when the sandbox works, then switch to `production`.

## 1. The account
- Sign up for a new Paddle Billing account for cyex.tv (a sandbox one, and later the live one).
- Live only: in **Checkout → Website approval**, add `cyex.tv` and let Paddle review it.
- **Checkout → Checkout settings → Default payment link:** `https://cyex.tv/shop`.

## 2. Products and prices
- **Catalog → Products → New product**, one per shop item (or one "cyex.tv chat cosmetics"
  product with one price per item). Tax category: **Standard digital goods**.
- On each, **New price**: one-time (not recurring), EUR, the amount you want. Copy its ID
  (`pri_…`); it goes into the item's "Paddle price ID" on the cyex.tv admin page.

## 3. Keys
- **Developer tools → Authentication → API keys → New API key**, named `cyex.tv api`, with
  write and read access to **Transactions** and **Adjustments** (nothing else needed). Sandbox
  keys start with `pdl_sdbx_`, live ones with `pdl_live_`; the api refuses a key of the wrong
  environment.
- **Developer tools → Authentication → Client-side tokens → New client-side token**, named
  `cyex.tv website` (sandbox ones start with `test_`, live ones with `live_`).

## 4. The webhook
- **Developer tools → Notifications → New destination**:
  - Description: `cyex.tv api`
  - Type: **Webhook**, URL `https://api.cyex.tv/v1/shop/paddle/webhook`
  - Usage type: platform
  - Events: `transaction.completed`, `transaction.canceled`, `adjustment.created`,
    `adjustment.updated`
- Save, then copy the destination's **secret key** (`pdl_ntfset_…`).
- Cloudflare: make sure no WAF or bot rule challenges `POST api.cyex.tv/v1/shop/paddle/webhook`
  (Paddle can't solve a challenge). The api checks the signature itself.

## 5. Env values
In `/srv/cyex-tv/infra/env/api.env` on the server (never in a repo):
```
CYEX_TV_PADDLE_ENV=sandbox
CYEX_TV_PADDLE_API_KEY=<the API key>
CYEX_TV_PADDLE_CLIENT_TOKEN=<the client-side token>
CYEX_TV_PADDLE_WEBHOOK_SECRET=<the destination's secret key>
```
Restart the api (`docker compose up -d api`). Its log says `paddle` with the environment, and
warns for anything still unset.

## 6. Check (sandbox)
- On cyex.tv, as staff, make an item with the sandbox price ID and make it visible.
- Buy it with Paddle's test card `4242 4242 4242 4242` (any future expiry, any CVC). The order
  turns paid within seconds and the badge or paint shows up under your cosmetics.
- In **Developer tools → Notifications → Logs**, the deliveries show `200`.
- Refund the order from the admin orders page: Paddle shows a refund adjustment, the cosmetics
  are gone.


# Hardening checks 2026/11

Three checks and rules for the Cloudflare dashboard, from the security assessment
(`docs/security/assessment-2026-11.md`). Do them in order. Nothing here touches cyex.app, and
none of it needs a secret, an account ID or the server's IP address in a chat or a repo. In the
`curl` checks, `<origin-ip>` stands for the server's public address, which you look up yourself.

## 1. The origin is reachable only through the tunnel

Goal: the server answers no request that did not come through Cloudflare. Check each layer.

**a. No public DNS record points at the server.**
Cloudflare → `cyex.tv` zone → DNS → Records. For `cyex.tv`, `api`, `cdn` and `www` the only
records must be `CNAME` records whose target ends in `.cfargotunnel.com` (made by the tunnel's
public hostnames), proxied (orange cloud). Delete any `A` or `AAAA` record for these names that
holds the server's address, and any record named like `origin`, `direct`, `ssh` or `ftp` that
does. Also look at MX and TXT records: they must not reveal the server's address.
Check from any machine:
```
dig +short A cyex.tv api.cyex.tv cdn.cyex.tv www.cyex.tv
dig +short AAAA cyex.tv api.cyex.tv cdn.cyex.tv www.cyex.tv
```
Every address printed must be a Cloudflare one (104.16.0.0/13, 172.64.0.0/13, 2606:4700::/32),
never the server's own.

**b. The tunnel's ingress names only the three hostnames and ends in a 404.**
Zero Trust → Networks → Tunnels → the tunnel → Public Hostnames (or the `ingress:` block of
`config.yml`; the expected content is in the infra repo's `docs/tunnel.md`). The order must be:
`api.cyex.tv` (path `^/v1/events$` first), `api.cyex.tv`, `cyex.tv`, `cdn.cyex.tv`, then the
existing cyex.app rules, and last the catch-all `http_status:404`. Each service URL names a
container (`http://cyex-tv-web-1:3000`, and so on) or `127.0.0.1`, never a public address. A
rule without a `hostname` anywhere but last would send unknown hosts to a service.
Check: `cloudflared tunnel ingress validate` on the server says the file is valid, and
`cloudflared tunnel ingress rule https://unrouted.cyex.tv/` prints the catch-all
(`http_status:404`), not one of our services.

**c. The host's firewall and Docker publish nothing.**
On the server (as the admin user):
```
sudo ufw status verbose
ss -ltnp | grep -E ':(80|443|3000|7070|808[0-3])\b'
```
`ufw` must say `Default: deny (incoming)` with only the SSH rule you use, nothing on 80, 443,
8080-8083 or 7070. Every listener the `ss` line prints must show `127.0.0.1:` (or `[::1]:`),
never `0.0.0.0:` or `*:`. The compose already publishes `127.0.0.1:...` only. Docker bypasses
`ufw` for published ports, so the listener check is the one that counts; if a port shows
`0.0.0.0`, fix the compose line instead of adding a firewall rule. If the provider has a cloud
firewall, set inbound to deny-all except SSH from your own address; `cloudflared` only makes
outbound connections.

**d. Verify from outside: a direct request to the origin fails.**
From a machine that is not the server:
```
curl -m 8 -sS -o /dev/null -w "%{http_code}\n" http://<origin-ip>/
curl -m 8 -sS -o /dev/null -w "%{http_code}\n" -k https://<origin-ip>/
curl -m 8 -sS -o /dev/null -w "%{http_code}\n" http://<origin-ip>:8080/
curl -m 8 -sS -o /dev/null -w "%{http_code}\n" -H "Host: cyex.tv" http://<origin-ip>/
```
Every line must fail (`Connection refused`, `Connection timed out`, or an empty `000`). Any
`200`, `301` or `404` printed means something answers on the public address: find the listener
with the `ss` command above. The public name still works:
```
curl -sI https://cyex.tv/ | head -1
```
which prints `HTTP/2 200`.

## 2. `nosniff` on cdn.cyex.tv

The CDN server (Caddy) already sends `X-Content-Type-Options: nosniff` on `/emotes/` and
`/cosmetics/` and on its errors. This rule adds it at the edge for the whole host, so it also
holds for `originals/` paths (they answer 404 or 403, never a file, but an error page must not
be sniffed either) and if the origin is ever changed or replaced. Emote, badge and paint files
are images with a Content-Type set from the sniffed format, so no `Content-Disposition` is
needed; leave that header alone (a forced `attachment` would break `<img>` use of the same URL
in some clients).

Where: `cyex.tv` zone → Rules → Overview → Create rule → **Response Header Transform Rule**
(older dashboards: Rules → Transform Rules → Modify Response Header → Create rule).
- Rule name: `cdn.cyex.tv nosniff`
- If incoming requests match: **Custom filter expression**, Edit expression:
  ```
  (http.host eq "cdn.cyex.tv")
  ```
- Then: operation **Set static**, header name `X-Content-Type-Options`, value `nosniff`.
- Order: any. It is the only response-header rule for this host (the 2026/11/17 "cyex.tv
  security headers" rule matches `cyex.tv` only). Deploy.

Check (a file that exists, and an originals path):
```
curl -sI https://cdn.cyex.tv/emotes/<a-known-hash>/2x.webp | grep -iE "^(HTTP|content-type|x-content-type-options)"
curl -sI https://cdn.cyex.tv/originals/anything | grep -iE "^(HTTP|x-content-type-options)"
```
The first shows `200`, `content-type: image/webp` and `x-content-type-options: nosniff`; the
second shows `404` or `403` and `x-content-type-options: nosniff` (one header line: Cloudflare
replaces the value, it does not add a second one).

## 3. CSP and frame rule: every prerendered page

Prerendered pages are static files the website's Node server sends without its header code, so
the rule "cyex.tv CSP for prerendered pages" from the 2026/11/17 steps must list every one of
them. The web app prerenders exactly these pages today (`export const prerender = true` in
`apps/web/src/routes/`): `/developers`, `/help`, `/guidelines`, `/changelog`, `/download`.
(`/developers/groups/<id>.json` is prerendered too, but it is JSON, not a page; it gets
`nosniff` and the other headers from the rule "cyex.tv security headers".) The test
`apps/web/src/prerender-doc.test.ts` reads the expression of rule 2 above and fails when a
prerendered page is missing from it.

Open the existing rule: `cyex.tv` zone → Rules → Overview → "cyex.tv CSP for prerendered pages"
→ Edit → Edit expression, and make the whole expression this:
```
(http.host eq "cyex.tv" and http.request.uri.path in {"/developers" "/developers/" "/help" "/help/" "/guidelines" "/guidelines/" "/changelog" "/changelog/" "/download" "/download/"})
```
- Then (unchanged): operation **Set static**, header `Content-Security-Policy`, value
  `frame-ancestors 'none'; base-uri 'self'; object-src 'none'; form-action 'self'`.
- Order: it sets a different header from "cyex.tv security headers", so it can sit before or
  after it. Keep the path list: without it the rule would replace the server's full policy on
  every other page. Deploy.
- When a release prerenders another page, add its path and its trailing-slash form to the list
  in the dashboard and in the 2026/11/17 section of this file.

Check every page, and one that is not prerendered:
```
for p in developers help guidelines changelog download; do
  printf "%s: " "$p"; curl -sI "https://cyex.tv/$p" | grep -ic "frame-ancestors 'none'"
done
curl -sI https://cyex.tv/help/ | grep -i "content-security-policy"
curl -sI https://cyex.tv/ | grep -i "content-security-policy"
```
Each page prints `1`. The last request (`/`) shows the long policy with `nonce-…`, not the
short one above.
