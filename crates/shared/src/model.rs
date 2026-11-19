//! The shapes of the public API (ADR 0008). They live in this crate so client code (the deck, a
//! future SDK) can reuse them. The GraphQL derives sit behind the `graphql` feature; REST and the
//! event WebSocket use the serde form (snake_case). This is the Phase 1 draft.
//!
//! IDs are ULIDs as text. Times are RFC 3339 strings in UTC.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub use crate::cosmetics::*;
pub use crate::shop::*;

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

/// How a session signed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[cfg_attr(feature = "graphql", graphql(name = "SessionKind"))]
#[serde(rename_all = "lowercase")]
pub enum SessionKindView {
    /// The cyex.tv website's cookie.
    Site,
    /// A Bearer token for an app (cyex deck, the extension).
    App,
}

/// One place you are signed in. Never holds the token or anything made from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSession {
    /// Pass to `revokeSession`.
    pub id: String,
    pub kind: SessionKindView,
    /// For example `Firefox on Windows`, `cyex.tv app` or `Unknown device`, from the user agent
    /// that signed in.
    pub device: String,
    pub created_at: String,
    /// Updated at most once an hour.
    pub last_seen_at: String,
    pub expires_at: String,
    /// The session this request came with.
    pub current: bool,
}

/// A short reference to an account, for owners and editors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSummary {
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

// Account editors (ADR 0014)

/// What an account editor may do on the account they edit. The account's owner has all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum EditorPermission {
    /// Add, remove, rename and alias emotes in the owner's sets, the active channel sets too.
    Emotes,
    /// Create, rename and delete sets, change their capacity, and change the owner's channels'
    /// active set and settings.
    Sets,
    /// Add, change and remove editors, never granting more than you hold.
    Editors,
    /// Pick the badge and paint the owner shows (ADR 0015). Never grants.
    Profile,
}

impl EditorPermission {
    pub const ALL: [Self; 4] = [Self::Emotes, Self::Sets, Self::Editors, Self::Profile];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Emotes => "emotes",
            Self::Sets => "sets",
            Self::Editors => "editors",
            Self::Profile => "profile",
        }
    }

    /// `None` for a name this version doesn't know.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == s)
    }
}

/// Someone who edits an account, with what they may do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountEditor {
    pub account: AccountSummary,
    pub permissions: Vec<EditorPermission>,
    /// Who added them: the owner, or an editor with `editors`.
    pub granted_by: Option<AccountSummary>,
    pub granted_at: String,
    pub updated_at: String,
}

/// An account you edit, with what you may do there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EditedAccount {
    pub account: AccountSummary,
    pub permissions: Vec<EditorPermission>,
    pub granted_at: String,
}

/// Who to add as an editor: an account ID, or a Twitch or Kick login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "AddAccountEditorInput"))]
pub struct AddAccountEditor {
    /// The account to add editors to. Empty: your own.
    #[serde(default)]
    pub owner_id: Option<String>,
    /// The editor's cyex.tv account ID. Or give `platform` and `login`.
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub platform: Option<Platform>,
    #[serde(default)]
    pub login: Option<String>,
    /// At least one.
    pub permissions: Vec<EditorPermission>,
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
    /// One of the owner's channel sets. An empty string clears it.
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

impl EmoteState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::TakenDown => "taken_down",
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown emote state: {0}")]
pub struct UnknownEmoteState(pub String);

impl FromStr for EmoteState {
    type Err = UnknownEmoteState;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "taken_down" => Ok(Self::TakenDown),
            other => Err(UnknownEmoteState(other.to_owned())),
        }
    }
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
    /// How many channels have this emote in their active set right now.
    #[serde(default)]
    #[schema(required = false)]
    pub channel_count: u32,
}

/// Multipart upload (REST only): the image file, its name, and optionally its tags and flags.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadEmote {
    pub name: String,
    /// Comma-separated.
    pub tags: Option<String>,
    /// `true` or `false` (also `1`/`0`, `on`/`off`); default `false`. Drawn over the emote before it.
    pub zero_width: Option<String>,
    /// As `zero_width`. Only its owner can add it to sets, and search leaves it out.
    pub private: Option<String>,
    /// As `zero_width`. Search leaves it out unless asked for NSFW.
    pub nsfw: Option<String>,
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

