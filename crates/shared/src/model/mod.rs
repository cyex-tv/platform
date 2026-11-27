//! The shapes of the public API (ADR 0008). They live in this crate so client code (the deck, a
//! future SDK) can reuse them. The GraphQL derives sit behind the `graphql` feature; REST and the
//! event WebSocket use the serde form (snake_case).
//!
//! IDs are ULIDs as text. Times are RFC 3339 strings in UTC.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

pub use crate::cosmetics::*;
pub use crate::developers::*;
pub use crate::public_limits::*;
pub use crate::shop::*;
pub use crate::site::*;

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
    /// The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
    pub id: String,
    pub identities: Vec<Identity>,
    /// Permission names from the account's roles (ADR 0010).
    pub permissions: Vec<String>,
    pub created_at: String,
    /// The platform whose name and picture the account goes by (`setDisplayPlatform`); null for
    /// the default, Twitch first. `identities` lists that one first.
    #[serde(default)]
    #[schema(required = false)]
    pub display_platform: Option<Platform>,
    /// Every role the account holds, public or not, most senior first. Only you see the ones
    /// that aren't public: your profile marks them hidden; others see `AccountProfile.roles`.
    #[serde(default)]
    #[schema(required = false)]
    pub roles: Vec<OwnRole>,
}

/// A deleted account that can still be restored (ADR 0019), for the page signing in to it opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PendingDeletion {
    /// When `deleteAccount` was called.
    pub requested_at: String,
    /// When it will be erased for good, unless restored first.
    pub erase_after: String,
    /// The name and avatar it showed, Twitch first.
    pub display_name: Option<String>,
    pub avatar: Option<String>,
}

/// One of your own roles, as `me.roles` lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct OwnRole {
    pub id: String,
    pub name: String,
    /// Shown on your public profile. When false, only you see it.
    pub public: bool,
    /// Never shown publicly, whatever `public` says (only you and staff see it).
    #[serde(default)]
    #[schema(required = false)]
    pub invisible: bool,
    /// The role's colour, `#rrggbb`, if it has one.
    #[serde(default)]
    pub color: Option<String>,
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
    /// A personal API token you made on cyex.tv (`createConsoleToken`, ADR 0028).
    Console,
}

/// One place you are signed in. Never holds the token or anything made from it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSession {
    /// Pass to `revokeSession`.
    pub id: String,
    pub kind: SessionKindView,
    /// For example `Firefox on Windows`, `cyex.tv app` or `Unknown device`, from the user agent
    /// that signed in; `Console token` for a console token.
    pub device: String,
    pub created_at: String,
    /// Updated at most once an hour.
    pub last_seen_at: String,
    pub expires_at: String,
    /// The session this request came with.
    pub current: bool,
    /// The app that signed in through `/v1/auth/app` (ADR 0021), such as `cyex-deck`; null for
    /// website sessions.
    #[serde(default)]
    pub app_client_id: Option<String>,
}

/// A personal API token (ADR 0028): a Bearer token that acts as you, with your own account's
/// rights, for a GraphQL playground, a script or moving things between your accounts. Only one
/// at a time: making a new one signs the last one out. It lasts 3 days, shows in `mySessions` as
/// `Console token`, and ends when you sign it out there, sign out any other session, or sign out
/// everywhere (ADR 0029). The token is in this answer only; it is never shown again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ConsoleToken {
    /// Send as `Authorization: Bearer <token>`. Anyone holding it can act as you.
    pub token: String,
    /// `Bearer`.
    pub token_type: String,
    /// When it stops working: 3 days after it was made, however much it is used.
    pub expires_at: String,
    /// Its session's ID, as `mySessions` lists it; pass to `revokeSession` to sign it out.
    pub session_id: String,
}

/// A short reference to an account, for owners and editors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSummary {
    /// The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
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
    /// Create, rename and delete sets, change their capacity, and pick a channel's active set.
    Sets,
    /// Add, change and remove editors, never granting more than you hold.
    Editors,
    /// Pick the badge and paint the owner shows (ADR 0015). Never grants.
    Profile,
    /// Change the owner's channels' settings (and pick their active set, as `Sets` does).
    Channels,
    /// Upload emotes for the owner (`POST /v1/emotes` with `owner_id`).
    #[serde(rename = "emote_upload")]
    EmoteUpload,
    /// See, rename, retag and delete the owner's emotes.
    #[serde(rename = "emote_manage")]
    EmoteManage,
    /// Retired (ADR 0014, amendment 2026/11/22): only an emote's owner gives it away, and nobody
    /// can ask for someone else's emote to be transferred. Kept so clients that send it still
    /// parse; it is never granted, never listed and grants nothing (stored grants ignore it).
    #[serde(rename = "emote_transfer")]
    #[cfg_attr(
        feature = "graphql",
        graphql(
            deprecation = "Retired: only an emote's owner can give it away. Never granted or listed; ignored when sent with other permissions."
        )
    )]
    EmoteTransfer,
    /// Manage the owner's payment methods. Stored; grants nothing yet.
    #[serde(rename = "billing_payment")]
    BillingPayment,
    /// Manage the owner's subscription. Stored; grants nothing yet.
    #[serde(rename = "billing_subscription")]
    BillingSubscription,
}

