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
The upload limits (file size, side, frames, pixels, duration per badge and paint) and the
badge and paint size ladder are `CYEX_TV_LIMIT_*` variables with the numbers the code ships with
as defaults; `.env.example` lists them all. Set the same values in `api.env` and `worker.env`
(the api checks uploads with them, the worker decodes with them), then restart both; either
refuses to start on a value that doesn't read. Nothing needs setting unless a limit should
change. Emotes have no variables: their rules are fixed in code and the same for everyone
(1000 x 1000 px, 8 MB, 1000 frames, 50 fps, at most 4:1 wide; ADR 0031), so delete any old
`CYEX_TV_LIMIT_EMOTE_*` or `CYEX_TV_LIMIT_ROLE_EMOTE_*` line (they are ignored now). `GET /v1/limits`
and the GraphQL `limits` query show what a running api enforces, so check them after a change.

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

## 2026/11/26: Panchang is fetched at build time
The landing page's headline font, Panchang (Indian Type Foundry, ITF Free Font License), is not in this repository: section 02 of its licence forbids distribution through public repositories. The website image build runs `npm run fonts` (`apps/web/scripts/fetch-fonts.mjs`), which downloads Fontshare's official zip and checks the woff2's sha256, so every deploy serves its own copy from cyex.tv (section 01). The build needs network access to api.fontshare.com. Dev, tests and the CI `web` job run without it and show the Arial fallback; run `npm run fonts` locally to see the real font. If Fontshare changes the file the build fails with a message: check it, then update the pin in the script.

# Owner steps 2026/11/27: on-the-fly resizing at the CDN (ADR 0037)
Optional. Until this is done, `?w=` and `?format=` on a picture URL are ignored (the file comes
back at its own size). The Worker's code is in the infra repo, `cloudflare/cdn-resize/`; it only
touches `cdn.cyex.tv` in the `cyex.tv` zone. Nothing here touches cyex.app.

## 1. Turn on transformations for the zone
Cloudflare dashboard (account level) → **Images** → **Transformations** → the row of the
**cyex.tv** zone → **Enable for zone**. Then open the zone's transformation settings and, under
**Sources**, choose **Specified origins** and add `cdn.cyex.tv` only (so no other host can be
resized through the zone). Don't turn on "Resize from any
origin". The URL format (`/cdn-cgi/image/...`) stays available to the zone, but only for
`cdn.cyex.tv` sources; the Worker is what clients use.

Billing: Images transformations are billed per unique transformation a month (check the
current free allowance on the Images page). The Worker allows 28 widths × 2 formats per file, and only those
actually asked for are made.

## 2. Deploy the Worker
From a checkout of cyex-tv/infra (Node 20 or later):
```bash
cd cloudflare/cdn-resize
npm test                                  # 14 tests, no packages
npx wrangler@4 login                      # opens the browser; your Cloudflare account
npx wrangler@4 deploy                     # name cyex-tv-cdn-resize, route cdn.cyex.tv/*
```
Or by hand: **Workers & Pages** → **Create** → **Create Worker** → name `cyex-tv-cdn-resize` →
Deploy → **Edit code**: paste `src/plan.js` and `src/index.js` as two files of the same names
(module syntax) → Deploy. Then **Settings** → **Variables and Secrets** → add the plain text
variable `CDN_HOST` = `cdn.cyex.tv` (no secrets are needed).

## 3. The route
If you deployed with wrangler, the route is already there; check it. Otherwise: the
**cyex.tv** zone → **Workers Routes** → **Add route**: route `cdn.cyex.tv/*`, Worker
`cyex-tv-cdn-resize`. Set **Request limit failure mode** to **Fail open (proceed)**, so if the
Worker's free request quota runs out, files are still served (without resizing). Don't add a
route for any other host.

Requests without `w` or `format` pass straight through, so the existing cache rule for
`cdn.cyex.tv` and the worker's takedown purge keep working as before.

## 4. Check
```bash
H=<an emote hash>
curl -sI "https://cdn.cyex.tv/emotes/$H/4x.webp"                         # 200, as before
curl -sI "https://cdn.cyex.tv/emotes/$H/4x.webp?w=48"                    # 200, image/webp,
                                                                       # cache-control: public, max-age=31536000, immutable
curl -s -o /tmp/a.bin -w '%{content_type}\n' -H 'Accept: image/avif' \
     "https://cdn.cyex.tv/emotes/$H/4x.webp?w=48&format=auto"           # image/avif
curl -s -o /dev/null -w '%{http_code}\n' "https://cdn.cyex.tv/emotes/$H/4x.webp?w=49"        # 400
curl -s -o /dev/null -w '%{http_code}\n' "https://cdn.cyex.tv/originals/$H?w=48"            # 400
curl -s -o /dev/null -w '%{http_code}\n' "https://cdn.cyex.tv/emotes/$H/4x.webp?w=48&x=1"   # 400
```
A second `?w=48` request comes from the Worker's edge cache (no new transformation is counted
on the Images page). Take down a test emote: its `?w=` URLs
answer 404 as soon as the original does (the Worker checks the original before serving a cached
variant).

To turn it off: remove the route (or the Worker). URLs with `?w=` then get the file at its own
size again.

# Owner steps 2026/11/27: direct uploads of badge and paint pictures (ADR 0036)
Optional. Until this is done the website sends badge and paint pictures as a form, as now. Once
it is, large originals (up to the 16 MB a role may allow) go from the browser straight to
`tv-s3`, and the API only checks them. Nothing here touches cyex.app.