impl SetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Personal => "personal",
            Self::Global => "global",
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown set kind: {0}")]
pub struct UnknownSetKind(pub String);

impl FromStr for SetKind {
    type Err = UnknownSetKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "channel" => Ok(Self::Channel),
            "personal" => Ok(Self::Personal),
            "global" => Ok(Self::Global),
            other => Err(UnknownSetKind(other.to_owned())),
        }
    }
}

/// Who can find and open a set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum SetVisibility {
    /// Anyone can open it, and it is listed on its owner's profile. New channel sets start here.
    #[default]
    Listed,
    /// Anyone with its link (its ID) can open it; it isn't listed. Personal sets start here.
    Unlisted,
    /// Only its owner, its editors and staff can open it, except while a channel shows it:
    /// a channel's active set is public. Channel sets only.
    Private,
}

impl SetVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Listed => "listed",
            Self::Unlisted => "unlisted",
            Self::Private => "private",
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown set visibility: {0}")]
pub struct UnknownSetVisibility(pub String);

impl FromStr for SetVisibility {
    type Err = UnknownSetVisibility;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "listed" => Ok(Self::Listed),
            "unlisted" => Ok(Self::Unlisted),
            "private" => Ok(Self::Private),
            other => Err(UnknownSetVisibility(other.to_owned())),
        }
    }
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
    /// Who can find and open it.
    #[serde(default)]
    pub visibility: SetVisibility,
    /// `visibility` is `LISTED`: shown on the owner's profile.
    #[serde(default)]
    pub listed: bool,
    /// How many of `capacity` the set uses. For whoever may edit the set it counts every emote in
    /// it, pending ones included, so `capacity - slotsUsed` is what still fits. For anyone else it
    /// counts the emotes this answer may show them (approved ones, plus their own pending ones).
    #[serde(default)]
    pub slots_used: u32,
    #[serde(default)]
    pub created_at: String,
}

/// A set as a card on a profile: its facts and the first few emotes (`accountSets`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetSummary {
    pub id: String,
    pub name: String,
    pub kind: SetKind,
    pub owner: Option<AccountSummary>,
    pub visibility: SetVisibility,
    pub listed: bool,
    pub capacity: u32,
    /// How many slots the set uses, as counted for you; see `EmoteSet.slotsUsed`.
    pub slots_used: u32,
    /// A channel shows this set now.
    pub active: bool,
    /// The newest emotes you may see, at most the `preview` asked for.
    pub preview: Vec<SetEmote>,
    pub created_at: String,
}

/// What `cloneSet` made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ClonedSet {
    /// The new channel set.
    pub set: EmoteSet,
    /// Emotes copied into it.
    pub copied: u32,
    /// Emotes left out: over capacity, private emotes of others, emotes not approved, or ones
    /// you can't see.
    pub skipped: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateSetInput"))]
pub struct CreateSet {
    pub name: String,
    /// `CHANNEL` or `PERSONAL`. The global set exists already.
    pub kind: SetKind,
    /// Make the set for an account you edit with `sets` (ADR 0014). Empty: for yourself.
    #[serde(default)]
    pub owner_id: Option<String>,
    /// Empty: `LISTED` for channel sets, `UNLISTED` for the personal set. Personal sets can't be
    /// `PRIVATE`.
    #[serde(default)]
    pub visibility: Option<SetVisibility>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateSetInput"))]
pub struct UpdateSet {
    pub name: Option<String>,
    /// Needs `sets.edit_any` to raise above the default.
    pub capacity: Option<u32>,
    /// Personal sets can't be `PRIVATE`; the global set is always listed.
    #[serde(default)]
    pub visibility: Option<SetVisibility>,
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

/// A linked Twitch or Kick account as anyone may see it: no IDs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PublicIdentity {
    pub platform: Platform,
    pub login: String,
    pub display_name: String,
}

/// One `searchAccounts` hit: the account's public summary and its linked logins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountHit {
    pub account: AccountSummary,
    /// Twitch first.
    pub identities: Vec<PublicIdentity>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountResults {
    pub items: Vec<AccountHit>,
    /// How many accounts match, counted up to 1000.
    pub total: u64,
    /// More than `total` match: show it as "1000+".
    pub total_capped: bool,
    pub page: u32,
    pub per_page: u32,
}

/// How `searchEmotes` orders its results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum EmoteSort {
    /// Best match first, then the most used. The default.
    #[default]
    Relevance,
    /// The most used first (channels showing the emote), then by name.
    Top,
    /// The newest first.
    New,
}

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

