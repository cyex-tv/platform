//! Account editors (ADR 0014): who may act on an account's sets and channels, with which flags.
//!
//! Flags are stored as their names (`emotes`, `sets`, `editors`, `profile`); the table's check
//! refuses anything else. Grants of a blocked or deleted editor, and grants on a blocked or deleted
//! owner, are ignored by `permissions_of` and `list_for_editor`. Who may change a grant is the
//! api's check.

use chrono::{DateTime, Utc};
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};

/// The most editors one account can have.
pub const MAX_PER_ACCOUNT: i64 = 20;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Grant {
    /// The account being edited (its owner).
    pub account_id: Uuid,
    pub editor_id: Uuid,
    pub permissions: Vec<String>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const GRANT: &str = "account_id, editor_id, permissions, granted_by, granted_at, updated_at";

/// The flags `editor` holds on `account`, when both accounts are live (not deleted or blocked).
pub async fn permissions_of(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
) -> DbResult<Option<Vec<String>>> {
    Ok(sqlx::query_scalar(
        "SELECT e.permissions
         FROM account_editors e
         JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND o.blocked_at IS NULL
         JOIN accounts x ON x.id = e.editor_id AND x.deleted_at IS NULL AND x.blocked_at IS NULL
         WHERE e.account_id = $1 AND e.editor_id = $2",
    )
    .bind(account)
    .bind(editor)
    .fetch_optional(conn)
    .await?)
}

/// One grant as stored, whatever the state of either account.
pub async fn get(conn: &mut PgConnection, account: Uuid, editor: Uuid) -> DbResult<Option<Grant>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {GRANT} FROM account_editors WHERE account_id = $1 AND editor_id = $2"
    ))
    .bind(account)
    .bind(editor)
    .fetch_optional(conn)
    .await?)
}

/// An account's editors, oldest grant first. Blocked editors are listed, so they can be removed.
pub async fn list_for_account(conn: &mut PgConnection, account: Uuid) -> DbResult<Vec<Grant>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {GRANT} FROM account_editors WHERE account_id = $1 ORDER BY granted_at, editor_id"
    ))
    .bind(account)
    .fetch_all(conn)
    .await?)
}

/// The live accounts `editor` edits, oldest grant first.
pub async fn list_for_editor(conn: &mut PgConnection, editor: Uuid) -> DbResult<Vec<Grant>> {
    Ok(sqlx::query_as(
        "SELECT e.account_id, e.editor_id, e.permissions, e.granted_by, e.granted_at, e.updated_at
         FROM account_editors e
         JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND o.blocked_at IS NULL
         WHERE e.editor_id = $1
         ORDER BY e.granted_at, e.account_id",
    )
    .bind(editor)
    .fetch_all(conn)
    .await?)
}

/// Adds an editor, holding the account row so the limit can't be raced past.
pub async fn add(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
    permissions: &[String],
    granted_by: Option<Uuid>,
) -> DbResult<Grant> {
    let mut tx = conn.begin().await?;
    sqlx::query("SELECT id FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(account)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DbError::NotFound)?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_editors WHERE account_id = $1")
            .bind(account)
            .fetch_one(&mut *tx)
            .await?;
    if count >= MAX_PER_ACCOUNT {
        return Err(DbError::TooManyEditors);
    }
    let grant = sqlx::query_as(&format!(
        "INSERT INTO account_editors (account_id, editor_id, permissions, granted_by)
         VALUES ($1, $2, $3, $4)
         RETURNING {GRANT}"
    ))
    .bind(account)
    .bind(editor)
    .bind(permissions)
    .bind(granted_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match unique_violation(&err) {
        Some("account_editors_pkey") => DbError::AlreadyEditor,
        _ => err.into(),
    })?;
    tx.commit().await?;
    Ok(grant)
}

pub async fn update(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
    permissions: &[String],
) -> DbResult<Grant> {
    sqlx::query_as(&format!(
        "UPDATE account_editors SET permissions = $3, updated_at = now()
         WHERE account_id = $1 AND editor_id = $2
         RETURNING {GRANT}"
    ))
    .bind(account)
    .bind(editor)
    .bind(permissions)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// The removed grant, or `None` when there was none.
pub async fn remove(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
) -> DbResult<Option<Grant>> {
    Ok(sqlx::query_as(&format!(
        "DELETE FROM account_editors WHERE account_id = $1 AND editor_id = $2 RETURNING {GRANT}"
    ))
    .bind(account)
    .bind(editor)
    .fetch_optional(conn)
    .await?)
}