## 1. A narrow upload front for tv-s3
`tv-s3` stays private. A second site in the `tv-cdn` Caddy passes only presigned `PUT`s under
`/<bucket>/uploads/` to it. In the infra checkout, append to `cdn/Caddyfile`:
```
# upload.cyex.tv: presigned PUTs of direct uploads (ADR 0036) to tv-s3, nothing else.
http://{$UPLOAD_HOST:upload.cyex.tv} {
	log {
		output stdout
		format json
	}
	request_body {
		max_size {$UPLOAD_MAX_BODY:17MB}
	}
	@preflight {
		method OPTIONS
		path /{$CDN_BUCKET:cyex-tv-cdn}/uploads/*
	}
	handle @preflight {
		header Access-Control-Allow-Origin "{$UPLOAD_SITE:https://cyex.tv}"
		header Access-Control-Allow-Methods "PUT"
		header Access-Control-Allow-Headers "content-type"
		header Access-Control-Max-Age "600"
		respond 204
	}
	@put {
		method PUT
		path /{$CDN_BUCKET:cyex-tv-cdn}/uploads/*
		query X-Amz-Signature=*
	}
	handle @put {
		header Access-Control-Allow-Origin "{$UPLOAD_SITE:https://cyex.tv}"
		# The Host header is passed on unchanged, so tv-s3 checks the signature against
		# upload.cyex.tv, the host the API signed.
		reverse_proxy tv-s3:7070
	}
	respond "404 not found" 404
}
```
The `tv-cdn` service is already on the `cyextv` network, so it reaches `tv-s3:7070`. The global
`read_body` timeout (`CDN_READ_TIMEOUT`, 10 s) also applies here: a 16 MB picture on a slow line
needs longer, so set `CDN_READ_TIMEOUT=120s` in `env/cdn.env` (it only matters for request bodies,
and the CDN takes none). Reload it:
```bash
cd /srv/cyex-tv/infra
docker compose -f compose/docker-compose.yaml --profile cdn up -d tv-cdn
docker compose -f compose/docker-compose.yaml --profile cdn exec tv-cdn caddy validate --config /etc/caddy/Caddyfile
```

## 2. The tunnel hostname
Cloudflare → Zero Trust → Networks → Tunnels → the cyex tunnel → **Public Hostnames** → Add a
public hostname: subdomain `upload`, domain `cyex.tv`, Service URL `http://cyex-tv-tv-cdn-1:80`
(the same container as `cdn.cyex.tv`; Caddy tells them apart by host). Save.

Then **Caching** → Cache Rules → Create rule "upload.cyex.tv: never cache": hostname equals
`upload.cyex.tv` → **Bypass cache**. Deploy. Uploads over 100 MB would hit Cloudflare's body
limit; ours are at most 16 MB.

## 3. The API's env
In `api.env` (not `worker.env`):
```
CYEX_TV_S3_UPLOAD_ENDPOINT=https://upload.cyex.tv
```
Restart the api: `docker compose -f compose/docker-compose.yaml up -d api`. The api signs for
this host with the same keys and region (`us-east-1`) as before.

## 4. Check
```bash
curl -s -o /dev/null -w '%{http_code}\n' -X GET https://upload.cyex.tv/cyex-tv-cdn/uploads/x   # 404
curl -s -o /dev/null -w '%{http_code}\n' -X PUT https://upload.cyex.tv/cyex-tv-cdn/emotes/x    # 404
curl -s -o /dev/null -w '%{http_code}\n' -X PUT https://upload.cyex.tv/cyex-tv-cdn/uploads/x   # 404 (unsigned)
```
Then on cyex.tv, upload a badge picture on the admin badge page. In the browser's network panel
you see `POST /v1/cosmetics/images/uploads` (201), a `PUT` to `upload.cyex.tv` (200) and
`POST .../finalize` (202). An upload never finalized is deleted by the worker within an hour
and a clean-up round.

To turn it off again, remove `CYEX_TV_S3_UPLOAD_ENDPOINT` and restart the api: the presign then
answers `501 direct_upload_off` and the website goes back to the form by itself.

# Deploy note 2026/11/27: indexes built `CONCURRENTLY` that ended up INVALID

Some migrations build an index with `CREATE INDEX CONCURRENTLY IF NOT EXISTS` (for example
`20261227000101_keyset_popular`, `..102_keyset_top`, `..103_owner_newest`), so a live table
keeps taking writes while it builds. If such a build is interrupted (the api restarted, a
deadlock, a duplicate row for a unique index), Postgres keeps the half-built index marked
**INVALID**: it is never used for reads but still slows every write, and `IF NOT EXISTS` skips
it on the next run, so the migration counts as done.

The api repairs this itself on every start: after the migrations it drops each invalid index the
migrations create and builds it again from its own definition, concurrently
(`cyex_tv_db::repair_invalid_indexes`, logged as `rebuilt an invalid index`). Invalid indexes
the migrations don't create are only logged (`invalid index our migrations don't create`), never
touched. If a rebuild fails again the api refuses to start; the log names the index.

To check by hand (`psql` against the cyex.tv database):

```sql
SELECT c.relname AS index, t.relname AS table
FROM pg_index i
JOIN pg_class c ON c.oid = i.indexrelid
JOIN pg_class t ON t.oid = i.indrelid
WHERE NOT i.indisvalid;
```

No rows: all good. For a row, drop the index and build it again, each statement on its own (not
in a transaction), then check again:

```sql
DROP INDEX CONCURRENTLY IF EXISTS emotes_keyset_popular;
-- then the CREATE INDEX CONCURRENTLY line from its migration file, for example
-- crates/db/migrations/20261227000101_keyset_popular.sql
```

Restarting the api does the same for the indexes the migrations create.
