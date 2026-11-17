//! Batch reads for building API answers: a whole set with its emotes, files, tags and owners in a
//! few queries instead of a few per emote.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::EmoteState;
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// A set member joined with its emote and the emote's file. Deleted emotes are left out.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct MemberRow {
    pub set_id: Uuid,
    pub emote_id: Uuid,
    /// The name in the set.
    pub name: String,
    pub zero_width: bool,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub emote_owner: Option<Uuid>,
    pub emote_name: String,
    /// The emote's own zero-width flag; `zero_width` is the set's.
    pub emote_zero_width: bool,
    pub private: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    pub emote_created_at: DateTime<Utc>,
    pub sha256: Option<Vec<u8>>,
    pub variants: Option<Value>,
}

/// An emote with its file, as the API shows it.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct EmoteRow {
    pub id: Uuid,
    pub owner: Option<Uuid>,
    pub name: String,
    pub zero_width: bool,
    pub private: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    pub created_at: DateTime<Utc>,
    pub sha256: Option<Vec<u8>>,
    pub variants: Option<Value>,
}

impl MemberRow {
    /// The emote part of the row.
    pub fn emote(&self) -> EmoteRow {
        EmoteRow {
            id: self.emote_id,
            owner: self.emote_owner,
            name: self.emote_name.clone(),
            zero_width: self.emote_zero_width,
            private: self.private,
            nsfw: self.nsfw,
            animated: self.animated,
            state: self.state,
            created_at: self.emote_created_at,
            sha256: self.sha256.clone(),
            variants: self.variants.clone(),
        }
    }
}

const EMOTE_ROW: &str = "SELECT e.id, e.owner_account_id AS owner, e.name, e.zero_width, e.private,
            e.nsfw, e.animated, e.state, e.created_at, f.sha256, f.variants
     FROM emotes e
     LEFT JOIN files f ON f.id = e.file_id";

/// Emotes by ID, in no particular order. Deleted ones are left out.
pub async fn emotes(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<EmoteRow>> {
    Ok(sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.id = ANY($1) AND e.deleted_at IS NULL"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

/// Emotes in `state`, oldest first (the review queue), and how many there are.
pub async fn emotes_in_state(
    conn: &mut PgConnection,
    state: EmoteState,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let items = sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.state = $1 AND e.deleted_at IS NULL
         ORDER BY e.created_at, e.id LIMIT $2 OFFSET $3"
    ))
    .bind(state)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total =
        sqlx::query_scalar("SELECT count(*) FROM emotes WHERE state = $1 AND deleted_at IS NULL")
            .bind(state)
            .fetch_one(conn)
            .await?;
    Ok((items, total))
}

/// An account's own emotes in every state, newest first, and how many there are.
pub async fn emotes_by_owner(
    conn: &mut PgConnection,
    owner: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let items = sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.owner_account_id = $1 AND e.deleted_at IS NULL
         ORDER BY e.created_at DESC, e.id DESC LIMIT $2 OFFSET $3"
    ))
    .bind(owner)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM emotes WHERE owner_account_id = $1 AND deleted_at IS NULL",
    )
    .bind(owner)
    .fetch_one(conn)
    .await?;
    Ok((items, total))
}

const MEMBER_ROW: &str =
    "SELECT m.set_id, m.emote_id, m.name, m.zero_width, m.added_by, m.added_at,
                e.owner_account_id AS emote_owner, e.name AS emote_name,
                e.zero_width AS emote_zero_width, e.private, e.nsfw,
                e.animated, e.state, e.created_at AS emote_created_at,
                f.sha256, f.variants
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         LEFT JOIN files f ON f.id = e.file_id
";

/// The members of the given sets, oldest first within each set.
pub async fn members(conn: &mut PgConnection, set_ids: &[Uuid]) -> DbResult<Vec<MemberRow>> {
    Ok(sqlx::query_as(&format!(
        "{MEMBER_ROW} WHERE m.set_id = ANY($1) ORDER BY m.set_id, m.added_at, m.emote_id"
    ))
    .bind(set_ids)
    .fetch_all(conn)
    .await?)
}

/// One member of one set, if the emote is in it and not deleted.
pub async fn member(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_id: Uuid,
) -> DbResult<Option<MemberRow>> {
    Ok(sqlx::query_as(&format!(
        "{MEMBER_ROW} WHERE m.set_id = $1 AND m.emote_id = $2"
    ))
    .bind(set_id)
    .bind(emote_id)
    .fetch_optional(conn)
    .await?)
}

/// Tags of each emote, sorted.
pub async fn tags(
    conn: &mut PgConnection,
    emote_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, Vec<String>>> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT emote_id, tag FROM emote_tags WHERE emote_id = ANY($1) ORDER BY emote_id, tag",
    )
    .bind(emote_ids)
    .fetch_all(conn)
    .await?;
    let mut out: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, tag) in rows {
        out.entry(id).or_default().push(tag);
    }
    Ok(out)
}

/// How an account is shown to others: its first identity's name and avatar (Twitch first).
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Summary {
    pub account_id: Uuid,
    pub display_name: String,
    pub avatar: Option<String>,
}

/// Summaries of live accounts. Deleted accounts and accounts with no identity are left out.
pub async fn summaries(
    conn: &mut PgConnection,
    account_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, Summary>> {
    let rows: Vec<Summary> = sqlx::query_as(
        "SELECT DISTINCT ON (i.account_id) i.account_id, i.display_name, i.avatar
         FROM identities i
         JOIN accounts a ON a.id = i.account_id AND a.deleted_at IS NULL
         WHERE i.account_id = ANY($1)
         ORDER BY i.account_id, i.platform DESC",
    )
    .bind(account_ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(|s| (s.account_id, s)).collect())
}
