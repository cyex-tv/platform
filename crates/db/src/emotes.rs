//! Emotes and their tags. New emotes start `pending` (D10).

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{ApprovalState, EmoteState};
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, retrying};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Emote {
    pub id: Uuid,
    pub owner_account_id: Option<Uuid>,
    pub file_id: Option<Uuid>,
    pub name: String,
    pub zero_width: bool,
    pub private: bool,
    /// Reachable by link, left out of listings (ADR 0020). Never with `private`.
    pub unlisted: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    /// Public listing and personal use (ADR 0039). `unlisted` follows `listing_review` (a trigger).
    pub listing_review: ApprovalState,
    pub personal_review: ApprovalState,
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
    /// Never with `private` (a CHECK refuses both).
    pub unlisted: bool,
    pub nsfw: bool,
}

/// Fields to change. `None` leaves a field as it is. `unlisted` is derived by a trigger from
/// `private` and `listing_review` (ADR 0039): whatever is written to it is replaced.
#[derive(Clone, Debug, Default)]
pub struct EmotePatch<'a> {
    pub name: Option<&'a str>,
    pub zero_width: Option<bool>,
    pub private: Option<bool>,
    pub unlisted: Option<bool>,
    pub nsfw: Option<bool>,
    pub listing_review: Option<ApprovalState>,
}

const EMOTE: &str = "id, owner_account_id, file_id, name, zero_width, private, unlisted, nsfw, \
                     animated, state, listing_review, personal_review, created_at, \
                     updated_at, deleted_at";

pub async fn create(conn: &mut PgConnection, new: &NewEmote<'_>) -> DbResult<Emote> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO emotes (id, owner_account_id, file_id, name, animated, zero_width, private,
                             nsfw, unlisted)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
    .bind(new.unlisted)
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
/// audited change. `FOR NO KEY UPDATE`: other writers of the emote wait, but a member insert's
/// foreign key check (`KEY SHARE`) doesn't, so `sets::add_member` holding a set never waits here
/// while this transaction's version trigger waits for that set.
pub async fn lock(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EMOTE} FROM emotes WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE"
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
             unlisted = coalesce($6, unlisted),
             listing_review = coalesce($7, listing_review),
             updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(patch.name)
    .bind(patch.zero_width)
    .bind(patch.private)
    .bind(patch.nsfw)
    .bind(patch.unlisted)
    .bind(patch.listing_review)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Sets an emote's state outright, with both approvals to match (ADR 0039): `approved` approves
/// the listing and personal use, `rejected` and `taken_down` deny both, `pending` makes both
/// pending. Takedowns use it; reviews go through [`review`], [`decide_listing`] and
/// [`decide_personal`]. Audit and events are the caller's part.
pub async fn set_state(conn: &mut PgConnection, id: Uuid, state: EmoteState) -> DbResult<Emote> {
    sqlx::query_as(&format!(
        "UPDATE emotes SET state = $2, updated_at = now(),
             listing_review = CASE $2 WHEN 'approved' THEN 'approved' WHEN 'pending' THEN 'pending'
                                      ELSE 'denied' END,
             personal_review = CASE $2 WHEN 'approved' THEN 'approved' WHEN 'pending' THEN 'pending'
                                       ELSE 'denied' END
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
/// moderators reviewing at once can't both win. Approving approves its public listing; rejecting
/// denies both approvals (ADR 0039).
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
             requeued_at = NULL, updated_at = now(),
             listing_review = CASE WHEN $2 = 'approved' THEN 'approved' ELSE 'denied' END,
             personal_review = CASE WHEN $2 = 'approved' THEN personal_review ELSE 'denied' END
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

/// Decides an emote's public listing (ADR 0039): approved lists it (unless private), denied
/// keeps it unlisted. Either way a pending emote becomes usable (`approved`): a denied listing
/// still goes in channel sets by link. With `only_pending`, only while its listing is pending.
/// `None` if nothing matched: rejected, taken down, deleted, or (with `only_pending`) decided.
pub async fn decide_listing(
    conn: &mut PgConnection,
    id: Uuid,
    to: ApprovalState,
    reviewer: Uuid,
    reason: Option<&str>,
    only_pending: bool,
) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emotes
         SET state = 'approved', listing_review = $2, reviewed_by = $3, reviewed_at = now(),
             review_reason = $4, requeued_at = NULL, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
           AND state IN ('pending', 'approved')
           AND (NOT $5 OR state = 'pending')
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(to)
    .bind(reviewer)
    .bind(reason)
    .bind(only_pending)
    .fetch_optional(conn)
    .await?)
}

/// Decides an emote's personal use (ADR 0039). Its state stays as it is. With `only_pending`,
/// only while its personal use is pending. `None` if nothing matched.
pub async fn decide_personal(
    conn: &mut PgConnection,
    id: Uuid,
    to: ApprovalState,
    reviewer: Uuid,
    reason: Option<&str>,
    only_pending: bool,
) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emotes
         SET personal_review = $2, personal_reviewed_by = $3, personal_reviewed_at = now(),
             personal_review_reason = $4, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
           AND state IN ('pending', 'approved')
           AND (NOT $5 OR personal_review = 'pending')
         RETURNING {EMOTE}"
    ))
    .bind(id)
    .bind(to)
    .bind(reviewer)
    .bind(reason)
    .bind(only_pending)
    .fetch_optional(conn)
    .await?)
}

