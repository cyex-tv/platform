//! Emote files from an original (ADR 0012): WebP at 1x to 4x (animated when the original is),
//! AVIF at 1x to 4x for still images, and static first frames (WebP and AVIF) for animated ones.
//!
//! The original is decoded frame by frame; each frame is resized to every scale at once and fed
//! to that scale's WebP encoder, so the full animation is never held in memory.
//!
//! Badges follow the cyex badge size ladder ([`Profile::BADGE`]): 18, 36 and 72 px (`1x`, `2x`,
//! `4x`), then `128`, `256` and `512`, each scaled from one 512 px master of the frame.

use image::RgbaImage;
use image::imageops::FilterType;
use webp_animation::{Encoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig};

use crate::{Canvas, Limits, MediaError, decode_frames};

/// 1x is this many pixels tall.
pub const BASE_HEIGHT: u32 = 32;
pub const SCALES: [u32; 4] = [1, 2, 3, 4];
/// Wider than 4:1 is scaled to fit.
pub const MAX_ASPECT: u32 = 4;

/// A size past a profile's scales: a square of `px`, named `{px}.webp` (`128.webp`,
/// `256_static.avif` and so on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Large {
    pub px: u32,
    /// An animated file at this size over this many bytes isn't kept (its still frame is).
    pub max_animated_bytes: usize,
}

/// The sizes one kind of picture is made at: emotes, chat badges or paint images (ADR 0015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// 1x is this many pixels tall.
    pub base_height: u32,
    pub scales: &'static [u32],
    /// Wider than this many times the height is scaled to fit.
    pub max_aspect: u32,
    /// Sizes past the scales (badges: 128, 256 and 512 px).
    pub large: &'static [Large],
    /// Every size is scaled from one master this many px tall: a bigger original is scaled down
    /// to it, a smaller one up. `None`: each size from the original, never above it.
    pub master: Option<u32>,
    /// An animated file at one of the scales over this many bytes fails the picture (chat sizes
    /// must animate).
    pub max_animated_scale_bytes: Option<usize>,
}

/// The cyex badge size ladder past the chat sizes, with the animated caps: 1, 2 and 4 MB.
const BADGE_LARGE: [Large; 3] = [
    Large {
        px: 128,
        max_animated_bytes: 1024 * 1024,
    },
    Large {
        px: 256,
        max_animated_bytes: 2 * 1024 * 1024,
    },
    Large {
        px: 512,
        max_animated_bytes: 4 * 1024 * 1024,
    },
];

/// The badge master, and the largest badge size: 512 px.
pub const BADGE_MASTER: u32 = 512;
/// An animated chat-size badge file (18, 36 or 72 px) is at most 512 KB.
pub const BADGE_MAX_ANIMATED_CHAT_BYTES: usize = 512 * 1024;

impl Profile {
    /// Emotes (ADR 0012): 32 px tall at 1x, 1x to 4x, up to 4:1.
    pub const EMOTE: Self = Self {
        base_height: BASE_HEIGHT,
        scales: &SCALES,
        max_aspect: MAX_ASPECT,
        large: &[],
        master: None,
        max_animated_scale_bytes: None,
    };
    /// Chat badges (ADR 0015), the cyex badge size ladder: square 18, 36 and 72 px (`1x`, `2x`,
    /// `4x`, as chat badges are), then 128, 256 and 512 px, all scaled from one 512 px master.
    /// Animated: 18, 36 and 72 always (each at most 512 KB), 128, 256 and 512 only within 1, 2
    /// and 4 MB.
    pub const BADGE: Self = Self {
        base_height: 18,
        scales: &[1, 2, 4],
        max_aspect: 1,
        large: &BADGE_LARGE,
        master: Some(BADGE_MASTER),
        max_animated_scale_bytes: Some(BADGE_MAX_ANIMATED_CHAT_BYTES),
    };
    /// Paint images (ADR 0015): 64 px tall at 1x, then 2x and 4x, up to 8:1.
    pub const PAINT: Self = Self {
        base_height: 64,
        scales: &[1, 2, 4],
        max_aspect: 8,
        large: &[],
        master: None,
        max_animated_scale_bytes: None,
    };

