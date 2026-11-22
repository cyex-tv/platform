# 0012: Image encoding for emote files

- Status: accepted (pending the owner's review)
- Date: 2026/11/16

## Context
PLAN.md Phase 3 asks for WebP and AVIF at 1x to 4x, a static first frame for animated emotes, and a benchmark of `image` + `ravif` against libavif and libwebp bindings before choosing. The worker must stay small, build on Linux and Windows, and pass `cargo deny` (ADR 0007). The workspace forbids `unsafe` in our own code, so encoders are used through safe wrappers.

## What was measured
A release build on the development machine (Windows, one thread per encode), encoding a deliberately noisy RGBA test image (worst case; real emotes compress far better):

| Encoder | Setting | 32 px | 128 px | 30 frames, 128 px |
|---|---|---|---|---|
| libwebp via `webp-animation` | lossy, quality 90, method 4 | 0.6 ms, 656 B | 2.1 ms, 6.5 KB | 59 ms, 194 KB |
| `ravif` (rav1e, pure Rust) | quality 80, speed 6 | 4.6 ms, 888 B | 36.7 ms, 5.8 KB | — |
| `ravif` | quality 80, speed 8 | 2.9 ms, 891 B | 34.0 ms, 5.8 KB | — |
| `ravif` | quality 80, speed 10 | 1.1 ms, 1.1 KB | 8.8 ms, 8.2 KB | — |

libavif was not measured: its bindings build libavif and an AV1 encoder (aom or rav1e) with CMake, which neither the development machine nor CI has set up. Its only gain here would be animated AVIF.

## Decision
- **WebP: libwebp**, through `webp-animation` (MIT/Apache-2.0; libwebp itself is BSD-3-Clause, compiled from source by `libwebp-sys2` with `cc`). Lossy, quality 90, method 4. The same encoder writes still images: a one-frame "animation" comes out as a plain still WebP (no `ANIM` chunk; checked). `image`'s own WebP encoder only writes lossless stills, so it can't do animated emotes.
- **AVIF: `ravif`** (BSD-3-Clause, pure Rust), quality 80, speed 8: nearly speed 6's size at a fraction of the time. Used for still emotes and for the static first frame of animated ones.
- **No animated AVIF in v1.** `ravif` writes still images only. Animated emotes get animated WebP plus static WebP and AVIF first frames. Revisit with libavif when the worker image (Phase 9) can build it.
- Sizes: 1x is 32 px tall, then 64, 96 and 128 px; width keeps the aspect ratio, capped at four times the height (wider emotes are scaled to fit). Nothing is scaled up: a small original gives its own size at the larger scales. Resizing is Lanczos3 from `image`.
- Files per emote (ADR 0003's names): `1x.webp` … `4x.webp`, `1x.avif` … `4x.avif` for still emotes; for animated ones `1x.webp` … `4x.webp` (animated), `1x_static.webp` … `4x_static.webp` and `1x_static.avif` … `4x_static.avif`.
- The worker writes the WebP files first and records them, so the emote is usable, then the AVIF files.

## Consequences
- The worker needs a C compiler at build time (for libwebp), which the Rust images already have.
- Clients that want animated AVIF don't get it yet; every client that shows WebP is covered.
- The pipeline lives in `crates/media`, with nothing emote-specific in it, so cosmetic badge art can use it later.

## Amendment (owner, 2026/11/22): at most 128 px tall
Owner: "emotes should be max 128px tall". They already were: 4x (128 px) is the largest emote size, and no emote size goes past it (`Profile::EMOTE` has no sizes past 4x). It is now pinned: `cyex_tv_media::encode::EMOTE_MAX_HEIGHT` (128) with a compile-time check against `BASE_HEIGHT × 4`, and tests that every size of any original (up to 1000 × 1000, tall, wide, odd sizes) is at most 128 px tall and at most 512 px wide. The emote page shows all four sizes at once, the 4x one in focus.
