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

/// A GIF of the given frames, 50 ms each.
fn gif_of(frames: &[RgbaImage]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for f in frames {
            enc.encode_frame(Frame::from_parts(
                f.clone(),
                0,
                0,
                Delay::from_numer_denom_ms(50, 1),
            ))
            .unwrap();
        }
    }
    out
}

#[test]
fn every_frame_is_hashed_not_only_the_first() {
    let benign = gradient(64, 64, 0);
    let mut blocked = gradient(64, 64, 0);
    image::imageops::flip_horizontal_in_place(&mut blocked);
    assert!(distance(dhash(&benign), dhash(&blocked)) > 20);

    let p = ok(&gif_of(&[benign.clone(), blocked.clone()]));
    assert_eq!(p.frames, 2);
    assert_eq!(p.frame_hashes.len(), 2);
    assert_eq!(p.frame_hashes[0], dhash(&p.first_frame));
    assert!(
        p.frame_hashes
            .iter()
            .any(|h| distance(*h, dhash(&blocked)) <= 4),
        "the second frame's hash must be present"
    );

    // Repeated frames are listed once.
    let p = ok(&gif_of(&[benign.clone(), benign.clone(), benign]));
    assert_eq!(p.frame_hashes.len(), 1);
}

#[test]
fn all_names_covers_every_name_the_encoder_makes() {
    let names = encode::all_names();
    assert_eq!(names.len(), 16);
    assert!(names.contains(&"1x.webp".to_owned()));
    assert!(names.contains(&"4x_static.avif".to_owned()));
    let out = encode::transcode(&gif(40, 30, 3), &Limits::default()).unwrap();
    for v in out.webp.iter().chain(&out.avif) {
        assert!(names.contains(&v.name), "{} is missing", v.name);
    }
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

// Encoding (ADR 0012)

mod encoding {
    use image::AnimationDecoder;
    use image::codecs::webp::WebPDecoder;

    use super::*;
    use crate::encode::{Kind, size_for, transcode};

    /// An animated WebP, made with the same encoder the worker uses.
    pub(crate) fn animated_webp(w: u32, h: u32, frames: u32) -> Vec<u8> {
        let mut enc = webp_animation::Encoder::new((w, h)).unwrap();
        for i in 0..frames {
            enc.add_frame(gradient(w, h, (i * 30) as u8).as_raw(), (i * 80) as i32)
                .unwrap();
        }
        enc.finalize((frames * 80) as i32).unwrap().to_vec()
    }

    fn webp_frames(bytes: &[u8]) -> (bool, usize, (u32, u32)) {
        let decoder = WebPDecoder::new(Cursor::new(bytes)).unwrap();
        let animated = decoder.has_animation();
        let dims = decoder.dimensions();
        let count = if animated {
            decoder.into_frames().count()
        } else {
            1
        };
        (animated, count, dims)
    }

    fn is_avif(bytes: &[u8]) -> bool {
        bytes.len() > 12 && &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"avif"
    }

    #[test]
    fn sizes_follow_adr_0012() {
        assert_eq!(size_for(112, 112, 1), (32, 32));
        assert_eq!(size_for(112, 112, 2), (64, 64));
        // Never larger than the original.
        assert_eq!(size_for(112, 112, 4), (112, 112));
        assert_eq!(size_for(20, 20, 4), (20, 20));
        // Wide emotes keep their aspect, up to 4:1.
        assert_eq!(size_for(200, 100, 1), (64, 32));
        assert_eq!(size_for(500, 100, 1), (128, 26));
        assert_eq!(size_for(1000, 1, 1), (128, 1));
    }

    #[test]
    fn no_emote_file_is_taller_than_128_px() {
        use crate::encode::{EMOTE_MAX_HEIGHT, Profile};
        assert_eq!(EMOTE_MAX_HEIGHT, 128);
        let p = Profile::EMOTE;
        assert!(p.large.is_empty(), "no emote sizes past 4x");
        for (w, h) in [
            (1000, 1000),
            (1000, 999),
            (999, 1000),
            (129, 129),
            (1000, 1),
            (1, 1000),
            (250, 1000),
            (1000, 250),
            (777, 333),
            (128, 128),
            (40, 40),
        ] {
            let steps = p.steps(w, h);
            assert_eq!(steps.len(), 4, "{w}×{h}");
            for s in &steps {
                assert!(
                    s.size.1 <= EMOTE_MAX_HEIGHT,
                    "{w}×{h} {}: {:?}",
                    s.stem,
                    s.size
                );
                assert!(
                    s.size.0 <= EMOTE_MAX_HEIGHT * 4,
                    "{w}×{h} {}: {:?}",
                    s.stem,
                    s.size
                );
            }
            // The largest file is 4x: as tall as the box allows, or the original when smaller.
            let largest = steps.iter().map(|s| s.size.1).max().unwrap();
            assert_eq!(steps.last().unwrap().size.1, largest);
        }
        assert_eq!(p.steps(1000, 1000).last().unwrap().size, (128, 128));
        assert_eq!(p.steps(250, 1000).last().unwrap().size, (32, 128));
        assert_eq!(
            p.steps(40, 40).last().unwrap().size,
            (40, 40),
            "never scaled up"
        );
    }

    #[test]
    fn a_large_upload_makes_a_4x_file_128_px_tall() {
        let out = transcode(&png(600, 600), &Limits::default()).unwrap();
        let tallest = out.webp.iter().chain(&out.avif).map(|v| v.height).max();
        assert_eq!(tallest, Some(128));
        let four = out.webp.iter().find(|v| v.name == "4x.webp").unwrap();
        assert_eq!((four.width, four.height), (128, 128));
    }

    #[test]
    fn an_animated_webp_upload_is_read() {
        let p = probe(&animated_webp(40, 40, 5), &Limits::default()).unwrap();
        assert_eq!((p.format, p.frames, p.animated), (Format::Webp, 5, true));
    }

    #[test]
    fn a_still_image_gets_webp_and_avif_at_four_sizes() {
        let out = transcode(&png(128, 128), &Limits::default()).unwrap();
        assert!(!out.animated);
        let names: Vec<&str> = out
            .webp
            .iter()
            .chain(&out.avif)
            .map(|v| v.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "1x.webp", "2x.webp", "3x.webp", "4x.webp", "1x.avif", "2x.avif", "3x.avif",
                "4x.avif"
            ]
        );
        for (v, side) in out.webp.iter().zip([32, 64, 96, 128]) {
            assert_eq!(
                (v.width, v.height, v.kind, v.is_static),
                (side, side, Kind::Webp, false)
            );
            assert_eq!(
                webp_frames(&v.bytes),
                (false, 1, (side, side)),
                "{}",
                v.name
            );
        }
        for v in &out.avif {
            assert!(is_avif(&v.bytes), "{}", v.name);
            assert!(!v.is_static);
        }
    }

    #[test]
    fn an_animation_stays_animated_and_gets_static_first_frames() {
        for (label, input) in [
            ("gif", gif(64, 48, 6)),
            ("apng", apng(64, 48, 6)),
            ("webp", animated_webp(64, 48, 6)),
        ] {
            let out = transcode(&input, &Limits::default()).unwrap();
            assert!(out.animated, "{label}");
            assert_eq!(out.frames, 6, "{label}");
            let names: Vec<&str> = out.webp.iter().map(|v| v.name.as_str()).collect();
            assert_eq!(
                names,
                [
                    "1x.webp",
                    "2x.webp",
                    "3x.webp",
                    "4x.webp",
                    "1x_static.webp",
                    "2x_static.webp",
                    "3x_static.webp",
                    "4x_static.webp"
                ],
                "{label}"
            );
            // 64×48 is 4:3: 1x is 43×32, and 4x stays at the original size.
            assert_eq!((out.webp[0].width, out.webp[0].height), (43, 32), "{label}");
            assert_eq!((out.webp[3].width, out.webp[3].height), (64, 48), "{label}");
            let (animated, frames, _) = webp_frames(&out.webp[0].bytes);
            assert!(animated, "{label}");
            assert_eq!(frames, 6, "{label}");
            let (animated, _, dims) = webp_frames(&out.webp[4].bytes);
            assert!(!animated, "{label}");
            assert_eq!(dims, (43, 32), "{label}");
            let avif: Vec<&str> = out.avif.iter().map(|v| v.name.as_str()).collect();
            assert_eq!(
                avif,
                [
                    "1x_static.avif",
                    "2x_static.avif",
                    "3x_static.avif",
                    "4x_static.avif"
                ],
                "{label}"
            );
            assert!(
                out.avif.iter().all(|v| v.is_static && is_avif(&v.bytes)),
                "{label}"
            );
        }
    }

    #[test]
    fn transcoding_keeps_the_upload_limits() {
        let work = Limits {
            max_total_pixels: 1000,
            ..Limits::default()
        };
        assert!(matches!(
            transcode(&gif(20, 20, 4), &work),
            Err(MediaError::TooLarge(_))
        ));
    }

    #[test]
    fn badge_and_paint_profiles() {
        use crate::encode::{Profile, transcode_with};
        let b = Profile::BADGE;
        assert_eq!(b.size_for(72, 72, 1), (18, 18));
        assert_eq!(b.size_for(72, 72, 2), (36, 36));
        assert_eq!(b.size_for(72, 72, 4), (72, 72));
        // Square boxes: a wide picture fits the width.
        assert_eq!(b.size_for(144, 72, 1), (18, 9));
        // Six sizes and the original, each as WebP, AVIF and their still forms.
        assert_eq!(b.all_names().len(), 28);
        assert_eq!(
            b.stems(),
            ["1x", "2x", "4x", "128", "256", "512", "original"]
        );
        assert!(!b.all_names().contains(&"3x.webp".to_owned()));
        assert!(b.all_names().contains(&"512_static.avif".to_owned()));
        let p = Profile::PAINT;
        assert_eq!(p.size_for(512, 256, 1), (128, 64));
        assert_eq!(p.size_for(1000, 100, 1), (512, 51));
        // 1x, 2x, 4x and 512, each as WebP, AVIF and their still forms.
        assert_eq!(p.all_names().len(), 16);
        assert_eq!(p.stems(), ["1x", "2x", "4x", "512"]);
        // The 512 px size is 512 tall (8:1 at most), never above the original.
        assert_eq!(p.steps(4096, 512)[3].size, (4096, 512));
        assert_eq!(p.steps(1000, 100)[3].size, (1000, 100));
        assert_eq!(p.steps(3000, 600)[3].size, (2560, 512));
        // A paint made before 512 px lacks it.
        assert!(!p.is_complete(["1x.webp", "2x.webp", "4x.webp"]));
        assert!(p.is_complete(["1x.webp", "2x.webp", "4x.webp", "512_static.webp"]));

        let still = transcode_with(&png(96, 96), &Limits::badge(), &b).unwrap();
        let sizes: Vec<_> = still
            .webp
            .iter()
            .map(|v| (v.name.as_str(), v.scale, v.width, v.height))
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
                // 96 px stays 96 px: not scaled to the 512 master.
                ("original.webp", 5, 96, 96)
            ]
        );
        let avif: Vec<_> = still.avif.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            avif,
            [
                "1x.avif",
                "2x.avif",
                "4x.avif",
                "128.avif",
                "256.avif",
                "512.avif",
                "original.avif"
            ]
        );
        assert!(still.avif.iter().all(|v| is_avif(&v.bytes)));
        let (_, _, dims) = webp_frames(&still.webp[5].bytes);
        assert_eq!(dims, (512, 512));

        let moving = transcode_with(&gif(80, 80, 3), &Limits::badge(), &b).unwrap();
        assert!(moving.animated);
        // Animated WebP at each size (all within their caps), plus still WebP and AVIF.
        let names: Vec<_> = moving.webp.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "1x.webp",
                "2x.webp",
                "4x.webp",
                "128.webp",
                "256.webp",
                "512.webp",
                "original.webp",
                "1x_static.webp",
                "2x_static.webp",
                "4x_static.webp",
                "128_static.webp",
                "256_static.webp",
                "512_static.webp",
                "original_static.webp"
            ]
        );
        assert!(moving.avif.iter().all(|v| v.is_static));
        assert_eq!(moving.avif.len(), 7);
        let (animated, frames, dims) = webp_frames(&moving.webp[0].bytes);
        assert!(animated);
        assert_eq!((frames, dims), (3, (18, 18)));
        let (animated, frames, dims) = webp_frames(&moving.webp[5].bytes);
        assert!(animated);
        assert_eq!((frames, dims), (3, (512, 512)));
        // The original keeps every frame at 80 px.
        let (animated, frames, dims) = webp_frames(&moving.webp[6].bytes);
        assert!(animated);
        assert_eq!((frames, dims), (3, (80, 80)));
        for v in moving.webp.iter().chain(&moving.avif) {
            assert!(b.all_names().contains(&v.name), "{}", v.name);
        }
    }

    #[test]
    fn badges_come_from_one_512_px_master() {
        use crate::encode::{Profile, transcode_with};
        let b = Profile::BADGE;
        assert_eq!(b.master_size(72, 72), Some((512, 512)));
        assert_eq!(b.master_size(2048, 2048), Some((512, 512)));
        assert_eq!(Profile::EMOTE.master_size(72, 72), None);
        let sizes = |bytes: &[u8]| -> Vec<(u32, u32)> {
            transcode_with(bytes, &Limits::badge(), &b)
                .unwrap()
                .webp
                .iter()
                .filter(|v| v.name != "original.webp")
                .map(|v| (v.width, v.height))
                .collect()
        };
        let ladder = [
            (18, 18),
            (36, 36),
            (72, 72),
            (128, 128),
            (256, 256),
            (512, 512),
        ];
        // The smallest upload is scaled up, a bigger one down: the same ladder either way.
        assert_eq!(sizes(&png(72, 72)), ladder);
        assert_eq!(sizes(&png(1024, 1024)), ladder);
        // Emotes still never grow past the original.
        assert_eq!(size_for(20, 20, 4), (20, 20));
    }

    #[test]
    fn a_badge_original_is_kept_at_its_size_up_to_512() {
        use crate::encode::{Profile, transcode_with};
        let b = Profile::BADGE;
        let original = |bytes: &[u8]| {
            let made = transcode_with(bytes, &Limits::badge(), &b).unwrap();
            let v = made
                .webp
                .iter()
                .find(|v| v.name == "original.webp")
                .unwrap();
            let (_, _, dims) = webp_frames(&v.bytes);
            assert_eq!(dims, (v.width, v.height));
            // The ladder is still all there.
            let ladder: Vec<_> = made
                .webp
                .iter()
                .filter(|v| v.name != "original.webp")
                .map(|v| v.width)
                .collect();
            assert_eq!(ladder, [18, 36, 72, 128, 256, 512]);
            dims
        };
        // Up to 512 it is as uploaded: 512 stays 512, 300 is not scaled up to the master.
        assert_eq!(original(&png(512, 512)), (512, 512));
        assert_eq!(original(&png(300, 300)), (300, 300));
        assert_eq!(original(&png(72, 72)), (72, 72));
        // Over 512 it is scaled down to 512, the only downscale.
        assert_eq!(original(&png(1024, 1024)), (512, 512));
        assert_eq!(b.steps(2048, 2048).last().unwrap().size, (512, 512));
    }

    #[test]
    fn a_badge_original_is_lossless() {
        use crate::encode::{Profile, transcode_with};
        let made = transcode_with(&png(64, 64), &Limits::badge(), &Profile::BADGE).unwrap();
        let v = made
            .webp
            .iter()
            .find(|v| v.name == "original.webp")
            .unwrap();
        let decoded = image::load_from_memory_with_format(&v.bytes, image::ImageFormat::WebP)
            .unwrap()
            .to_rgba8();
        let source = image::load_from_memory(&png(64, 64)).unwrap().to_rgba8();
        assert_eq!(decoded.as_raw(), source.as_raw());
    }

    #[test]
    fn animated_badges_keep_to_their_caps() {
        use crate::encode::{
            BADGE_MAX_ANIMATED_CHAT_BYTES, Large, Profile, check_animated_scales, transcode_with,
        };
        assert_eq!(BADGE_MAX_ANIMATED_CHAT_BYTES, 512 * 1024);
        let caps: Vec<_> = Profile::BADGE
            .large
            .iter()
            .map(|l| (l.px, l.max_animated_bytes))
            .collect();
        assert_eq!(
            caps,
            [(128, 1 << 20), (256, 2 << 20), (512, 4 << 20)],
            "1, 2 and 4 MB"
        );
        let file = gif(80, 80, 3);
        assert!(check_animated_scales(&file, &Limits::badge(), &Profile::BADGE).is_ok());

        // A chat size over its cap fails the picture, in the upload check and in the job.
        let tight = Profile {
            max_animated_scale_bytes: Some(64),
            ..Profile::BADGE
        };
        let Err(MediaError::TooLarge(message)) =
            check_animated_scales(&file, &Limits::badge(), &tight)
        else {
            panic!("over the chat cap is refused");
        };
        assert!(message.contains("at 72 px"), "{message}");
        assert!(matches!(
            transcode_with(&file, &Limits::badge(), &tight),
            Err(MediaError::TooLarge(_))
        ));
        // A still picture has nothing to animate.
        assert!(check_animated_scales(&png(96, 96), &Limits::badge(), &tight).is_ok());

        // A large size over its cap keeps only its still frame.
        const HEAVY: [Large; 3] = [
            Large {
                px: 128,
                max_animated_bytes: 1 << 20,
            },
            Large {
                px: 256,
                max_animated_bytes: 64,
            },
            Large {
                px: 512,
                max_animated_bytes: 64,
            },
        ];
        let heavy = Profile {
            large: &HEAVY,
            ..Profile::BADGE
        };
        let out = transcode_with(&file, &Limits::badge(), &heavy).unwrap();
        let names: Vec<_> = out.webp.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"128.webp"));
        assert!(!names.contains(&"256.webp") && !names.contains(&"512.webp"));
        assert!(names.contains(&"256_static.webp") && names.contains(&"512_static.webp"));
        assert!(heavy.is_complete(names.iter().copied()));
    }

    #[test]
    fn a_picture_made_before_the_ladder_is_not_complete() {
        use crate::encode::Profile;
        let b = Profile::BADGE;
        let old = [
            "1x.webp", "2x.webp", "4x.webp", "1x.avif", "2x.avif", "4x.avif",
        ];
        assert!(!b.is_complete(old));
        let new = [
            "1x.webp", "2x.webp", "4x.webp", "128.webp", "256.webp", "512.webp",
        ];
        // A ladder without the original lacks it (made again).
        assert!(!b.is_complete(new));
        let new = [
            "1x.webp",
            "2x.webp",
            "4x.webp",
            "128.webp",
            "256.webp",
            "512.webp",
            "original.webp",
        ];
        assert!(b.is_complete(new));
        let animated = [
            "1x.webp",
            "2x.webp",
            "4x.webp",
            "128.webp",
            "256_static.webp",
            "512_static.webp",
            "original_static.webp",
        ];
        assert!(b.is_complete(animated));
        // Emotes have no larger sizes: their four scales are all.
        assert!(Profile::EMOTE.is_complete(["1x.webp", "2x.webp", "3x.webp", "4x.webp"]));
    }
}

