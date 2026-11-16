# 0003: File storage and CDN

- Status: accepted
- Date: 2026/11/16

## Context
Emote files are small, immutable once processed, and read far more often than written. Egress cost is the main cost.

## Decision
- **Cloudflare R2**, served through `cdn.cyex.tv` (custom domain on the bucket).
- Key layout:
  - `emotes/{sha256}/{variant}` with variants `1x.webp … 4x.webp`, `1x.avif … 4x.avif`, `1x_static.webp …` for animated emotes;
  - `originals/{sha256}`, never public;
  - `cosmetics/` reserved for the later cosmetics phase.
- Content-addressed by the hash of the original, so a duplicate upload reuses the files and URLs never change.
- `Cache-Control: public, max-age=31536000, immutable` on derivatives. A takedown deletes the objects and purges the URLs.
- Local dev uses MinIO with the same S3 API.
- The worker holds the only write credential. The API gets pre-signed reads only where needed (originals for moderators).

## Consequences
- No egress fees, and the same Cloudflare account as DNS.
- Takedowns need a purge call as well as a delete. That is part of the takedown flow (Phase 7).
