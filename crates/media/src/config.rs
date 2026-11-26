//! The media limits and the size ladder, read once from the environment (`CYEX_TV_LIMIT_*`).
//!
//! The API and the worker both call [`MediaConfig::current`], so they read the same variables and
//! agree on every number. An unset or empty variable keeps the built-in default; one that doesn't
//! read is an error naming it ([`MediaConfig::init_from_env`], called when a process starts).
//!
//! Per kind (`DEFAULT` for cosmetic pictures in general, `EMOTE`, `BADGE`, `PAINT`):
//! `CYEX_TV_LIMIT_<KIND>_MAX_BYTES`, `_MAX_SIDE`, `_MAX_FRAMES`, `_MAX_TOTAL_PIXELS` and
//! `_MAX_DURATION_MS` (a number of ms, or `none` for no cap). The ladder:
//! `CYEX_TV_LIMIT_BADGE_MASTER_PX`, `_BADGE_CHAT_ANIMATED_BYTES`, `_BADGE_128_ANIMATED_BYTES`,
//! `_BADGE_256_ANIMATED_BYTES`, `_BADGE_512_ANIMATED_BYTES`, `_BADGE_ORIGINAL_PX`,
//! `_BADGE_ORIGINAL_ANIMATED_BYTES` and `_PAINT_512_ANIMATED_BYTES`.
//! An emote's side can't be set above [`EMOTE_SIDE_CEILING`] (1000 px, the owner's rule).

use std::sync::OnceLock;

use crate::Limits;
use crate::encode::{BADGE_MASTER, BADGE_MAX_ANIMATED_CHAT_BYTES, Large, Profile};

/// The prefix of every variable.
pub const ENV_PREFIX: &str = "CYEX_TV_LIMIT_";
/// An emote is never wider or taller than this, whatever the variables say.
pub const EMOTE_SIDE_CEILING: u32 = 1000;

const MAX_BYTES_CEILING: u64 = 256 * 1024 * 1024;
const MAX_SIDE_CEILING: u64 = 16_384;
const MAX_FRAMES_CEILING: u64 = 100_000;
const MAX_PIXELS_CEILING: u64 = 100_000_000_000;

/// The sizes past the chat sizes, and the caps on animated files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ladder {
    /// Every badge size is scaled from a master this many px square.
    pub badge_master: u32,
    /// An animated 18, 36 or 72 px badge file over this many bytes fails the badge.
    pub badge_chat_animated_bytes: usize,
    pub badge_large: [Large; 3],
    /// The badge `original` is fitted into this many px.
    pub badge_original: Large,
    pub paint_large: [Large; 1],
}

impl Ladder {
    pub const BUILTIN: Self = Self {
        badge_master: BADGE_MASTER,
        badge_chat_animated_bytes: BADGE_MAX_ANIMATED_CHAT_BYTES,
        badge_large: [
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
        ],
        badge_original: Large {
            px: 512,
            max_animated_bytes: 8 * 1024 * 1024,
        },
        paint_large: [Large {
            px: 512,
            max_animated_bytes: 4 * 1024 * 1024,
        }],
    };
}

/// Every limit a process runs with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaConfig {
    pub default: Limits,
    pub emote: Limits,
    pub badge: Limits,
    pub paint: Limits,
    pub ladder: Ladder,
}

static CURRENT: OnceLock<MediaConfig> = OnceLock::new();

fn whole(name: &str, raw: &str, min: u64, max: u64) -> Result<u64, String> {
    raw.parse::<u64>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("{name}: {raw:?} isn't a whole number from {min} to {max}"))
}

impl MediaConfig {
    /// The built-in numbers.
    pub fn builtin() -> Self {
        Self {
            default: Limits::builtin_default(),
            emote: Limits::builtin_emote(),
            badge: Limits::builtin_badge(),
            paint: Limits::builtin_paint(),
            ladder: Ladder::BUILTIN,
        }
    }

