//! The `limits` query and `GET /v1/limits`: every upload and field limit in one public answer.
//! The numbers are the ones the upload checks run with (`MediaConfig`, `Rates`, the constants of
//! each area), so the answer can't drift from what is enforced.

use cyex_tv_media::EmoteRules;
use cyex_tv_media::Limits as Media;
use cyex_tv_media::config::MediaConfig;
use cyex_tv_media::encode::EMOTE_MAX_HEIGHT;
use cyex_tv_shared::model::{
    EffectiveLimit, MediaLimits, NameLimits, PublicLimits, RateInfo, TagLimits, TextLimit,
};

use crate::cosmetics::{BADGE_MIN_SIDE, NAME_MAX, PAINT_MAX_ASPECT, PAINT_MIN_HEIGHT, TOOLTIP_MAX};
use crate::limits::{AccountLimit, AccountLimits, Rates};
use crate::moderation::{MAX_TAG_LEN, MAX_TAGS};
use crate::state::AppState;

fn int(n: impl TryInto<i32>) -> i32 {
    n.try_into().unwrap_or(i32::MAX)
}

fn wide(n: impl TryInto<i64>) -> i64 {
    n.try_into().unwrap_or(i64::MAX)
}

fn media(l: &Media, min_side: Option<u32>, max_aspect: Option<u32>, served: u32) -> MediaLimits {
    let max_fps = l.max_aspect.map(|_| EmoteRules::MAX_FPS); // only emotes have the rule
    MediaLimits {
        max_bytes: int(l.max_bytes),
        max_side: int(l.max_side),
        max_frames: int(l.max_frames),
        max_total_pixels: wide(l.max_total_pixels),
        max_duration_ms: (l.max_duration_ms != u64::MAX).then(|| wide(l.max_duration_ms)),
        min_side: min_side.map(int),
        max_aspect: max_aspect.map(int),
        max_fps: max_fps.map(int),
        max_served_side: int(served),
    }
}

fn text(min: usize, max: usize) -> TextLimit {
    TextLimit {
        min: int(min),
        max: int(max),
    }
}

