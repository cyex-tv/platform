//! The `cosmetic.transcode` job (ADR 0015): a staff-uploaded badge or paint picture becomes its
//! files, the emote pipeline at the cosmetic's sizes (`Profile::BADGE`, `Profile::PAINT`), stored
//! under `cosmetics/{badges|paints}/{sha256}/{name}` with an immutable cache header.
//!
//! Badges follow the cyex badge size ladder: `1x`, `2x` and `4x` (18, 36 and 72 px, the names
//! clients already read), then `128`, `256` and `512`, all from one 512 px master.
//!
//! WebP first, recorded so the image can be used, then AVIF. Running the job again is safe: the
//! keys are content-addressed, and the files recorded are replaced by the new set, so a picture
//! made before a size existed gets it by running the job again (`remakeCosmeticImage`).

use cyex_tv_db::cosmetics;
use cyex_tv_db::jobs::Job;
use cyex_tv_media::encode::{self, Encoded, Kind, Profile, Variant};
use cyex_tv_media::{Limits, MediaError};
use cyex_tv_shared::model::{CosmeticKind, EmoteFile, ImageFormat};
use cyex_tv_storage::{IMMUTABLE, Put, Store, cosmetic_key, sigv4};
use serde_json::Value;

use crate::{Ctx, JobError};

/// The job kind the image upload queues.
pub const KIND: &str = "cosmetic.transcode";

fn retry(err: impl std::fmt::Display) -> JobError {
    JobError::Retry(err.to_string())
}

/// The sizes a kind of cosmetic picture is made at.
pub fn profile(kind: CosmeticKind) -> &'static Profile {
    match kind {
        CosmeticKind::Badge => &Profile::BADGE,
        CosmeticKind::Paint => &Profile::PAINT,
    }
}

/// The limits a kind of cosmetic picture is decoded under (the API checks uploads with the same).
pub fn limits(kind: CosmeticKind) -> Limits {
    match kind {
        CosmeticKind::Badge => Limits::badge(),
        CosmeticKind::Paint => Limits::default(),
    }
}

/// Decodes and encodes a picture at its kind's sizes. CPU work: run it off the async threads.
pub fn make(kind: CosmeticKind, bytes: &[u8]) -> Result<Encoded, MediaError> {
    encode::transcode_with(bytes, &limits(kind), profile(kind))
}

/// The file entries, in the shape emote files are recorded in.
pub fn entries(variants: &[&Variant]) -> Value {
    let files: Vec<EmoteFile> = variants
        .iter()
        .map(|v| EmoteFile {
            name: v.name.clone(),
            format: match v.kind {
                Kind::Webp => ImageFormat::Webp,
                Kind::Avif => ImageFormat::Avif,
            },
            scale: u8::try_from(v.scale).unwrap_or(u8::MAX),
            width: u16::try_from(v.width).unwrap_or(u16::MAX),
            height: u16::try_from(v.height).unwrap_or(u16::MAX),
            size: u32::try_from(v.bytes.len()).unwrap_or(u32::MAX),
            is_static: v.is_static,
        })
        .collect();
    serde_json::to_value(files).unwrap_or_else(|_| Value::Array(Vec::new()))
}

/// Writes `variants` under the picture's folder.
pub async fn store_all(
    store: &Store,
    kind: CosmeticKind,
    sha_hex: &str,
    variants: &[Variant],
) -> Result<(), JobError> {
    for v in variants {
        store
            .put(
                &cosmetic_key(kind.as_str(), sha_hex, &v.name),
                v.bytes.clone().into(),
                Put {
                    content_type: v.kind.mime(),
                    cache_control: Some(IMMUTABLE),
                },
            )
            .await
            .map_err(retry)?;
    }
    Ok(())
}