impl EditorPermission {
    /// Every flag that can be granted. `EmoteTransfer` is retired and left out.
    pub const ALL: [Self; 9] = [
        Self::Emotes,
        Self::Sets,
        Self::Editors,
        Self::Profile,
        Self::Channels,
        Self::EmoteUpload,
        Self::EmoteManage,
        Self::BillingPayment,
        Self::BillingSubscription,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Emotes => "emotes",
            Self::Sets => "sets",
            Self::Editors => "editors",
            Self::Profile => "profile",
            Self::Channels => "channels",
            Self::EmoteUpload => "emote_upload",
            Self::EmoteManage => "emote_manage",
            Self::EmoteTransfer => "emote_transfer",
            Self::BillingPayment => "billing_payment",
            Self::BillingSubscription => "billing_subscription",
        }
    }

    /// `None` for a name this version doesn't know, and for the retired `emote_transfer`: a
    /// stored grant of it grants nothing.
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
    /// They may work on every set of the account, new ones included. False: only `setIds`.
    #[serde(default = "yes")]
    pub all_sets: bool,
    /// The sets they may work on when `allSets` is false. Empty when it is true.
    #[serde(default)]
    pub set_ids: Vec<String>,
}

fn yes() -> bool {
    true
}

/// Who you are on a channel you can manage, highest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ChannelRole {
    /// The channel is yours.
    Owner,
    /// You are an editor of the channel's account.
    AccountEditor,
    /// You edit at least one set of the channel's account.
    SetEditor,
}

/// A channel you can manage, with your highest role on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EditableChannel {
    pub role: ChannelRole,
    pub channel: Channel,
}

/// How you may change a set's emotes (`channelEditableSets`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum SetAccess {
    /// The set is yours.
    Owner,
    /// You are an account editor of its owner with `EMOTES`, and your grant covers this set.
    AccountEditor,
    /// You are an editor of this set.
    SetEditor,
    /// Through a staff permission (`sets.edit_any`); your changes are audited.
    Staff,
}

/// A set you may add, rename and remove emotes in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EditableSet {
    /// The set, without preview emotes. `capacity - slotsUsed` is what still fits.
    pub set: SetSummary,
    /// The channel shows this set now (`channel.activeSetId`).
    pub active_here: bool,
    pub access: SetAccess,
}

/// A channel and the sets of its account you may change (`channelEditableSets`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ChannelEditableSets {
    pub channel: Channel,
    /// Your highest role on the channel; null when you have none.
    pub role: Option<ChannelRole>,
    /// The channel's active set can take your changes (it is among `sets`).
    pub can_edit_active_set: bool,
    /// Oldest first. Empty when you may change none.
    pub sets: Vec<EditableSet>,
}

/// Where an emote transfer request stands (ADR 0014).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum TransferRequestState {
    /// Waiting for the emote's owner.
    Pending,
    Accepted,
    Declined,
    /// Withdrawn by the editor, or closed because the emote moved or went away.
    Cancelled,
    /// Nobody answered within 7 days.
    Expired,
}

impl TransferRequestState {
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "pending" => Self::Pending,
            "accepted" => Self::Accepted,
            "declined" => Self::Declined,
            "cancelled" => Self::Cancelled,
            "expired" => Self::Expired,
            _ => return None,
        })
    }
}

/// An account editor's request to give one of the owner's emotes to another account; the emote
/// moves only when the owner accepts (ADR 0014).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmoteTransferRequest {
    pub id: String,
    pub emote_id: String,
    pub emote_name: String,
    /// The emote's owner, who accepts or declines.
    pub from: AccountSummary,
    pub to: AccountSummary,
    pub requested_by: AccountSummary,
    pub state: TransferRequestState,
    pub created_at: String,
    pub expires_at: String,
    pub resolved_at: Option<String>,
}

/// An account you edit, with what you may do there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EditedAccount {
    pub account: AccountSummary,
    pub permissions: Vec<EditorPermission>,
    pub granted_at: String,
    /// You may work on every set of the account. False: only `setIds`.
    #[serde(default = "yes")]
    pub all_sets: bool,
    #[serde(default)]
    pub set_ids: Vec<String>,
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
    /// Limit them to these sets of the account. Empty or missing: every set, new ones included.
    #[serde(default)]
    pub set_ids: Option<Vec<String>>,
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

/// Whether an emote's image is made. Separate from [`EmoteState`] (review), so adding it
/// breaks nobody: a client that doesn't know it keeps showing `Processing` as before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum EmoteProcessing {
    /// The files are made.
    #[default]
    Ready,
    /// The transcode job is queued or running.
    Processing,
    /// The transcode ran out of attempts: the emote never gets files. Delete it or upload again.
    Failed,
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
    /// Only its owner can add it to sets. True exactly when `visibility` is `PRIVATE`.
    pub private: bool,
    /// Retired (ADR 0020, amendment 2026/11/22): NSFW emotes aren't allowed on cyex.tv, so this
    /// is always `false`.
    #[schema(deprecated)]
    #[cfg_attr(
        feature = "graphql",
        graphql(deprecation = "NSFW emotes aren't allowed on cyex.tv; always false.")
    )]
    pub nsfw: bool,
    /// About twice as wide as tall or more: the 1x file is at least 1.75 times as wide as it is
    /// tall. Derived from the files, not set by the uploader.
    pub wide: bool,
}

/// Who can find an emote and add it to sets (ADR 0020).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum EmoteVisibility {
    /// Found by search and browse, listed on its creator's page. New emotes start here.
    #[default]
    Public,
    /// Anyone with its link can open it and add it to sets; search, browse, activity feeds and
    /// its creator's page leave it out for everyone but its owner. Set by staff only (a
    /// moderator taking an emote off the listings); creators choose public or private.
    Unlisted,
    /// Only its owner can add it to sets, and it is left out of listings (`flags.private`).
    Private,
}

