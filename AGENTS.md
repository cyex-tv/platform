# cyex.tv platform: rules for coding agents

Read `PLAN.md` (the plan) and `PROGRESS.md` (where we are) before any work. Decisions live in `docs/adr/`.

## Working rules
- The source of truth is `PLAN.md` and `PROGRESS.md`. Update `PROGRESS.md` at the end of every task, and log every write action to GitHub with its exact command.
- One task = one branch = one PR. Commits stay small. The owner reviews and merges; the agent never merges.
- Never push to `main`. No force pushes, no history rewrites.
- Architecture decisions go in `docs/adr/NNNN-title.md`. Never decide silently.
- No code before its phase's "Decisions needed" are answered.
- No feature is done without a test, or a manual check written in `PROGRESS.md`.
- Commits are authored by the owner's GitHub account. No co-author trailers, and no tool or AI attribution in code, docs, commits or PRs.
- Secrets never go into the repo. This repo is **public** (AGPL-3.0). Use `.env.example` and a secrets manager.
- Accounts, app registrations, DNS, billing and store listings are done by the owner. The agent prepares the exact values and steps.

## Separation from cyex.app (hard limits)
cyex-api (api.cyex.app) serves cyex logs, recap, deck badges, the store and payments. cyex.tv must never be able to break it.
1. No changes to `cyex-app/cyex-api` for cyex.tv work. If something seems to need one, stop and ask the owner.
2. cyex.tv never connects to `deck-db`, or to cyex-api's Redis or ClickHouse.
3. The only contact with cyex.app is clients reading the public badge API `api.cyex.app/badges/v1`, following its spec.
4. Deploying adds new containers, hostnames and tunnel routes only. It never edits cyex-api's containers, compose files, env or Cloudflare rules.
5. Nothing is copied from cyex-api. Similar features (sign-in, sessions, blocklist, audit) are written in Rust here.

## GitHub scope (hard limits)
- Repos the agent may create: `cyex-tv/platform`, `cyex-tv/extension`, `cyex-tv/infra`. Nothing else in the org: no teams, members, org settings, Actions secrets, billing or webhooks.
- Other repos the agent may work in: `cyex-app/cyex-deck`, on feature branches with PRs only.
- No repo deletion, no visibility changes, no API calls that change org settings.

## Borrowed code
- None from 7TV or BTTV; their licences forbid it.
- FFZ (Apache-2.0) may be reused, with its notice kept.
- Public protocols and response shapes may be studied.

## Design
- Every web surface follows the cyex design system: the shared core, plus the tool, landing-page or docs layer as fits.
- Tokens only, both themes, and reduced motion.
- Product names are lowercase, and the platform is written "cyex.tv".
