# 0026: Paint pictures: a 512 px size, picking by drawn size, badge upload limits

- Status: accepted
- Date: 2026/11/21
- Amends: 0015 (paint picture sizes, upload limits), 0025 (badge size ladder, paints were left out)

## Context
Paint pictures were made at 64, 128 and 256 px tall (`1x`, `2x`, `4x`) and the site always drew `1x`, so a name drawn large (profile header, shop cards, paint studio) or on a dense screen was blurry. Paint uploads also had their own, older limits (5 MB, 1000 px a side, no animation caps, no JPEG).

## Decision
**Sizes made** (`crates/media` `Profile::PAINT`): 64, 128, 256 and 512 px tall: `1x.webp`, `2x.webp`, `4x.webp` and the new `512.webp` (plus `_static` and `.avif`, the same pattern as badges' `512.webp`). Each is scaled from the original, never above it (a picture smaller than a step has several files of its own height), up to 8:1. The 512 px file animates only within 4 MB (else its `_static` frame stays). A file entry's `scale` is 8 for `512`; `width` and `height` are the real size. Clients that read only `1x` to `4x` ignore it.

**Picking** (website, `lib/cosmetics/paintFile.ts`): the smallest file whose height is at or above what the picture must cover × the device pixel ratio, else the largest. What it must cover is the larger of the text box's height and, for a name wider than the picture's shape under `background-size: cover`, the name's width ÷ the picture's aspect (a repeating picture is `auto 100%`: the height only). Animated paints pick among their animated files; under reduced motion the `_static` frames. Where the size isn't known: `2x` from a pixel ratio of 1.5, else `1x`. `PaintedName` reads its font size once after mount (style read, no layout) and again when the window settles, for image paints only, and estimates the width from the name's length; the paint studio picks for its biggest previews (30 px, 25 characters).

**Uploads** (`Limits::paint`, the API, the worker; the page and the studio check first, `upload.ts`): the badge file rules, PNG, APNG, JPEG, WebP or GIF up to 8 MB; an animation at most 120 frames, 10 seconds and 64 MP (width × height × frames). Paint's own shape: at least 64 px tall, at most 8:1 wide (new: refused with `bad_dimensions`, before it was squeezed to fit), up to 4096 px a side (was 1000) so a 512 px tall picture can be 8:1.

**Older pictures** have no `512` file. `remakeCosmeticImage` makes them again from the stored original; the admin paints page offers "Make larger sizes" for picture paints that lack it. Uploading the same bytes again also queues it.

## Consequences
- Storage per paint upload grows by up to four files (512 px WebP and AVIF, still forms); an original shorter than 512 px gets a copy at its own size.
- Additive API change only: a new file name, wider limits, one new refusal for pictures wider than 8:1.
