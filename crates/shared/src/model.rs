//! The shapes of the public API (ADR 0008). They live in this crate so client code (the deck, a
//! future SDK) can reuse them. This is the Phase 1 draft; later phases fill in what is missing.
//!
//! IDs are ULIDs as text. Times are RFC 3339 strings in UTC.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// A streaming platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    Kick,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Twitch => "twitch",
            Self::Kick => "kick",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown platform: {0}")]
pub struct UnknownPlatform(pub String);

impl FromStr for Platform {
    type Err = UnknownPlatform;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "twitch" => Ok(Self::Twitch),
            "kick" => Ok(Self::Kick),
            other => Err(UnknownPlatform(other.to_owned())),
        }
    }
}

/// `GET /health` and `GET /health/ready`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Health {
    pub status: String,
}

// Accounts

/// A Twitch or Kick account linked to a cyex.tv account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Identity {
    pub platform: Platform,
    /// The platform's numeric user ID, as text.
    pub platform_user_id: String,
    pub login: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

/// The signed-in account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Me {
    pub id: String,
    pub identities: Vec<Identity>,
    /// Permission names from the account's roles (ADR 0010).
    pub permissions: Vec<String>,
    pub created_at: String,
}

/// A short reference to an account, for owners and editors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AccountSummary {
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

// Channels

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Channel {
    pub platform: Platform,
    /// Twitch: the broadcaster's user ID. Kick: `broadcaster_user_id`.
    pub platform_channel_id: String,
    pub login: String,
    pub account: AccountSummary,
    pub active_set_id: Option<String>,
    /// Whether other people's personal emotes show in this channel.
    pub allow_personal_emotes: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateChannelSettings {
    pub active_set_id: Option<String>,
    pub allow_personal_emotes: Option<bool>,
}

// Emotes

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmoteState {
    Pending,
    Approved,
    Rejected,
    TakenDown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EmoteFlags {
    pub zero_width: bool,
    /// Only its owner can add it to sets.
    pub private: bool,
    pub nsfw: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Webp,
    Avif,
}

/// One file of an emote, served at `{host}/{name}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EmoteFile {
    /// For example `2x.webp` or `1x_static.avif`.
    pub name: String,
    pub format: ImageFormat,
    /// 1 to 4.
    pub scale: u8,
    pub width: u16,
    pub height: u16,
    /// Bytes.
    pub size: u32,
    /// The first frame of an animated emote.
    pub is_static: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Emote {
    pub id: String,
    pub name: String,
    pub owner: Option<AccountSummary>,
    pub state: EmoteState,
    pub flags: EmoteFlags,
    pub animated: bool,
    pub tags: Vec<String>,
    /// For example `https://cdn.cyex.tv/emotes/<sha256>`.
    pub host: String,
    pub files: Vec<EmoteFile>,
    pub created_at: String,
}

/// Multipart upload: the image file and its name.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadEmote {
    pub name: String,
    /// Comma-separated.
    pub tags: Option<String>,
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateEmote {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub zero_width: Option<bool>,
    pub private: Option<bool>,
    pub nsfw: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RejectEmote {
    pub reason: String,
}

// Sets

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SetKind {
    Channel,
    Personal,
    Global,
}

/// An emote as it sits in a set: the name it is typed as there, and who added it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetEmote {
    /// The name in this set. Differs from `emote.name` when the set renamed it.
    pub name: String,
    pub zero_width: bool,
    pub added_by: Option<String>,
    pub added_at: String,
    pub emote: Emote,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EmoteSet {
    pub id: String,
    pub name: String,
    pub kind: SetKind,
    pub owner: Option<AccountSummary>,
    pub capacity: u32,
    pub emotes: Vec<SetEmote>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateSet {
    pub name: String,
    /// `channel` or `personal`. The global set exists already.
    pub kind: SetKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateSet {
    pub name: Option<String>,
    /// Needs `sets.edit_any` to raise above the default.
    pub capacity: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AddSetEmote {
    /// Rename it in this set. Defaults to the emote's own name.
    pub name: Option<String>,
    pub zero_width: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateSetEmote {
    pub name: Option<String>,
    pub zero_width: Option<bool>,
}

// Personal sets

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PersonalLookup {
    pub platform: Platform,
    /// Up to 100 platform user IDs, for example the people seen in chat.
    pub user_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PersonalSetEntry {
    pub platform_user_id: String,
    pub set: EmoteSet,
}

/// Only users with a non-empty personal set are listed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PersonalLookupResult {
    pub sets: Vec<PersonalSetEntry>,
}

// Search

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    /// Name or tag to look for.
    pub q: String,
    pub animated: Option<bool>,
    /// Only emotes owned by this account ID.
    pub owner: Option<String>,
    /// From 1.
    pub page: Option<u32>,
    /// 1 to 100, default 50.
    pub per_page: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchResults {
    pub items: Vec<Emote>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

// Moderation and roles

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueue {
    pub items: Vec<Emote>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Higher means more senior. You can only manage roles below your own highest one.
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateRole {
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateRole {
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i32>,
    pub permissions: Option<Vec<String>>,
}

// Auth and events

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LoginQuery {
    /// Where to send the browser afterwards. Must be a cyex.tv URL.
    pub return_to: Option<String>,
    /// Link this platform to the signed-in account instead of signing in.
    pub link: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventsQuery {
    /// Repeatable, up to 100: `channel:twitch:<id>`, `channel:kick:<id>` or `set:<id>`.
    pub sub: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_round_trips() {
        for p in [Platform::Twitch, Platform::Kick] {
            assert_eq!(p.as_str().parse::<Platform>().unwrap(), p);
            let json = serde_json::to_string(&p).unwrap();
            assert_eq!(json, format!("\"{p}\""));
        }
        assert!("youtube".parse::<Platform>().is_err());
        assert!("Twitch".parse::<Platform>().is_err());
    }

    #[test]
    fn emote_state_uses_snake_case() {
        assert_eq!(serde_json::to_string(&EmoteState::TakenDown).unwrap(), "\"taken_down\"");
    }
}
