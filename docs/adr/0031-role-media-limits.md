# 0031: Per-role media limits

- Status: accepted (owner approved, 2026/11/26)
- Date: 2026/11/26
- Amends: 0025 (badge uploads), 0026 (paint uploads); builds on the configurable media limits (`CYEX_TV_LIMIT_*`)

## Context
Media limits (bytes, side, frames) came from the environment and were the same for everyone. Supporters and partners should be able to upload bigger badges and paints, and a trial role smaller emotes, without a deploy.

## Decision
- A role sets media numbers like any other per-role limit (`setRoleLimits`, `roles.limits`): `emote_max_bytes`, `emote_max_side`, `emote_max_frames`, `badge_max_bytes`, `badge_max_side`, `badge_max_frames`, `paint_max_bytes`, `paint_max_side`. Unset means the configured number. An account gets the most generous override among its roles, as for every limit.
- **Ceilings.** A role may set a number from 1 to a ceiling in the media config (`CYEX_TV_LIMIT_ROLE_<KIND>_MAX_BYTES|SIDE|FRAMES`; by default 16 MB for all, side 1000 for emotes, 4096 for badges, 6144 for paints; frames 2000, 240, 240). A ceiling is never below the configured number. **An emote's side is at most 1000 px**: it can be lowered, never raised, whatever a role or the environment says. The API refuses a higher value (`bad_limit`), reads a stored higher value as unset, and the limits are cut again where they are made.
- **One place makes the numbers.** `MediaConfig::effective(kind, overrides)` returns the limits an upload is checked under. The API calls it with the uploader's resolved limits; the worker calls it with the numbers stored on the job.
- **The worker never re-derives from a role.** The API puts `limits` (`max_bytes`, `max_side`, `max_frames`) in the `emote.transcode` and `cosmetic.transcode` payloads. A cosmetic picture also keeps them (`cosmetic_images.media_limits`, widened when the same bytes are uploaded under bigger limits), so a remake uses them, not the remaker's. A job without them (queued before this) runs under the configured limits. The worker cuts what a job carries at the ceiling.
- **Safe scaling.** The upload body limit is the largest ceiling plus 64 KB; the decoders' allocation cap is derived from the limits being checked (a frame of the largest allowed side, twice, at least 64 MB) instead of a fixed 64 MB.
- `limits.account` and `uploadQuota` report the effective numbers; the upload dialog and the badge and paint upload show them.

## Consequences
- A bigger badge is accepted and stored as its original, but the size ladder is unchanged: files are made from the 512 px master and the `original` is fitted into `CYEX_TV_LIMIT_BADGE_ORIGINAL_PX` (512 by default). Raise that too for larger originals.
- Additive: a nullable column, new limit keys, a new `MEDIA` group and a `max` field on `RoleLimit`.