/// A page of emotes and how many there are in all.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmotePage {
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
    /// Shown on its holders' public profiles (`accountProfile`). Off by default.
    #[serde(default)]
    pub public: bool,
}

/// A role as anyone may see it on a profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PublicRole {
    pub id: String,
    pub name: String,
}

/// The set a channel shows now, on a profile ("Active emotes").
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActiveChannelSet {
    pub platform: Platform,
    /// The channel's login.
    pub login: String,
    pub set_id: String,
    pub set_name: String,
}

/// An account's public profile, like 7TV's public user pages. Listed sets come from
/// `accountSets`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountProfile {
    pub account: AccountSummary,
    /// Linked Twitch and Kick accounts: platform, login and display name only. Twitch first.
    pub identities: Vec<PublicIdentity>,
    /// Roles marked public, most senior first.
    pub roles: Vec<PublicRole>,
    /// Each channel's active set, for channels showing one.
    pub active_sets: Vec<ActiveChannelSet>,
    pub created_at: String,
}

// Reports and takedowns (Phase 7)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ReportState {
    Open,
    Dismissed,
    /// The emote was taken down.
    Actioned,
}

/// What a moderator does with a report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ReportAction {
    Dismiss,
    /// Take the emote down (needs `emotes.takedown`).
    TakeDown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Report {
    pub id: String,
    /// The reported emote, as moderators see it. Empty if it was deleted since.
    pub emote: Option<Emote>,
    pub reporter_id: Option<String>,
    pub reason: String,
    pub state: ReportState,
    pub resolution: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ReportQueue {
    pub items: Vec<Report>,
    pub total: u64,
}

/// Reports about an account's emotes and reports it filed (`emotes.review` or
/// `emotes.takedown`), newest first, at most 50 each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountReports {
    pub about: Vec<Report>,
    pub by: Vec<Report>,
}

/// An account as staff see it (`users.view` or `users.restrict`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AdminAccount {
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub identities: Vec<Identity>,
    pub created_at: String,
    pub uploads_blocked: bool,
    pub blocked: bool,
}

/// Something the platform tells you, such as a takedown of your emote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Notification {
    pub id: String,
    /// For example `emote.taken_down`.
    pub kind: String,
    /// JSON, as text.
    pub payload: String,
    pub created_at: String,
    pub read: bool,
}

/// What an account may do (`users.restrict`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountStatus {
    pub id: String,
    /// No uploads.
    pub uploads_blocked: bool,
    /// No sign-in, no sessions.
    pub blocked: bool,
}

// Account activity (ADR 0016)

/// What one line of an account's public activity feed says happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    SetEmoteAdded,
    SetEmoteRemoved,
    /// The emote's name in the set changed (`from_name`, `to_name`).
    SetEmoteRenamed,
    /// A channel's active set changed (`set` is the new one; `from_name`, `to_name`).
    ActiveSetChanged,
    /// The badge and/or paint shown changed (`badge`, `paint`).
    CosmeticsSelected,
    /// An emote of the account was approved and listed.
    EmoteUploaded,
    SetCreated,
    /// A set was renamed (`from_name`, `to_name`).
    SetRenamed,
}

/// A set, as a feed line names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivitySet {
    pub id: String,
    pub name: String,
}

/// An emote, as a feed line names it. `name` is what it was called in the set at the time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivityEmote {
    pub id: String,
    pub name: String,
    /// For example `https://cdn.cyex.tv/emotes/<sha256>`.
    pub host: String,
    pub files: Vec<EmoteFile>,
}

/// A badge or paint change: what it was and what it is now. Empty ends mean none; names are
/// empty when the cosmetic was deleted since.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivityCosmeticChange {
    pub from_id: Option<String>,
    pub from_name: Option<String>,
    pub to_id: Option<String>,
    pub to_name: Option<String>,
}

