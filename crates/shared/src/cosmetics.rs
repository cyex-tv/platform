//! Chat cosmetics (ADR 0015): badges and name paints, in the API's shapes. Re-exported from
//! `model`. Separate from cyex deck supporter badges (ADR 0011), which cyex.tv never serves.

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::model::{AccountSummary, EmoteFile, Platform};

/// A badge or a paint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum CosmeticKind {
    Badge,
    Paint,
}

impl CosmeticKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Badge => "badge",
            Self::Paint => "paint",
        }
    }

    /// `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "badge" => Some(Self::Badge),
            "paint" => Some(Self::Paint),
            _ => None,
        }
    }
}

/// How a paint is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum PaintKind {
    /// `linear-gradient(<angle>deg, <stops>)`.
    LinearGradient,
    /// `radial-gradient(<shape>, <stops>)`.
    RadialGradient,
    /// The paint's image, drawn into the name's text.
    Image,
}

impl PaintKind {
    pub const ALL: [Self; 3] = [Self::LinearGradient, Self::RadialGradient, Self::Image];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LinearGradient => "linear_gradient",
            Self::RadialGradient => "radial_gradient",
            Self::Image => "image",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// The shape of a radial paint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum PaintShape {
    #[default]
    Circle,
    Ellipse,
}

impl PaintShape {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Circle => "circle",
            Self::Ellipse => "ellipse",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "circle" => Some(Self::Circle),
            "ellipse" => Some(Self::Ellipse),
            _ => None,
        }
    }
}

/// One colour stop of a gradient paint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject, async_graphql::InputObject)
)]
#[cfg_attr(feature = "graphql", graphql(input_name = "PaintStopInput"))]
pub struct PaintStop {
    /// Where along the gradient, 0 to 1. Each stop is at or after the one before.
    pub at: f64,
    /// `#RRGGBB` or `#RRGGBBAA`.
    pub color: String,
}

/// A drop shadow under a painted name (CSS `drop-shadow(x y radius color)`, in px).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject, async_graphql::InputObject)
)]
#[cfg_attr(feature = "graphql", graphql(input_name = "PaintShadowInput"))]
pub struct PaintShadow {
    /// −16 to 16.
    pub x: f64,
    /// −16 to 16.
    pub y: f64,
    /// Blur radius, 0 to 16.
    pub radius: f64,
    /// `#RRGGBB` or `#RRGGBBAA`.
    pub color: String,
}

/// A badge's or an image paint's picture, served at `{host}/{name}` like emote files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CosmeticImage {
    pub id: String,
    pub kind: CosmeticKind,
    /// For example `https://cdn.cyex.tv/cosmetics/badges/<sha256>`.
    pub host: String,
    /// WebP and AVIF at 1x, 2x and 4x, and static first frames for animations. Empty until the
    /// worker made them.
    pub files: Vec<EmoteFile>,
    pub animated: bool,
    /// The original's size in px.
    pub width: u32,
    pub height: u32,
    /// The files are made: only then can a badge or paint use it.
    pub ready: bool,
    pub created_at: String,
}

/// A chat badge, shown before a chatter's name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub tooltip: String,
    /// Square, 18 px at 1x.
    pub image: CosmeticImage,
    pub created_at: String,
    pub updated_at: String,
}

/// A name paint: a gradient or a picture drawn into the chatter's name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Paint {
    pub id: String,
    pub name: String,
    pub kind: PaintKind,
    /// Degrees, 0 to 360, CSS convention (0 points up, 90 to the right). Linear only.
    pub angle: u16,
    /// `repeating-*-gradient`, or a tiled image.
    pub repeat: bool,
    /// Radial only.
    pub shape: PaintShape,
    /// Shown where a client can't draw the paint.
    pub color: Option<String>,
    /// 2 to 10 for gradients; none for image paints.
    pub stops: Vec<PaintStop>,
    /// At most 4.
    pub shadows: Vec<PaintShadow>,
    /// Image paints only. 64 px tall at 1x.
    pub image: Option<CosmeticImage>,
    pub created_at: String,
    pub updated_at: String,
}

/// Every badge and paint, for clients to cache (`GET /v1/cosmetics`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CosmeticCatalog {
    pub badges: Vec<Badge>,
    pub paints: Vec<Paint>,
}

/// What one chatter shows, as IDs from the catalog. Only chatters showing something are listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct UserCosmetics {
    pub platform_user_id: String,
    pub badge_id: Option<String>,
    pub paint_id: Option<String>,
}