impl EmoteVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Unlisted => "unlisted",
            Self::Private => "private",
        }
    }

    /// From the two stored flags (never both set).
    pub fn of(private: bool, unlisted: bool) -> Self {
        if private {
            Self::Private
        } else if unlisted {
            Self::Unlisted
        } else {
            Self::Public
        }
    }

    /// The stored flags: `(private, unlisted)`.
    pub fn flags(self) -> (bool, bool) {
        (self == Self::Private, self == Self::Unlisted)
    }

    /// The visibility after a change that may name it, the old `private` flag, or both, from
    /// `current`. `private: true` means private; `private: false` makes a private emote public
    /// and leaves an unlisted one unlisted, so clients that only know the flag keep working.
    /// `None` when the two say different things.
    pub fn after(current: Self, visibility: Option<Self>, private: Option<bool>) -> Option<Self> {
        match (visibility, private) {
            (Some(v), Some(p)) if p != (v == Self::Private) => None,
            (Some(v), _) => Some(v),
            (None, Some(true)) => Some(Self::Private),
            (None, Some(false)) if current == Self::Private => Some(Self::Public),
            (None, _) => Some(current),
        }
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("unknown emote visibility: {0}")]
pub struct UnknownEmoteVisibility(pub String);

impl FromStr for EmoteVisibility {
    type Err = UnknownEmoteVisibility;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "public" => Ok(Self::Public),
            "unlisted" => Ok(Self::Unlisted),
            "private" => Ok(Self::Private),
            other => Err(UnknownEmoteVisibility(other.to_owned())),
        }
    }
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
    /// 1 to 4. A badge's larger sizes (`128`, `256`, `512`) carry their px ÷ 18, rounded: 7, 14
    /// and 28; read `width` for the size. **Deprecated for badge and paint files**: `width` is
    /// the contract (`file_scale_non_emote` in `/v1/meta`). Emote files keep `scale` as it is.
    #[cfg_attr(
        feature = "graphql",
        graphql(
            deprecation = "For badge and paint files: read `width`. Emote files keep `scale` (1 to 4)."
        )
    )]
    #[schema(deprecated)]
    pub scale: u8,
    pub width: u16,
    pub height: u16,
    /// Bytes.
    pub size: u32,
    /// The first frame of an animated emote.
    pub is_static: bool,
    /// The full CDN address of this file: `{host}/{name}`. Filled in by the API (empty only in
    /// the records the worker writes).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schema(required = true)]
    pub url: String,
    /// `image/webp` or `image/avif`. Filled in by the API like `url`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[schema(required = true)]
    pub mime: String,
}

impl EmoteFile {
    /// The media type of `format`.
    pub fn mime_of(format: ImageFormat) -> &'static str {
        match format {
            ImageFormat::Webp => "image/webp",
            ImageFormat::Avif => "image/avif",
        }
    }

    /// The files with their `url` (`{host}/{name}`) and `mime` filled in.
    pub fn with_urls(mut files: Vec<Self>, host: &str) -> Vec<Self> {
        for f in &mut files {
            f.url = format!("{host}/{}", f.name);
            f.mime = Self::mime_of(f.format).to_owned();
        }
        files
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Emote {
    /// The emote's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
    pub id: String,
    pub name: String,
    pub owner: Option<AccountSummary>,
    pub state: EmoteState,
    /// `READY`, `PROCESSING` or `FAILED`: whether the image is made (or could not be).
    #[serde(default)]
    #[schema(required = false)]
    pub processing: EmoteProcessing,
    pub flags: EmoteFlags,
    /// `PUBLIC`, `UNLISTED` or `PRIVATE` (ADR 0020). `flags.private` says the same for
    /// `PRIVATE`, for clients from before `UNLISTED`.
    #[serde(default)]
    #[schema(required = false)]
    pub visibility: EmoteVisibility,
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
    /// `public` (default) or `private` (ADR 0020). With `private` too, the two must agree.
    /// `unlisted` is for staff with `emotes.flags` (`bad_visibility` for anyone else).
    pub visibility: Option<String>,
    /// Retired: NSFW emotes aren't allowed. `false` is still taken; `true` is `nsfw_not_allowed`.
    #[schema(deprecated)]
    pub nsfw: Option<String>,
    /// The account that will own the emote: yours by default, or one you edit with
    /// `EMOTE_UPLOAD` on every set (ADR 0014).
    pub owner_id: Option<String>,
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
    /// `true` makes it private; `false` makes a private emote public (an unlisted one stays
    /// unlisted). Prefer `visibility`.
    pub private: Option<bool>,
    /// Retired: NSFW emotes aren't allowed. `false` is ignored; `true` is `nsfw_not_allowed`.
    #[cfg_attr(
        feature = "graphql",
        graphql(
            deprecation = "NSFW emotes aren't allowed on cyex.tv. false is ignored; true is refused (nsfw_not_allowed)."
        )
    )]
    pub nsfw: Option<bool>,
    /// `PUBLIC` or `PRIVATE`. With `private` too, the two must agree (`bad_visibility`).
    /// `UNLISTED` (and changing an unlisted emote) is for staff with `emotes.flags` or
    /// `emotes.edit_any`: a moderator takes an emote off the listings, not its owner.
    pub visibility: Option<EmoteVisibility>,
}

/// What `setEmoteFlags` changes (ADR 0024). Every field is optional: what is left out stays.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
pub struct EmoteFlagsInput {
    /// `UNLISTED` takes a public emote off search, browse and its creator's page.
    pub visibility: Option<EmoteVisibility>,
    /// Retired: NSFW emotes aren't allowed. `false` clears an old flag (any change here clears
    /// it too); `true` is `nsfw_not_allowed`.
    #[cfg_attr(
        feature = "graphql",
        graphql(
            deprecation = "NSFW emotes aren't allowed on cyex.tv. true is refused (nsfw_not_allowed); saving flags clears an old NSFW flag."
        )
    )]
    pub nsfw: Option<bool>,
    pub zero_width: Option<bool>,
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
    /// The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
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
    /// Goes up by one or more with every change to what the set shows: a member added, removed
    /// or renamed, the set renamed, an emote in it changed (ADR 0035). Keep it with your copy
    /// and ask `setChanges` (or `GET /v1/sets/{id}?since=`) for what changed after it.
    #[serde(default)]
    pub version: i64,
}

