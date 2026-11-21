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
        // Six sizes, each as WebP, AVIF and their still forms.
        assert_eq!(b.all_names().len(), 24);
        assert_eq!(b.stems(), ["1x", "2x", "4x", "128", "256", "512"]);
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
                ("512.webp", 28, 512, 512)
            ]
        );
        let avif: Vec<_> = still.avif.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            avif,
            [
                "1x.avif", "2x.avif", "4x.avif", "128.avif", "256.avif", "512.avif"
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
                "1x_static.webp",
                "2x_static.webp",
                "4x_static.webp",
                "128_static.webp",
                "256_static.webp",
                "512_static.webp"
            ]
        );
        assert!(moving.avif.iter().all(|v| v.is_static));
        assert_eq!(moving.avif.len(), 6);
        let (animated, frames, dims) = webp_frames(&moving.webp[0].bytes);
        assert!(animated);
        assert_eq!((frames, dims), (3, (18, 18)));
        let (animated, frames, dims) = webp_frames(&moving.webp[5].bytes);
        assert!(animated);
        assert_eq!((frames, dims), (3, (512, 512)));
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
        assert!(b.is_complete(new));
        let animated = [
            "1x.webp",
            "2x.webp",
            "4x.webp",
            "128.webp",
            "256_static.webp",
            "512_static.webp",
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
fn emotes_may_be_up_to_8_mb() {
    assert_eq!(Limits::emote().max_bytes, 8 * 1024 * 1024);
    assert_eq!(Limits::default().max_bytes, 5 * 1024 * 1024);
    assert_eq!(Limits::emote().max_side, Limits::default().max_side);
    let six = padded_png(64, 64, 6 * 1024 * 1024);
    // Over the default (cosmetics), inside an emote's limit, and it decodes.
    assert!(matches!(
        probe(&six, &Limits::default()),
        Err(MediaError::TooLarge(_))
    ));
    let p = probe(&six, &Limits::emote()).unwrap();
    assert_eq!((p.format, p.width, p.height), (Format::Png, 64, 64));
    let exactly = padded_png(64, 64, EMOTE_MAX_BYTES);
    assert!(probe(&exactly, &Limits::emote()).is_ok());
    let over = padded_png(64, 64, EMOTE_MAX_BYTES + 1);
    let Err(MediaError::TooLarge(message)) = probe(&over, &Limits::emote()) else {
        panic!("an 8 MB + 1 byte file is refused");
    };
    assert_eq!(message, "The file is over 8 MB, the most an upload can be.");
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