/// `GET /v1/cosmetics/users`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UserCosmeticsResult {
    pub users: Vec<UserCosmetics>,
}

/// The query of `GET /v1/cosmetics/users`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct UserCosmeticsQuery {
    /// `twitch` or `kick`.
    pub platform: Platform,
    /// Up to 100 platform user IDs, comma-separated.
    pub ids: String,
}

/// What one account shows, with the badge and paint themselves (`accountCosmetics`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountCosmetics {
    pub account_id: String,
    pub badge: Option<Badge>,
    pub paint: Option<Paint>,
}

/// An account's cosmetics: what it owns and what it shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct OwnedCosmetics {
    pub account: AccountSummary,
    pub badges: Vec<Badge>,
    pub paints: Vec<Paint>,
    /// The badge shown, one of `badges`.
    pub badge_id: Option<String>,
    /// The paint shown, one of `paints`.
    pub paint_id: Option<String>,
}

/// How an account came to own a cosmetic: given by staff, bought in the shop (ADR 0016), from an
/// event, or from a redeem code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum EntitlementSource {
    Grant,
    Purchase,
    Event,
    Code,
}

impl EntitlementSource {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "grant" => Some(Self::Grant),
            "purchase" => Some(Self::Purchase),
            "event" => Some(Self::Event),
            "code" => Some(Self::Code),
            _ => None,
        }
    }
}

/// One account owning one cosmetic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Entitlement {
    pub account: AccountSummary,
    pub kind: CosmeticKind,
    pub cosmetic_id: String,
    pub source: EntitlementSource,
    pub reason: Option<String>,
    pub granted_by: Option<AccountSummary>,
    pub granted_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EntitlementPage {
    pub items: Vec<Entitlement>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateBadgeInput"))]
pub struct CreateBadge {
    /// 1 to 32 characters, unique.
    pub name: String,
    /// 1 to 64 characters.
    pub tooltip: String,
    /// A ready badge image from `POST /v1/cosmetics/images`.
    pub image_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateBadgeInput"))]
pub struct UpdateBadge {
    pub name: Option<String>,
    pub tooltip: Option<String>,
    /// Another ready badge image.
    pub image_id: Option<String>,
}

/// A whole paint: `createPaint` makes one, `updatePaint` replaces every field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "PaintInput"))]
pub struct PaintInput {
    /// 1 to 32 characters, unique.
    pub name: String,
    pub kind: PaintKind,
    /// 0 to 360, default 90.
    pub angle: Option<u16>,
    /// Default false.
    pub repeat: Option<bool>,
    /// Default `CIRCLE`.
    pub shape: Option<PaintShape>,
    /// `#RRGGBB` or `#RRGGBBAA`.
    pub color: Option<String>,
    /// 2 to 10 for gradients, none for `IMAGE`.
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub stops: Vec<PaintStop>,
    /// At most 4.
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub shadows: Vec<PaintShadow>,
    /// A ready paint image; `IMAGE` paints only.
    pub image_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "GrantCosmeticInput"))]
pub struct GrantCosmetic {
    pub account_id: String,
    pub kind: CosmeticKind,
    pub cosmetic_id: String,
    /// Why, for the audit log. 1 to 500 characters.
    pub reason: String,
}

/// Multipart upload (REST only): a badge or paint picture.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadCosmeticImage {
    /// `badge` or `paint`.
    pub kind: String,
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for k in [CosmeticKind::Badge, CosmeticKind::Paint] {
            assert_eq!(CosmeticKind::parse(k.as_str()), Some(k));
            assert_eq!(
                serde_json::to_string(&k).unwrap(),
                format!("\"{}\"", k.as_str())
            );
        }
        for k in PaintKind::ALL {
            assert_eq!(PaintKind::parse(k.as_str()), Some(k));
            assert_eq!(
                serde_json::to_string(&k).unwrap(),
                format!("\"{}\"", k.as_str())
            );
        }
        for s in [PaintShape::Circle, PaintShape::Ellipse] {
            assert_eq!(PaintShape::parse(s.as_str()), Some(s));
        }
        assert_eq!(
            EntitlementSource::parse("grant"),
            Some(EntitlementSource::Grant)
        );
        assert_eq!(CosmeticKind::parse("Badge"), None);
        assert_eq!(PaintKind::parse("url"), None);
    }
}
