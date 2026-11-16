//! Fixture images are made here, in code, so every case is reproducible and none is a binary
//! blob in the repo.

use std::io::Cursor;

use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, Rgba, RgbaImage};

use super::*;

fn gradient(w: u32, h: u32, shift: u8) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([
            (x * 255 / w.max(1)) as u8,
            (y * 255 / h.max(1)) as u8,
            shift,
            255,
        ])
    })
}

pub(crate) fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    gradient(w, h, 0)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

pub(crate) fn webp(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    gradient(w, h, 0)
        .write_to(&mut out, ImageFormat::WebP)
        .unwrap();
    out.into_inner()
}

pub(crate) fn gif(w: u32, h: u32, frames: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            enc.encode_frame(Frame::from_parts(
                gradient(w, h, (i * 40) as u8),
                0,
                0,
                Delay::from_numer_denom_ms(50, 1),
            ))
            .unwrap();
        }
    }
    out
}

pub(crate) fn apng(w: u32, h: u32, frames: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(frames, 0).unwrap();
        let mut writer = enc.write_header().unwrap();
        for i in 0..frames {
            writer
                .write_image_data(gradient(w, h, (i * 40) as u8).as_raw())
                .unwrap();
        }
        writer.finish().unwrap();
    }
    out
}

/// CRC-32 as PNG uses it (ISO-HDLC).
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A PNG whose header claims `w`×`h` but carries almost no data: the classic pixel bomb shape.
/// The header's checksum is fixed up, so only the size gives it away.
fn png_header_only(w: u32, h: u32) -> Vec<u8> {
    let mut out = png(1, 1);
    // IHDR: length at 8, type at 12, data at 16 (width, then height), CRC at 29.
    out[16..20].copy_from_slice(&w.to_be_bytes());
    out[20..24].copy_from_slice(&h.to_be_bytes());
    let crc = crc32(&out[12..29]);
    out[29..33].copy_from_slice(&crc.to_be_bytes());
    out
}

#[test]
fn the_crc_helper_matches_png() {
    // The 1×1 fixture's own IHDR checksum, as the encoder wrote it.
    let png = png(1, 1);
    assert_eq!(crc32(&png[12..29]).to_be_bytes(), png[29..33]);
}

fn ok(bytes: &[u8]) -> Probe {
    probe(bytes, &Limits::default()).unwrap()
}

#[test]
fn sniffs_by_content() {
    assert_eq!(sniff(&png(2, 2)), Some(Format::Png));
    assert_eq!(sniff(&gif(2, 2, 1)), Some(Format::Gif));
    assert_eq!(sniff(&webp(2, 2)), Some(Format::Webp));
    assert_eq!(
        sniff(b"\x00\x00\x00\x1cftypavif\x00\x00\x00\x00"),
        Some(Format::Avif)
    );
    assert_eq!(sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"), None);
    assert_eq!(sniff(b"\x00\x00\x00\x18ftypmp42"), None);
    assert_eq!(sniff(b""), None);
    assert_eq!(Format::Webp.mime(), "image/webp");
}

#[test]
fn a_static_png() {
    let p = ok(&png(112, 100));
    assert_eq!(
        (p.format, p.width, p.height, p.frames, p.animated),
        (Format::Png, 112, 100, 1, false)
    );
    assert_eq!(p.first_frame.dimensions(), (112, 100));
}

#[test]
fn an_apng() {
    let p = ok(&apng(32, 32, 4));
    assert_eq!((p.format, p.frames, p.animated), (Format::Png, 4, true));
}

#[test]
fn an_animated_gif_and_a_still_one() {
    let p = ok(&gif(40, 30, 6));
    assert_eq!(
        (p.format, p.width, p.height, p.frames, p.animated),
        (Format::Gif, 40, 30, 6, true)
    );
    let p = ok(&gif(40, 30, 1));
    assert!(!p.animated);
}

#[test]
fn a_static_webp() {
    let p = ok(&webp(64, 48));
    assert_eq!(
        (p.format, p.width, p.height, p.animated),
        (Format::Webp, 64, 48, false)
    );
}

#[test]
fn svg_video_and_avif_are_refused() {
    for bytes in [
        &b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"></svg>"[..],
        b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00",
        b"\x1aE\xdf\xa3 webm",
        b"\x00\x00\x00\x1cftypavif\x00\x00\x00\x00",
    ] {
        assert!(matches!(
            probe(bytes, &Limits::default()),
            Err(MediaError::Unsupported(_))
        ));
    }
}

#[test]
fn corrupt_files_are_invalid() {
    let mut broken = png(50, 50);
    let cut = broken.len() / 2;
    broken.truncate(cut);
    assert!(matches!(
        probe(&broken, &Limits::default()),
        Err(MediaError::Invalid(_))
    ));
    assert!(matches!(
        // A 2×2 GIF header, then nothing that decodes.
        probe(
            b"GIF89a\x02\x00\x02\x00\x00\x00\x00garbage!",
            &Limits::default()
        ),
        Err(MediaError::Invalid(_))
    ));
    assert!(matches!(
        probe(b"RIFF\x10\x00\x00\x00WEBPVP8 junk", &Limits::default()),
        Err(MediaError::Invalid(_))
    ));
}

#[test]
fn limits_are_enforced() {
    // Too big a file.
    let limits = Limits {
        max_bytes: 100,
        ..Limits::default()
    };
    assert!(matches!(
        probe(&png(64, 64), &limits),
        Err(MediaError::TooLarge(_))
    ));

    // Too wide, read from the header before decoding.
    assert!(matches!(
        probe(&png(1001, 10), &Limits::default()),
        Err(MediaError::TooLarge(_))
    ));

    // A pixel bomb: a tiny file that claims 50000×50000.
    let bomb = png_header_only(50_000, 50_000);
    assert!(bomb.len() < 200);
    assert!(matches!(
        probe(&bomb, &Limits::default()),
        Err(MediaError::TooLarge(_))
    ));

    // Too many frames, and too much work in total.
    let frames = Limits {
        max_frames: 3,
        ..Limits::default()
    };
    assert!(matches!(
        probe(&gif(8, 8, 4), &frames),
        Err(MediaError::TooLarge(_))
    ));
    let work = Limits {
        max_total_pixels: 1000,
        ..Limits::default()
    };
    assert!(matches!(
        probe(&apng(10, 10, 11), &work),
        Err(MediaError::TooLarge(_))
    ));
    assert!(probe(&apng(10, 10, 10), &work).is_ok());
}

#[test]
fn dhash_sees_near_duplicates() {
    let a = gradient(120, 80, 0);
    let mut b = a.clone();
    b.put_pixel(3, 3, Rgba([0, 0, 0, 255]));
    let mut c = gradient(120, 80, 0);
    image::imageops::flip_horizontal_in_place(&mut c);
    assert_eq!(dhash(&a), dhash(&a));
    assert!(distance(dhash(&a), dhash(&b)) <= 2);
    assert!(distance(dhash(&a), dhash(&c)) > 20);
    assert_eq!(distance(0, u64::MAX), 64);
}
