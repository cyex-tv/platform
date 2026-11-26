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

use std::sync::atomic::AtomicBool;

use cyex_tv_db::cosmetics;
use cyex_tv_db::jobs::Job;
use cyex_tv_media::encode::{self, Encoded, Kind, Profile, Variant};
use cyex_tv_media::{Limits, MediaError};
use cyex_tv_shared::model::{CosmeticKind, EmoteFile, ImageFormat};
use cyex_tv_storage::{IMMUTABLE, Put, Store, cosmetic_key, sigv4};
use serde_json::Value;

use crate::{CancelOnDrop, Ctx, JobError, media_failure};

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
        CosmeticKind::Paint => Limits::paint(),
    }
}

/// Decodes and encodes a picture at its kind's sizes. CPU work: run it off the async threads.
pub fn make(kind: CosmeticKind, bytes: &[u8]) -> Result<Encoded, MediaError> {
    make_cancellable(kind, bytes, &AtomicBool::new(false))
}

/// [`make`] that stops with [`MediaError::Cancelled`] once `cancel` is set.
pub fn make_cancellable(
    kind: CosmeticKind,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> Result<Encoded, MediaError> {
    encode::transcode_cancellable(bytes, &limits(kind), profile(kind), cancel)
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
    // The guard sets the flag if the time limit drops this future, so the thread stops too.
    let cancel = CancelOnDrop::default();
    let flag = cancel.flag();
    let encoded = tokio::task::spawn_blocking(move || make_cancellable(kind, &bytes, &flag))
        .await
        .map_err(retry)?
        .map_err(media_failure)?;
    drop(cancel);

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
        let mut want: Vec<String> = ["1x", "2x", "4x", "128", "256", "512", "original"]
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
                ("512.webp", 28, 512, 512),
                // 128 px is the picture's own size, not scaled up to the master.
                ("original.webp", 7, 128, 128)
            ]
        );
        assert!(Profile::BADGE.is_complete(files.iter().map(|f| f.name.as_str())));
    }

    #[test]
    fn a_512_badge_is_kept_at_512_and_a_big_one_is_fitted_to_it() {
        let size_of = |bytes: &[u8]| {
            let encoded = make(CosmeticKind::Badge, bytes).unwrap();
            let all: Vec<&Variant> = encoded.webp.iter().collect();
            let files: Vec<EmoteFile> = serde_json::from_value(entries(&all)).unwrap();
            let sizes: Vec<_> = files.iter().map(|f| (f.width, f.name.as_str())).collect();
            assert!(sizes.contains(&(512, "512.webp")), "the ladder stays");
            files
                .iter()
                .find(|f| f.name == "original.webp")
                .map(|f| (f.width, f.height))
        };
        assert_eq!(size_of(&png(512, 512)), Some((512, 512)));
        assert_eq!(size_of(&png(300, 300)), Some((300, 300)));
        assert_eq!(size_of(&png(1024, 1024)), Some((512, 512)));
    }

    #[test]
    fn paints_take_the_badge_animation_limits_and_their_own_side() {
        assert_eq!(limits(CosmeticKind::Badge), Limits::badge());
        assert_eq!(limits(CosmeticKind::Paint), Limits::paint());
        // A 1024 px badge is scaled down to the 512 px master; a paint 1024 px wide is fine now.
        assert!(make(CosmeticKind::Badge, &png(1024, 1024)).is_ok());
        assert!(make(CosmeticKind::Paint, &png(1024, 128)).is_ok());
        // 121 frames is over the 120 of badges and paints alike.
        let long = gif(8, 8, 121);
        for kind in [CosmeticKind::Badge, CosmeticKind::Paint] {
            assert!(matches!(
                cyex_tv_media::probe(&long, &limits(kind)),
                Err(MediaError::TooLarge(_))
            ));
        }
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
        // 1x, 2x and 4x, then 512: none above the original (400×100).
        assert_eq!(sizes, [(256, 64), (400, 100), (400, 100), (400, 100)]);
        let names: Vec<_> = encoded.webp.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["1x.webp", "2x.webp", "4x.webp", "512.webp"]);
        // A tall original gets the 512 px tall file, scaled from the original.
        let tall = make(CosmeticKind::Paint, &png(1024, 640)).unwrap();
        let sizes: Vec<_> = tall.webp.iter().map(|v| (v.width, v.height)).collect();
        assert_eq!(sizes, [(102, 64), (205, 128), (410, 256), (819, 512)]);
        assert!(make(CosmeticKind::Badge, b"not an image").is_err());
    }
}
