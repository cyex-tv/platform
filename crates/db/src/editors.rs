//! Account editors (ADR 0014): who may act on an account's sets and channels, with which flags.
//!
//! Flags are stored as their names (`emotes`, `sets`, `editors`, `profile`, `channels` and the
//! reserved emote and billing flags); the table's check refuses anything else. A grant covers
//! every set of the owner (`all_sets`), or only those in `account_editor_sets`. Grants of a blocked or deleted editor, and grants on a blocked or deleted
//! owner, are ignored by `permissions_of` and `list_for_editor`. Who may change a grant is the
//! api's check.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};

/// The most editors one account can have.
pub const MAX_PER_ACCOUNT: i64 = 50;

/// The most sets one grant can be limited to.
pub const MAX_SCOPED_SETS: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Grant {
    /// The account being edited (its owner).
    pub account_id: Uuid,
    pub editor_id: Uuid,
    pub permissions: Vec<String>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Every set of the owner, new ones included. False: only the sets in `account_editor_sets`.
    pub all_sets: bool,
}

const GRANT: &str =
    "account_id, editor_id, permissions, granted_by, granted_at, updated_at, all_sets";

/// Locks `owner`'s account row until the transaction ends, so changes to one account's editors
/// run one after another: each checks the manager's access and the grants it changes against
/// what the one before it committed. `FOR NO KEY UPDATE`: it waits for another editor change,
/// not for writes that only reference the account.
pub async fn lock_owner(conn: &mut PgConnection, owner: Uuid) -> DbResult<()> {
    sqlx::query("SELECT 1 FROM accounts WHERE id = $1 FOR NO KEY UPDATE")
        .bind(owner)
        .fetch_optional(conn)
        .await?;
    Ok(())
}

/// The flags `editor` holds on `account`, when both accounts are live (not deleted or blocked).
pub async fn permissions_of(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
) -> DbResult<Option<Vec<String>>> {
    Ok(sqlx::query_scalar(
        "SELECT e.permissions
         FROM account_editors e
         JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND (o.blocked_at IS NULL OR o.blocked_until <= now())
         JOIN accounts x ON x.id = e.editor_id AND x.deleted_at IS NULL AND (x.blocked_at IS NULL OR x.blocked_until <= now())
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
        "SELECT e.account_id, e.editor_id, e.permissions, e.granted_by, e.granted_at, e.updated_at,
                e.all_sets
         FROM account_editors e
         JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND (o.blocked_at IS NULL OR o.blocked_until <= now())
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

/// Whether `editor`'s live grant on `account` covers `set`: every set, or a list naming it.
/// False without a live grant.
pub async fn covers_set(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
    set: Uuid,
) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT e.all_sets OR EXISTS (
                SELECT 1 FROM account_editor_sets s
                WHERE s.account_id = e.account_id AND s.editor_id = e.editor_id AND s.set_id = $3
            )
         FROM account_editors e
         JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND (o.blocked_at IS NULL OR o.blocked_until <= now())
         JOIN accounts x ON x.id = e.editor_id AND x.deleted_at IS NULL AND (x.blocked_at IS NULL OR x.blocked_until <= now())
         WHERE e.account_id = $1 AND e.editor_id = $2",
    )
    .bind(account)
    .bind(editor)
    .bind(set)
    .fetch_optional(conn)
    .await?
    .unwrap_or(false))
}

/// The sets a limited grant names, by editor, for grants on `account`. Grants on every set are
/// missing from the map.
pub async fn scoped_sets_of_account(
    conn: &mut PgConnection,
    account: Uuid,
) -> DbResult<HashMap<Uuid, Vec<Uuid>>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT editor_id, set_id FROM account_editor_sets WHERE account_id = $1
         ORDER BY editor_id, set_id",
    )
    .bind(account)
    .fetch_all(conn)
    .await?;
    let mut out: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (editor, set) in rows {
        out.entry(editor).or_default().push(set);
    }
    Ok(out)
}

/// The sets a limited grant names, by owner, for grants held by `editor`.
pub async fn scoped_sets_of_editor(
    conn: &mut PgConnection,
    editor: Uuid,
) -> DbResult<HashMap<Uuid, Vec<Uuid>>> {
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT account_id, set_id FROM account_editor_sets WHERE editor_id = $1
         ORDER BY account_id, set_id",
    )
    .bind(editor)
    .fetch_all(conn)
    .await?;
    let mut out: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (account, set) in rows {
        out.entry(account).or_default().push(set);
    }
    Ok(out)
}

