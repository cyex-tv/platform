//! The shapes of the public API (ADR 0008). They live in this crate so client code (the deck, a
//! future SDK) can reuse them. The GraphQL derives sit behind the `graphql` feature; REST and the
//! event WebSocket use the serde form (snake_case). This is the Phase 1 draft.
//!
//! IDs are ULIDs as text. Times are RFC 3339 strings in UTC.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// A streaming platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
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
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
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
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Me {
    pub id: String,
    pub identities: Vec<Identity>,
    /// Permission names from the account's roles (ADR 0010).
    pub permissions: Vec<String>,
    pub created_at: String,
}

/// A short reference to an account, for owners and editors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSummary {
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

// Channels

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateChannelSettingsInput"))]
pub struct UpdateChannelSettings {
    pub active_set_id: Option<String>,
    pub allow_personal_emotes: Option<bool>,
}

// Emotes

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum EmoteState {
    Pending,
    Approved,
    Rejected,
    TakenDown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmoteFlags {
    pub zero_width: bool,
    /// Only its owner can add it to sets.
    pub private: bool,
    pub nsfw: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Webp,
    Avif,
}

/// One file of an emote, served at `{host}/{name}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
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
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
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

/// Multipart upload (REST only): the image file and its name.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadEmote {
    pub name: String,
    /// Comma-separated.
    pub tags: Option<String>,
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateEmoteInput"))]
pub struct UpdateEmote {
    pub name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub zero_width: Option<bool>,
    pub private: Option<bool>,
    pub nsfw: Option<bool>,
}

// Sets

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum SetKind {
    Channel,
    Personal,
    Global,
}

/// An emote as it sits in a set: the name it is typed as there, and who added it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetEmote {
    /// The name in this set. Differs from `emote.name` when the set renamed it.
    pub name: String,
    pub zero_width: bool,
    pub added_by: Option<String>,
    pub added_at: String,
    pub emote: Emote,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmoteSet {
    pub id: String,
    pub name: String,
    pub kind: SetKind,
    pub owner: Option<AccountSummary>,
    pub capacity: u32,
    pub emotes: Vec<SetEmote>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateSetInput"))]
pub struct CreateSet {
    pub name: String,
    /// `CHANNEL` or `PERSONAL`. The global set exists already.
    pub kind: SetKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateSetInput"))]
pub struct UpdateSet {
    pub name: Option<String>,
    /// Needs `sets.edit_any` to raise above the default.
    pub capacity: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "AddSetEmoteInput"))]
pub struct AddSetEmote {
    /// Rename it in this set. Defaults to the emote's own name.
    pub name: Option<String>,
    pub zero_width: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateSetEmoteInput"))]
pub struct UpdateSetEmote {
    pub name: Option<String>,
    pub zero_width: Option<bool>,
}

// Personal sets

/// `POST /v1/personal-sets/lookup`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PersonalLookup {
    pub platform: Platform,
    /// Up to 100 platform user IDs, for example the people seen in chat.
    pub user_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SearchResults {
    pub items: Vec<Emote>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
}

// Moderation and roles

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ReviewQueue {
    pub items: Vec<Emote>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Higher means more senior. You can only manage roles below your own highest one.
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateRoleInput"))]
pub struct CreateRole {
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateRoleInput"))]
pub struct UpdateRole {
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i32>,
    pub permissions: Option<Vec<String>>,
}

// Live updates (ADR 0005)

/// What changed. On the event WebSocket these are written `set.emote_added` and so on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum EventType {
    #[serde(rename = "set.emote_added")]
    SetEmoteAdded,
    #[serde(rename = "set.emote_removed")]
    SetEmoteRemoved,
    #[serde(rename = "set.emote_renamed")]
    SetEmoteRenamed,
    #[serde(rename = "channel.active_set_changed")]
    ChannelActiveSetChanged,
    #[serde(rename = "personal.updated")]
    PersonalUpdated,
    #[serde(rename = "emote.updated")]
    EmoteUpdated,
    /// Too much was missed: refetch what you show.
    #[serde(rename = "resync")]
    Resync,
}

/// One change, as delivered on both sockets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Event {
    /// Increases with every event. Send the last one you saw to resume.
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// The topic it was delivered for, for example `channel:twitch:123`.
    pub topic: String,
    pub set_id: Option<String>,
    pub emote_id: Option<String>,
    /// The emote's name in the set, for added and renamed.
    pub name: Option<String>,
    pub created_at: String,
}

// REST query strings

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
        assert_eq!(
            serde_json::to_string(&EmoteState::TakenDown).unwrap(),
            "\"taken_down\""
        );
    }

    #[test]
    fn event_types_use_dotted_names() {
        assert_eq!(
            serde_json::to_string(&EventType::ChannelActiveSetChanged).unwrap(),
            "\"channel.active_set_changed\""
        );
        let event = Event {
            id: "1".to_owned(),
            event_type: EventType::SetEmoteAdded,
            topic: "set:abc".to_owned(),
            set_id: Some("abc".to_owned()),
            emote_id: Some("e1".to_owned()),
            name: Some("wave".to_owned()),
            created_at: "2026-11-16T12:00:00Z".to_owned(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "set.emote_added");
    }
}
