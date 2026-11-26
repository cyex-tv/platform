//! What the two upload routes answer with (API review 2026-11-26, item 9).
//!
//! `POST /v1/emotes` and `POST /v1/cosmetics/images` answer `202` with their usual object
//! (`Emote`, `CosmeticImage`) plus the fields of [`UploadResult`], so a client can handle both the
//! same way: `id`, `state`, `files` and `limits_used`. They share one error list too (below).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::cosmetics::CosmeticImage;
use crate::model::{Emote, EmoteFile};

/// The file limits an upload was checked under: the uploader's own, after their roles'
/// overrides (the same numbers as `uploadQuota` and `limits.account`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LimitsUsed {
    /// `emote`, `badge` or `paint`.
    pub kind: String,
    /// The most bytes a file may have.
    pub max_bytes: u64,
    /// The longest side in px.
    pub max_side: u32,
    /// The most frames of an animation.
    pub max_frames: u32,
}

/// The fields every upload answer has, whatever was uploaded. The routes answer with these next
/// to the full object, so this is the part a generic upload client reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UploadResult {
    /// The new object's ID (a Snowflake for an emote).
    pub id: String,
    /// An emote: `pending` (waiting for review), `approved`, `rejected` or `taken_down`. A badge
    /// or paint picture: `processing` while the worker makes its files, then `ready`.
    #[schema(example = "pending")]
    pub state: String,
    /// The files made so far; empty right after the upload, until the worker has run.
    pub files: Vec<EmoteFile>,
    /// What the upload was checked against.
    pub limits_used: LimitsUsed,
}

/// `POST /v1/emotes` answers: the emote, and the limits it was checked under.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EmoteUploaded {
    #[serde(flatten)]
    pub emote: Emote,
    pub limits_used: LimitsUsed,
}

/// `POST /v1/cosmetics/images` answers: the picture, its `state` and the limits it was checked
/// under.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CosmeticImageUploaded {
    #[serde(flatten)]
    pub image: CosmeticImage,
    /// `processing` until the files are made (`ready` is false), then `ready`.
    pub state: String,
    pub limits_used: LimitsUsed,
}
