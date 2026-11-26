# 0025: The badge size ladder

- Status: accepted
- Date: 2026/11/20
- Amends: 0015 (badge picture sizes, upload limits)

## Context
The owner wants cyex.tv badges to behave like cyex deck badges: the same size ladder, the same rule for picking a size, the same upload limits and the same preview sizes. Until now a badge was made at 18, 36 and 72 px only (`1x`, `2x`, `4x`), uploads shared the paint rules (PNG, APNG, GIF or WebP up to 5 MB at the time), and the site picked a file by the pixel ratio alone, whatever size it drew the badge at, so a 28 px or 72 px badge was soft on dense screens.

The deck's open badge spec was read for its rules only; nothing was copied.

## Decision
**Sizes made** (`crates/media` `Profile::BADGE`): square 18, 36, 72, 128, 256 and 512 px, every size scaled from one master of 512 px (a bigger upload is scaled down to it, a smaller one up). The chat sizes keep their names, so clients and the extension keep working: `1x.webp`, `2x.webp`, `4x.webp` (plus `_static` and `.avif` as before). The larger sizes are `128.webp`, `256.webp` and `512.webp`, with `128_static.webp`, `128.avif`, `128_static.avif` and so on, the same pattern. A file entry's `scale` stays 1, 2 or 4 for the chat sizes; the larger ones carry their px ÷ 18 rounded (7, 14, 28), and `width` gives the size. The extension reads only scales 1 to 4, so it ignores them.

**Original (added later):** each badge also has `original.webp` (with `original_static.webp` and `original.avif` forms): the picture at its real size, kept apart from the master. An upload up to 512 px on its longest side is stored untouched in size (no resampling, never scaled up to the 512 master; the WebP is lossless, so its pixels are exactly the upload's, every frame kept); one over 512 px (up to 2048, as before) is scaled down to 512, the only downscale, since 512 is the largest preview. `width` and `height` of the file entry are the real size (its `scale` is the longest side ÷ 18 rounded). An animated original is kept up to 8 MB (the upload cap); over it only its still frame is. The ladder sizes are unchanged, made from the 512 master as above, and a badge made before the original gets it by "Make larger sizes". The admin and the shop previews show the original at its real pixel size, up to 512 px, scaled with `image-rendering: pixelated` when enlarged. Additive: clients that read only the ladder ignore `original`.

**Animated:** 18, 36 and 72 px always animate, each at most 512 KB once converted (over it, the upload is refused: the API encodes the 72 px file to check before it accepts, and the worker keeps the same rule). 128, 256 and 512 px animate only within 1, 2 and 4 MB; over it, only their still frame (`_static`) is kept.

**Uploads** (`Limits::badge`, checked by the API and used by the worker; the page checks the same first): PNG, APNG, JPEG, WebP or GIF, square, at least 72×72 px and at most 2048×2048 px, up to 8 MB. An animation: at most 120 frames, 10 seconds (frames under 20 ms count as 100 ms, as browsers show them) and 64 MP (width × height × frames). Errors keep their existing codes (`too_large`, `bad_dimensions`, `unsupported_format`, `invalid_image`) with readable messages. Paints later took the same file rules (8 MB, ADR 0026).

**Picking** (website, `lib/cosmetics/badgeFile.ts`): the smallest size at or above the drawn size × the device pixel ratio; if none is big enough, the largest. Animated badges pick among their animated files (still frames under reduced motion). Before mount the `<img>` offers a width-descriptor `srcset` with `sizes`; after mount the file is picked in script with no `srcset` (the Chromium leak noted at `pictureFor`).

**Previews:** the account Cosmetics picker shows a badge at 28 px, the upload field at 72 px, the admin badge list at 36 px.

**Pictures made before the ladder** have only 18, 36 and 72 px; everything falls back to the largest file there is. They are made again from their stored original by `remakeCosmeticImage(id)` (GraphQL, `cosmetics.manage`, audited as `cosmetic_image.remade`, `original_missing` when the original is gone), offered on the admin badge page as "Make larger sizes" for badges that lack a size. Uploading the same bytes again also queues it. The job is safe to repeat: keys are content-addressed and the recorded files are replaced by the new set, the old ones serving until then.

## Consequences
- Storage per badge grows from 6 files to 12 (24 for an animated one); the 512 px files are the bulk.
- The API decodes an animated badge twice on upload (the probe, then the 72 px check). Both are capped by the limits above.
- Additive API change only: a new mutation, a new audit action and new file names; existing fields keep their meaning.