/// A valid PNG of `w`×`h` padded to `total` bytes with a private ancillary chunk after IHDR, so a
/// file can be big without being a big picture (decoders skip unknown ancillary chunks).
pub(crate) fn padded_png(w: u32, h: u32, total: usize) -> Vec<u8> {
    let small = png(w, h);
    // Signature (8) and IHDR (25) come first; the padding chunk has 12 bytes of framing.
    let data_len = total.saturating_sub(small.len() + 12);
    let mut chunk = Vec::with_capacity(data_len + 12);
    chunk.extend_from_slice(&u32::try_from(data_len).unwrap().to_be_bytes());
    chunk.extend_from_slice(b"prVt");
    chunk.resize(8 + data_len, 0x5a);
    let crc = crc32(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    let mut out = small[..33].to_vec();
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&small[33..]);
    assert_eq!(out.len(), total);
    out
}

pub(crate) fn jpeg(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(gradient(w, h, 0))
        .to_rgb8()
        .write_to(&mut out, ImageFormat::Jpeg)
        .unwrap();
    out.into_inner()
}

#[test]
fn a_jpeg_is_a_still_image() {
    let file = jpeg(96, 64);
    assert_eq!(sniff(&file), Some(Format::Jpeg));
    assert_eq!(Format::Jpeg.mime(), "image/jpeg");
    let p = ok(&file);
    assert_eq!(
        (p.format, p.width, p.height, p.frames, p.animated),
        (Format::Jpeg, 96, 64, 1, false)
    );
    // Its files are made like any still: WebP and AVIF at every scale.
    let made = encode::transcode(&file, &Limits::emote()).unwrap();
    assert!(!made.animated);
    assert!(!made.webp.is_empty());
    // A JPEG cut short doesn't decode.
    let mut cut = jpeg(96, 64);
    cut.truncate(cut.len() / 3);
    assert!(matches!(
        probe(&cut, &Limits::default()),
        Err(MediaError::Invalid(_))
    ));
}