/// The answer for `config`, `rates` and (signed in) the caller's account limits.
pub fn build(config: &MediaConfig, rates: &Rates, account: Option<&AccountLimits>) -> PublicLimits {
    let ladder = &config.ladder;
    let paint_served = ladder.paint_large.iter().map(|l| l.px).max().unwrap_or(0);
    PublicLimits {
        emote: media(
            &EmoteRules::limits(),
            None,
            Some(EmoteRules::MAX_ASPECT),
            EMOTE_MAX_HEIGHT,
        ),
        badge: media(
            &config.badge,
            Some(BADGE_MIN_SIDE),
            Some(1),
            ladder.badge_master.max(ladder.badge_original.px),
        ),
        paint: media(
            &config.paint,
            Some(PAINT_MIN_HEIGHT),
            Some(PAINT_MAX_ASPECT),
            paint_served,
        ),
        names: NameLimits {
            emote: text(1, 100),
            set: text(1, 100),
            badge: text(1, NAME_MAX),
            paint: text(1, NAME_MAX),
            tooltip: text(0, TOOLTIP_MAX),
        },
        tags: TagLimits {
            max: int(MAX_TAGS),
            max_len: int(MAX_TAG_LEN),
        },
        rates: rates
            .entries()
            .into_iter()
            .map(|(key, rate)| RateInfo {
                key: key.to_owned(),
                count: int(rate.burst),
                window_seconds: int(rate.per.as_secs()),
            })
            .collect(),
        account: account
            .map(|limits| {
                AccountLimit::ALL
                    .into_iter()
                    .map(|l| {
                        let (default, window) = rates.default_of(l);
                        EffectiveLimit {
                            key: l.key().to_owned(),
                            default: int(default),
                            effective: int(limits.get(l)),
                            window_seconds: window.map(|w| int(w.as_secs())),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// `limits`: the same for everyone, plus the caller's own per-account limits when signed in.
pub async fn load(state: &AppState, account: Option<uuid::Uuid>) -> PublicLimits {
    let own = match account {
        Some(id) => Some(state.limits.load(&state.db, id).await),
        None => None,
    };
    build(MediaConfig::current(), &state.limits.rates, own.as_deref())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::limits::RoleOverrides;

    #[test]
    fn the_answer_holds_every_kind_with_the_numbers_the_checks_use() {
        let config = MediaConfig::builtin();
        let l = build(&config, &Rates::default(), None);
        assert_eq!(l.emote.max_bytes, 8 * 1024 * 1024);
        assert_eq!(l.emote.max_side, 1000);
        assert_eq!(l.emote.max_frames, 1000);
        assert_eq!(l.emote.max_total_pixels, 1_000_000_000);
        assert_eq!(l.emote.max_duration_ms, None);
        assert_eq!((l.emote.max_aspect, l.emote.max_fps), (Some(4), Some(50)));
        assert_eq!((l.badge.max_aspect, l.badge.max_fps), (Some(1), None));
        assert_eq!(l.paint.max_fps, None);
        assert_eq!(l.emote.max_served_side, 128);
        assert_eq!(l.badge.max_side, 2048);
        assert_eq!(l.badge.min_side, Some(72));
        assert_eq!(l.badge.max_duration_ms, Some(10_000));
        assert_eq!(l.badge.max_served_side, 512);
        assert_eq!(l.paint.max_side, 4096);
        assert_eq!(l.paint.min_side, Some(64));
        assert_eq!(l.paint.max_aspect, Some(8));
        assert_eq!(l.names.emote, TextLimit { min: 1, max: 100 });
        assert_eq!(l.names.badge.max, 32);
        assert_eq!(
            l.tags,
            TagLimits {
                max: 10,
                max_len: 32
            }
        );
        assert!(l.account.is_empty());
        let reads = l.rates.iter().find(|r| r.key == "reads").unwrap();
        assert_eq!((reads.count, reads.window_seconds), (1200, 60));
        assert_eq!(l.rates.len(), 20);
    }

    #[test]
    fn it_follows_the_configured_numbers() {
        let config = MediaConfig::from_env(|n| {
            match n {
                "CYEX_TV_LIMIT_BADGE_MAX_BYTES" => Some("2097152"),
                "CYEX_TV_LIMIT_BADGE_MAX_SIDE" => Some("4096"),
                "CYEX_TV_LIMIT_BADGE_MASTER_PX" => Some("1024"),
                _ => None,
            }
            .map(str::to_owned)
        })
        .unwrap();
        let l = build(&config, &Rates::default(), None);
        assert_eq!(l.badge.max_bytes, 2 * 1024 * 1024);
        assert_eq!(l.badge.max_side, 4096);
        assert_eq!(l.badge.max_served_side, 1024);
        // The emote numbers are fixed: no variable moves them.
        assert_eq!(l.emote.max_side, 1000);
    }

    #[test]
    fn the_callers_role_overrides_show_as_effective_next_to_the_default() {
        let rates = Rates::default();
        let mut overrides: RoleOverrides = BTreeMap::new();
        overrides.insert(AccountLimit::PendingEmotes, 40);
        overrides.insert(AccountLimit::UploadsPerHour, 5);
        overrides.insert(AccountLimit::BadgeMaxSide, 4096);
        let account = AccountLimits::resolve(&rates, &[overrides]);
        let l = build(&MediaConfig::builtin(), &rates, Some(&account));
        assert_eq!(l.account.len(), AccountLimit::ALL.len());
        let find = |key: &str| l.account.iter().find(|e| e.key == key).unwrap();
        let pending = find("pending_emotes");
        assert_eq!((pending.default, pending.effective), (20, 40));
        assert_eq!(pending.window_seconds, None);
        let hour = find("uploads_per_hour");
        assert_eq!((hour.default, hour.effective), (20, 5));
        assert_eq!(hour.window_seconds, Some(3600));
        let prefs = find("prefs");
        assert_eq!(prefs.default, prefs.effective);
        // The media numbers show what the caller uploads under, next to the configured ones.
        let side = find("badge_max_side");
        assert_eq!((side.default, side.effective), (2048, 4096));
        assert_eq!(side.window_seconds, None);
        // Emote numbers aren't per-account limits: they are fixed and listed above.
        assert!(l.account.iter().all(|e| !e.key.starts_with("emote_max_")));
    }
}
