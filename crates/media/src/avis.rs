//! Animated AVIF (an AV1 image sequence, brand `avis`), ADR 0012 as amended.
//!
//! The frames go through `rav1e` (pure Rust, the encoder `ravif` already uses for still AVIF) as
//! one AV1 video, with a second monochrome video for the alpha plane. The container is written
//! here: ISOBMFF boxes per the AVIF and AV1-in-ISOBMFF specifications, with two tracks (colour,
//! and alpha as an auxiliary track) and, for readers that show only still images, a primary item
//! that is the first frame (the item points at the first sample's bytes, so they are stored once).
//!
//! Frames are encoded as they arrive (`add_frame`), so the whole animation is never held as
//! pixels; only the compressed samples are kept until `finish`.

use rav1e::prelude::{
    ChromaSamplePosition, ChromaSampling, ColorDescription, Config, Context, EncoderConfig,
    EncoderStatus, Frame, FrameType, MatrixCoefficients, PixelRange, Rational, SpeedSettings,
    TransferCharacteristics, Tune,
};

use crate::MediaError;

/// The URN that marks an auxiliary image or track as an alpha plane (AVIF, MIAF).
const ALPHA_URN: &[u8] = b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0";
/// Media time is in milliseconds, as frame delays are.
const TIMESCALE: u32 = 1000;
/// A duration that never ends (`mvhd`, `tkhd`): with the edit list's repeat flag, readers loop
/// the animation forever (how libavif marks an infinite repetition count).
const INDEFINITE: u64 = u64::MAX;

fn err(e: impl std::fmt::Debug) -> MediaError {
    MediaError::Encode(format!("animated avif: {e:?}"))
}

/// `quality` 0 to 100 to rav1e's quantizer (0 to 255), the curve `ravif` uses for still AVIF, so
/// the same quality number means the same in both.
pub(crate) fn quantizer(quality: f32) -> usize {
    let q = (quality / 100.0).clamp(0.0, 1.0);
    let x = if q >= 0.82 {
        (1.0 - q) * 2.6
    } else if q > 0.25 {
        q.mul_add(-0.5, 1.0 - 0.125)
    } else {
        1.0 - q
    };
    // At most 255.
    (x * 255.0).round().clamp(0.0, 255.0) as usize
}

/// One coded frame.
struct Sample {
    data: Vec<u8>,
    sync: bool,
}

/// One AV1 stream (colour or alpha) being encoded.
struct Stream {
    ctx: Context<u8>,
    /// The av1C box's four bytes.
    av1c: Vec<u8>,
    samples: Vec<Sample>,
    /// Keep the decoded luma of each frame in `recs` (the colour stream, for the quality check).
    keep_recs: bool,
    /// Decoded luma not yet compared with its source frame, in frame order.
    recs: Vec<Vec<u8>>,
}

impl Stream {
    fn new(
        width: u32,
        height: u32,
        quality: f32,
        speed: u8,
        chroma: ChromaSampling,
        keep_recs: bool,
    ) -> Result<Self, MediaError> {
        let quantizer = quantizer(quality);
        let mut speed_settings = SpeedSettings::from_preset(speed);
        // Short clips at chat sizes: a deep lookahead costs time and memory for little.
        speed_settings.rdo_lookahead_frames = speed_settings.rdo_lookahead_frames.min(10);
        let config = Config::new()
            .with_encoder_config(EncoderConfig {
                width: usize::try_from(width).map_err(err)?,
                height: usize::try_from(height).map_err(err)?,
                time_base: Rational::new(1, u64::from(TIMESCALE)),
                sample_aspect_ratio: Rational::new(1, 1),
                bit_depth: 8,
                chroma_sampling: chroma,
                chroma_sample_position: ChromaSamplePosition::Unknown,
                pixel_range: PixelRange::Full,
                color_description: Some(ColorDescription {
                    color_primaries: rav1e::prelude::ColorPrimaries::BT709,
                    transfer_characteristics: TransferCharacteristics::SRGB,
                    matrix_coefficients: if chroma == ChromaSampling::Cs400 {
                        MatrixCoefficients::Unspecified
                    } else {
                        MatrixCoefficients::BT601
                    },
                }),
                mastering_display: None,
                content_light: None,
                level_idx: None,
                enable_timing_info: false,
                still_picture: false,
                error_resilient: false,
                switch_frame_interval: 0,
                min_key_frame_interval: 12,
                max_key_frame_interval: 240,
                reservoir_frame_delay: None,
                low_latency: false,
                quantizer,
                min_quantizer: 0,
                bitrate: 0,
                tune: Tune::Psychovisual,
                film_grain_params: None,
                tile_cols: 0,
                tile_rows: 0,
                tiles: 0,
                speed_settings,
            })
            // One thread per stream: the worker runs several jobs at once.
            .with_threads(1);
        let ctx: Context<u8> = config.new_context().map_err(err)?;
        let av1c = ctx.container_sequence_header();
        Ok(Self {
            ctx,
            av1c,
            samples: Vec::new(),
            keep_recs,
            recs: Vec::new(),
        })
    }