#[test]
fn emote_rules_are_fixed_and_one_place_holds_them() {
    assert_eq!(EmoteRules::MAX_BYTES, 8 * 1024 * 1024);
    assert_eq!(EmoteRules::MAX_SIDE, 1000);
    assert_eq!(EmoteRules::MAX_FRAMES, 1000);
    assert_eq!(EmoteRules::MAX_FPS, 50);
    assert_eq!(EmoteRules::MIN_FRAME_MS, 20);
    assert_eq!(EmoteRules::MAX_ASPECT, 4);
    let l = Limits::emote();
    assert_eq!(l, EmoteRules::limits());
    assert_eq!(l.max_bytes, 8 * 1024 * 1024);
    assert_eq!((l.max_side, l.max_frames), (1000, 1000));
    // No hidden sixth rule: the work cap is exactly 1000 x 1000 px x 1000 frames, no time cap.
    assert_eq!(l.max_total_pixels, 1_000_000_000);
    assert_eq!(l.max_duration_ms, u64::MAX);
    assert_eq!(l.max_aspect, Some(4));
    assert_eq!(MIN_DELAY_MS, EmoteRules::MIN_FRAME_MS);
    assert_eq!(Limits::default().max_bytes, 5 * 1024 * 1024);
}

#[test]
fn an_emote_file_is_at_most_8_mb() {
    let six = padded_png(64, 64, 6 * 1024 * 1024);
    // Over the default (cosmetics), inside an emote's limit, and it decodes.
    assert!(matches!(
        probe(&six, &Limits::default()),
        Err(MediaError::TooLarge(_))
    ));
    let p = probe(&six, &Limits::emote()).unwrap();
    assert_eq!((p.format, p.width, p.height), (Format::Png, 64, 64));
    let exactly = padded_png(64, 64, EmoteRules::MAX_BYTES);
    assert!(probe(&exactly, &Limits::emote()).is_ok());
    let over = padded_png(64, 64, EmoteRules::MAX_BYTES + 1);
    let Err(MediaError::TooLarge(message)) = probe(&over, &Limits::emote()) else {
        panic!("an 8 MB + 1 byte file is refused");
    };
    assert_eq!(message, "The file is over 8 MB, the most an upload can be.");
}

