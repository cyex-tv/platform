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