    fn send(&mut self, frame: Frame<u8>) -> Result<(), MediaError> {
        self.ctx.send_frame(frame).map_err(err)?;
        self.drain()
    }

    fn drain(&mut self) -> Result<(), MediaError> {
        loop {
            match self.ctx.receive_packet() {
                Ok(packet) => {
                    // The frame as a decoder shows it: rav1e's own reconstruction.
                    if self.keep_recs {
                        let rec = packet
                            .rec
                            .as_ref()
                            .ok_or_else(|| err("no reconstruction"))?;
                        let p = &rec.planes[0];
                        let w = p.cfg.width;
                        let mut luma = Vec::with_capacity(w * p.cfg.height);
                        for row in p.rows_iter().take(p.cfg.height) {
                            luma.extend_from_slice(&row[..w]);
                        }
                        self.recs.push(luma);
                    }
                    // A sample carries no temporal delimiter (AV1 in ISOBMFF, 2.4).
                    let mut data = packet.data;
                    if data.starts_with(&[0x12, 0x00]) {
                        data.drain(..2);
                    }
                    self.samples.push(Sample {
                        data,
                        sync: packet.frame_type == FrameType::KEY,
                    });
                }
                Err(EncoderStatus::Encoded) => {}
                Err(EncoderStatus::NeedMoreData | EncoderStatus::LimitReached) => return Ok(()),
                Err(e) => return Err(err(e)),
            }
        }
    }

    fn flush(&mut self) -> Result<(), MediaError> {
        self.ctx.flush();
        self.drain()
    }
}

/// BT.601 full-range Y, Cb and Cr of one sRGB pixel (as `ravif` converts for still AVIF).
fn ycbcr(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let (r, g, b) = (f32::from(r), f32::from(g), f32::from(b));
    let y = 0.114f32.mul_add(b, 0.299f32.mul_add(r, 0.587 * g));
    let cb = (b - y).mul_add(0.5 / (1.0 - 0.114), 128.0);
    let cr = (r - y).mul_add(0.5 / (1.0 - 0.299), 128.0);
    let clamp = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    (clamp(y), clamp(cb), clamp(cr))
}

/// An animated AVIF being made, frame by frame.
pub struct AvisEncoder {
    width: u32,
    height: u32,
    color: Stream,
    alpha: Stream,
    /// Whether any frame has a pixel that isn't fully opaque (else the alpha track is left out).
    has_alpha: bool,
    durations: Vec<u32>,
    /// Width × height × frames sent, the work done so far.
    pixels: u64,
    /// Every source frame's luma and alpha, for the quality check.
    src_luma: Vec<u8>,
    src_alpha: Vec<u8>,
    /// Frames compared with their reconstruction so far, and their error.
    compared: usize,
    error: Error,
}

/// Squared luma error and the pixels counted (those not fully transparent).
#[derive(Clone, Copy, Debug, Default)]
struct Error {
    sum: f64,
    pixels: f64,
}

impl Error {
    fn add(&mut self, src: &[u8], alpha: &[u8], got: &[u8]) {
        for ((&s, &a), &g) in src.iter().zip(alpha).zip(got) {
            if a > 0 {
                let d = f64::from(s) - f64::from(g);
                self.sum += d * d;
                self.pixels += 1.0;
            }
        }
    }

    /// Luma PSNR in dB (99 when exact, or when nothing is visible).
    fn psnr(self) -> f64 {
        if self.sum == 0.0 || self.pixels == 0.0 {
            99.0
        } else {
            10.0 * (255.0f64 * 255.0 / (self.sum / self.pixels)).log10()
        }
    }
}

/// A finished animated AVIF and how close it is to its source.
#[derive(Clone, Debug)]
pub struct Finished {
    pub bytes: Vec<u8>,
    /// Luma PSNR against the source frames, in dB (pixels not fully transparent).
    pub psnr: f64,
    /// The same for the animated WebP it was checked against, if one was given and decoded.
    pub webp_psnr: Option<f64>,
    /// That WebP's size in bytes, if one was given.
    pub webp_bytes: Option<usize>,
}

impl Finished {
    /// No larger than the WebP and at least as close to the source: worth listing beside it.
    pub fn beats_webp(&self) -> bool {
        matches!(
            (self.webp_bytes, self.webp_psnr),
            (Some(bytes), Some(psnr)) if self.bytes.len() <= bytes && self.psnr >= psnr
        )
    }
}