#[test]
fn an_emote_side_is_at_most_1000_px() {
    let l = Limits::emote();
    assert!(probe(&png(1000, 1000), &l).is_ok());
    // Tall pictures have no ratio rule: 1 x 1000 is fine.
    assert!(probe(&png(1, 1000), &l).is_ok());
    for (w, h) in [(1001, 1000), (1000, 1001), (1001, 1001)] {
        let Err(MediaError::TooLarge(message)) = probe(&png(w, h), &l) else {
            panic!("{w}x{h} is refused");
        };
        assert!(message.contains("1000×1000 px"), "{message}");
    }
}

#[test]
fn an_emote_is_at_most_4_to_1_wide_and_tall_pictures_have_no_ratio_rule() {
    let l = Limits::emote();
    assert!(probe(&png(400, 100), &l).is_ok());
    assert!(probe(&png(1000, 250), &l).is_ok());
    for (w, h) in [(401, 100), (1000, 249), (5, 1)] {
        let Err(MediaError::BadShape(message)) = probe(&png(w, h), &l) else {
            panic!("{w}x{h} is refused as too wide");
        };
        assert!(message.contains("4:1"), "{message}");
    }
    // Tall: 10 x 1000 is 1:100 and fine.
    assert!(probe(&png(10, 1000), &l).is_ok());
    // Badges and paints have their own shape rules (the API's), none here.
    assert!(probe(&png(500, 100), &Limits::badge()).is_ok());
}