/// What changed in a set after a version (`setChanges`, `GET /v1/sets/{id}?since=`; ADR 0035).
/// Apply `added`, `renamed` and `updated` as upserts by `emote.id`, and `removedIds` as deletes;
/// then keep `version`. With `full: true` the change log doesn't reach back to `since` (or
/// `since` is not a version this set had): replace your copy with `set` instead.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetChanges {
    pub id: String,
    /// The version this answer brings you to.
    pub version: i64,
    /// The `since` asked for.
    pub since: i64,
    /// True when this is a full resync: `set` holds the whole set and the lists are empty.
    pub full: bool,
    /// The whole set, only when `full` is true.
    pub set: Option<EmoteSet>,
    /// The set's name now.
    pub name: String,
    /// Members added since (or newly visible to you, for example an emote approved).
    pub added: Vec<SetEmote>,
    /// Members whose name or zero-width flag in the set changed.
    pub renamed: Vec<SetEmote>,
    /// Members whose emote changed (its files, flags, state or own name).
    pub updated: Vec<SetEmote>,
    /// Emotes no longer in the set as you see it (removed, taken down, deleted, hidden).
    pub removed_ids: Vec<String>,
}

/// What you may set a set's capacity to (`setCapacityRange`; ADR 0017, amendment 2026/11/22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetCapacityRange {
    /// The fewest slots: 1, or the emotes the set holds now if that is more.
    pub min: u32,
    /// The most slots you may pick.
    pub max: u32,
    /// The most without a staff permission: the owner's `set_capacity_max` role limit (1.000 by
    /// default) for a channel set, 5 for a personal set, or the set's capacity if it is higher.
    pub owner_max: u32,
    /// Emotes in the set now, pending ones included.
    pub used: u32,
    /// `max` comes from a staff permission (`sets.edit_any`, or `global_set.edit` for the global
    /// set): up to 1.000.000.
    pub staff: bool,
}
/// A page of a set's members and how many match, in all (`setEmotes`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetEmotePage {
    pub items: Vec<SetEmote>,
    /// Members the viewer may see that match `query`, over every page.
    pub total: u64,
    /// Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
    /// page. Opaque: a list's cursor only works for the same list, filters and order.
    #[serde(default)]
    pub next: Option<String>,
}

/// A set as a card on a profile: its facts and the first few emotes (`accountSets`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SetSummary {
    /// The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
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
    /// Empty or `LISTED`: every set is public. `UNLISTED` and `PRIVATE` need `sets.edit_any`
    /// (staff); personal sets can't be `PRIVATE`.
    #[serde(default)]
    pub visibility: Option<SetVisibility>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateSetInput"))]
pub struct UpdateSet {
    pub name: Option<String>,
    /// 1 to the owner's `set_capacity_max` role limit (1.000 by default; 5 for a personal set),
    /// never below the emotes the set holds; up to 1.000.000 with `sets.edit_any`.
    /// `setCapacityRange` says what you may pick.
    pub capacity: Option<u32>,
    /// Every set is public: changing it needs `sets.edit_any` (staff hiding a set). Personal sets
    /// can't be `PRIVATE`; the global set is always listed.
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

/// `POST /v1/sets/lookup`: several sets at once (ADR 0035).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetsLookup {
    /// Up to 25 set IDs (Snowflakes, or old ULIDs while clients move over).
    pub ids: Vec<String>,
}

/// The sets found, in the order asked; unknown and hidden ones are left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetsLookupResult {
    pub sets: Vec<EmoteSet>,
}

/// `POST /v1/channels/lookup`: several channels of one platform at once (ADR 0035).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ChannelsLookup {
    pub platform: Platform,
    /// Up to 100 broadcaster platform user IDs.
    pub ids: Vec<String>,
}

/// The channels found, in the order asked; channels not on cyex.tv are left out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ChannelsLookupResult {
    pub channels: Vec<Channel>,
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

/// One person in a "most important" list (`activityTopActors`, `auditTopPeople`,
/// `orderTopBuyers`, `reviewTopUploaders`, `entitlementTopHolders`): their public summary and
/// logins, and how many entries they have in that list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PersonCount {
    pub account: AccountSummary,
    /// Twitch first.
    pub identities: Vec<PublicIdentity>,
    /// Their entries in the list (the newest 2000 rows of a log or feed are counted).
    pub count: u64,
}

/// Which side of the audit log `auditTopPeople` counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditPersonRole {
    /// Who made the changes.
    #[default]
    Actor,
    /// The accounts the changes were about (target kind `account`).
    Target,
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
    /// Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
    /// page. Opaque: a list's cursor only works for the same list, filters and order.
    #[serde(default)]
    pub next: Option<String>,
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
    /// Most channels added it in the `window` (net of removals), then the most used.
    Trending,
}

/// A flag `searchEmotes` can require.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum EmoteFlag {
    /// Overlays the emote before it.
    ZeroWidth,
    /// About twice as wide as tall or more.
    Wide,
    /// Retired: NSFW emotes aren't allowed, so this finds nothing.
    #[cfg_attr(
        feature = "graphql",
        graphql(deprecation = "NSFW emotes aren't allowed on cyex.tv; this flag finds nothing.")
    )]
    Nsfw,
}

/// How far back `TRENDING` counts channel adds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum TrendingWindow {
    /// The last day.
    Day,
    /// The last 7 days. The default.
    #[default]
    Week,
    /// The last 30 days.
    Month,
}

impl TrendingWindow {
    pub fn days(self) -> i32 {
        match self {
            Self::Day => 1,
            Self::Week => 7,
            Self::Month => 30,
        }
    }
}

