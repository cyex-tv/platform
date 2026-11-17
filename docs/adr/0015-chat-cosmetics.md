# 0015: Chat cosmetics (badges and name paints)

- Status: accepted
- Date: 2026/11/17

## Context
The owner wants cyex.tv's own chat cosmetics, the kind 7TV has: a badge next to a chatter's name and a "paint" that colours the name with a gradient or a picture. The system comes first; no actual cosmetics exist yet. ADR 0011 fixed the boundary: cyex.tv cosmetics never mix with the cyex deck's supporter badges, and they get their own provider name ("cyex.tv"), the `/v1/cosmetics` namespace, `cosmetic.*` events, the `cosmetics/` R2 prefix and `cosmetics.*` permissions. ADR 0014 reserved the account editor flag `profile` for them.

7TV's public model was studied from its public API docs and v3 GraphQL types (no code taken, AGENTS.md): a cosmetic is a badge (name, tooltip, image files at several scales) or a paint (a function `LINEAR_GRADIENT`, `RADIAL_GRADIENT` or `URL`, an angle, `repeat`, colour stops, drop shadows, an image for the URL kind); users own cosmetics through entitlements and pick one badge and one paint in their style; clients hear `cosmetic.create` and `entitlement.create`/`delete` over the event API. We keep the ideas and write our own shapes.

## Decision
- **Badges:** id, name, tooltip, an image. Made by staff with `cosmetics.manage`.
- **Paints:** id, name, kind (`linear_gradient`, `radial_gradient`, `image`), angle (0 to 360 degrees, CSS convention: 0 points up, 90 to the right), repeat, shape (`circle` or `ellipse`, radial only), an optional fallback colour, 2 to 10 stops (`at` 0 to 1, never decreasing, colour), up to 4 drop shadows (x, y, radius in px, colour), and an image for the image kind (no stops then). Made by staff with `cosmetics.manage`.
- **Images:** staff upload a PNG, APNG, GIF or WebP to `POST /v1/cosmetics/images` (`kind` badge or paint). The original goes to `originals/cosmetics/{sha256}` (never public); the worker's `cosmetic.transcode` job makes WebP (animated when the original is) and AVIF at 1x, 2x and 4x, with static first frames for animations, the emote pipeline with a size profile (`crates/media`): badges 18 px at 1x (square, at least 72×72 px), paint images 64 px tall at 1x (up to 8:1). Files go to `cosmetics/badges/{sha256}/{name}` or `cosmetics/paints/{sha256}/{name}`, immutable. A badge or image paint can only use an image whose files are made (`image_not_ready`).
- **Entitlements:** `badge_entitlements` and `paint_entitlements` (account, cosmetic, source `grant` | `purchase` | `event`, reason, granted by, when). Staff with `cosmetics.grant` grant (with a reason) and revoke. Only `grant` is written now; the other sources wait for a shop or events.
- **Selection:** `cosmetic_selections` (account, badge, paint), both nullable. The database enforces "only owned ones": each column is a foreign key into its entitlement table with `ON DELETE SET NULL (column)`, so revoking an entitlement or deleting a cosmetic clears the selection in the same statement.
- **Who may select:** the account itself, or its account editors with `profile` (ADR 0014; audited with `"via": "account_editor"`). Editors never grant.
- **Permissions:** `cosmetics.manage` (create, change, delete badges, paints and their images) and `cosmetics.grant` (grant, revoke, read entitlements). The owner holds both (ADR 0010: the owner holds every permission in `permissions::ALL`); there are no seeded roles, so admins get them by adding them to a role.
- **Names:** 1 to 32 characters, tooltips 1 to 64, refused with hidden or control characters (`crates/shared/src/text.rs`), unique per kind ignoring case. Colours are `#RRGGBB` or `#RRGGBBAA`, stored upper case. Shadow offsets −16 to 16 px, radius 0 to 16 px.
- **Reads (public):** GraphQL `cosmetics` (the catalog) and `userCosmetics(platform, userIds)` (up to 100 chatters: their selected badge and paint IDs); REST `GET /v1/cosmetics` and `GET /v1/cosmetics/users?platform=&ids=`, both with `ETag` and `Cache-Control` like the other client reads. Blocked and deleted accounts show nothing. Signed in: `myCosmetics(accountId)` (owned and selected; an account you edit with `profile` by id).
- **Events (ADR 0005):** a new public topic `cosmetics` carries `cosmetic.created`, `cosmetic.updated` and `cosmetic.deleted` with the cosmetic's kind and id (the event socket also sends the full definition). A selection change sends `user.cosmetics_updated` with the new badge and paint IDs on the account's `user:<platform>:<id>` topics, from a selection or from a revoke that cleared it. Deleting a cosmetic sends only `cosmetic.deleted`; clients drop it everywhere. Grants are private and send nothing.
- **Audit:** every staff action writes `audit_log`: `cosmetic_image.uploaded`, `badge.created`/`updated`/`deleted`, `paint.created`/`updated`/`deleted`, `cosmetic.granted`/`revoked` (target the account). A selection made by an editor is audited as `cosmetics.selected`.
- **Limits:** the image upload counts against the per-IP upload limit; every cosmetics write counts against a per-account limit of 300 an hour (`429 rate_limited`). Reads fall under the public read limit.
- **API (additive, ADR 0008):** new types, queries, mutations, REST routes and event types only. Existing fields keep their meaning.

## Consequences
- Clients (extension, deck) fetch the catalog once with `If-None-Match`, look chatters up in batches, and follow `cosmetics` plus the user topics they already watch.
- Image files are content-addressed and never deleted with their badge or paint, so a cached URL never breaks; unused files can be cleaned later.
- The owner must allow `/cosmetics/` in the CDN's WAF rule (`docs/setup/owner-steps.md`).
- Covered by Postgres tests in CI (`apps/api/tests/cosmetics_flow.rs`, `apps/worker/tests/cosmetics.rs`) and unit tests of the validation and the image profiles.