#[test]
fn an_emote_animation_has_at_most_1000_frames() {
    let l = Limits::emote();
    assert!(probe(&gif(2, 2, 1000), &l).is_ok());
    let Err(MediaError::TooLarge(message)) = probe(&gif(2, 2, 1001), &l) else {
        panic!("1001 frames is refused");
    };
    assert_eq!(
        message,
        "The animation has more than 1000 frames, the most it can have."
    );
}

#[test]
fn an_emote_gif_with_0_or_10_ms_delays_is_taken_and_shown_at_100_ms() {
    // At most 50 fps: a frame is never shown for less than 20 ms. Shorter delays aren't refused;
    // browsers show them for 100 ms and so do we.
    assert_eq!(shown_ms(0), 100);
    assert_eq!(shown_ms(10), 100);
    assert_eq!(shown_ms(19), 100);
    assert_eq!(shown_ms(20), 20);
    let l = Limits::emote();
    for delay in [0, 10] {
        let file = gif_with_delay(8, 8, 3, delay);
        let mut shown = Vec::new();
        let (_, frames) = decode_frames(&file, &l, &mut |_, _, ms| {
            shown.push(shown_ms(ms));
            Ok(())
        })
        .unwrap();
        assert_eq!(frames, 3);
        assert_eq!(shown, vec![100, 100, 100], "delay {delay} ms");
        assert!(probe(&file, &l).is_ok());
    }
    // 20 ms (50 fps) is kept as it is.
    let mut shown = Vec::new();
    decode_frames(&gif_with_delay(8, 8, 2, 20), &l, &mut |_, _, ms| {
        shown.push(shown_ms(ms));
        Ok(())
    })
    .unwrap();
    assert_eq!(shown, vec![20, 20]);
}

fn gif_with_delay(w: u32, h: u32, frames: u32, delay_ms: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            enc.encode_frame(Frame::from_parts(
                gradient(w, h, (i * 40) as u8),
                0,
                0,
                Delay::from_numer_denom_ms(delay_ms, 1),
            ))
            .unwrap();
        }
    }
    out
}

