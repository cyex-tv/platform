# 0031: Per-role media limits (badges and paints)

- Status: accepted (owner approved, 2026/11/26); amended 2026/11/26: emotes are fixed, not per role (see Amendment); amended 2026/11/27: emotes are at most 1:3 tall (see Amendment 2)
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

## Amendment (2026/11/26): emote rules are fixed
The owner decided that emote upload rules are the same for everyone, whoever uploads. This replaces the emote parts above: a role can no longer set emote limits, and no environment variable can.
- **Rules**, in one place (`cyex_tv_media::EmoteRules`): at most 1000 × 1000 px (each side), at most 8 MB (8 × 1024 × 1024 bytes), at most 1000 frames, at most 50 fps, and width at most 4 × height (4:1).
- **50 fps** means no frame is shown for less than 20 ms. A GIF frame with a shorter delay (0 or 10 ms, as many tools write) isn't refused: as in browsers it is shown for 100 ms (`shown_ms`), so the effective rate never goes over 50 fps. The rule never rejects a file.
- **Shape.** Wider than 4:1 is refused at upload with `bad_dimensions` (422, "Emotes can be at most 4 times as wide as tall (4:1). This one is W×H px."). Before this, a wider picture was accepted and scaled to fit. A tall picture has no ratio rule (only the 1000 px side): 10 × 1000 is fine. Emotes already stored are untouched; making files again from an original wider than 4:1 now fails in the worker.
- **No hidden sixth rule.** The decode-work cap (width × height × frames) is exactly 1000 × 1000 × 1000, and there is no duration cap, so the five rules are all there is. (The 100 MP cap emotes had before would have refused 1000 × 1000 at more than 100 frames.)
- **Checked three times**: `POST /v1/emotes` (the probe), the worker before it makes files (`emote.transcode` decodes with `Limits::emote()` and ignores a `limits` that an older job carries), and the client before the upload (read from the API with a built-in copy as fallback). The API no longer puts `limits` in the `emote.transcode` payload.
- **Reported**, the same for everyone: the `limits` query / `GET /v1/limits` (`emote`: `maxBytes`, `maxSide`, `maxFrames`, `maxFps` 50, `maxAspect` 4, `maxTotalPixels`), `uploadQuota` (`maxBytes`, `maxSide`, `maxFrames`, `maxFps`, `maxAspect`) and `limits_used` in the upload answer.
- **Removed**: `emote_max_bytes`, `emote_max_side` and `emote_max_frames` from role limits (`setRoleLimits`, `roleLimits`, `limitDefaults`, `limits.account`, the role editor's File sizes group), and `CYEX_TV_LIMIT_EMOTE_*` and `CYEX_TV_LIMIT_ROLE_EMOTE_*` (ignored if still set). They were added hours before in #193 and never released. For compatibility `setRoleLimits` answers `bad_limit` ("emote limits are fixed") to an input that still names one, and a migration strips the keys from stored role limits. Badge and paint role limits are unchanged.
- Errors: a side or frame count over the rule, or a file over 8 MB, stays `too_large` (413; the byte cap carries `details {limit, current}`, ADR 0032). The messages state the limit.

## Amendment 2 (2026/11/27): at most 1:3 tall
The owner asked for a minimum width: an emote's shape must lie between 1:3 (tall) and 4:1 (wide). This replaces "a tall picture has no ratio rule" above.
- **Rule** (`EmoteRules::MAX_TALL` = 3, `Limits::max_tall`): height at most 3 × width. Exactly 1:3 is fine (100 × 300, 334 × 1000); 100 × 301 or 10 × 1000 is refused.
- **Error**: as for the wide rule, `bad_dimensions` (422, "Emotes can be at most 3 times as tall as wide (1:3). This one is W×H px.").
- **New uploads only.** The probe checks it at `POST /v1/emotes`; the worker makes files again from any stored original with `Limits::emote_original()`, which drops both shape rules, so emotes already taller than 1:3 stay valid and are made again as they are (fitted by height, as before).
- **Reported**: `maxTall` 3 next to `maxAspect` in the `limits` query / `GET /v1/limits` (`emote`; null for badges and paints, which keep their own shape rules) and in `uploadQuota`. Additive fields.
- **Client**: the quick upload checks it before sending, and the editor lets a picture be stretched to any shape while editing but its rules checklist blocks Continue until the result is between 1:3 and 4:1.
