//! Checking an uploaded image (PLAN.md Phase 3): what format it really is, that it decodes, and
//! that it stays inside the limits, frame by frame, so a small file can't expand into gigabytes.
//! Also its dHash, for the near-duplicate blocklist.
//!
//! Inputs: PNG, APNG, GIF and WebP (still and animated). AVIF is recognised but refused for now:
//! decoding it needs dav1d, a C library (see PROGRESS.md). SVG and video are refused.
//!
//! Written for emotes, but nothing here is emote-specific, so cosmetic badge art can use it.

use std::io::Cursor;

use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::imageops::FilterType;
use image::{AnimationDecoder, DynamicImage, Frame, ImageDecoder, RgbaImage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Gif,
    Webp,
    Avif,
}

impl Format {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Avif => "image/avif",
        }
    }
}

/// Upload limits (PLAN.md Phase 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_side: u32,
    pub max_frames: u32,
    /// Width × height × frames, all frames together: the cap on decoding work.
    pub max_total_pixels: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 5 * 1024 * 1024,
            max_side: 1000,
            max_frames: 1000,
            max_total_pixels: 100_000_000,
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MediaError {
    /// Not a format we take (SVG, video, AVIF for now, anything else).
    #[error("{0}")]
    Unsupported(&'static str),
    /// Over a limit.
    #[error("{0}")]
    TooLarge(String),
    /// Says it is an image but doesn't decode.
    #[error("the image doesn't decode: {0}")]
    Invalid(String),
}

/// What a valid image is.
#[derive(Clone, Debug)]
pub struct Probe {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub animated: bool,
    pub first_frame: RgbaImage,
}

/// The format by magic bytes, never by name or declared type.
pub fn sniff(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(Format::Png);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some(Format::Gif);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(Format::Webp);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && matches!(&bytes[8..12], b"avif" | b"avis") {
        return Some(Format::Avif);
    }
    None
}

fn invalid(err: impl std::fmt::Display) -> MediaError {
    MediaError::Invalid(err.to_string())
}

/// Checks the canvas before anything is decoded.
fn check_side(width: u32, height: u32, limits: &Limits) -> Result<(), MediaError> {
    if width == 0 || height == 0 {
        return Err(MediaError::Invalid("the image has no pixels".into()));
    }
    if width > limits.max_side || height > limits.max_side {
        return Err(MediaError::TooLarge(format!(
            "{width}×{height} px is over the {0}×{0} px limit",
            limits.max_side
        )));
    }
    Ok(())
}

fn decoder_limits() -> image::Limits {
    let mut l = image::Limits::default();
    // One frame of at most 1000×1000 RGBA is 4 MB; leave room for the decoders' own buffers.
    l.max_alloc = Some(64 * 1024 * 1024);
    l
}

/// Walks the frames, stopping as soon as a limit is passed.
fn frames<'a>(
    iter: impl Iterator<Item = image::ImageResult<Frame>> + 'a,
    width: u32,
    height: u32,
    limits: &Limits,
) -> Result<(u32, RgbaImage), MediaError> {
    let per_frame = u64::from(width) * u64::from(height);
    let mut count: u32 = 0;
    let mut first: Option<RgbaImage> = None;
    for frame in iter {
        let frame = frame.map_err(invalid)?;
        count += 1;
        if count > limits.max_frames {
            return Err(MediaError::TooLarge(format!(
                "more than {} frames",
                limits.max_frames
            )));
        }
        if u64::from(count) * per_frame > limits.max_total_pixels {
            return Err(MediaError::TooLarge(
                "the animation is too big to process (frames × pixels)".into(),
            ));
        }
        if first.is_none() {
            first = Some(frame.into_buffer());
        }
    }
    let first = first.ok_or_else(|| MediaError::Invalid("the animation has no frames".into()))?;
    Ok((count, first))
}

fn still(decoder: impl ImageDecoder, format: Format, limits: &Limits) -> Result<Probe, MediaError> {
    let (width, height) = decoder.dimensions();
    check_side(width, height, limits)?;
    let image = DynamicImage::from_decoder(decoder).map_err(invalid)?;
    Ok(Probe {
        format,
        width,
        height,
        frames: 1,
        animated: false,
        first_frame: image.to_rgba8(),
    })
}

fn animated(format: Format, width: u32, height: u32, (frames, first): (u32, RgbaImage)) -> Probe {
    Probe {
        format,
        width,
        height,
        frames,
        // A one-frame "animation" is a still image.
        animated: frames > 1,
        first_frame: first,
    }
}

/// Validates an upload by decoding all of it under `limits`.
pub fn probe(bytes: &[u8], limits: &Limits) -> Result<Probe, MediaError> {
    if bytes.len() > limits.max_bytes {
        return Err(MediaError::TooLarge(format!(
            "the file is over {} MB",
            limits.max_bytes / (1024 * 1024)
        )));
    }
    let format = sniff(bytes).ok_or(MediaError::Unsupported(
        "Upload PNG, APNG, GIF or WebP. SVG and video aren't supported.",
    ))?;
    match format {
        Format::Avif => Err(MediaError::Unsupported(
            "AVIF uploads aren't supported yet. Upload PNG, APNG, GIF or WebP.",
        )),
        Format::Png => {
            let mut decoder = PngDecoder::new(Cursor::new(bytes)).map_err(invalid)?;
            decoder.set_limits(decoder_limits()).map_err(invalid)?;
            let (width, height) = decoder.dimensions();
            check_side(width, height, limits)?;
            if decoder.is_apng().map_err(invalid)? {
                let apng = decoder.apng().map_err(invalid)?;
                let found = frames(apng.into_frames(), width, height, limits)?;
                Ok(animated(format, width, height, found))
            } else {
                still(decoder, format, limits)
            }
        }
        Format::Gif => {
            let mut decoder = GifDecoder::new(Cursor::new(bytes)).map_err(invalid)?;
            decoder.set_limits(decoder_limits()).map_err(invalid)?;
            let (width, height) = decoder.dimensions();
            check_side(width, height, limits)?;
            let found = frames(decoder.into_frames(), width, height, limits)?;
            Ok(animated(format, width, height, found))
        }
        Format::Webp => {
            let mut decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(invalid)?;
            decoder.set_limits(decoder_limits()).map_err(invalid)?;
            let (width, height) = decoder.dimensions();
            check_side(width, height, limits)?;
            if decoder.has_animation() {
                let found = frames(decoder.into_frames(), width, height, limits)?;
                Ok(animated(format, width, height, found))
            } else {
                still(decoder, format, limits)
            }
        }
    }
}

/// The 64-bit difference hash of an image: shrink to 9×8 grey, then one bit per neighbour pair
/// (left brighter than right). Near-identical images differ in few bits.
pub fn dhash(image: &RgbaImage) -> u64 {
    let grey = DynamicImage::ImageRgba8(image.clone())
        .resize_exact(9, 8, FilterType::Triangle)
        .to_luma8();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let left = grey.get_pixel(x, y)[0];
            let right = grey.get_pixel(x + 1, y)[0];
            hash = (hash << 1) | u64::from(left > right);
        }
    }
    hash
}

/// Bits that differ between two dHashes.
pub fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests;
