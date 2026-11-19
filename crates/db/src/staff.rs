//! What staff read about an account beyond its own data: their own notes on it, and where each
//! cosmetic and role it holds (or held) came from (the entitlement graph).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

/// A note staff keep on an account. Only staff with `users.notes` see it.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Note {
    pub id: Uuid,
    pub account_id: Uuid,
    pub author_id: Option<Uuid>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

const NOTE: &str = "id, account_id, author_id, body, created_at";

pub async fn add_note(
    conn: &mut PgConnection,
    account_id: Uuid,
    author_id: Uuid,
    body: &str,
) -> DbResult<Note> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO staff_notes (id, account_id, author_id, body) VALUES ($1, $2, $3, $4)
         RETURNING {NOTE}"
    ))
    .bind(new_id())
    .bind(account_id)
    .bind(author_id)
    .bind(body)
    .fetch_one(conn)
    .await?)
}

/// An account's notes that aren't deleted, newest first (at most `limit`).
pub async fn notes(conn: &mut PgConnection, account_id: Uuid, limit: i64) -> DbResult<Vec<Note>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {NOTE} FROM staff_notes WHERE account_id = $1 AND deleted_at IS NULL
         ORDER BY created_at DESC, id DESC LIMIT $2"
    ))
    .bind(account_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn note(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Note>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {NOTE} FROM staff_notes WHERE id = $1 AND deleted_at IS NULL"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Marks a note deleted (it stays for the audit trail). False if it was gone already.
pub async fn delete_note(conn: &mut PgConnection, id: Uuid, by: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE staff_notes SET deleted_at = now(), deleted_by = $2
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

// The entitlement graph

/// One badge or paint an account holds now, with where it came from.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct CosmeticGrant {
    /// `badge` or `paint`.
    pub kind: String,
    pub item_id: Uuid,
    pub item_name: String,
    pub account_id: Uuid,
    /// `grant`, `purchase`, `event` or `code`.
    pub source: String,
    pub reason: Option<String>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub order_id: Option<Uuid>,
    pub order_name: Option<String>,
    pub order_status: Option<String>,
    pub batch_id: Option<Uuid>,
    pub batch_name: Option<String>,
}

/// The badges and paints held now, by an account and/or of one cosmetic (newest first, at most
/// `limit`).
pub async fn cosmetic_grants(
    conn: &mut PgConnection,
    account: Option<Uuid>,
    kind: Option<&str>,
    item: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<CosmeticGrant>> {
    let part = |kind: &str, table: &str, col: &str, items: &str| {
        format!(
            "SELECT '{kind}' AS kind, e.{col} AS item_id, x.name AS item_name, e.account_id,
                    e.source, e.reason, e.granted_by, e.granted_at,
                    e.order_id, o.item_name AS order_name, o.status AS order_status,
                    cb.id AS batch_id, cb.name AS batch_name
             FROM {table} e
             JOIN {items} x ON x.id = e.{col}
             LEFT JOIN shop_orders o ON o.id = e.order_id
             LEFT JOIN redeem_codes rc ON rc.id = e.code_id
             LEFT JOIN code_batches cb ON cb.id = rc.batch_id
             WHERE ($1::uuid IS NULL OR e.account_id = $1)
               AND ($2::text IS NULL OR $2 = '{kind}')
               AND ($3::uuid IS NULL OR e.{col} = $3)"
        )
    };
    let sql = format!(
        "{} UNION ALL {} ORDER BY granted_at DESC, item_id LIMIT $4",
        part("badge", "badge_entitlements", "badge_id", "badges"),
        part("paint", "paint_entitlements", "paint_id", "paints"),
    );
    Ok(sqlx::query_as(&sql)
        .bind(account)
        .bind(kind)
        .bind(item)
        .bind(limit)
        .fetch_all(conn)
        .await?)
}

/// A badge or paint an order gave and its refund took back.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct RefundedGrant {
    pub kind: String,
    pub item_id: Uuid,
    pub item_name: Option<String>,
    pub account_id: Uuid,
    pub order_id: Uuid,
    pub order_name: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
}

/// Cosmetics refunded orders gave, by an account and/or of one cosmetic.
pub async fn refunded_grants(
    conn: &mut PgConnection,
    account: Option<Uuid>,
    kind: Option<&str>,
    item: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<RefundedGrant>> {
    Ok(sqlx::query_as(
        "SELECT g.kind, g.cosmetic_id AS item_id,
                coalesce(b.name, p.name) AS item_name, g.account_id,
                o.id AS order_id, o.item_name AS order_name, o.paid_at, o.refunded_at
         FROM shop_order_entitlements g
         JOIN shop_orders o ON o.id = g.order_id AND o.status = 'refunded'
         LEFT JOIN badges b ON g.kind = 'badge' AND b.id = g.cosmetic_id
         LEFT JOIN paints p ON g.kind = 'paint' AND p.id = g.cosmetic_id
         WHERE ($1::uuid IS NULL OR g.account_id = $1)
           AND ($2::text IS NULL OR g.kind = $2)
           AND ($3::uuid IS NULL OR g.cosmetic_id = $3)
         ORDER BY o.refunded_at DESC NULLS LAST LIMIT $4",
    )
    .bind(account)
    .bind(kind)
    .bind(item)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// A role an account holds now, with who gave it.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct RoleGrant {
    pub role_id: Uuid,
    pub role_name: String,
    pub system: Option<String>,
    pub account_id: Uuid,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

/// The roles held now (through an assignment), by an account and/or of one role.
pub async fn role_grants(
    conn: &mut PgConnection,
    account: Option<Uuid>,
    role: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<RoleGrant>> {
    Ok(sqlx::query_as(
        "SELECT r.id AS role_id, r.name AS role_name, r.system, ar.account_id,
                ar.granted_by, ar.granted_at
         FROM account_roles ar JOIN roles r ON r.id = ar.role_id
         WHERE ($1::uuid IS NULL OR ar.account_id = $1) AND ($2::uuid IS NULL OR ar.role_id = $2)
         ORDER BY ar.granted_at DESC, r.position DESC LIMIT $3",
    )
    .bind(account)
    .bind(role)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// An audit row about a grant: `cosmetic.granted`, `cosmetic.revoked`, `role.assigned`,
/// `role.unassigned` or `role.deleted`.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct GrantEvent {
    pub action: String,
    pub actor: Option<Uuid>,
    pub target_id: String,
    pub before_json: Option<serde_json::Value>,
    pub after_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// The audit rows behind the graph. For an account (`account`, its text ID) every grant event
/// about it, a deleted role it held included; for one item (`item`, the cosmetic's or role's text
/// ID) every event about that item. Newest first, at most `limit`.
pub async fn grant_events(
    conn: &mut PgConnection,
    account: Option<&str>,
    item: Option<&str>,
    limit: i64,
) -> DbResult<Vec<GrantEvent>> {
    Ok(sqlx::query_as(
        "SELECT action, actor_account_id AS actor, target_id, before_json, after_json, created_at
         FROM audit_log
         WHERE action IN ('cosmetic.granted', 'cosmetic.revoked', 'role.assigned', 'role.unassigned', 'role.deleted')
           AND ($1::text IS NULL
                OR (target_kind = 'account' AND target_id = $1)
                OR (action = 'role.deleted' AND before_json->'members' ? $1))
           AND ($2::text IS NULL
                OR coalesce(after_json->>'cosmetic_id', before_json->>'cosmetic_id',
                            after_json->>'role_id', before_json->>'role_id') = $2
                OR (action = 'role.deleted' AND target_id = $2))
         ORDER BY created_at DESC, id DESC
         LIMIT $3",
    )
    .bind(account)
    .bind(item)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