/// Sends pending emotes to the end of the review queue (a reviewer took back an approval in its
/// undo time, ADR 0024). Returns the ones moved: emotes that aren't pending are left alone.
pub async fn requeue(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "UPDATE emotes SET requeued_at = clock_timestamp()
         WHERE id = ANY($1) AND deleted_at IS NULL AND state = 'pending'
         RETURNING id",
    )
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

/// Sends an approved emote back to review: `pending` again, with the last review cleared. `None`
/// if it wasn't approved (or doesn't exist).
pub async fn resubmit(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Emote>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emotes
         SET state = 'pending', reviewed_by = NULL, reviewed_at = NULL, review_reason = NULL,
             listing_review = 'pending', updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL AND state = 'approved'
         RETURNING {EMOTE}"
    ))
    .bind(id)
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
///
/// The sets are locked first, ordered by ID, and only then the member rows: the order every set
/// write uses (`sets::add_member` locks the set, then inserts). Deleting first and letting the
/// version trigger lock the sets afterwards could deadlock with an `add_member` of the same emote
/// to one of them. Tried again on a deadlock (see `retrying!`).
pub async fn remove_from_all_sets(
    conn: &mut PgConnection,
    id: Uuid,
) -> DbResult<Vec<(Uuid, String)>> {
    retrying!(conn, |tx| {
        sqlx::query(
            "SELECT s.id FROM emote_sets s
             WHERE s.id IN (SELECT set_id FROM emote_set_members WHERE emote_id = $1)
             ORDER BY s.id
             FOR NO KEY UPDATE",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        Ok(sqlx::query_as(
            "DELETE FROM emote_set_members WHERE emote_id = $1 RETURNING set_id, name",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?)
    })
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

/// The live emotes made from a file, with their owners: who to tell when its transcode failed.
pub async fn live_for_file(
    conn: &mut PgConnection,
    file_id: Uuid,
) -> DbResult<Vec<(Uuid, Option<Uuid>, String)>> {
    Ok(sqlx::query_as(
        "SELECT id, owner_account_id, name FROM emotes
         WHERE file_id = $1 AND deleted_at IS NULL ORDER BY id",
    )
    .bind(file_id)
    .fetch_all(conn)
    .await?)
}

/// How many of an account's emotes wait for review: what the review queue lists as pending
/// (`state = 'pending'`, not deleted), ones still transcoding included.
pub async fn count_pending(conn: &mut PgConnection, owner: Uuid) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM emotes
         WHERE owner_account_id = $1 AND state = 'pending' AND deleted_at IS NULL",
    )
    .bind(owner)
    .fetch_one(conn)
    .await?)
}

/// Serialises uploads that count against the same accounts' hourly limit, until the transaction
/// ends: a check made after this sees every upload that got in before. A transaction-scoped
/// advisory lock per account (in ID order, so two uploads can't wait on each other), so nothing
/// else on the account row waits.
pub async fn lock_upload_quota(conn: &mut PgConnection, accounts: &[Uuid]) -> DbResult<()> {
    let mut ids = accounts.to_vec();
    ids.sort_unstable();
    ids.dedup();
    for id in ids {
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended('emote_upload:' || $1::text, 0))",
        )
        .bind(id)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// How many emotes an account has uploaded since `since`, for itself or as an account editor for
/// someone else, deleted ones included (the uploader's side of the hourly limit).
pub async fn count_uploaded_since(
    conn: &mut PgConnection,
    uploader: Uuid,
    since: DateTime<Utc>,
) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM emotes
         WHERE ((owner_account_id = $1 AND uploaded_by IS NULL) OR uploaded_by = $1)
           AND created_at >= $2",
    )
    .bind(uploader)
    .bind(since)
    .fetch_one(conn)
    .await?)
}

/// Marks an emote as uploaded by an account editor of its owner (ADR 0014).
pub async fn set_uploader(conn: &mut PgConnection, id: Uuid, uploader: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE emotes SET uploaded_by = $2 WHERE id = $1")
        .bind(id)
        .bind(uploader)
        .execute(conn)
        .await?;
    Ok(())
}

/// Who uploaded each emote for its owner, for the emotes an account editor uploaded; the owner's
/// own uploads aren't in the map.
pub async fn uploaders(
    conn: &mut PgConnection,
    ids: &[Uuid],
) -> DbResult<std::collections::HashMap<Uuid, Uuid>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, uploaded_by FROM emotes WHERE id = ANY($1) AND uploaded_by IS NOT NULL",
    )
    .bind(ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Gives a live emote to another account. False when there was none.
pub async fn transfer(conn: &mut PgConnection, id: Uuid, to: Uuid) -> DbResult<bool> {
    Ok(sqlx::query(
        "UPDATE emotes SET owner_account_id = $2, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(to)
    .execute(conn)
    .await?
    .rows_affected()
        == 1)
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