    /// The size at `scale`: `base_height × scale` tall, at most `max_aspect` times as wide,
    /// aspect kept, never larger than the original.
    pub fn size_for(&self, width: u32, height: u32, scale: u32) -> (u32, u32) {
        self.fit(width, height, self.base_height * scale, false)
    }

    /// `width`×`height` fitted in a box `box_height` tall and `max_aspect` times as wide, aspect
    /// kept; larger than the original only when `grow`.
    fn fit(&self, width: u32, height: u32, box_height: u32, grow: bool) -> (u32, u32) {
        let box_h = f64::from(box_height);
        let box_w = box_h * f64::from(self.max_aspect);
        let mut factor = (box_h / f64::from(height)).min(box_w / f64::from(width));
        if !grow {
            factor = factor.min(1.0);
        }
        let w = (f64::from(width) * factor).round().max(1.0);
        let h = (f64::from(height) * factor).round().max(1.0);
        // Both are at most the larger of the original side and the box, so they fit in u32.
        (w as u32, h as u32)
    }

    /// The master's size for an original of `width`×`height`, when the profile has one.
    pub fn master_size(&self, width: u32, height: u32) -> Option<(u32, u32)> {
        self.master.map(|m| self.fit(width, height, m, true))
    }

    /// Every size this profile makes for an original of `width`×`height`, smallest first.
    pub fn steps(&self, width: u32, height: u32) -> Vec<Step> {
        // With a master, every size comes from it; without, from the original.
        let (w, h) = self.master_size(width, height).unwrap_or((width, height));
        let mut out: Vec<Step> = self
            .scales
            .iter()
            .map(|&scale| Step {
                stem: format!("{scale}x"),
                scale,
                size: self.size_for(w, h, scale),
                max_animated_bytes: self.max_animated_scale_bytes,
                required: true,
            })
            .collect();
        for large in self.large {
            out.push(Step {
                stem: large.px.to_string(),
                scale: (large.px + self.base_height / 2) / self.base_height,
                size: self.fit(w, h, large.px, false),
                max_animated_bytes: Some(large.max_animated_bytes),
                required: false,
            });
        }
        out
    }

    /// Every file name stem (`1x`, `128`, …) of this profile, smallest first.
    pub fn stems(&self) -> Vec<String> {
        self.steps(1, 1).into_iter().map(|s| s.stem).collect()
    }

    /// Every file name `transcode_with` can produce for this profile.
    pub fn all_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for stem in self.stems() {
            for kind in [Kind::Webp, Kind::Avif] {
                for is_static in [false, true] {
                    names.push(name(&stem, kind, is_static));
                }
            }
        }
        names
    }

    /// Whether `names` (a picture's files) hold every size of this profile, as an animated or
    /// a still WebP. A picture made before a size was added lacks it.
    pub fn is_complete<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> bool {
        let names: Vec<&str> = names.into_iter().collect();
        self.stems().iter().all(|stem| {
            names.contains(&name(stem, Kind::Webp, false).as_str())
                || names.contains(&name(stem, Kind::Webp, true).as_str())
        })
    }
}

/// One size a profile makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// `1x`, `4x`, `128`: the file name before `_static` and the extension.
    pub stem: String,
    /// `1` to `4` for the scales; a larger size carries its px ÷ the 1x height, rounded (a 128 px
    /// badge: 7).
    pub scale: u32,
    pub size: (u32, u32),
    /// The cap on this size's animated WebP.
    pub max_animated_bytes: Option<usize>,
    /// Over the cap fails the picture (`true`), or only drops the animated file (`false`).
    pub required: bool,
}

pub use crate::MIN_DELAY_MS;

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

/// The size at `scale` for an emote (see [`Profile::size_for`]).
pub fn size_for(width: u32, height: u32, scale: u32) -> (u32, u32) {
    Profile::EMOTE.size_for(width, height, scale)
}

fn name(stem: &str, kind: Kind, is_static: bool) -> String {
    let suffix = if is_static { "_static" } else { "" };
    format!("{stem}{suffix}.{}", kind.extension())
}

/// Every file name `transcode` can produce, whatever the image: each scale as WebP, AVIF, and
/// the static WebP and AVIF forms of an animation. Takedown deletes all of them, so a transcode
/// that was still running leaves nothing behind.
pub fn all_names() -> Vec<String> {
    Profile::EMOTE.all_names()
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
    step: Step,
    encoder: Encoder,
}