/// Where an emote stands among the trending ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmoteTrending {
    /// 1 is the most added emote in the window.
    pub rank: u32,
    /// Channels that added it in the window, net of removals.
    pub channels: u32,
    /// The window the rank is in.
    pub window: TrendingWindow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct SearchResults {
    pub items: Vec<Emote>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    /// Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
    /// page. Opaque: a list's cursor only works for the same list, filters and order.
    #[serde(default)]
    pub next: Option<String>,
}

/// The header search: the best emotes and accounts for one query.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct QuickSearch {
    pub emotes: Vec<Emote>,
    pub users: Vec<AccountHit>,
}

// Moderation and roles

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ReviewQueue {
    pub items: Vec<Emote>,
    /// How many match the read in all (its tab and search).
    pub total: u64,
    /// How many wait in each pending tab, whatever the read asked for.
    #[serde(default)]
    pub counts: ReviewCounts,
    /// Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
    /// page. Opaque: a list's cursor only works for the same list, filters and order.
    #[serde(default)]
    pub next: Option<String>,
}

/// What a batch review does to every emote in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    Approve,
    Reject,
}

/// What a batch did: the IDs it changed, and the ones it left alone with why. Nothing is half
/// done: the changes commit together.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct BatchOutcome {
    pub done: Vec<String>,
    pub failed: Vec<BatchFailure>,
}

/// One item a batch left alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct BatchFailure {
    pub id: String,
    /// A machine-readable reason, as an error's `code` (`not_pending`, `not_open`, `not_found`).
    pub code: String,
    pub message: String,
}

/// Another emote that may be the same as one in the review queue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct DuplicateCandidate {
    pub emote: Emote,
    /// Made from the very same file (the same sha256).
    pub same_file: bool,
    /// How alike the names are, 0 to 1.
    pub similarity: f64,
}

/// One login from a pasted list, and the account it names (none when it names no account, or
/// two different ones on Twitch and Kick).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct LoginMatch {
    /// The login as it was asked for, trimmed.
    pub login: String,
    pub account: Option<AccountSummary>,
    /// Why there is no account: `not_found`, `ambiguous` or `bad_login`.
    pub problem: Option<String>,
}

/// How many emotes wait for review, by tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ReviewCounts {
    /// Pending and public.
    pub public: u64,
    /// Pending and unlisted or private.
    pub limited: u64,
}

/// A part of the review queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum ReviewTab {
    /// Every pending emote. The default.
    #[default]
    Pending,
    /// Pending and public: search shows it once approved.
    Public,
    /// Pending and unlisted or private.
    Limited,
    /// Approved or rejected by a reviewer.
    Resolved,
}

/// How the review queue is ordered. Ties go by upload time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum ReviewSort {
    /// Upload time. The default for the pending tabs.
    #[default]
    Uploaded,
    /// Review time. The default for `RESOLVED`.
    Reviewed,
    /// The uploader's display name.
    Uploader,
    /// The emote's name.
    Name,
}

/// What `reviewQueue` lists. Every field is optional: none is every pending emote, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "ReviewQueueFilter"))]
pub struct ReviewQueueFilter {
    pub tab: Option<ReviewTab>,
    /// Part of the emote's name or of its uploader's login or display name, any case.
    pub search: Option<String>,
    pub sort: Option<ReviewSort>,
    /// Newest (or Z) first. Default: oldest first, except `RESOLVED`, latest review first.
    pub descending: Option<bool>,
    /// Only this uploader's emotes (an account ID).
    #[serde(default)]
    pub uploader_id: Option<String>,
}

/// A note reviewers keep on one emote (`emotes.review`). The uploader never sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmoteNote {
    pub id: String,
    pub emote_id: String,
    /// Empty when the author's account is gone.
    pub author_id: Option<String>,
    pub body: String,
    pub created_at: String,
}

/// A page of emotes and how many there are in all.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EmotePage {
    pub items: Vec<Emote>,
    pub total: u64,
    /// Pass as `after` to read the page that follows (cursor paging, ADR 0030); null on the last
    /// page. Opaque: a list's cursor only works for the same list, filters and order.
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Higher means more senior. You can only manage roles below your own highest one.
    pub position: i32,
    /// What it allows, sorted.
    pub permissions: Vec<String>,
    /// What it denies, sorted: a Deny here beats an Allow on any lower role (the User role
    /// included). Never also in `permissions`; a permission in neither is left to the roles below.
    #[serde(default)]
    pub denies: Vec<String>,
    /// Shown on its holders' public profiles (`accountProfile`). Off by default.
    #[serde(default)]
    pub public: bool,
    /// Never shown publicly (profiles, user cards), even when `public`, and ignored wherever roles
    /// rank people. Staff and its holders still see it.
    #[serde(default)]
    pub invisible: bool,
    /// Optional colour, `#rrggbb` in lower case. Drawn as a dot on its chips and, for the most
    /// senior coloured public role, as the ring round its holders' profile pictures
    /// (`AccountProfile.roleColor`). Readable on both themes: the editor shows the contrast.
    #[serde(default)]
    pub color: Option<String>,
    /// Set on the built-in roles: `USER` (every account holds it), `ADMIN` and `ROOT` (the
    /// owners' top role, shown only to its holders).
    #[serde(default)]
    pub built_in: Option<BuiltInRole>,
    /// Whether you may change it: below your highest role, not one you hold, not the top role.
    #[serde(default)]
    pub editable: bool,
    /// Whether you may give it to someone (and take it away): below your highest role, you hold
    /// what it grants, and not the User or top role.
    #[serde(default)]
    pub assignable: bool,
}

