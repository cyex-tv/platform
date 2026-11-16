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
- OAuth Redirect URLs (add all three):
  - `https://api.cyex.tv/v1/auth/twitch/callback`
  - `http://localhost:8080/v1/auth/twitch/callback`
  - `http://127.0.0.1:8080/v1/auth/twitch/callback`
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
- Scopes: **`user:read` only**

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
In R2 → Manage API tokens, create one token with **Object Read & Write** on `cyex-tv-cdn` only. Put its values in the server's `.env` (never the repo or a chat):
```
CYEX_TV_S3_ENDPOINT=https://<account id>.r2.cloudflarestorage.com
CYEX_TV_S3_BUCKET=cyex-tv-cdn
CYEX_TV_S3_ACCESS_KEY_ID=<from the token>
CYEX_TV_S3_SECRET_ACCESS_KEY=<from the token>
CYEX_TV_S3_REGION=auto
```