/// A page of an activity feed. A page can hold fewer lines than asked for without being the last
/// one: the feed skips what isn't public.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivityPage {
    pub entries: Vec<ActivityEntry>,
    /// Pass as `before` to get the next page; null when there is none.
    pub next: Option<String>,
}

/// One line of an account's public activity feed (7TV's "Activity" tab).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivityEntry {
    /// An audit row ID; page back with it.
    pub id: String,
    pub kind: ActivityKind,
    /// Who did it: the account or one of its editors.
    pub actor: AccountSummary,
    pub set: Option<ActivitySet>,
    pub emote: Option<ActivityEmote>,
    pub from_name: Option<String>,
    pub to_name: Option<String>,
    pub badge: Option<ActivityCosmeticChange>,
    pub paint: Option<ActivityCosmeticChange>,
    pub created_at: String,
}

/// One audited change (ADR 0010). `before` and `after` are JSON, as text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AuditEntry {
    pub id: String,
    /// The account that made the change; empty for changes the system made.
    pub actor_id: Option<String>,
    /// For example `role.created` or `set.editor_added`.
    pub action: String,
    /// For example `role`, `account` or `set`.
    pub target_kind: String,
    pub target_id: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateRoleInput"))]
pub struct CreateRole {
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub permissions: Vec<String>,
    /// Show it on its holders' public profiles. Default false.
    #[serde(default)]
    pub public: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateRoleInput"))]
pub struct UpdateRole {
    pub name: Option<String>,
    /// An empty string clears it.
    pub description: Option<String>,
    pub position: Option<i32>,
    pub permissions: Option<Vec<String>>,
    /// Show it on its holders' public profiles.
    #[serde(default)]
    pub public: Option<bool>,
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
    /// A listed set was made (`createSet`, `cloneSet`), on its owner's `user:` topics. Unlisted
    /// and private sets make no event, so their IDs stay unknown.
    #[serde(rename = "set.created")]
    SetCreated,
    /// A badge or paint was made (topic `cosmetics`, ADR 0015).
    #[serde(rename = "cosmetic.created")]
    CosmeticCreated,
    #[serde(rename = "cosmetic.updated")]
    CosmeticUpdated,
    /// Gone: stop showing it on anyone.
    #[serde(rename = "cosmetic.deleted")]
    CosmeticDeleted,
    /// The badge or paint a user shows changed (their `user:` topics).
    #[serde(rename = "user.cosmetics_updated")]
    UserCosmeticsUpdated,
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
    /// `cosmetic.*`: which kind of cosmetic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cosmetic_kind: Option<CosmeticKind>,
    /// `cosmetic.*`: its ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cosmetic_id: Option<String>,
    /// `user.cosmetics_updated`: the badge shown now; empty for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge_id: Option<String>,
    /// `user.cosmetics_updated`: the paint shown now; empty for none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paint_id: Option<String>,
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
    fn text_forms_match_serde() {
        use EmoteState::*;
        for state in [Pending, Approved, Rejected, TakenDown] {
            assert_eq!(state.as_str().parse::<EmoteState>().unwrap(), state);
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, format!("\"{}\"", state.as_str()));
        }
        for kind in [SetKind::Channel, SetKind::Personal, SetKind::Global] {
            assert_eq!(kind.as_str().parse::<SetKind>().unwrap(), kind);
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
        }
        for v in [
            SetVisibility::Listed,
            SetVisibility::Unlisted,
            SetVisibility::Private,
        ] {
            assert_eq!(v.as_str().parse::<SetVisibility>().unwrap(), v);
            let json = serde_json::to_string(&v).unwrap();
            assert_eq!(json, format!("\"{}\"", v.as_str()));
        }
        assert!("deleted".parse::<EmoteState>().is_err());
        assert!("Channel".parse::<SetKind>().is_err());
        assert!("public".parse::<SetVisibility>().is_err());
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
            cosmetic_kind: None,
            cosmetic_id: None,
            badge_id: None,
            paint_id: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "set.emote_added");
        assert!(json.get("cosmetic_id").is_none());
        assert_eq!(
            serde_json::to_string(&EventType::UserCosmeticsUpdated).unwrap(),
            "\"user.cosmetics_updated\""
        );
    }
}