/// Limits a grant to `sets`, or opens it to every set (`None`). The caller checks that each set
/// is the owner's. Returns the grant as it is now.
pub async fn set_scope(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
    sets: Option<&[Uuid]>,
) -> DbResult<Grant> {
    let mut tx = conn.begin().await?;
    let grant: Grant = sqlx::query_as(&format!(
        "UPDATE account_editors SET all_sets = $3, updated_at = now()
         WHERE account_id = $1 AND editor_id = $2
         RETURNING {GRANT}"
    ))
    .bind(account)
    .bind(editor)
    .bind(sets.is_none())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DbError::NotFound)?;
    sqlx::query("DELETE FROM account_editor_sets WHERE account_id = $1 AND editor_id = $2")
        .bind(account)
        .bind(editor)
        .execute(&mut *tx)
        .await?;
    if let Some(sets) = sets {
        sqlx::query(
            "INSERT INTO account_editor_sets (account_id, editor_id, set_id)
             SELECT $1, $2, unnest($3::uuid[])
             ON CONFLICT DO NOTHING",
        )
        .bind(account)
        .bind(editor)
        .bind(sets)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(grant)
}

/// Accounts editing `account`, live ones only, oldest grant first: the public list (no flags).
pub async fn live_editors_of(conn: &mut PgConnection, account: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT e.editor_id
         FROM account_editors e
         JOIN accounts x ON x.id = e.editor_id AND x.deleted_at IS NULL AND (x.blocked_at IS NULL OR x.blocked_until <= now())
         WHERE e.account_id = $1
         ORDER BY e.granted_at, e.editor_id",
    )
    .bind(account)
    .fetch_all(conn)
    .await?)
}

/// The sets one limited grant names; empty for a grant on every set.
pub async fn scoped_sets(
    conn: &mut PgConnection,
    account: Uuid,
    editor: Uuid,
) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT set_id FROM account_editor_sets WHERE account_id = $1 AND editor_id = $2
         ORDER BY set_id",
    )
    .bind(account)
    .bind(editor)
    .fetch_all(conn)
    .await?)
}

/// What `editor` may do with each of `set_ids`, in one query (bulk reads, ADR 0032).
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromRow)]
pub struct SetAccess {
    pub set_id: Uuid,
    /// Owns the set or is one of its set editors (`sets::can_edit`).
    pub direct: bool,
    /// An account editor of the owner whose grant covers the set (`covers_set`).
    pub covered: bool,
    /// `covered`, with the `emotes` or `sets` flag: works on the set's emotes.
    pub works_on_sets: bool,
}

/// [`SetAccess`] of `editor` for every set of `set_ids` that exists.
pub async fn set_access(
    conn: &mut PgConnection,
    editor: Uuid,
    set_ids: &[Uuid],
) -> DbResult<Vec<SetAccess>> {
    if set_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as(
        "SELECT s.id AS set_id,
                (s.owner_account_id = $1
                 OR EXISTS (SELECT 1 FROM set_editors se
                            WHERE se.set_id = s.id AND se.account_id = $1)) AS direct,
                coalesce(g.covers, false) AS covered,
                coalesce(g.covers AND ('emotes' = ANY(g.permissions) OR 'sets' = ANY(g.permissions)),
                         false) AS works_on_sets
         FROM emote_sets s
         LEFT JOIN LATERAL (
             SELECT e.permissions,
                    (e.all_sets OR EXISTS (
                        SELECT 1 FROM account_editor_sets a
                        WHERE a.account_id = e.account_id AND a.editor_id = e.editor_id
                          AND a.set_id = s.id)) AS covers
             FROM account_editors e
             JOIN accounts o ON o.id = e.account_id AND o.deleted_at IS NULL AND (o.blocked_at IS NULL OR o.blocked_until <= now())
             JOIN accounts x ON x.id = e.editor_id AND x.deleted_at IS NULL AND (x.blocked_at IS NULL OR x.blocked_until <= now())
             WHERE e.account_id = s.owner_account_id AND e.editor_id = $1
         ) g ON true
         WHERE s.id = ANY($2)",
    )
    .bind(editor)
    .bind(set_ids)
    .fetch_all(conn)
    .await?)
}