/// Decodes `bytes` under `limits` and makes every emote file.
pub fn transcode(bytes: &[u8], limits: &Limits) -> Result<Encoded, MediaError> {
    transcode_with(bytes, limits, &Profile::EMOTE)
}

/// Decodes `bytes` under `limits` and makes every file at `profile`'s sizes.
pub fn transcode_with(
    bytes: &[u8],
    limits: &Limits,
    profile: &Profile,
) -> Result<Encoded, MediaError> {
    run(bytes, limits, profile, false)
}

/// Whether an animated picture's chat sizes fit their cap once converted
/// ([`Profile::max_animated_scale_bytes`]), without making the rest: what an upload checks
/// before it is accepted. Encodes only the largest scale (the others are smaller), straight from
/// the original. A still picture, or a profile without the cap, always fits.
pub fn check_animated_scales(
    bytes: &[u8],
    limits: &Limits,
    profile: &Profile,
) -> Result<(), MediaError> {
    run(bytes, limits, profile, true).map(|_| ())
}

fn kb(bytes: usize) -> usize {
    bytes.div_ceil(1024)
}

fn run(
    bytes: &[u8],
    limits: &Limits,
    profile: &Profile,
    check_only: bool,
) -> Result<Encoded, MediaError> {
    let mut scales: Vec<Scale> = Vec::new();
    let mut master: Option<(u32, u32)> = None;
    let mut first: Option<RgbaImage> = None;
    let mut timestamp: i32 = 0;
    let mut failure: Option<MediaError> = None;

    let (canvas, frames) = decode_frames(bytes, limits, &mut |canvas, frame, delay| {
        if scales.is_empty() {
            // The check resizes straight from the original: the 72 px file comes out the same
            // size in bytes, without scaling every frame to the master first.
            master = profile
                .master_size(canvas.width, canvas.height)
                .filter(|_| !check_only);
            let mut steps = profile.steps(canvas.width, canvas.height);
            if check_only {
                steps.retain(|s| s.required);
                steps.drain(..steps.len().saturating_sub(1));
            }
            for step in steps {
                let encoder = webp_encoder(step.size)?;
                scales.push(Scale { step, encoder });
            }
        }
        let frame = match master {
            Some(size) => resized(&frame, size),
            None => frame,
        };
        for s in &mut scales {
            let small = resized(&frame, s.step.size);
            if let Err(err) = s.encoder.add_frame(small.as_raw(), timestamp) {
                failure = Some(encode_err(err));
            }
        }
        let shown = crate::shown_ms(delay);
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
    let steps: Vec<Step> = scales.iter().map(|s| s.step.clone()).collect();

    let mut webp = Vec::new();
    for s in scales {
        let bytes = s
            .encoder
            .finalize(timestamp.max(1))
            .map_err(encode_err)?
            .to_vec();
        if animated
            && let Some(cap) = s.step.max_animated_bytes
            && bytes.len() > cap
        {
            if s.step.required {
                return Err(MediaError::TooLarge(format!(
                    "The animation is {} KB at {} px once converted, over the {} KB a chat                      size can be. Use fewer frames, fewer colours or a simpler picture.",
                    kb(bytes.len()),
                    s.step.size.1,
                    kb(cap)
                )));
            }
            // Too heavy to animate at this size: its still frame stands in.
            continue;
        }
        webp.push(Variant {
            name: name(&s.step.stem, Kind::Webp, false),
            kind: Kind::Webp,
            scale: s.step.scale,
            width: s.step.size.0,
            height: s.step.size.1,
            is_static: false,
            bytes,
        });
    }
    if check_only {
        return Ok(Encoded {
            canvas,
            frames,
            animated,
            webp,
            avif: Vec::new(),
        });
    }

    let mut avif = Vec::new();
    for step in &steps {
        let small = resized(&first, step.size);
        if animated {
            webp.push(Variant {
                name: name(&step.stem, Kind::Webp, true),
                kind: Kind::Webp,
                scale: step.scale,
                width: step.size.0,
                height: step.size.1,
                is_static: true,
                bytes: webp_still(&small)?,
            });
        }
        avif.push(Variant {
            name: name(&step.stem, Kind::Avif, animated),
            kind: Kind::Avif,
            scale: step.scale,
            width: step.size.0,
            height: step.size.1,
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
