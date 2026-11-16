# Progress

Checkpoint log for `PLAN.md`. Newest phase at the top. Every write action to GitHub is listed with its exact command.

## Phase 0: Setup and decisions (in progress)

### Tasks
- [x] 1. Owner: GitHub CLI installed and logged in as `sery-yx` (scopes `repo`, `read:org`, `workflow`, `gist`), 2026/11/16
- [ ] 2. Owner: register cyex.tv's own Twitch and Kick OAuth apps (values in `docs/setup/owner-steps.md`)
- [ ] 3. Owner: Cloudflare zone `cyex.tv`, DNS, R2 bucket on `cdn.cyex.tv` (tunnel hostnames wait until deploy)
- [x] 4. Created `cyex-tv/platform`, `cyex-tv/extension`, `cyex-tv/infra` with the scoped procedure, after the owner confirmed, 2026/11/16. Checks before creation: all three names were free and the org was empty.
- [x] 5. ADRs 0001-0011 written (`docs/adr/`), 2026/11/16
- [x] Q3: Kick API researched (`docs/research/kick-api.md`, ADR 0009), 2026/11/16. Owner still has to read Kick's Developer Terms.

### Decisions made 2026/11/16
D1-D21 in `PLAN.md`. The licence is AGPL-3.0 (ADR 0007).

### GitHub write log
2026/11/16, account `sery-yx`. All of these ran after the owner's explicit OK.

```
gh repo create cyex-tv/platform  --public --license agpl-3.0 --description "cyex.tv emote platform: Rust API, live events, image worker and the SvelteKit website"
gh repo create cyex-tv/extension --public --license agpl-3.0 --description "cyex.tv browser extension: emotes, badges and paints in Twitch and Kick chat"
gh repo create cyex-tv/infra     --public --license agpl-3.0 --description "cyex.tv deployment, Cloudflare config, backups and runbooks"
```

For each of `platform`, `extension` and `infra`:

```
gh api -X PUT   repos/cyex-tv/<repo>/branches/main/protection --input protection.json
gh api -X PATCH repos/cyex-tv/<repo> --input security.json
git push -u origin setup/initial
gh pr create --base main --head setup/initial
```

- `protection.json`: PR required (0 approvals), `enforce_admins: true`, no force pushes, no deletion.
- `security.json`: secret scanning and push protection on.
- Result checked by reading the settings back: all three match.

Same day, at the owner's request: no tool attribution anywhere. The rules file was renamed to `AGENTS.md`. The one setup commit per repo was re-authored as the owner's account, with no trailers, and force-pushed to the unmerged `setup/initial` branches only. This was a one-time exception the owner approved; `main` was not touched. The PR descriptions were edited.

```
git commit --amend --reset-author
git push --force-with-lease origin setup/initial
gh pr edit 1 --body-file <file>
```
