# 0036: Direct uploads of badge and paint pictures

- Status: accepted (owner approved the media infrastructure, 2026/11/26)
- Date: 2026/11/27
- Builds on: 0003 (storage), 0015 (cosmetics), 0031 (role media limits), 0032 (one upload answer)

## Context
Badge and paint originals can be up to 16 MB under a role's limits (ADR 0031), and every byte passes through the API process on `POST /v1/cosmetics/images`, which holds the whole body in memory (the upload body limit is the largest ceiling plus 64 KB). The API review (future item 12) suggested signed uploads straight to storage, then a finalize call. Production storage is `tv-s3` (versitygw) on the host, reached by the API and worker on the private network; browsers can't reach it today.

## Decision
- **Two REST calls, additive.** `POST /v1/cosmetics/images/uploads` (`{ kind, size, content_type }`) answers `201` with `{ id, kind, method: "PUT", url, headers, expires_at }`. The client `PUT`s the file to `url` with `headers`, then calls `POST /v1/cosmetics/images/uploads/{id}/finalize`, which answers exactly as `POST /v1/cosmetics/images` does (`202`, `UploadResult` fields). The multipart route is unchanged.
- **The URL only after the checks the API makes before it takes bytes**: signed in, no app token, `cosmetics.manage`, the declared `size` within the uploader's byte cap for the kind (their roles' limits, `413 too_large` with `details`), the type one of PNG, GIF, WebP or JPEG (`415`), at most 10 uploads open per account (`409 too_many_uploads`, a limit error with `details`), and the cosmetics rate limit, counted once here (finalize isn't counted again). The per-IP upload limit covers both calls.
- **What is signed**: SigV4 query authentication for `PUT` with `content-length`, `content-type` and `host` as signed headers and an unsigned payload, valid 15 minutes. Storage refuses another length or type (checked against versitygw in CI). The key is `uploads/{id}`: outside `emotes/` and `cosmetics/`, so the CDN never serves it.
- **Finalize checks the stored object before anything is made**: it must exist (`409 upload_missing`), its size must equal the declared one (`422 size_mismatch`), then the bytes go through the same code as a multipart upload (format by magic bytes, decoding under the limits, side, frames, shape, animated chat sizes), under the uploader's limits as they are at finalize. A refused object and its row are deleted; a storage or database failure leaves them for a retry. On success the picture is stored under `originals/cosmetics/{sha256}` as before, the job queued, the upload marked finalized (once: `409 already_finalized`) and the staging object deleted. Someone else's upload reads as `404`. Past one hour: `410 upload_expired`.
- **Expiry**: the worker's clean-up round deletes uploads never finalized once `expires_at` (one hour) has passed, object first, and forgets finalized rows after a day. A bucket lifecycle rule on `uploads/` is an optional extra.
- **Off unless configured**: the URL is made for `CYEX_TV_S3_UPLOAD_ENDPOINT`, the HTTPS address browsers send uploads to (it must be `https://`, or `http://` to this machine). Without it the presign answers `501 direct_upload_off`. The owner steps put a narrow Caddy front on `upload.cyex.tv` that passes only presigned `PUT`s under `/{bucket}/uploads/` to `tv-s3` and answers CORS for the site.
- **Website**: badge and paint pictures (`uploadCosmeticImage`) try the direct path first and fall back to the form on `404`, `405` or `501`, a type it doesn't take, or a `PUT` that can't reach storage (no CORS, blocked). Refusals the form would get as well are shown as they are.
- **Emotes are not included**: they are capped at 8 MB for everyone (ADR 0031) and their upload carries a name, tags and flags; the multipart route stays theirs.
- New stable error codes: `direct_upload_off`, `too_many_uploads`, `upload_missing`, `upload_expired`, `size_mismatch`, `already_finalized`.

## Alternatives
- *A presigned `POST` policy* (with `content-length-range`): R2 doesn't support POST policies, and versitygw's support is partial; a `PUT` with a signed length works on both.
- *Exposing `tv-s3` itself*: the whole S3 API on the internet. The Caddy front passes only presigned `PUT`s to one prefix.
- *Finalizing through GraphQL*: uploads are REST (multipart lives there); the same answer shape keeps one client path.

## Consequences
- The API no longer holds large originals in memory for direct uploads; finalize reads the object back from storage (the private network) once.
- One table (`direct_uploads`), one key prefix, one worker clean-up step, one optional env value. Nothing changes for clients that keep using the form.