#[test]
fn badge_limits_follow_the_badge_spec() {
    let l = Limits::badge();
    assert_eq!(l.max_bytes, 8 * 1024 * 1024);
    assert_eq!(l.max_frames, 120);
    assert_eq!(l.max_duration_ms, 10_000);
    assert_eq!(l.max_total_pixels, 64_000_000);
    // A 512×512 animation of 120 frames fits the work cap.
    assert!(512 * 512 * 120 <= l.max_total_pixels);
    // Bigger than the master is fine: it is scaled down.
    assert!(probe(&png(1024, 1024), &l).is_ok());
    assert!(matches!(
        probe(&png(2049, 2049), &l),
        Err(MediaError::TooLarge(_))
    ));

    assert!(probe(&gif(8, 8, 120), &l).is_ok());
    let Err(MediaError::TooLarge(message)) = probe(&gif(8, 8, 121), &l) else {
        panic!("121 frames is refused");
    };
    assert_eq!(
        message,
        "The animation has more than 120 frames, the most it can have."
    );

    // 10 seconds exactly fits; one frame more doesn't.
    assert!(probe(&gif_with_delay(8, 8, 5, 2000), &l).is_ok());
    let Err(MediaError::TooLarge(message)) = probe(&gif_with_delay(8, 8, 6, 2000), &l) else {
        panic!("12 seconds is refused");
    };
    assert_eq!(
        message,
        "The animation runs longer than 10 seconds, the most it can."
    );
    // Frames under 20 ms show for 100 ms, so 101 of them run 10.1 s.
    assert!(matches!(
        probe(&gif_with_delay(8, 8, 101, 10), &l),
        Err(MediaError::TooLarge(_))
    ));
    // Emotes have no duration cap.
    assert!(probe(&gif_with_delay(8, 8, 6, 2000), &Limits::default()).is_ok());

    // Over 8 MB is refused, at most 8 MB decodes.
    assert!(probe(&padded_png(96, 96, BADGE_MAX_BYTES), &l).is_ok());
    assert!(matches!(
        probe(&padded_png(96, 96, BADGE_MAX_BYTES + 1), &l),
        Err(MediaError::TooLarge(_))
    ));
}

#[test]
fn paint_limits_follow_the_badge_animation_rules() {
    let l = Limits::paint();
    assert_eq!(l.max_bytes, 8 * 1024 * 1024);
    assert_eq!(l.max_frames, 120);
    assert_eq!(l.max_duration_ms, 10_000);
    assert_eq!(l.max_total_pixels, 64_000_000);
    // Wide pictures: 4096 px a side, so 512 tall can be 8:1.
    assert!(probe(&png(4096, 512), &l).is_ok());
    assert!(matches!(
        probe(&png(4097, 512), &l),
        Err(MediaError::TooLarge(_))
    ));
    assert!(probe(&gif(8, 8, 120), &l).is_ok());
    assert!(matches!(
        probe(&gif(8, 8, 121), &l),
        Err(MediaError::TooLarge(_))
    ));
    assert!(matches!(
        probe(&gif_with_delay(8, 8, 6, 2000), &l),
        Err(MediaError::TooLarge(_))
    ));
    assert!(probe(&padded_png(96, 96, PAINT_MAX_BYTES), &l).is_ok());
    assert!(matches!(
        probe(&padded_png(96, 96, PAINT_MAX_BYTES + 1), &l),
        Err(MediaError::TooLarge(_))
    ));
}

#[test]
fn a_set_cancel_flag_stops_a_transcode_at_once() {
    use std::sync::atomic::AtomicBool;
    let cancel = AtomicBool::new(true);
    let started = std::time::Instant::now();
    let result = encode::transcode_cancellable(
        &png(512, 512),
        &Limits::emote(),
        &encode::Profile::EMOTE,
        &cancel,
    );
    assert!(matches!(result, Err(MediaError::Cancelled)));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    let cancel = AtomicBool::new(false);
    assert!(
        encode::transcode_cancellable(
            &png(64, 64),
            &Limits::emote(),
            &encode::Profile::EMOTE,
            &cancel
        )
        .is_ok()
    );
}

mod config_env {
    use crate::Limits;
    use crate::config::{Ladder, MediaConfig};
    use crate::encode::Profile;

    fn env(pairs: &'static [(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn nothing_set_is_the_built_in_numbers() {
        let c = MediaConfig::from_env(|_| None).unwrap();
        assert_eq!(c, MediaConfig::builtin());
        assert_eq!(c.badge.max_side, 2048);
        assert_eq!(c.paint.max_side, 4096);
        assert_eq!(c.ladder, Ladder::BUILTIN);
        // An empty value counts as unset.
        let blank = MediaConfig::from_env(env(&[("CYEX_TV_LIMIT_BADGE_MAX_BYTES", "  ")])).unwrap();
        assert_eq!(blank, c);
        // The compiled profiles hold the same ladder.
        let c: &'static MediaConfig = Box::leak(Box::new(c));
        assert_eq!(c.badge_profile(), Profile::BADGE);
        assert_eq!(c.paint_profile(), Profile::PAINT);
        assert_eq!(*Profile::badge(), Profile::BADGE);
        assert_eq!(*Profile::paint(), Profile::PAINT);
    }