    /// The numbers this process runs with: read from the environment on first use. A variable
    /// that doesn't read falls back to the built-in numbers here (and is printed); a process
    /// checks them at start with [`Self::init_from_env`], so that doesn't happen in production.
    pub fn current() -> &'static Self {
        CURRENT.get_or_init(|| {
            Self::from_env(|n| std::env::var(n).ok()).unwrap_or_else(|err| {
                eprintln!("media limits: {err}; using the built-in numbers");
                Self::builtin()
            })
        })
    }

    /// Reads the environment now, failing on a variable that doesn't read. Call once at start.
    pub fn init_from_env() -> Result<&'static Self, String> {
        if let Some(c) = CURRENT.get() {
            return Ok(c);
        }
        let config = Self::from_env(|n| std::env::var(n).ok())?;
        Ok(CURRENT.get_or_init(|| config))
    }

    /// The numbers with `get` as the environment.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut c = Self::builtin();
        let read = |suffix: &str| {
            let name = format!("{ENV_PREFIX}{suffix}");
            let value = get(&name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty());
            (name, value)
        };
        let number = |suffix: &str, min: u64, max: u64, to: &mut u64| -> Result<(), String> {
            if let (name, Some(raw)) = read(suffix) {
                *to = whole(&name, &raw, min, max)?;
            }
            Ok(())
        };
        for (kind, limits) in [
            ("DEFAULT", &mut c.default),
            ("EMOTE", &mut c.emote),
            ("BADGE", &mut c.badge),
            ("PAINT", &mut c.paint),
        ] {
            let side_max = if kind == "EMOTE" {
                u64::from(EMOTE_SIDE_CEILING)
            } else {
                MAX_SIDE_CEILING
            };
            let mut bytes = limits.max_bytes as u64;
            let mut side = u64::from(limits.max_side);
            let mut frames = u64::from(limits.max_frames);
            let mut pixels = limits.max_total_pixels;
            number(
                &format!("{kind}_MAX_BYTES"),
                1,
                MAX_BYTES_CEILING,
                &mut bytes,
            )?;
            number(&format!("{kind}_MAX_SIDE"), 1, side_max, &mut side)?;
            number(
                &format!("{kind}_MAX_FRAMES"),
                1,
                MAX_FRAMES_CEILING,
                &mut frames,
            )?;
            number(
                &format!("{kind}_MAX_TOTAL_PIXELS"),
                1,
                MAX_PIXELS_CEILING,
                &mut pixels,
            )?;
            if let (name, Some(raw)) = read(&format!("{kind}_MAX_DURATION_MS")) {
                limits.max_duration_ms = if raw.eq_ignore_ascii_case("none") {
                    u64::MAX
                } else {
                    raw.parse::<u64>().ok().filter(|n| *n >= 1).ok_or_else(|| {
                        format!("{name}: {raw:?} isn't a number of ms (1 or more) or `none`")
                    })?
                };
            }
            // Bounded above, so these fit.
            limits.max_bytes = usize::try_from(bytes).unwrap_or(usize::MAX);
            limits.max_side = u32::try_from(side).unwrap_or(u32::MAX);
            limits.max_frames = u32::try_from(frames).unwrap_or(u32::MAX);
            limits.max_total_pixels = pixels;
        }
        let px = |suffix: &str, to: &mut u32| -> Result<(), String> {
            let mut n = u64::from(*to);
            number(suffix, 72, MAX_SIDE_CEILING, &mut n)?;
            *to = u32::try_from(n).unwrap_or(u32::MAX);
            Ok(())
        };
        px("BADGE_MASTER_PX", &mut c.ladder.badge_master)?;
        px("BADGE_ORIGINAL_PX", &mut c.ladder.badge_original.px)?;
        let bytes = |suffix: &str, to: &mut usize| -> Result<(), String> {
            let mut n = *to as u64;
            number(suffix, 1, MAX_BYTES_CEILING, &mut n)?;
            *to = usize::try_from(n).unwrap_or(usize::MAX);
            Ok(())
        };
        bytes(
            "BADGE_CHAT_ANIMATED_BYTES",
            &mut c.ladder.badge_chat_animated_bytes,
        )?;
        for large in &mut c.ladder.badge_large {
            bytes(
                &format!("BADGE_{}_ANIMATED_BYTES", large.px),
                &mut large.max_animated_bytes,
            )?;
        }
        bytes(
            "BADGE_ORIGINAL_ANIMATED_BYTES",
            &mut c.ladder.badge_original.max_animated_bytes,
        )?;
        for large in &mut c.ladder.paint_large {
            bytes(
                &format!("PAINT_{}_ANIMATED_BYTES", large.px),
                &mut large.max_animated_bytes,
            )?;
        }
        Ok(c)
    }

    /// The biggest file any upload may be: what an upload request's body is cut at.
    pub fn max_upload_bytes(&self) -> usize {
        self.emote
            .max_bytes
            .max(self.badge.max_bytes)
            .max(self.paint.max_bytes)
    }

    /// The badge sizes this config makes.
    pub fn badge_profile(&'static self) -> Profile {
        Profile {
            large: &self.ladder.badge_large,
            master: Some(self.ladder.badge_master),
            max_animated_scale_bytes: Some(self.ladder.badge_chat_animated_bytes),
            original: Some(self.ladder.badge_original),
            ..Profile::BADGE
        }
    }

    /// The paint sizes this config makes.
    pub fn paint_profile(&'static self) -> Profile {
        Profile {
            large: &self.ladder.paint_large,
            ..Profile::PAINT
        }
    }
}
