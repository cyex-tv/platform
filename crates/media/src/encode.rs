//! Emote files from an original (ADR 0012): WebP at 1x to 4x (animated when the original is),
//! AVIF at 1x to 4x for still images, and static first frames (WebP and AVIF) for animated ones.
//!
//! The original is decoded frame by frame; each frame is resized to every scale at once and fed
//! to that scale's WebP encoder, so the full animation is never held in memory.

use image::RgbaImage;
use image::imageops::FilterType;
use webp_animation::{Encoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig};

use crate::{Canvas, Limits, MediaError, decode_frames};

/// 1x is this many pixels tall.
pub const BASE_HEIGHT: u32 = 32;
pub const SCALES: [u32; 4] = [1, 2, 3, 4];
/// Wider than 4:1 is scaled to fit.
pub const MAX_ASPECT: u32 = 4;
/// Frames shorter than this show for 100 ms, as browsers do with GIFs.
pub const MIN_DELAY_MS: u32 = 20;

const WEBP_QUALITY: f32 = 90.0;
const AVIF_QUALITY: f32 = 80.0;
const AVIF_SPEED: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Webp,
    Avif,
}

impl Kind {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Webp => "webp",
            Self::Avif => "avif",
        }
    }
}

/// One emote file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// `2x.webp`, `1x_static.avif` and so on (ADR 0003).
    pub name: String,
    pub kind: Kind,
    pub scale: u32,
    pub width: u32,
    pub height: u32,
    /// The first frame of an animated emote.
    pub is_static: bool,
    pub bytes: Vec<u8>,
}

/// What `transcode` made.
#[derive(Clone, Debug)]
pub struct Encoded {
    pub canvas: Canvas,
    pub frames: u32,
    pub animated: bool,
    /// Written first: once these are stored the emote is usable.
    pub webp: Vec<Variant>,
    pub avif: Vec<Variant>,
}

/// The size at `scale`: `BASE_HEIGHT × scale` tall, at most four times as wide, aspect kept,
/// never larger than the original.
pub fn size_for(width: u32, height: u32, scale: u32) -> (u32, u32) {
    let box_h = f64::from(BASE_HEIGHT * scale);
    let box_w = box_h * f64::from(MAX_ASPECT);
    let factor = (box_h / f64::from(height))
        .min(box_w / f64::from(width))
        .min(1.0);
    let w = (f64::from(width) * factor).round().max(1.0);
    let h = (f64::from(height) * factor).round().max(1.0);
    // Both are at most the original side (≤ 1000), so they fit in u32.
    (w as u32, h as u32)
}

fn name(scale: u32, kind: Kind, is_static: bool) -> String {
    let suffix = if is_static { "_static" } else { "" };
    format!("{scale}x{suffix}.{}", kind.extension())
}

fn encode_err(err: impl std::fmt::Debug) -> MediaError {
    MediaError::Encode(format!("{err:?}"))
}

fn resized(frame: &RgbaImage, (w, h): (u32, u32)) -> RgbaImage {
    if frame.dimensions() == (w, h) {
        frame.clone()
    } else {
        image::imageops::resize(frame, w, h, FilterType::Lanczos3)
    }
}

fn webp_encoder((w, h): (u32, u32)) -> Result<Encoder, MediaError> {
    Encoder::new_with_options(
        (w, h),
        EncoderOptions {
            encoding_config: Some(EncodingConfig {
                quality: WEBP_QUALITY,
                method: 4,
                encoding_type: EncodingType::Lossy(LossyEncodingConfig::default()),
            }),
            ..EncoderOptions::default()
        },
    )
    .map_err(encode_err)
}

/// A still WebP: one frame through the same encoder comes out without an `ANIM` chunk.
fn webp_still(frame: &RgbaImage) -> Result<Vec<u8>, MediaError> {
    let mut encoder = webp_encoder(frame.dimensions())?;
    encoder.add_frame(frame.as_raw(), 0).map_err(encode_err)?;
    Ok(encoder.finalize(100).map_err(encode_err)?.to_vec())
}

fn avif_still(frame: &RgbaImage) -> Result<Vec<u8>, MediaError> {
    let (w, h) = frame.dimensions();
    let pixels: Vec<ravif::RGBA8> = frame
        .pixels()
        .map(|p| ravif::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
    let width = usize::try_from(w).map_err(encode_err)?;
    let height = usize::try_from(h).map_err(encode_err)?;
    let out = ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .encode_rgba(ravif::Img::new(&pixels[..], width, height))
        .map_err(encode_err)?;
    Ok(out.avif_file)
}

struct Scale {
    scale: u32,
    size: (u32, u32),
    encoder: Encoder,
}

/// Decodes `bytes` under `limits` and makes every emote file.
pub fn transcode(bytes: &[u8], limits: &Limits) -> Result<Encoded, MediaError> {
    let mut scales: Vec<Scale> = Vec::new();
    let mut first: Option<RgbaImage> = None;
    let mut timestamp: i32 = 0;
    let mut failure: Option<MediaError> = None;

    let (canvas, frames) = decode_frames(bytes, limits, &mut |canvas, frame, delay| {
        if scales.is_empty() {
            for scale in SCALES {
                let size = size_for(canvas.width, canvas.height, scale);
                scales.push(Scale {
                    scale,
                    size,
                    encoder: webp_encoder(size)?,
                });
            }
        }
        for s in &mut scales {
            let small = resized(&frame, s.size);
            if let Err(err) = s.encoder.add_frame(small.as_raw(), timestamp) {
                failure = Some(encode_err(err));
            }
        }
        let shown = if delay < MIN_DELAY_MS { 100 } else { delay };
        timestamp = timestamp.saturating_add(i32::try_from(shown).unwrap_or(i32::MAX));
        if first.is_none() {
            first = Some(frame);
        }
        Ok(())
    })?;
    if let Some(err) = failure {
        return Err(err);
    }
    let first = first.ok_or_else(|| MediaError::Invalid("no frames".into()))?;
    let animated = frames > 1;

    let mut webp = Vec::new();
    for s in scales {
        let bytes = s
            .encoder
            .finalize(timestamp.max(1))
            .map_err(encode_err)?
            .to_vec();
        webp.push(Variant {
            name: name(s.scale, Kind::Webp, false),
            kind: Kind::Webp,
            scale: s.scale,
            width: s.size.0,
            height: s.size.1,
            is_static: false,
            bytes,
        });
    }

    let mut avif = Vec::new();
    for scale in SCALES {
        let size = size_for(canvas.width, canvas.height, scale);
        let small = resized(&first, size);
        if animated {
            webp.push(Variant {
                name: name(scale, Kind::Webp, true),
                kind: Kind::Webp,
                scale,
                width: size.0,
                height: size.1,
                is_static: true,
                bytes: webp_still(&small)?,
            });
        }
        avif.push(Variant {
            name: name(scale, Kind::Avif, animated),
            kind: Kind::Avif,
            scale,
            width: size.0,
            height: size.1,
            is_static: animated,
            bytes: avif_still(&small)?,
        });
    }

    Ok(Encoded {
        canvas,
        frames,
        animated,
        webp,
        avif,
    })
}
