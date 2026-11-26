# 0012: Image encoding for emote files

- Status: accepted (pending the owner's review); amended 2026/11/26 (animated AVIF)
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
- **No animated AVIF in v1** (replaced 2026/11/26, see the amendment below). `ravif` writes still images only. Animated emotes get animated WebP plus static WebP and AVIF first frames. Revisit with libavif when the worker image (Phase 9) can build it.
- Sizes: 1x is 32 px tall, then 64, 96 and 128 px; width keeps the aspect ratio, capped at four times the height (wider emotes are scaled to fit). Nothing is scaled up: a small original gives its own size at the larger scales. Resizing is Lanczos3 from `image`.
- Files per emote (ADR 0003's names): `1x.webp` … `4x.webp`, `1x.avif` … `4x.avif` for still emotes; for animated ones `1x.webp` … `4x.webp` (animated), `1x_static.webp` … `4x_static.webp` and `1x_static.avif` … `4x_static.avif`.
- The worker writes the WebP files first and records them, so the emote is usable, then the AVIF files.

## Consequences
- The worker needs a C compiler at build time (for libwebp), which the Rust images already have.
- Clients that want animated AVIF don't get it yet; every client that shows WebP is covered.
- The pipeline lives in `crates/media`, with nothing emote-specific in it, so cosmetic badge art can use it later.

## Amendment (owner, 2026/11/22): at most 128 px tall
Owner: "emotes should be max 128px tall". They already were: 4x (128 px) is the largest emote size, and no emote size goes past it (`Profile::EMOTE` has no sizes past 4x). It is now pinned: `cyex_tv_media::encode::EMOTE_MAX_HEIGHT` (128) with a compile-time check against `BASE_HEIGHT × 4`, and tests that every size of any original (up to 1000 × 1000, tall, wide, odd sizes) is at most 128 px tall and at most 512 px wide. The emote page shows all four sizes at once, the 4x one in focus.

## Amendment (owner approved, 2026/11/26): animated AVIF
The worker now also makes animated AVIF for animated emotes, badges and paints, next to the animated WebP, and lists one only where it is no larger than that WebP **and** at least as close to the source.

**Encoder.** `rav1e` 0.8 (BSD-2-Clause, pure Rust), the AV1 encoder `ravif` already brought in, driven directly as a video: no new crate, no C library, nothing new for `cargo deny`. libavif and SVT-AV1 were left out: their bindings build C code with CMake, which the build image and CI don't have; the worker image has no ffmpeg (checked: `Dockerfile` installs only `libwebp*`, `curl` and CA certificates). The container is written in `crates/media/src/avis.rs`: an AV1 image sequence (`ftyp` `avis`), a colour track (4:2:0, BT.601, full range) and, when any pixel isn't opaque, an alpha track (monochrome, `auxl`/`auxv`, the alpha URN), an edit list that repeats with an indefinite duration (readers loop it forever), and a primary item that is the first frame (pointing at the first sample's bytes) for readers that show only still AVIF. Checked in Chromium: every test file decodes with all its frames, loops forever (`repetitionCount` Infinity) and keeps its transparency.

**What was measured** (release build, one thread, the test pictures of `avis::tests::measure_animated_avif`: a flat cartoon disc with an outline over transparency, a smooth gradient, and per-pixel noise as the worst case; luma PSNR against the source over the pixels that aren't fully transparent; WebP is the pipeline's lossy quality 90, method 4):

| Picture | Size × frames | WebP | AVIF q96, speed 9 | AVIF ÷ WebP |
|---|---|---|---|---|
| cartoon | 32² × 30 | 14.8 KB, 13 ms, 41.4 dB | 13.7 KB, 118 ms, 41.9 dB | 93 % |
| cartoon | 128² × 30 | 51.3 KB, 26 ms, 44.1 dB | 40.2 KB, 0.9 s, 47.2 dB | 78 % |
| cartoon | 128² × 120 | 221 KB, 104 ms, 44.1 dB | 150 KB, 3.6 s, 46.7 dB | 68 % |
| cartoon | 512×128 × 60 | 112 KB, 71 ms, 44.0 dB | 79 KB, 4.7 s, 47.2 dB | 71 % |
| cartoon | 512² × 60 | 401 KB, 0.4 s, 46.7 dB | 222 KB, 18.6 s, 53.9 dB | 55 % |
| gradient | 128² × 120 | 72 KB, 0.1 s, 47.4 dB | 21 KB, 3.7 s, 57.3 dB | 28 % |
| gradient | 512² × 60 | 200 KB, 0.7 s, 51.2 dB | 60 KB, 28.9 s, 56.9 dB | 30 % |
| noise | 128² × 120 | 838 KB, 0.2 s, 41.3 dB | 827 KB, 7.1 s, 43.2 dB | 99 % |
| noise | 512×128 × 60 | 1.62 MB, 0.4 s, 41.4 dB | 1.65 MB, 14.7 s, 43.3 dB | 102 % |
| noise | 512² × 60 | 6.43 MB, 1.7 s, 41.4 dB | 6.58 MB, 58.5 s, 43.2 dB | 102 % |

Settings tried and dropped: quality 80 (the still files' setting) is much smaller but far worse (cartoon 29 dB against WebP's 44); quality 90 to 98 at speed 10 needs about 1.5 to 2 times WebP's bytes to match its quality on flat art; speed 8 is 20 to 30 % smaller than 9 but 1.3 to 1.8 times slower. Quality 96 at speed 9 was at least as close to the source as WebP on every picture, smaller on flat art and smooth pictures, about even on noise. AVIF costs 10 to 40 times WebP's encode time, which is why the work is capped.

**Decision.**
- Every animated size except `original` (kept lossless WebP, its pixels are the upload's) gets an animated AVIF, named like the WebP: `1x.avif` … `4x.avif` for emotes, `1x.avif`, `128.avif` … for badges, `is_static` false, `mime` `image/avif`. The still `_static.avif` files stay as they were.
- **Listed only when it beats the WebP**: no more bytes than the animated WebP at that size, and a luma PSNR at least that WebP's (the AVIF's from rav1e's own reconstruction, the WebP's by decoding it; both against the same source frames). Otherwise it isn't stored or listed and clients use the WebP. So a size can have WebP only; clients read the file list.
- **Work and time caps**, so a job stays inside its time limit (120 s, #175): at most 8 M pixel-frames of animated AVIF per picture (width × height × frames summed over the sizes; past it the largest size stops first, so the chat sizes keep theirs: a 4:1 emote keeps every size up to 65 frames, a square one up to 260), and at most 30 s of AVIF work in all, after which none is kept. On the measuring machine that is about 10 s for flat art and 30 s for noise at the cap.
- One rav1e thread per stream (the worker runs jobs side by side). The tests build `rav1e` optimised (`[profile.dev.package.rav1e]`), as unoptimised it is too slow for them.

**Consequences.** Animated pictures take longer to make (seconds, bounded as above). Storage grows by at most one file per animated size, only where it saves bytes. Clients that prefer AVIF (`mime`) pick the animated one where it is listed; the website and the extension keep using WebP for now.