impl AvisEncoder {
    /// An encoder for frames of `width`×`height`, at `quality` (0 to 100, as for still AVIF) and
    /// rav1e `speed` (0 to 10).
    pub fn new(width: u32, height: u32, quality: f32, speed: u8) -> Result<Self, MediaError> {
        Ok(Self {
            width,
            height,
            color: Stream::new(width, height, quality, speed, ChromaSampling::Cs420, true)?,
            alpha: Stream::new(width, height, quality, speed, ChromaSampling::Cs400, false)?,
            has_alpha: false,
            durations: Vec::new(),
            pixels: 0,
            src_luma: Vec::new(),
            src_alpha: Vec::new(),
            compared: 0,
            error: Error::default(),
        })
    }

    /// Width × height × frames encoded so far.
    pub fn pixels(&self) -> u64 {
        self.pixels
    }

    /// Adds a frame shown for `duration_ms` (at least 1).
    pub fn add_frame(
        &mut self,
        rgba: &image::RgbaImage,
        duration_ms: u32,
    ) -> Result<(), MediaError> {
        if rgba.dimensions() != (self.width, self.height) {
            return Err(err("frame size differs"));
        }
        let w = usize::try_from(self.width).map_err(err)?;
        let h = usize::try_from(self.height).map_err(err)?;
        let px = rgba.as_raw();
        let mut color = self.color.ctx.new_frame();
        let mut alpha = self.alpha.ctx.new_frame();
        {
            let [yp, up, vp] = &mut color.planes;
            let mut y_rows = yp.mut_slice(Default::default());
            for (row, out) in y_rows.rows_iter_mut().take(h).enumerate() {
                for (col, o) in out[..w].iter_mut().enumerate() {
                    let i = (row * w + col) * 4;
                    *o = ycbcr(px[i], px[i + 1], px[i + 2]).0;
                    self.src_luma.push(*o);
                    self.src_alpha.push(px[i + 3]);
                }
            }
            // 4:2:0: each chroma sample is the mean of up to 2×2 pixels.
            let cw = w.div_ceil(2);
            let ch = h.div_ceil(2);
            let mut u_rows = up.mut_slice(Default::default());
            let mut v_rows = vp.mut_slice(Default::default());
            for (row, (u_out, v_out)) in u_rows
                .rows_iter_mut()
                .zip(v_rows.rows_iter_mut())
                .take(ch)
                .enumerate()
            {
                for col in 0..cw {
                    let (mut cb, mut cr, mut n) = (0u32, 0u32, 0u32);
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let (x, y) = (col * 2 + dx, row * 2 + dy);
                            if x < w && y < h {
                                let i = (y * w + x) * 4;
                                let (_, b, r) = ycbcr(px[i], px[i + 1], px[i + 2]);
                                cb += u32::from(b);
                                cr += u32::from(r);
                                n += 1;
                            }
                        }
                    }
                    // The mean of at most four u8 values is a u8.
                    u_out[col] = ((cb + n / 2) / n) as u8;
                    v_out[col] = ((cr + n / 2) / n) as u8;
                }
            }
        }
        {
            let mut a_rows = alpha.planes[0].mut_slice(Default::default());
            for (row, out) in a_rows.rows_iter_mut().take(h).enumerate() {
                for (col, o) in out[..w].iter_mut().enumerate() {
                    let a = px[(row * w + col) * 4 + 3];
                    self.has_alpha |= a != 255;
                    *o = a;
                }
            }
        }
        self.durations.push(duration_ms.max(1));
        self.color.send(color)?;
        self.alpha.send(alpha)?;
        self.compare()?;
        self.pixels += u64::from(self.width) * u64::from(self.height);
        Ok(())
    }

    fn frame_px(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// Compares the frames rav1e has given back with their sources.
    fn compare(&mut self) -> Result<(), MediaError> {
        let n = self.frame_px();
        for rec in std::mem::take(&mut self.color.recs) {
            let at = self.compared * n;
            let (Some(src), Some(alpha)) = (
                self.src_luma.get(at..at + n),
                self.src_alpha.get(at..at + n),
            ) else {
                return Err(err("a frame came back that wasn't sent"));
            };
            if rec.len() != n {
                return Err(err("a frame came back at another size"));
            }
            self.error.add(src, alpha, &rec);
            self.compared += 1;
        }
        Ok(())
    }

    /// The luma error of an animated WebP of the same frames (`None` if it doesn't decode to
    /// them). libwebp folds a frame identical to the one before into it, so frames are matched
    /// by time.
    fn webp_error(&self, webp: &[u8]) -> Option<Error> {
        use image::AnimationDecoder;
        let n = self.frame_px();
        let decoder = image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(webp)).ok()?;
        let mut starts = Vec::with_capacity(self.durations.len());
        let mut t = 0u64;
        for &d in &self.durations {
            starts.push(t);
            t += u64::from(d);
        }
        let mut error = Error::default();
        let (mut at, mut next) = (0u64, 0usize);
        for frame in decoder.into_frames() {
            let frame = frame.ok()?;
            let (num, den) = frame.delay().numer_denom_ms();
            let end = at + u64::from(num) / u64::from(den.max(1));
            let luma: Vec<u8> = frame
                .buffer()
                .pixels()
                .map(|p| ycbcr(p[0], p[1], p[2]).0)
                .collect();
            if luma.len() != n {
                return None;
            }
            // Every source frame that starts while this one shows.
            while starts.get(next).is_some_and(|&start| start < end) {
                let i = next * n;
                error.add(&self.src_luma[i..i + n], &self.src_alpha[i..i + n], &luma);
                next += 1;
            }
            at = end;
        }
        (next == starts.len()).then_some(error)
    }

    /// Flushes the encoders and writes the file; with `webp`, also measures that animated WebP
    /// against the same source frames, for [`Finished::beats_webp`].
    pub fn finish(mut self, webp: Option<&[u8]>) -> Result<Finished, MediaError> {
        self.color.flush()?;
        self.alpha.flush()?;
        self.compare()?;
        if self.compared != self.durations.len() {
            return Err(err("not every frame came back"));
        }
        let webp_psnr = webp.and_then(|w| self.webp_error(w)).map(Error::psnr);
        let psnr = self.error.psnr();
        let (color_c, color) = (self.color.av1c, self.color.samples);
        let (alpha_c, alpha) = (self.alpha.av1c, self.alpha.samples);
        if color.len() != self.durations.len() || alpha.len() != self.durations.len() {
            return Err(err("the encoder gave a different number of frames"));
        }
        if color.first().is_none_or(|s| !s.sync) {
            return Err(err("the first frame isn't a key frame"));
        }
        let alpha = self.has_alpha.then_some(Track {
            av1c: alpha_c,
            samples: alpha,
        });
        let bytes = write(
            self.width,
            self.height,
            &self.durations,
            &Track {
                av1c: color_c,
                samples: color,
            },
            alpha.as_ref(),
        );
        Ok(Finished {
            bytes,
            psnr,
            webp_psnr,
            webp_bytes: webp.map(<[u8]>::len),
        })
    }
}