/// The roles cyex.tv comes with (ADR 0010).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum BuiltInRole {
    /// "User": every account holds it without an assignment. Starts with no permissions.
    User,
    /// "Admin": can't be deleted.
    Admin,
    /// The top role: every permission, held by the owners only, never granted or edited.
    Root,
}

/// What a role says about a permission (`PermissionDecision.effect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum PermissionEffect {
    Allow,
    Deny,
}

/// Whether an account holds one permission, and which role decided it (`accountPermissions`).
/// Its most senior role that allows or denies the permission decides; the User role comes last.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PermissionDecision {
    pub permission: String,
    pub granted: bool,
    /// What the deciding role says; null when no role names it (not granted).
    pub effect: Option<PermissionEffect>,
    /// The deciding role; null when none decided, and for an owner unless you are one.
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    /// Granted because the account is an owner, who holds every permission and can't be denied.
    pub owner: bool,
}

/// A role as anyone may see it on a profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct PublicRole {
    pub id: String,
    pub name: String,
    /// The role's colour, `#rrggbb`, if it has one.
    #[serde(default)]
    pub color: Option<String>,
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
    /// The colour of the profile picture's ring: the `#rrggbb` of the most senior of the
    /// public `roles` that has one; null when none does (the neutral ring). Never from the
    /// badge or paint.
    #[serde(default)]
    pub role_color: Option<String>,
    /// Each channel's active set, for channels showing one.
    pub active_sets: Vec<ActiveChannelSet>,
    pub created_at: String,
    /// The account's editors, oldest first: who they are only, never what they may do.
    #[serde(default)]
    pub editors: Vec<AccountSummary>,
    /// The account shows the badges and paints it owns (`accountCollection`). Off by default.
    #[serde(default)]
    pub collection_public: bool,
}

// Reports and takedowns

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
    /// The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
    pub id: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub identities: Vec<Identity>,
    pub created_at: String,
    pub uploads_blocked: bool,
    pub blocked: bool,
    /// No reports or tickets.
    #[serde(default)]
    pub tickets_blocked: bool,
    /// When each hold ends; empty while it is off or for good.
    #[serde(default)]
    pub uploads_blocked_until: Option<String>,
    #[serde(default)]
    pub blocked_until: Option<String>,
    #[serde(default)]
    pub tickets_blocked_until: Option<String>,
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
    /// For `emote.transfer_requested`: where the request stands now (it may have been answered,
    /// withdrawn or run out since). Null for other kinds.
    #[serde(default)]
    pub transfer_state: Option<TransferRequestState>,
}

/// What an account may do (`users.restrict`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountStatus {
    /// The account's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
    pub id: String,
    /// No uploads.
    pub uploads_blocked: bool,
    /// No sign-in, no sessions.
    pub blocked: bool,
    /// No reports or tickets.
    #[serde(default)]
    pub tickets_blocked: bool,
    /// When each hold ends; empty while it is off or for good.
    #[serde(default)]
    pub uploads_blocked_until: Option<String>,
    #[serde(default)]
    pub blocked_until: Option<String>,
    #[serde(default)]
    pub tickets_blocked_until: Option<String>,
}

/// What a restriction stops.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum RestrictionKind {
    /// Uploading emotes (`users.restrict_uploads`).
    Uploads,
    /// Filing reports and tickets (`users.restrict_tickets`).
    Tickets,
    /// The whole account: no sign-in, sessions ended (`users.block`).
    Account,
}

/// One restriction placed on an account, running or over.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Restriction {
    pub id: String,
    pub kind: RestrictionKind,
    /// Why, as staff wrote it. Empty on restrictions made without one (`restrictAccount`).
    pub reason: Option<String>,
    /// When it ends; empty for good.
    pub ends_at: Option<String>,
    /// The staff account that placed it.
    pub created_by: Option<String>,
    pub created_at: String,
    /// When staff lifted it early, or a newer restriction of the same kind replaced it.
    pub lifted_at: Option<String>,
    pub lifted_by: Option<String>,
    /// Still holding now.
    pub active: bool,
}

/// A restriction to place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "RestrictionInput"))]
pub struct RestrictionInput {
    pub kind: RestrictionKind,
    /// When it ends (RFC 3339, in the future, at most ten years out); empty for good.
    pub until: Option<String>,
    /// Why: 1 to 500 characters. Staff see it; the account doesn't.
    pub reason: String,
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
    /// Someone became an account editor (`editor`). Never their permissions. Only listed when
    /// the filter asks for `EDITORS`.
    EditorAdded,
    /// An account editor was removed or left (`editor`). Only listed when the filter asks for
    /// `EDITORS`.
    EditorRemoved,
}

/// A group of activity lines, for filtering a feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "snake_case")]
pub enum ActivityGroup {
    /// Emotes added to, removed from or renamed in sets, and uploads that went public.
    Emotes,
    /// Sets made or renamed, and active set changes.
    Sets,
    /// Badge and paint changes.
    Cosmetics,
    /// Account editors added and removed.
    Editors,
}

/// Narrows `accountActivityPage`. Every field is optional; an empty filter is the whole feed
/// (without editor lines, as before filters existed).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "ActivityFilter"))]
pub struct ActivityFilter {
    /// Only these groups. Editor lines (`EDITOR_ADDED`, `EDITOR_REMOVED`) show only when this
    /// lists `EDITORS`.
    pub groups: Option<Vec<ActivityGroup>>,
    /// Only lines by this account (the owner or one of its editors).
    pub actor_id: Option<String>,
    /// Only lines at or after this time (RFC 3339).
    pub since: Option<String>,
    /// Only lines before this time (RFC 3339).
    pub until: Option<String>,
}

/// A set, as a feed line names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivitySet {
    /// The set's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
    pub id: String,
    pub name: String,
}

