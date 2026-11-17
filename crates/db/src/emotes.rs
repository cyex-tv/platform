//! Emotes and their tags. New emotes start `pending` (D10).

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::EmoteState;
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Emote {
    pub id: Uuid,
    pub owner_account_id: Option<Uuid>,
    pub file_id: Option<Uuid>,
    pub name: String,
    pub zero_width: bool,
    pub private: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Default)]
pub struct NewEmote<'a> {
    pub owner_account_id: Uuid,
    pub file_id: Option<Uuid>,
    pub name: &'a str,
    pub animated: bool,
    pub zero_width: bool,
    pub private: bool,
    pub nsfw: bool,
}

/// Fields to change. `None` leaves a field as it is.
#[derive(Clone, Debug, Default)]
pub struct EmotePatch<'a> {
    pub name: Option<&'a str>,
    pub zero_width: Option<bool>,
    pub private: Option<bool>,
    pub nsfw: Option<bool>,
}

const EMOTE: &str = "id, owner_account_id, file_id, name, zero_width, private, nsfw, animated, \
                     state, created_at, updated_at, deleted_at";

pub async fn create(conn: &mut PgConnection, new: &NewEmote<'_>) -> DbResult<Emote> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO emotes (id, owner_account_id, file_id, name, animated, zero_width, private, nsfw)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING {EMOTE}"
    ))
    .bind(new_id())
    .bind(new.owner_account_id)
    .bind(new.file_id)
    .bind(new.name)
    .bind(new.animated)
    .bind(new.zero_width)
    .bind(new.private)
    .bind(new.nsfw)
    .fetch_one(conn)
    .await?)
}

/// An emote that isn't deleted, in any state.
pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes WHERE id = $1 AND deleted_at IS NULL"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Like `get`, but locks the row until the transaction ends. Use it to read the "before" of an
/// audited change.
pub async fn lock(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes WHERE id = $1 AND deleted_at IS NULL FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn update(conn: &mut PgConnection, id: Uuid, patch: &EmotePatch<'_>) -> DbResult<Emote> {
    sqlx::query_as(&format!(
        "UPDATE emotes
         SET name = coalesce($2, name),
             zero_width = coalesce($3, zero_width),
             private = coalesce($4, private),
             nsfw = coalesce($5, nsfw),
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(patch.name)
    .bind(patch.zero_width)
    .bind(patch.private)
    .bind(patch.nsfw)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Approves, rejects or takes down an emote. Audit and events are the caller's part.
pub async fn set_state(conn: &mut PgConnection, id: Uuid, state: EmoteState) -> DbResult<Emote> {
    sqlx::query_as(&format!(
        "UPDATE emotes SET state = $2, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(state)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Deletes an emote softly. It disappears from reads; the row stays for the audit trail.
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE emotes SET deleted_at = now(), updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// The emotes an account uploaded, newest first.
pub async fn list_by_owner(
    conn: &mut PgConnection,
    owner: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<Emote>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes
         WHERE owner_account_id = $1 AND deleted_at IS NULL
         ORDER BY id DESC
         LIMIT $2 OFFSET $3"
    ))
    .bind(owner)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?)
}

/// The review queue: emotes in `state`, oldest first, and how many there are.
pub async fn list_by_state(
    conn: &mut PgConnection,
    state: EmoteState,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<Emote>, i64)> {
    let items = sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes
         WHERE state = $1 AND deleted_at IS NULL
         ORDER BY created_at, id
         LIMIT $2 OFFSET $3"
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

/// An emote's tags, sorted.
pub async fn tags(conn: &mut PgConnection, id: Uuid) -> DbResult<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT tag FROM emote_tags WHERE emote_id = $1 ORDER BY tag")
            .bind(id)
            .fetch_all(conn)
            .await?,
    )
}

/// Replaces an emote's tags. Duplicates are dropped.
pub async fn set_tags(conn: &mut PgConnection, id: Uuid, tags: &[String]) -> DbResult<()> {
    let mut tx = conn.begin().await?;
    sqlx::query("DELETE FROM emote_tags WHERE emote_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO emote_tags (emote_id, tag)
         SELECT $1, tag FROM unnest($2::text[]) AS t (tag)
         ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(tags)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Approves or rejects a pending emote. `None` if it isn't pending (or doesn't exist), so two
/// moderators reviewing at once can't both win.
pub async fn review(
    conn: &mut PgConnection,
    id: Uuid,
    state: EmoteState,
    reviewer: Uuid,
    reason: Option<&str>,
) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emotes
         SET state = $2, reviewed_by = $3, reviewed_at = now(), review_reason = $4,
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL AND state = 'pending'
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(state)
    .bind(reviewer)
    .bind(reason)
    .fetch_optional(conn)
    .await?)
}

/// The sets an emote is in.
pub async fn sets_containing(conn: &mut PgConnection, id: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT set_id FROM emote_set_members WHERE emote_id = $1 ORDER BY set_id",
    )
    .bind(id)
    .fetch_all(conn)
    .await?)
}

/// Takes an emote out of every set (when it is deleted). Returns the sets it was in, with the
/// name it had there.
pub async fn remove_from_all_sets(
    conn: &mut PgConnection,
    id: Uuid,
) -> DbResult<Vec<(Uuid, String)>> {
    Ok(
        sqlx::query_as("DELETE FROM emote_set_members WHERE emote_id = $1 RETURNING set_id, name")
            .bind(id)
            .fetch_all(conn)
            .await?,
    )
}

/// How many emotes an account has made since `since`, deleted ones included (for the upload rate
/// limit, so deleting doesn't refill it).
pub async fn count_since(
    conn: &mut PgConnection,
    owner: Uuid,
    since: DateTime<Utc>,
) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM emotes WHERE owner_account_id = $1 AND created_at >= $2",
    )
    .bind(owner)
    .bind(since)
    .fetch_one(conn)
    .await?)
}

/// The emotes made from one file (the same image, uploaded by anyone).
pub async fn by_file(conn: &mut PgConnection, file_id: Uuid) -> DbResult<Vec<Emote>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes WHERE file_id = $1 AND deleted_at IS NULL ORDER BY id"
    ))
    .bind(file_id)
    .fetch_all(conn)
    .await?)
}