struct Track {
    av1c: Vec<u8>,
    samples: Vec<Sample>,
}

/// An ISOBMFF box writer: `bx` opens a box and closes it (fixing its size) after `f`.
struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    fn bx(&mut self, kind: &[u8; 4], f: impl FnOnce(&mut Self)) {
        let start = self.0.len();
        self.u32(0);
        self.bytes(kind);
        f(self);
        let size = u32::try_from(self.0.len() - start).unwrap_or(u32::MAX);
        self.0[start..start + 4].copy_from_slice(&size.to_be_bytes());
    }
    /// A full box: version and flags first.
    fn full(&mut self, kind: &[u8; 4], version: u8, flags: u32, f: impl FnOnce(&mut Self)) {
        self.bx(kind, |w| {
            w.u32((u32::from(version) << 24) | (flags & 0x00ff_ffff));
            f(w);
        });
    }
}

/// The identity matrix of `mvhd` and `tkhd`.
const MATRIX: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// `colr` (nclx): sRGB, BT.601 for colour (or unspecified for alpha), full range.
fn colr(w: &mut W, alpha: bool) {
    w.bx(b"colr", |w| {
        w.bytes(b"nclx");
        w.u16(1); // BT.709 primaries
        w.u16(13); // sRGB transfer
        w.u16(if alpha { 2 } else { 6 }); // BT.601 matrix, unspecified for alpha
        w.u8(0x80); // full range
    });
}

/// The file: `ftyp`, `meta` (the first frame as the primary item), `moov`, then `mdat`.
fn write(
    width: u32,
    height: u32,
    durations: &[u32],
    color: &Track,
    alpha: Option<&Track>,
) -> Vec<u8> {
    // Offsets point into `mdat`, which comes after `meta` and `moov`: lay out once to learn
    // where it starts (no field's size depends on the offsets), then write it for real.
    let header = |base: u32| -> Vec<u8> {
        let mut w = W(Vec::new());
        w.bx(b"ftyp", |w| {
            w.bytes(b"avis");
            w.u32(0);
            for brand in [
                b"avif", b"avis", b"msf1", b"iso8", b"mif1", b"miaf", b"MA1B",
            ] {
                w.bytes(brand);
            }
        });
        meta(&mut w, width, height, base, color, alpha);
        moov(&mut w, width, height, base, durations, color, alpha);
        w.0
    };
    let base = len32(header(0).len() + 8);
    let data_len: usize = color
        .samples
        .iter()
        .chain(alpha.map_or(&[][..], |a| &a.samples[..]))
        .map(|s| s.data.len())
        .sum();
    let mut w = W(header(base));
    w.u32(len32(data_len + 8));
    w.bytes(b"mdat");
    for s in &color.samples {
        w.bytes(&s.data);
    }
    if let Some(a) = alpha {
        for s in &a.samples {
            w.bytes(&s.data);
        }
    }
    w.0
}