/// An emote, as a feed line names it. `name` is what it was called in the set at the time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ActivityEmote {
    /// The emote's Snowflake ID, as decimal digits (ADR 0027). Its old ULID is still taken as input.
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
    /// The editor added or removed (`EDITOR_ADDED`, `EDITOR_REMOVED`).
    #[serde(default)]
    #[schema(required = false)]
    pub editor: Option<AccountSummary>,
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
    /// What it denies. Never also in `permissions` (`bad_permissions`).
    #[serde(default)]
    pub denies: Option<Vec<String>>,
    /// Show it on its holders' public profiles. Default false.
    #[serde(default)]
    pub public: Option<bool>,
    /// Never show it publicly, and leave it out wherever roles rank people. Default false.
    #[serde(default)]
    pub invisible: Option<bool>,
    /// Its colour, `#rrggbb` (any case, stored lower case). Default none.
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateRoleInput"))]
pub struct UpdateRole {
    pub name: Option<String>,
    /// An empty string clears it.
    pub description: Option<String>,
    pub position: Option<i32>,
    /// Replaces what it allows.
    pub permissions: Option<Vec<String>>,
    /// Replaces what it denies. A name in both lists is `bad_permissions`; one given in only one
    /// of them leaves the other list (moving a name there from the other is fine).
    #[serde(default)]
    pub denies: Option<Vec<String>>,
    /// Show it on its holders' public profiles.
    #[serde(default)]
    pub public: Option<bool>,
    /// Never show it publicly, and leave it out wherever roles rank people.
    #[serde(default)]
    pub invisible: Option<bool>,
    /// Its colour, `#rrggbb`. An empty string clears it.
    #[serde(default)]
    pub color: Option<String>,
}

/// Which part of the site a limit covers, for grouping them on the role page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum LimitGroup {
    /// How often an account may call the API and search.
    Requests,
    /// Uploads an hour, and how many may wait for review.
    Uploads,
    /// Writes to your account: editors, cosmetics, sets, lists, settings, the shop, your data.
    Account,
    /// How big the files an account uploads may be: bytes, side and frames of emotes, badges
    /// and paints.
    Media,
}

/// One per-account limit and a role's override of it (`roleLimits`, `limitDefaults`). An
/// account's limit is the most generous override among its roles, else `default`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct RoleLimit {
    /// `gql_account`, `uploads_per_hour`, `pending_emotes`, `editors`, `cosmetics`,
    /// `set_clones`, `account_search`, `quick_search`, `heavy_reads`, `personal_lists`,
    /// `emote_transfers`, `prefs`, `checkout`, `redeem`, `data_export`, `set_capacity_max`, or a
    /// media limit: `badge_max_bytes`,
    /// `badge_max_side`, `badge_max_frames`, `paint_max_bytes` or `paint_max_side` (bytes, px
    /// and frames of the largest file an account may upload).
    pub key: String,
    pub group: LimitGroup,
    /// What everyone gets without an override (set when the API starts).
    pub default: i32,
    /// The most this limit may be set to: 1,000,000 for a count, and for a media limit the
    /// ceiling the API runs with (an emote's side never above 1000 px).
    pub max: i32,
    /// The window the count is per, in seconds; null for `pending_emotes`, a cap on how many
    /// wait at once.
    pub window_seconds: Option<i32>,
    /// The role's own count; null when it keeps the default.
    pub value: Option<i32>,
}

/// One override for `setRoleLimits`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "RoleLimitInput"))]
pub struct RoleLimitInput {
    pub key: String,
    /// 1 up to the limit's `max` (1,000,000 for a count); null keeps the default.
    pub value: Option<i32>,
}

/// Where an account stands with uploads (`uploadQuota`): how many of its emotes wait for review
/// and how many may, and its uploads this hour.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct UploadQuota {
    /// The account uploads go to.
    pub account_id: String,
    /// Its emotes waiting for review, ones still being processed included.
    pub pending: i32,
    /// How many may wait at once; an upload at this many is refused (`too_many_pending`). Null when the caller isn't capped (the owners).
    pub pending_max: Option<i32>,
    /// Emotes it got in the last hour.
    pub uploads_this_hour: i32,
    pub uploads_per_hour: i32,
    /// The most bytes one emote file may be (8 MB). Fixed, the same for everyone.
    pub max_bytes: i32,
    /// The longest side of an emote, in px (1000). Fixed.
    pub max_side: i32,
    /// The most frames an animated emote may have (1000). Fixed.
    pub max_frames: i32,
    /// The most frames a second (50). A frame shown for under 20 ms (a 0 or 10 ms GIF delay) is
    /// shown for 100 ms, as browsers do, so an animation never runs faster. Fixed.
    pub max_fps: i32,
    /// How many times as wide as tall an emote may be (4, so 4:1). Fixed; a tall emote has no rule.
    pub max_aspect: i32,
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

/// The query of `GET /v1/sets/{id}/emotes`: a page size and where to start (ADR 0030).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SetEmotesQuery {
    /// Emotes per page, 1 to 200 (default 120).
    pub per_page: Option<u32>,
    /// The `next` of the page before (also in its `Link: rel="next"` header); none for the first
    /// page.
    pub after: Option<String>,
}

/// The query of `GET /v1/sets/{id}`: with `since`, only what changed after that version
/// (ADR 0035).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SetQuery {
    /// A `version` of the set you hold. The answer is then a `SetChanges`, not the set.
    pub since: Option<i64>,
}

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

// Personal lists ("Hide for me" and favourites)

/// An emote you hid from your own view. `emote` is null once the emote is gone or you may no
/// longer see it; unhiding it by `emoteId` still works.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct HiddenEmote {
    pub emote_id: String,
    pub emote: Option<Emote>,
    pub hidden_at: String,
}

