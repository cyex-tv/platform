//! The `limits` query: every upload and field limit in one public list, in the API's shapes.
//! Re-exported from `model`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// What one kind of picture (emote, badge, paint) may be. Sizes are px, the file size is bytes.
/// Emote numbers are fixed rules, the same for everyone; badge and paint numbers are the
/// configured ones (a role may raise them).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct MediaLimits {
    /// The most bytes an uploaded file may be.
    pub max_bytes: i32,
    /// The longest side of an upload.
    pub max_side: i32,
    /// The most frames an animation may have.
    pub max_frames: i32,
    /// Width x height x frames, all frames together: the cap on decoding work.
    pub max_total_pixels: i64,
    /// The longest one run of an animation may last, in ms; null when there is no cap.
    pub max_duration_ms: Option<i64>,
    /// The smallest side an upload may have (badges: both sides; paints: the height); null for none.
    pub min_side: Option<i32>,
    /// How many times as wide as tall a picture may be (emotes 4, badges 1, paints 8); null for
    /// no rule at upload. Emotes have no rule for a tall picture.
    pub max_aspect: Option<i32>,
    /// The most frames a second (emotes 50; null for the others). Frames shown for under
    /// 1000 / `maxFps` ms (20 ms: a 0 or 10 ms GIF delay) are shown for 100 ms, as browsers do.
    pub max_fps: Option<i32>,
    /// The tallest file the CDN serves for this kind (emotes 128 px, badges the master size).
    pub max_served_side: i32,
}

/// A text field's length in characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct TextLimit {
    pub min: i32,
    pub max: i32,
}

/// The lengths of the names and notes clients type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct NameLimits {
    /// An emote's name in a set (no spaces).
    pub emote: TextLimit,
    pub set: TextLimit,
    pub badge: TextLimit,
    pub paint: TextLimit,
    /// A badge's tooltip.
    pub tooltip: TextLimit,
}

/// Tags on an emote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct TagLimits {
    /// The most tags one emote has.
    pub max: i32,
    /// The longest tag, in characters.
    pub max_len: i32,
}

/// A count of requests or writes per window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct RateInfo {
    /// For example `gql`, `reads`, `upload`: the name of `CYEX_TV_LIMIT_<NAME>`.
    pub key: String,
    /// Requests allowed per window (the burst).
    pub count: i32,
    pub window_seconds: i32,
}

/// One of the signed-in caller's per-account limits, with the default and what applies to them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EffectiveLimit {
    /// `uploads_per_hour`, `pending_emotes`, `set_capacity_max` and so on (see `RoleLimit`).
    pub key: String,
    /// What everyone gets.
    pub default: i32,
    /// What the caller gets: the most generous override among their roles, else the default.
    pub effective: i32,
    /// The window the count is per, in seconds; null for a cap on how many exist at once.
    pub window_seconds: Option<i32>,
}

/// Every limit, as `limits` answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PublicLimits {
    pub emote: MediaLimits,
    pub badge: MediaLimits,
    pub paint: MediaLimits,
    pub names: NameLimits,
    pub tags: TagLimits,
    /// The request rates, by the name of their `CYEX_TV_LIMIT_*` variable: per IP for the route
    /// classes (`auth`, `gql`, `reads`, `upload`, `socket`), per account (or per IP signed out)
    /// for the account limits, and per developer API key for `api_key` (ADR 0033).
    pub rates: Vec<RateInfo>,
    /// The caller's per-account limits with their roles' overrides applied. Empty when signed out
    /// and on `GET /v1/limits`.
    pub account: Vec<EffectiveLimit>,
}