    #[test]
    fn each_kind_reads_its_own_variables() {
        let c = MediaConfig::from_env(env(&[
            ("CYEX_TV_LIMIT_BADGE_MAX_SIDE", "4096"),
            ("CYEX_TV_LIMIT_BADGE_MAX_TOTAL_PIXELS", "128000000"),
            ("CYEX_TV_LIMIT_BADGE_MAX_DURATION_MS", "none"),
            ("CYEX_TV_LIMIT_PAINT_MAX_BYTES", " 16777216 "),
            ("CYEX_TV_LIMIT_DEFAULT_MAX_BYTES", "1048576"),
        ]))
        .unwrap();
        assert_eq!(c.badge.max_side, 4096);
        assert_eq!(c.badge.max_total_pixels, 128_000_000);
        assert_eq!(c.badge.max_duration_ms, u64::MAX);
        assert_eq!(c.badge.max_bytes, 8 * 1024 * 1024);
        assert_eq!(c.paint.max_bytes, 16 * 1024 * 1024);
        assert_eq!(c.default.max_bytes, 1024 * 1024);
        assert_eq!(c.max_upload_bytes(), 16 * 1024 * 1024);
    }

    #[test]
    fn the_ladder_reads_its_variables_and_the_profiles_follow() {
        let c = MediaConfig::from_env(env(&[
            ("CYEX_TV_LIMIT_BADGE_MASTER_PX", "1024"),
            ("CYEX_TV_LIMIT_BADGE_CHAT_ANIMATED_BYTES", "262144"),
            ("CYEX_TV_LIMIT_BADGE_256_ANIMATED_BYTES", "3145728"),
            ("CYEX_TV_LIMIT_BADGE_ORIGINAL_PX", "2048"),
            ("CYEX_TV_LIMIT_BADGE_ORIGINAL_ANIMATED_BYTES", "1048576"),
            ("CYEX_TV_LIMIT_PAINT_512_ANIMATED_BYTES", "2097152"),
        ]))
        .unwrap();
        assert_eq!(c.ladder.badge_master, 1024);
        assert_eq!(c.ladder.badge_chat_animated_bytes, 256 * 1024);
        assert_eq!(c.ladder.badge_large[0].max_animated_bytes, 1024 * 1024);
        assert_eq!(c.ladder.badge_large[1].max_animated_bytes, 3 * 1024 * 1024);
        assert_eq!(c.ladder.badge_original.px, 2048);
        assert_eq!(c.ladder.paint_large[0].max_animated_bytes, 2 * 1024 * 1024);
        // A profile built from it: a 2000 px badge is mastered at 1024 and its original is 2000.
        let c: &'static MediaConfig = Box::leak(Box::new(c));
        let badge = c.badge_profile();
        assert_eq!(badge.master_size(2000, 2000), Some((1024, 1024)));
        let steps = badge.steps(2000, 2000);
        let original = steps.iter().find(|s| s.stem == "original").unwrap();
        assert_eq!(original.size, (2000, 2000));
        assert_eq!(original.max_animated_bytes, Some(1024 * 1024));
        let large = steps.iter().find(|s| s.stem == "256").unwrap();
        assert_eq!(large.max_animated_bytes, Some(3 * 1024 * 1024));
        assert_eq!(
            steps
                .iter()
                .find(|s| s.stem == "1x")
                .unwrap()
                .max_animated_bytes,
            Some(256 * 1024)
        );
    }

    #[test]
    fn a_value_that_does_not_read_is_an_error_naming_the_variable() {
        for (name, value) in [
            ("CYEX_TV_LIMIT_BADGE_MAX_BYTES", "eight"),
            ("CYEX_TV_LIMIT_BADGE_MAX_BYTES", "0"),
            ("CYEX_TV_LIMIT_BADGE_MAX_BYTES", "-5"),
            ("CYEX_TV_LIMIT_BADGE_MAX_FRAMES", "1.5"),
            ("CYEX_TV_LIMIT_BADGE_MAX_DURATION_MS", "soon"),
            ("CYEX_TV_LIMIT_BADGE_MAX_DURATION_MS", "0"),
            ("CYEX_TV_LIMIT_BADGE_MASTER_PX", "8"),
            ("CYEX_TV_LIMIT_BADGE_512_ANIMATED_BYTES", "lots"),
            ("CYEX_TV_LIMIT_PAINT_MAX_SIDE", "99999999"),
        ] {
            let err = MediaConfig::from_env(|n| (n == name).then(|| value.to_owned())).unwrap_err();
            assert!(err.starts_with(name), "{err}");
        }
    }

    #[test]
    fn emote_variables_are_not_read_any_more() {
        // Emote rules are fixed (ADR 0031): these change nothing, and don't fail either.
        let c = MediaConfig::from_env(env(&[
            ("CYEX_TV_LIMIT_EMOTE_MAX_SIDE", "16"),
            ("CYEX_TV_LIMIT_EMOTE_MAX_BYTES", "1"),
            ("CYEX_TV_LIMIT_ROLE_EMOTE_MAX_FRAMES", "3"),
        ]))
        .unwrap();
        assert_eq!(c, MediaConfig::builtin());
        assert_eq!(Limits::emote().max_side, 1000);
    }

    #[test]
    fn the_process_wide_config_is_the_built_in_numbers_without_variables() {
        // The test environment sets none of the variables.
        assert_eq!(*MediaConfig::current(), MediaConfig::builtin());
        assert_eq!(Limits::emote(), crate::EmoteRules::limits());
        assert_eq!(Limits::badge(), Limits::builtin_badge());
        assert_eq!(Limits::paint(), Limits::builtin_paint());
        assert_eq!(Limits::default(), Limits::builtin_default());
    }
}

mod role_overrides {
    use super::{gif, png};
    use crate::config::{MediaConfig, MediaKind, Overrides};
    use crate::{Limits, MediaError, decoder_limits, probe};

    #[test]
    fn no_override_is_the_configured_limits() {
        let c = MediaConfig::builtin();
        for (kind, l) in [
            (MediaKind::Badge, c.badge),
            (MediaKind::Paint, c.paint),
        ] {
            assert_eq!(c.effective(kind, Overrides::default()), l);
        }
    }

