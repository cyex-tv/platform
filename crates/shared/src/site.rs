//! The site announcement, notification preferences and the public collection, in the API's
//! shapes. Re-exported from `model`.

use serde::{Deserialize, Serialize};

use crate::model::{Emote, GrantKind, GrantSource};

/// What an announcement is about: its icon and tone under the header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum AnnouncementKind {
    /// News, a tip.
    Info,
    /// Downtime or something not working.
    Maintenance,
    /// An event or a sale, often with an emote or a badge.
    Campaign,
}

impl AnnouncementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Maintenance => "maintenance",
            Self::Campaign => "campaign",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "maintenance" => Self::Maintenance,
            "campaign" => Self::Campaign,
            _ => Self::Info,
        }
    }
}

/// One line staff put under the header for everyone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Announcement {
    pub id: String,
    pub kind: AnnouncementKind,
    /// The line itself, 1 to 200 characters. Plain text.
    pub text: String,
    /// An `https://` address or a path on cyex.tv (`/shop`).
    pub link_url: Option<String>,
    /// The link's words; empty means "Learn more".
    pub link_label: Option<String>,
    /// An emote shown small beside the line, while it is approved and public.
    pub emote: Option<Emote>,
    /// A badge (from `cosmetics`) shown small beside the line.
    pub badge_id: Option<String>,
    /// Empty: from when it was made.
    pub starts_at: Option<String>,
    /// Empty: until staff end it.
    pub ends_at: Option<String>,
    /// When staff ended it early.
    pub ended_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// Shown right now.
    pub live: bool,
}

/// What `createAnnouncement` makes and `updateAnnouncement` replaces it with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "AnnouncementInput"))]
pub struct AnnouncementInput {
    pub kind: AnnouncementKind,
    /// 1 to 200 characters, one line.
    pub text: String,
    /// `https://…` or a path on cyex.tv starting with `/`, up to 500 characters.
    pub link_url: Option<String>,
    /// Up to 40 characters; only with `linkUrl`.
    pub link_label: Option<String>,
    /// An approved emote's ID.
    pub emote_id: Option<String>,
    /// A badge's ID.
    pub badge_id: Option<String>,
    /// RFC 3339.
    pub starts_at: Option<String>,
    /// RFC 3339, in the future and after `startsAt`.
    pub ends_at: Option<String>,
}

/// One kind of notification and what the account chose for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct NotificationPref {
    /// For example `emote.approved` (the same as `Notification.kind`).
    pub kind: String,
    /// Created at all. Off: nothing is stored for this kind.
    pub notify: bool,
    /// Pops up on the website when it arrives.
    pub toast: bool,
    /// Can't be turned off (a hand-over waiting for your answer, a takedown).
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "NotificationPrefInput"))]
pub struct NotificationPrefInput {
    pub kind: String,
    pub notify: bool,
    pub toast: bool,
}

/// One badge or paint in a public collection: what it is, since when and how it came. Never who
/// gave it, why, or what it cost.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CollectionItem {
    /// `BADGE` or `PAINT`.
    pub kind: GrantKind,
    /// The badge's or paint's ID in `cosmetics`.
    pub cosmetic_id: String,
    pub name: String,
    pub owned_since: String,
    /// `STAFF`, `PURCHASE` (the shop), `CODE` or `EVENT`.
    pub source: GrantSource,
}

/// The badges and paints an account owns, when it shows them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountCollection {
    pub account_id: String,
    /// Shown on the public profile. The owner sees their own collection either way.
    pub public: bool,
    /// Newest first.
    pub items: Vec<CollectionItem>,
}