/// Where each track's samples start in the file: colour first, then alpha.
fn offsets(base: u32, track: &Track) -> Vec<u32> {
    let mut at = base;
    track
        .samples
        .iter()
        .map(|s| {
            let here = at;
            at = at.saturating_add(len32(s.data.len()));
            here
        })
        .collect()
}

fn alpha_base(base: u32, color: &Track) -> u32 {
    base.saturating_add(len32(color.samples.iter().map(|s| s.data.len()).sum()))
}

fn meta(w: &mut W, width: u32, height: u32, base: u32, color: &Track, alpha: Option<&Track>) {
    w.full(b"meta", 0, 0, |w| {
        w.full(b"hdlr", 0, 0, |w| {
            w.u32(0);
            w.bytes(b"pict");
            w.bytes(&[0; 12]);
            w.u8(0);
        });
        w.full(b"pitm", 0, 0, |w| w.u16(1));
        // Version 0, 4-byte offsets and lengths, no base offset.
        w.full(b"iloc", 0, 0, |w| {
            w.u8(0x44);
            w.u8(0x00);
            let items: Vec<(u16, u32, usize)> =
                std::iter::once((1, base, color.samples[0].data.len()))
                    .chain(alpha.map(|a| (2, alpha_base(base, color), a.samples[0].data.len())))
                    .collect();
            w.u16(u16::try_from(items.len()).unwrap_or(1));
            for (id, offset, len) in items {
                w.u16(id);
                w.u16(0); // data reference: this file
                w.u16(1); // one extent
                w.u32(offset);
                w.u32(len32(len));
            }
        });
        w.full(b"iinf", 0, 0, |w| {
            w.u16(if alpha.is_some() { 2 } else { 1 });
            for id in 1..=if alpha.is_some() { 2u16 } else { 1 } {
                w.full(b"infe", 2, 0, |w| {
                    w.u16(id);
                    w.u16(0);
                    w.bytes(b"av01");
                    w.u8(0); // no name
                });
            }
        });
        if alpha.is_some() {
            w.full(b"iref", 0, 0, |w| {
                w.bx(b"auxl", |w| {
                    w.u16(2);
                    w.u16(1);
                    w.u16(1);
                });
            });
        }
        w.bx(b"iprp", |w| {
            w.bx(b"ipco", |w| {
                // 1: ispe, 2: colour av1C, 3: colour pixi, 4: colr
                w.full(b"ispe", 0, 0, |w| {
                    w.u32(width);
                    w.u32(height);
                });
                w.bx(b"av1C", |w| w.bytes(&color.av1c));
                w.full(b"pixi", 0, 0, |w| w.bytes(&[3, 8, 8, 8]));
                colr(w, false);
                if let Some(a) = alpha {
                    // 5: alpha av1C, 6: alpha pixi, 7: auxC
                    w.bx(b"av1C", |w| w.bytes(&a.av1c));
                    w.full(b"pixi", 0, 0, |w| w.bytes(&[1, 8]));
                    w.full(b"auxC", 0, 0, |w| w.bytes(ALPHA_URN));
                }
            });
            w.full(b"ipma", 0, 0, |w| {
                w.u32(if alpha.is_some() { 2 } else { 1 });
                // Essential properties have the top bit set (av1C is).
                w.u16(1);
                w.u8(4);
                w.bytes(&[0x01, 0x82, 0x03, 0x04]);
                if alpha.is_some() {
                    w.u16(2);
                    w.u8(4);
                    w.bytes(&[0x01, 0x85, 0x06, 0x87]);
                }
            });
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn moov(
    w: &mut W,
    width: u32,
    height: u32,
    base: u32,
    durations: &[u32],
    color: &Track,
    alpha: Option<&Track>,
) {
    let total: u32 = durations.iter().fold(0u32, |a, &d| a.saturating_add(d));
    w.bx(b"moov", |w| {
        // Version 1 (64-bit times): readers take an all-ones duration as indefinite only there.
        w.full(b"mvhd", 1, 0, |w| {
            w.u64(0);
            w.u64(0);
            w.u32(TIMESCALE);
            w.u64(INDEFINITE);
            w.u32(0x0001_0000); // rate 1.0
            w.u16(0x0100); // volume 1.0
            w.u16(0);
            w.u32(0);
            w.u32(0);
            for m in MATRIX {
                w.u32(m);
            }
            w.bytes(&[0; 24]);
            w.u32(if alpha.is_some() { 3 } else { 2 }); // next track id
        });
        trak(
            w,
            1,
            width,
            height,
            total,
            durations,
            color,
            offsets(base, color),
            None,
        );
        if let Some(a) = alpha {
            let offs = offsets(alpha_base(base, color), a);
            trak(w, 2, width, height, total, durations, a, offs, Some(1));
        }
    });
}

/// One track. `aux_of`: the colour track an alpha track belongs to.
#[allow(clippy::too_many_arguments)]
fn trak(
    w: &mut W,
    id: u32,
    width: u32,
    height: u32,
    total: u32,
    durations: &[u32],
    track: &Track,
    offsets: Vec<u32>,
    aux_of: Option<u32>,
) {
    w.bx(b"trak", |w| {
        // Enabled and in the movie (flags 1 | 2); the alpha track isn't shown on its own.
        w.full(b"tkhd", 1, if aux_of.is_some() { 2 } else { 3 }, |w| {
            w.u64(0);
            w.u64(0);
            w.u32(id);
            w.u32(0);
            // An indefinite duration with the repeating edit list below: loop forever.
            w.u64(INDEFINITE);
            w.bytes(&[0; 8]);
            w.u16(0); // layer
            w.u16(0); // alternate group
            w.u16(0); // volume
            w.u16(0);
            for m in MATRIX {
                w.u32(m);
            }
            w.u32(width << 16);
            w.u32(height << 16);
        });
        if let Some(of) = aux_of {
            w.bx(b"tref", |w| w.bx(b"auxl", |w| w.u32(of)));
        }
        // Loop: one edit of the whole animation with the repeat flag set.
        w.bx(b"edts", |w| {
            w.full(b"elst", 0, 1, |w| {
                w.u32(1);
                w.u32(total);
                w.u32(0);
                w.u16(1);
                w.u16(0);
            });
        });
        w.bx(b"mdia", |w| {
            w.full(b"mdhd", 0, 0, |w| {
                w.u32(0);
                w.u32(0);
                w.u32(TIMESCALE);
                w.u32(total);
                w.u16(0x55c4); // language "und"
                w.u16(0);
            });
            w.full(b"hdlr", 0, 0, |w| {
                w.u32(0);
                w.bytes(if aux_of.is_some() { b"auxv" } else { b"pict" });
                w.bytes(&[0; 12]);
                w.u8(0);
            });
            w.bx(b"minf", |w| {
                w.full(b"vmhd", 0, 1, |w| w.bytes(&[0; 8]));
                w.bx(b"dinf", |w| {
                    w.full(b"dref", 0, 0, |w| {
                        w.u32(1);
                        w.full(b"url ", 0, 1, |_| {});
                    });
                });
                w.bx(b"stbl", |w| {
                    w.full(b"stsd", 0, 0, |w| {
                        w.u32(1);
                        w.bx(b"av01", |w| {
                            w.bytes(&[0; 6]);
                            w.u16(1); // data reference
                            w.bytes(&[0; 16]);
                            w.u16(u16::try_from(width).unwrap_or(u16::MAX));
                            w.u16(u16::try_from(height).unwrap_or(u16::MAX));
                            w.u32(0x0048_0000); // 72 dpi
                            w.u32(0x0048_0000);
                            w.u32(0);
                            w.u16(1); // frame count
                            w.bytes(&[0; 32]); // compressor name
                            w.u16(0x0018); // depth
                            w.u16(0xffff);
                            w.bx(b"av1C", |w| w.bytes(&track.av1c));
                            colr(w, aux_of.is_some());
                            if aux_of.is_some() {
                                w.full(b"auxi", 0, 0, |w| w.bytes(ALPHA_URN));
                            }
                        });
                    });
                    // Durations, run-length coded.
                    let mut runs: Vec<(u32, u32)> = Vec::new();
                    for &d in durations {
                        match runs.last_mut() {
                            Some((n, last)) if *last == d => *n += 1,
                            _ => runs.push((1, d)),
                        }
                    }
                    w.full(b"stts", 0, 0, |w| {
                        w.u32(len32(runs.len()));
                        for (n, d) in runs {
                            w.u32(n);
                            w.u32(d);
                        }
                    });
                    let sync: Vec<u32> = (1u32..)
                        .zip(&track.samples)
                        .filter(|(_, s)| s.sync)
                        .map(|(i, _)| i)
                        .collect();
                    if sync.len() < track.samples.len() {
                        w.full(b"stss", 0, 0, |w| {
                            w.u32(len32(sync.len()));
                            for i in sync {
                                w.u32(i);
                            }
                        });
                    }
                    // One chunk per sample: simple, and the offsets are already known.
                    w.full(b"stsc", 0, 0, |w| {
                        w.u32(1);
                        w.u32(1);
                        w.u32(1);
                        w.u32(1);
                    });
                    w.full(b"stsz", 0, 0, |w| {
                        w.u32(0);
                        w.u32(len32(track.samples.len()));
                        for s in &track.samples {
                            w.u32(len32(s.data.len()));
                        }
                    });
                    w.full(b"stco", 0, 0, |w| {
                        w.u32(len32(offsets.len()));
                        for o in offsets {
                            w.u32(o);
                        }
                    });
                });
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// A cartoon-like frame: a flat disc with an outline moving over a transparent ground.
    pub(crate) fn cartoon(w: u32, h: u32, i: u32, frames: u32) -> RgbaImage {
        let t = i as f32 / frames.max(1) as f32;
        let cx = w as f32 * (0.3 + 0.4 * (t * std::f32::consts::TAU).sin().abs());
        let cy = h as f32 * 0.5;
        let r = w.min(h) as f32 * 0.3;
        RgbaImage::from_fn(w, h, |x, y| {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d < r - 3.0 {
                Rgba([240, 190, 40, 255])
            } else if d < r {
                Rgba([30, 20, 10, 255])
            } else if d < r + 1.0 {
                Rgba([30, 20, 10, 128])
            } else {
                Rgba([0, 0, 0, 0])
            }
        })
    }

    /// The boxes at the top level of a file, and inside the box at `path`.
    fn boxes(data: &[u8]) -> Vec<(String, &[u8])> {
        let mut out = Vec::new();
        let mut at = 0;
        while at + 8 <= data.len() {
            let size = u32::from_be_bytes(data[at..at + 4].try_into().unwrap()) as usize;
            assert!(
                size >= 8 && at + size <= data.len(),
                "a box runs past its parent"
            );
            let kind = String::from_utf8_lossy(&data[at + 4..at + 8]).into_owned();
            out.push((kind, &data[at + 8..at + size]));
            at += size;
        }
        assert_eq!(at, data.len(), "trailing bytes");
        out
    }

    fn find<'a>(data: &'a [u8], kind: &str) -> &'a [u8] {
        boxes(data)
            .into_iter()
            .find(|(k, _)| k == kind)
            .unwrap_or_else(|| panic!("no {kind}"))
            .1
    }

    fn encode(frames: u32, alpha: bool) -> Vec<u8> {
        let mut enc = AvisEncoder::new(48, 32, 80.0, 10).unwrap();
        for i in 0..frames {
            let mut f = cartoon(48, 32, i, frames);
            if !alpha {
                for p in f.pixels_mut() {
                    p[3] = 255;
                }
            }
            enc.add_frame(&f, 40 + i % 2 * 20).unwrap();
        }
        enc.finish(None).unwrap().bytes
    }

    #[test]
    fn an_animated_avif_has_the_avis_layout_with_every_frame() {
        let file = encode(7, true);
        let top: Vec<String> = boxes(&file).into_iter().map(|(k, _)| k).collect();
        assert_eq!(top, ["ftyp", "meta", "moov", "mdat"]);
        assert_eq!(&find(&file, "ftyp")[..4], b"avis");
        let moov = find(&file, "moov");
        let traks: Vec<_> = boxes(moov)
            .into_iter()
            .filter(|(k, _)| k == "trak")
            .collect();
        assert_eq!(traks.len(), 2, "colour and alpha tracks");
        // Every sample is where stco and stsz say, inside mdat.
        let mdat_start = file.len() - find(&file, "mdat").len();
        for (_, trak) in traks {
            let stbl = find(find(find(trak, "mdia"), "minf"), "stbl");
            let stsz = find(stbl, "stsz");
            let count = u32::from_be_bytes(stsz[8..12].try_into().unwrap());
            assert_eq!(count, 7);
            let stco = find(stbl, "stco");
            let mut end = 0;
            for i in 0..count as usize {
                let off =
                    u32::from_be_bytes(stco[8 + i * 4..12 + i * 4].try_into().unwrap()) as usize;
                let len =
                    u32::from_be_bytes(stsz[12 + i * 4..16 + i * 4].try_into().unwrap()) as usize;
                assert!(off >= mdat_start && off + len <= file.len());
                // An AV1 sample starts with an OBU header, not a temporal delimiter.
                assert_ne!(file[off], 0x12);
                end = end.max(off + len);
            }
            assert!(end <= file.len());
            // Durations add up: 4 × 40 + 3 × 60 ms.
            let stts = find(stbl, "stts");
            let runs = u32::from_be_bytes(stts[4..8].try_into().unwrap()) as usize;
            let total: u32 = (0..runs)
                .map(|r| {
                    let n = u32::from_be_bytes(stts[8 + r * 8..12 + r * 8].try_into().unwrap());
                    let d = u32::from_be_bytes(stts[12 + r * 8..16 + r * 8].try_into().unwrap());
                    n * d
                })
                .sum();
            assert_eq!(total, 4 * 40 + 3 * 60);
        }
        // The primary item is the first colour sample.
        let meta = &find(&file, "meta")[4..];
        assert!(boxes(meta).iter().any(|(k, _)| k == "pitm"));
        assert!(
            boxes(meta).iter().any(|(k, _)| k == "iref"),
            "the alpha item refers to the colour one"
        );
    }

    #[test]
    fn an_opaque_animation_has_no_alpha_track() {
        let file = encode(3, false);
        let moov = find(&file, "moov");
        assert_eq!(boxes(moov).iter().filter(|(k, _)| k == "trak").count(), 1);
        let meta = &find(&file, "meta")[4..];
        assert!(!boxes(meta).iter().any(|(k, _)| k == "iref"));
    }

    #[test]
    fn the_quality_curve_matches_ravif() {
        // ravif's quality 80 and 90 quantizers.
        assert_eq!(quantizer(80.0), 121);
        assert_eq!(quantizer(100.0), 0);
        assert_eq!(quantizer(0.0), 255);
    }

    fn noisy(w: u32, h: u32, i: u32) -> RgbaImage {
        let mut seed = 0x9e37_79b9u32 ^ i.wrapping_mul(7919);
        RgbaImage::from_fn(w, h, |x, y| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let n = (seed & 0x3f) as u8;
            Rgba([(x * 4) as u8 ^ n, (y * 4) as u8, (i * 9) as u8 ^ n, 255])
        })
    }

    fn webp_of(frames: &[RgbaImage], delay: i32) -> Vec<u8> {
        use webp_animation::{
            Encoder, EncoderOptions, EncodingConfig, EncodingType, LossyEncodingConfig,
        };
        let mut enc = Encoder::new_with_options(
            frames[0].dimensions(),
            EncoderOptions {
                encoding_config: Some(EncodingConfig {
                    quality: 90.0,
                    method: 4,
                    encoding_type: EncodingType::Lossy(LossyEncodingConfig::default()),
                }),
                ..EncoderOptions::default()
            },
        )
        .unwrap();
        let mut t = 0;
        for f in frames {
            enc.add_frame(f.as_raw(), t).unwrap();
            t += delay;
        }
        enc.finalize(t).unwrap().to_vec()
    }

    const QS: [(f32, u8); 1] = [(96.0, 9)];

    /// Animated AVIF against animated WebP (the pipeline's settings) on the test pictures.
    /// `cargo test --release -p cyex-tv-media --lib measure_animated_avif -- --ignored --nocapture`;
    /// with `CYEX_TV_AVIS_OUT=<dir>` the files are written there with a page that shows them.
    #[test]
    #[ignore = "measurement, run by hand"]
    fn measure_animated_avif() {
        let out = std::env::var_os("CYEX_TV_AVIS_OUT").map(std::path::PathBuf::from);
        let mut html = String::from("<!doctype html><body style=background:#888>");
        type Gen = fn(u32, u32, u32, u32) -> RgbaImage;
        let pics: [(&str, Gen); 3] = [
            ("cartoon", cartoon),
            ("gradient", |w, h, i, _| {
                RgbaImage::from_fn(w, h, |x, y| {
                    Rgba([
                        (x * 255 / w) as u8,
                        (y * 255 / h) as u8,
                        (i * 40) as u8,
                        255,
                    ])
                })
            }),
            ("noisy", |w, h, i, _| noisy(w, h, i)),
        ];
        for (name, make) in pics {
            for (w, h, n) in [
                (32u32, 32u32, 30u32),
                (128, 128, 30),
                (128, 128, 120),
                (512, 128, 60),
                (512, 512, 60),
            ] {
                let frames: Vec<RgbaImage> = (0..n).map(|i| make(w, h, i, n)).collect();
                let t = std::time::Instant::now();
                let webp = webp_of(&frames, 50);
                let webp_ms = t.elapsed().as_millis();
                for (quality, speed) in QS {
                    let t = std::time::Instant::now();
                    let mut enc = AvisEncoder::new(w, h, quality, speed).unwrap();
                    for f in &frames {
                        enc.add_frame(f, 50).unwrap();
                    }
                    let done = enc.finish(Some(&webp)).unwrap();
                    let avif_ms = t.elapsed().as_millis();
                    let (avif, avif_psnr) = (done.bytes, done.psnr);
                    let webp_psnr = done.webp_psnr.unwrap();
                    println!(
                        "{name:9} {w}x{h} {n:3} frames | webp {:7} B {webp_ms:5} ms {webp_psnr:5.1} dB | avif q{quality} s{speed:2} {:7} B {avif_ms:5} ms {avif_psnr:5.1} dB | {:5.1}%",
                        webp.len(),
                        avif.len(),
                        avif.len() as f64 * 100.0 / webp.len() as f64
                    );
                    if let Some(dir) = &out {
                        let stem = format!("{name}-{w}x{h}-{n}-s{speed}");
                        std::fs::write(dir.join(format!("{stem}.avif")), &avif).unwrap();
                        std::fs::write(dir.join(format!("{stem}.webp")), &webp).unwrap();
                        html.push_str(&format!(
                            "<p>{stem}<br><img src={stem}.webp> <img src={stem}.avif></p>"
                        ));
                    }
                }
            }
        }
        if let Some(dir) = &out {
            std::fs::write(dir.join("index.html"), html).unwrap();
        }
    }
}
