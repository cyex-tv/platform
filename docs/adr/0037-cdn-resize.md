# 0037: Pictures at any listed width from the CDN

- Status: accepted (owner approved the media infrastructure, 2026/11/26); not live until the owner enables it
- Date: 2026/11/27
- Builds on: 0003 (storage and CDN), 0012 (encoding), 0025 (badge ladder)

## Context
Every picture is made at fixed sizes: emotes 1x to 4x, badges 18 to 512 px plus `original`, paints 64 to 512 px tall. A client that draws a badge at 28 px or an emote at 1.5x picks the next size up and lets the browser scale. The API review (future item 1) suggested resizing at the CDN on request, with a cache, keeping the fixed names. `cdn.cyex.tv` is `tv-cdn` (Caddy) behind the Cloudflare tunnel, with Cloudflare caching in front (infra `docs/self-hosted-cdn.md`).

## Decision
- **A Cloudflare Worker on `cdn.cyex.tv/*`** (infra repo, `cloudflare/cdn-resize/`) using Cloudflare Images transformations (`fetch` with `cf.image`). It changes nothing for a request without `w` or `format`: those pass through to the origin exactly as before, so the fixed files, the cache rules and the takedown purge keep working.
- **What can be asked**: `?w=` one of 16, 18, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 96, 112, 128, 144, 160, 192, 224, 256, 320, 384, 448, 512, 640, 768, 1024 (a list, not a range, so the number of variants per file is bounded: 28 widths × 2 formats), and `?format=` `webp`, `avif` or `auto` (AVIF when the `Accept` header has `image/avif`, else WebP; the answer then carries `Vary: Accept`). Never wider than the file (`fit=scale-down`), animation kept (`anim`), metadata dropped. Any other parameter, a repeated one, another method than `GET`/`HEAD`, or a path that isn't a picture file is `400`.
- **Only our files, no SSRF**: the path must be `/emotes/<64 hex>/<name>.(webp|avif)` or `/cosmetics/(badges|paints)/<64 hex>/<name>.(webp|avif)`, the name letters, digits and `_` only. The original is fetched from `https://cdn.cyex.tv` plus that path, never from anything the client names; `/originals/` and `/uploads/` can't be reached. The resizer's own fetch of the original (`Via: image-resizing`) passes straight to the origin.
- **Caching**: one edge cache entry per file, width and resolved format (`?w=<w>&format=<fmt>` on the file's URL), `Cache-Control: public, max-age=31536000, immutable`, like the files themselves: the sha in the path means the pixels behind a URL never change. A missing file is cached 60 s, a failed resize not at all.
- **Takedown**: the worker's purge removes a file's own URL from Cloudflare's cache, not the resized variants (their keys differ). So the Worker serves a cached variant only after a `HEAD` of its original, which the edge answers from its own cache for every live file; once the original is gone (deleted and purged by the takedown) the variant is dropped and answers `404`. A variant not yet cached can't be made either. No change to the purge job.
- **Clients don't change.** The ladder names stay, and the developer docs say `?w=` is available once enabled. No API field changes.

## Alternatives
- *A width range instead of a list*: every integer width is a separate variant to make and pay for; a list of the sizes clients draw at covers them.
- *Resizing in our own service* (an image proxy next to `tv-cdn`): more code to run and secure, and the host would do the work Cloudflare's edge does.
- *URL-format transformations* (`/cdn-cgi/image/width=48/...`): no place to validate parameters; any width and any source path on the zone could be asked for.

## Consequences
- Cloudflare Images transformations are billed per unique transformation each month; the list keeps that bounded (at most 56 per file, only those asked for).
- Animated AVIF output depends on Cloudflare; a client that needs an animation should ask `format=webp` (or rely on `auto` only for still files).
- Nothing is live until the owner enables transformations for the zone and deploys the Worker (`docs/setup/owner-steps.md`).
