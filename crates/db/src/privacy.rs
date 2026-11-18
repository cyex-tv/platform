//! Deleting an account (`deleteAccount`): what goes, what stays, in statements the api runs in
//! one transaction with its events and audit row. The account row stays, marked deleted, so audit
//! rows and other people's records keep pointing at something; everything that says who it was
//! goes.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::DbResult;

/// Locks the account row for the deletion. `None` when it is unknown or already deleted.
pub async fn lock_live(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM accounts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(conn)
    .await?)
}

/// The account's emotes that stay: approved and in a set someone else owns (or the global set).
/// They lose their owner and show as made by a deleted user.
pub async fn emotes_to_keep(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT e.id FROM emotes e
         WHERE e.owner_account_id = $1 AND e.deleted_at IS NULL AND e.state = 'approved'
           AND EXISTS (SELECT 1 FROM emote_set_members m
                       JOIN emote_sets s ON s.id = m.set_id
                       WHERE m.emote_id = e.id
                         AND (s.owner_account_id IS NULL OR s.owner_account_id <> $1))
         ORDER BY e.id",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// Every other emote of the account, not deleted yet: these are taken down with the account.
pub async fn emotes_to_remove(
    conn: &mut PgConnection,
    account_id: Uuid,
    keep: &[Uuid],
) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM emotes
         WHERE owner_account_id = $1 AND deleted_at IS NULL AND NOT (id = ANY($2))
         ORDER BY id",
    )
    .bind(account_id)
    .bind(keep)
    .fetch_all(conn)
    .await?)
}

/// Kept emotes lose their owner.
pub async fn disown_emotes(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE emotes SET owner_account_id = NULL, updated_at = now() WHERE id = ANY($1)",
    )
    .bind(ids)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Removes everything else that belongs to or names the account, then marks it deleted. The
/// caller has taken the account's emotes out of sets and deleted or disowned them, and deleted
/// its sets and channels, publishing their events first.
pub async fn erase(conn: &mut PgConnection, account_id: Uuid) -> DbResult<()> {
    for sql in [
        "DELETE FROM set_editors WHERE account_id = $1",
        "DELETE FROM account_editors WHERE account_id = $1 OR editor_id = $1",
        "DELETE FROM cosmetic_selections WHERE account_id = $1",
        "DELETE FROM badge_entitlements WHERE account_id = $1",
        "DELETE FROM paint_entitlements WHERE account_id = $1",
        "DELETE FROM notifications WHERE account_id = $1",
        "DELETE FROM account_roles WHERE account_id = $1",
        "UPDATE reports SET reporter_id = NULL WHERE reporter_id = $1",
        "UPDATE emote_set_members SET added_by = NULL WHERE added_by = $1",
        "DELETE FROM sessions WHERE account_id = $1",
        "DELETE FROM identities WHERE account_id = $1",
        "UPDATE accounts SET deleted_at = now() WHERE id = $1",
    ] {
        sqlx::query(sql)
            .bind(account_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}