    #[test]
    fn a_badge_or_paint_may_go_above_the_configured_numbers_up_to_the_ceiling() {
        let c = MediaConfig::builtin();
        let big = c.effective(
            MediaKind::Badge,
            Overrides {
                max_bytes: Some(12 * 1024 * 1024),
                max_side: Some(4096),
                max_frames: Some(200),
            },
        );
        assert_eq!(big.max_bytes, 12 * 1024 * 1024);
        assert_eq!((big.max_side, big.max_frames), (4096, 200));
        // The rest stays as configured.
        assert_eq!(big.max_total_pixels, c.badge.max_total_pixels);
        assert_eq!(big.max_duration_ms, c.badge.max_duration_ms);
        // Past the ceiling is cut to it.
        let cut = c.effective(
            MediaKind::Badge,
            Overrides {
                max_bytes: Some(usize::MAX),
                max_side: Some(u32::MAX),
                max_frames: Some(u32::MAX),
            },
        );
        let ceiling = c.ceiling(MediaKind::Badge);
        assert_eq!(
            (cut.max_bytes, cut.max_side, cut.max_frames),
            (ceiling.max_bytes, ceiling.max_side, ceiling.max_frames)
        );
        assert_eq!(ceiling.max_bytes, 16 * 1024 * 1024);
        assert_eq!(c.ceiling(MediaKind::Paint).max_side, 6144);
        // Lowering works too, and never to nothing.
        let low = c.effective(
            MediaKind::Paint,
            Overrides {
                max_bytes: Some(0),
                max_side: Some(256),
                ..Overrides::default()
            },
        );
        assert_eq!((low.max_bytes, low.max_side), (1, 256));
    }

    #[test]
    fn the_ceilings_read_their_variables_and_never_sit_below_the_configured_numbers() {
        let c = MediaConfig::from_env(|n| {
            match n {
                "CYEX_TV_LIMIT_ROLE_BADGE_MAX_SIDE" => Some("3000"),
                "CYEX_TV_LIMIT_ROLE_BADGE_MAX_BYTES" => Some("4194304"),
                "CYEX_TV_LIMIT_BADGE_MAX_BYTES" => Some("6291456"),
                "CYEX_TV_LIMIT_ROLE_PAINT_MAX_FRAMES" => Some("300"),
                _ => None,
            }
            .map(str::to_owned)
        })
        .unwrap();
        let b = c.ceiling(MediaKind::Badge);
        assert_eq!(b.max_side, 3000);
        // The configured 6 MB is above the 4 MB ceiling asked for: the ceiling follows it.
        assert_eq!(b.max_bytes, 6 * 1024 * 1024);
        assert_eq!(c.ceiling(MediaKind::Paint).max_frames, 300);
        assert!(
            MediaConfig::from_env(
                |n| (n == "CYEX_TV_LIMIT_ROLE_BADGE_MAX_SIDE").then(|| "0".into())
            )
            .is_err()
        );
        // The upload body is cut at the biggest ceiling.
        assert_eq!(MediaConfig::builtin().max_upload_bytes(), 16 * 1024 * 1024);
    }

    #[test]
    fn the_decoder_allocation_grows_with_the_side_and_stays_bounded() {
        let c = MediaConfig::builtin();
        let alloc = |l: &Limits| decoder_limits(l).max_alloc.unwrap();
        // What it was for every kind: 64 MB.
        assert_eq!(alloc(&Limits::emote()), 64 * 1024 * 1024);
        assert_eq!(alloc(&c.badge), 64 * 1024 * 1024);
        let badge = c.effective(
            MediaKind::Badge,
            Overrides {
                max_side: Some(4096),
                ..Overrides::default()
            },
        );
        assert_eq!(alloc(&badge), 4096 * 4096 * 8);
        // The biggest anything can be: bounded by the ceilings and the pixel cap.
        let biggest = [MediaKind::Badge, MediaKind::Paint]
            .into_iter()
            .map(|k| {
                let l = c.effective(
                    k,
                    Overrides {
                        max_side: Some(u32::MAX),
                        ..Overrides::default()
                    },
                );
                alloc(&l)
            })
            .max()
            .unwrap();
        assert!(biggest <= 6144 * 6144 * 8, "{biggest}");
    }

    #[test]
    fn a_raised_side_lets_a_bigger_picture_through_and_the_default_still_refuses_it() {
        let c = MediaConfig::builtin();
        let file = png(2100, 2100);
        assert!(matches!(
            probe(&file, &c.badge),
            Err(MediaError::TooLarge(_))
        ));
        let raised = c.effective(
            MediaKind::Badge,
            Overrides {
                max_side: Some(3000),
                ..Overrides::default()
            },
        );
        assert_eq!(probe(&file, &raised).unwrap().width, 2100);
        // And lowered: a small picture is refused where the default takes it.
        let small = c.effective(
            MediaKind::Badge,
            Overrides {
                max_side: Some(64),
                ..Overrides::default()
            },
        );
        assert!(probe(&png(96, 96), &c.badge).is_ok());
        assert!(matches!(
            probe(&png(96, 96), &small),
            Err(MediaError::TooLarge(_))
        ));
        let few = c.effective(
            MediaKind::Badge,
            Overrides {
                max_frames: Some(2),
                ..Overrides::default()
            },
        );
        assert!(matches!(
            probe(&gif(8, 8, 3), &few),
            Err(MediaError::TooLarge(_))
        ));
    }
}