pub async fn run(ctx: Ctx, job: Job) -> Result<(), JobError> {
    let image_id = job
        .payload
        .get("image_id")
        .and_then(Value::as_str)
        .and_then(cyex_tv_db::ids::from_text)
        .ok_or_else(|| JobError::Permanent("the job has no valid image_id".into()))?;
    let store = ctx
        .store
        .clone()
        .ok_or_else(|| JobError::Retry("storage isn't configured".into()))?;

    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    let image = cosmetics::get_image(&mut conn, image_id)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent("the image is gone".into()))?;
    drop(conn);
    let kind = CosmeticKind::parse(&image.kind)
        .ok_or_else(|| JobError::Permanent(format!("unknown kind {}", image.kind)))?;
    let original = store
        .get(&image.storage_key)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent(format!("{} is missing", image.storage_key)))?;

    let bytes = original.bytes.clone();
    let encoded = tokio::task::spawn_blocking(move || make(kind, &bytes))
        .await
        .map_err(retry)?
        .map_err(|err| match err {
            MediaError::Encode(message) => JobError::Retry(message),
            other => JobError::Permanent(other.to_string()),
        })?;

    let sha_hex = sigv4::hex(&image.sha256);
    store_all(&store, kind, &sha_hex, &encoded.webp).await?;
    let webp: Vec<&Variant> = encoded.webp.iter().collect();
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    cosmetics::set_image_variants(&mut conn, image.id, &entries(&webp))
        .await
        .map_err(retry)?;
    drop(conn);

    store_all(&store, kind, &sha_hex, &encoded.avif).await?;
    let all: Vec<&Variant> = encoded.webp.iter().chain(&encoded.avif).collect();
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    cosmetics::set_image_variants(&mut conn, image.id, &entries(&all))
        .await
        .map_err(retry)?;
    tracing::info!(
        image = %cyex_tv_db::ids::to_text(image.id),
        kind = kind.as_str(),
        files = all.len(),
        "cosmetic image transcoded"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The job's image path without a database: fixtures made in code (a still PNG, an animated
    //! GIF), encoded at the cosmetic sizes and written to the in-memory store.

    use std::io::Cursor;

    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame, ImageFormat as Fmt, Rgba, RgbaImage};

    use super::*;

    fn picture(w: u32, h: u32, seed: u8) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([(x * 255 / w) as u8, (y * 255 / h) as u8, seed, 255])
        })
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        picture(w, h, 3).write_to(&mut out, Fmt::Png).unwrap();
        out.into_inner()
    }

    fn gif(w: u32, h: u32, frames: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut out);
            enc.set_repeat(Repeat::Infinite).unwrap();
            for i in 0..frames {
                enc.encode_frame(Frame::from_parts(
                    picture(w, h, (i * 60) as u8),
                    0,
                    0,
                    Delay::from_numer_denom_ms(80, 1),
                ))
                .unwrap();
            }
        }
        out
    }

    #[tokio::test]
    async fn a_badge_becomes_the_size_ladder_under_cosmetics() {
        let encoded = make(CosmeticKind::Badge, &png(128, 128)).unwrap();
        let store = Store::memory();
        store_all(&store, CosmeticKind::Badge, "abcd", &encoded.webp)
            .await
            .unwrap();
        store_all(&store, CosmeticKind::Badge, "abcd", &encoded.avif)
            .await
            .unwrap();
        let mut keys = store.keys();
        keys.sort();
        let mut want: Vec<String> = ["1x", "2x", "4x", "128", "256", "512"]
            .iter()
            .flat_map(|stem| {
                ["avif", "webp"].map(|ext| format!("cosmetics/badges/abcd/{stem}.{ext}"))
            })
            .collect();
        want.sort();
        assert_eq!(keys, want);
        for name in ["4x.webp", "512.webp"] {
            let object = store
                .get(&format!("cosmetics/badges/abcd/{name}"))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(object.content_type.as_deref(), Some("image/webp"));
            assert_eq!(object.cache_control.as_deref(), Some(IMMUTABLE));
        }

        let all: Vec<&Variant> = encoded.webp.iter().chain(&encoded.avif).collect();
        let files: Vec<EmoteFile> = serde_json::from_value(entries(&all)).unwrap();
        let sizes: Vec<_> = files
            .iter()
            .filter(|f| f.format == ImageFormat::Webp)
            .map(|f| (f.name.as_str(), f.scale, f.width, f.height))
            .collect();
        assert_eq!(
            sizes,
            [
                ("1x.webp", 1, 18, 18),
                ("2x.webp", 2, 36, 36),
                ("4x.webp", 4, 72, 72),
                ("128.webp", 7, 128, 128),
                ("256.webp", 14, 256, 256),
                ("512.webp", 28, 512, 512)
            ]
        );
        assert!(Profile::BADGE.is_complete(files.iter().map(|f| f.name.as_str())));
    }

    #[test]
    fn badges_take_the_badge_limits_and_paints_keep_theirs() {
        assert_eq!(limits(CosmeticKind::Badge), Limits::badge());
        assert_eq!(limits(CosmeticKind::Paint), Limits::default());
        // A 1024 px badge is scaled down to the 512 px master; a paint that wide is over its limit.
        assert!(make(CosmeticKind::Badge, &png(1024, 1024)).is_ok());
        assert!(make(CosmeticKind::Paint, &png(1024, 128)).is_err());
        // 121 frames is over a badge's 120, not a paint's.
        let long = gif(8, 8, 121);
        assert!(matches!(
            cyex_tv_media::probe(&long, &limits(CosmeticKind::Badge)),
            Err(MediaError::TooLarge(_))
        ));
        assert!(cyex_tv_media::probe(&long, &limits(CosmeticKind::Paint)).is_ok());
    }

    #[tokio::test]
    async fn an_animated_badge_keeps_its_frames_and_gets_stills() {
        let encoded = make(CosmeticKind::Badge, &gif(90, 90, 4)).unwrap();
        assert!(encoded.animated);
        assert_eq!(encoded.frames, 4);
        let names: Vec<&str> = encoded.webp.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"1x.webp") && names.contains(&"4x_static.webp"));
        // Small enough to animate at every size.
        assert!(names.contains(&"512.webp") && names.contains(&"512_static.webp"));
        assert!(
            encoded
                .avif
                .iter()
                .all(|v| v.name.ends_with("_static.avif"))
        );
    }

    #[test]
    fn paint_pictures_use_the_paint_sizes() {
        let encoded = make(CosmeticKind::Paint, &png(400, 100)).unwrap();
        let sizes: Vec<_> = encoded.webp.iter().map(|v| (v.width, v.height)).collect();
        assert_eq!(sizes, [(256, 64), (400, 100), (400, 100)]);
        assert!(make(CosmeticKind::Badge, b"not an image").is_err());
    }
}