/// A creator whose emotes you hid from your own view. `account` is null once the account is gone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct HiddenCreator {
    pub account_id: String,
    pub account: Option<AccountSummary>,
    pub hidden_at: String,
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

// The staff account dashboard (ADR 0010 amendment 2026/11/19)

/// A note staff keep on an account (`users.notes`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct StaffNote {
    pub id: String,
    pub account_id: String,
    /// Empty when the author's account is gone.
    pub author_id: Option<String>,
    pub body: String,
    pub created_at: String,
}

/// Where a subscription is in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionState {
    Past,
    Current,
    Future,
}

/// A subscription to a plan. None exist yet; `accountSubscriptions` returns an empty list until
/// they do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct AccountSubscription {
    pub id: String,
    pub plan: String,
    pub state: SubscriptionState,
    pub starts_at: String,
    /// Empty while it renews.
    pub ends_at: Option<String>,
    /// `purchase`, `gift` or `staff`.
    pub source: String,
}

/// What an entitlement is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum GrantKind {
    Badge,
    Paint,
    Role,
}

/// How an account came to hold something.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum GrantSource {
    /// Given by staff (a cosmetic grant or a role assignment).
    Staff,
    /// Bought in the shop (`orderId`).
    Purchase,
    /// A redeemed code (`codeBatchId`).
    Code,
    /// An event.
    Event,
    /// The User role every account holds.
    Default,
    /// The owners' top role, from the owner list.
    Owner,
}

/// One badge, paint or role an account holds or held, and where it came from: one edge of the
/// entitlement graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct EntitlementGrant {
    pub kind: GrantKind,
    pub item_id: String,
    /// Empty when the item was deleted since.
    pub item_name: String,
    pub account_id: String,
    pub source: GrantSource,
    /// What staff wrote when they gave it.
    pub reason: Option<String>,
    /// The staff account that gave it; empty for purchases, codes and the built-in roles.
    pub granted_by: Option<String>,
    /// Empty when it isn't known (built-in roles; grants older than the audit log).
    pub granted_at: Option<String>,
    pub order_id: Option<String>,
    pub order_name: Option<String>,
    /// `paid`, `refunded` and so on.
    pub order_status: Option<String>,
    pub code_batch_id: Option<String>,
    pub code_batch_name: Option<String>,
    /// When it was taken away; empty while held.
    pub revoked_at: Option<String>,
    /// Who took it away; empty for refunds.
    pub revoked_by: Option<String>,
    pub revoke_reason: Option<String>,
    /// Held now.
    pub active: bool,
}

#[cfg(test)]
mod visibility_tests {
    use super::EmoteVisibility as V;

    #[test]
    fn emote_visibility_follows_both_fields() {
        // Named: wins.
        assert_eq!(
            V::after(V::Public, Some(V::Unlisted), None),
            Some(V::Unlisted)
        );
        // The old flag alone.
        assert_eq!(V::after(V::Unlisted, None, Some(true)), Some(V::Private));
        assert_eq!(V::after(V::Private, None, Some(false)), Some(V::Public));
        assert_eq!(V::after(V::Unlisted, None, Some(false)), Some(V::Unlisted));
        assert_eq!(V::after(V::Public, None, None), Some(V::Public));
        // Both, agreeing or not.
        assert_eq!(
            V::after(V::Public, Some(V::Private), Some(true)),
            Some(V::Private)
        );
        assert_eq!(
            V::after(V::Public, Some(V::Unlisted), Some(false)),
            Some(V::Unlisted)
        );
        assert_eq!(V::after(V::Public, Some(V::Public), Some(true)), None);
        assert_eq!(V::after(V::Public, Some(V::Private), Some(false)), None);
        assert_eq!(V::of(true, false).flags(), (true, false));
        assert_eq!(V::of(false, true), V::Unlisted);
        assert_eq!(" Unlisted ".parse::<V>(), Ok(V::Unlisted));
        assert!("hidden".parse::<V>().is_err());
    }
}

#[cfg(test)]
mod file_url_tests {
    use super::*;

    fn file(name: &str, format: ImageFormat) -> EmoteFile {
        EmoteFile {
            name: name.into(),
            format,
            scale: 1,
            width: 32,
            height: 32,
            size: 100,
            is_static: false,
            url: String::new(),
            mime: String::new(),
        }
    }

    #[test]
    fn files_get_their_full_cdn_url_and_media_type() {
        let files = EmoteFile::with_urls(
            vec![
                file("1x.webp", ImageFormat::Webp),
                file("1x_static.avif", ImageFormat::Avif),
            ],
            "https://cdn.cyex.tv/emotes/abc",
        );
        assert_eq!(files[0].url, "https://cdn.cyex.tv/emotes/abc/1x.webp");
        assert_eq!(files[0].mime, "image/webp");
        assert_eq!(
            files[1].url,
            "https://cdn.cyex.tv/emotes/abc/1x_static.avif"
        );
        assert_eq!(files[1].mime, "image/avif");
    }

    #[test]
    fn a_stored_entry_without_url_and_mime_still_reads_and_writes_without_them() {
        let stored = r#"{"name":"1x.webp","format":"webp","scale":1,"width":32,"height":32,"size":9,"is_static":false}"#;
        let f: EmoteFile = serde_json::from_str(stored).unwrap();
        assert!(f.url.is_empty() && f.mime.is_empty());
        let out = serde_json::to_value(&f).unwrap();
        assert!(out.get("url").is_none() && out.get("mime").is_none());
        let sent = EmoteFile::with_urls(vec![f], "https://cdn/x");
        let out = serde_json::to_value(&sent[0]).unwrap();
        assert_eq!(out["url"], "https://cdn/x/1x.webp");
        assert_eq!(out["mime"], "image/webp");
    }
}
