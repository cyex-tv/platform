//! Emote transfer requests: an account editor asks, the owner accepts or declines (ADR 0014,
//! owner decision of 2026/11/19). The api checks who may do what; this only stores.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Request {
    pub id: Uuid,
    pub emote_id: Uuid,
    /// The emote's owner when it was asked for: the one who accepts.
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub requested_by: Uuid,
    /// `pending`, `accepted`, `declined`, `cancelled` or `expired`.
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
}

const REQUEST: &str = "id, emote_id, from_account_id, to_account_id, requested_by, state, \
                       created_at, expires_at, resolved_at, resolved_by";

/// A new pending request, or `None` when the emote has one open already.
pub async fn create(
    conn: &mut PgConnection,
    emote: Uuid,
    from: Uuid,
    to: Uuid,
    by: Uuid,
    expires_at: DateTime<Utc>,
) -> DbResult<Option<Request>> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO emote_transfer_requests
             (id, emote_id, from_account_id, to_account_id, requested_by, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (emote_id) WHERE state = 'pending' DO NOTHING
         RETURNING {REQUEST}"
    ))
    .bind(new_id())
    .bind(emote)
    .bind(from)
    .bind(to)
    .bind(by)
    .bind(expires_at)
    .fetch_optional(conn)
    .await?)
}

/// Marks pending requests past their time as `expired` (of one emote, or all) and returns them.
pub async fn expire_due(conn: &mut PgConnection, emote: Option<Uuid>) -> DbResult<Vec<Request>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emote_transfer_requests SET state = 'expired', resolved_at = now()
         WHERE state = 'pending' AND expires_at <= now() AND ($1::uuid IS NULL OR emote_id = $1)
         RETURNING {REQUEST}"
    ))
    .bind(emote)
    .fetch_all(conn)
    .await?)
}

/// One request, not locked: for finding its emote before taking locks in order.
pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Request>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {REQUEST} FROM emote_transfer_requests WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// One request, locked for the rest of the transaction.
pub async fn lock(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Request>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {REQUEST} FROM emote_transfer_requests WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Closes a pending request as `state`. `None` when it wasn't pending.
pub async fn resolve(
    conn: &mut PgConnection,
    id: Uuid,
    state: &str,
    by: Option<Uuid>,
) -> DbResult<Option<Request>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emote_transfer_requests SET state = $2, resolved_at = now(), resolved_by = $3
         WHERE id = $1 AND state = 'pending'
         RETURNING {REQUEST}"
    ))
    .bind(id)
    .bind(state)
    .bind(by)
    .fetch_optional(conn)
    .await?)
}

/// Closes the open request of an emote, if any (it moved or went away another way).
pub async fn close_open_for_emote(
    conn: &mut PgConnection,
    emote: Uuid,
    by: Option<Uuid>,
) -> DbResult<Vec<Request>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE emote_transfer_requests SET state = 'cancelled', resolved_at = now(), resolved_by = $2
         WHERE emote_id = $1 AND state = 'pending'
         RETURNING {REQUEST}"
    ))
    .bind(emote)
    .bind(by)
    .fetch_all(conn)
    .await?)
}

/// Open requests an account takes part in: to accept (its emotes) or that it made, newest first;
/// of one emote when given.
pub async fn open_for(
    conn: &mut PgConnection,
    account: Uuid,
    emote: Option<Uuid>,
) -> DbResult<Vec<Request>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {REQUEST} FROM emote_transfer_requests
         WHERE state = 'pending' AND expires_at > now()
           AND (from_account_id = $1 OR requested_by = $1)
           AND ($2::uuid IS NULL OR emote_id = $2)
         ORDER BY created_at DESC, id DESC
         LIMIT 200"
    ))
    .bind(account)
    .bind(emote)
    .fetch_all(conn)
    .await?)
}
