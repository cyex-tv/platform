//! Chat cosmetics (ADR 0015): images, badges, paints, entitlements and selections. See the
//! migration `20261121000001_cosmetics.sql`. The api checks permissions and input; the database
//! keeps a selection to owned cosmetics (foreign keys into the entitlement tables).

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{CosmeticKind, Platform};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

// Images

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Image {
    pub id: Uuid,
    /// `badge` or `paint`.
    pub kind: String,
    pub sha256: Vec<u8>,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
    pub animated: bool,
    /// The original's key (never public).
    pub storage_key: String,
    pub variants: Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewImage<'a> {
    pub kind: CosmeticKind,
    pub sha256: &'a [u8; 32],
    pub mime: &'a str,
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
    pub animated: bool,
    pub storage_key: &'a str,
    pub created_by: Option<Uuid>,
}

const IMAGE: &str = "id, kind, sha256, mime, width, height, frame_count, animated, storage_key, \
                     variants, created_by, created_at";

/// Stores an image, or returns the one of the same kind with the same bytes. True when new.
pub async fn insert_image(conn: &mut PgConnection, new: &NewImage<'_>) -> DbResult<(Image, bool)> {
    let inserted: Option<Image> = sqlx::query_as(&format!(
        "INSERT INTO cosmetic_images
             (id, kind, sha256, mime, width, height, frame_count, animated, storage_key, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT (kind, sha256) DO NOTHING
         RETURNING {IMAGE}"
    ))
    .bind(new_id())
    .bind(new.kind.as_str())
    .bind(&new.sha256[..])
    .bind(new.mime)
    .bind(new.width)
    .bind(new.height)
    .bind(new.frame_count)
    .bind(new.animated)
    .bind(new.storage_key)
    .bind(new.created_by)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(image) = inserted {
        return Ok((image, true));
    }
    let image = sqlx::query_as(&format!(
        "SELECT {IMAGE} FROM cosmetic_images WHERE kind = $1 AND sha256 = $2"
    ))
    .bind(new.kind.as_str())
    .bind(&new.sha256[..])
    .fetch_one(conn)
    .await?;
    Ok((image, false))
}

pub async fn get_image(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Image>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {IMAGE} FROM cosmetic_images WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn images(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<Image>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {IMAGE} FROM cosmetic_images WHERE id = ANY($1)"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

/// Records the files the worker made.
pub async fn set_image_variants(
    conn: &mut PgConnection,
    id: Uuid,
    variants: &Value,
) -> DbResult<bool> {
    let done = sqlx::query("UPDATE cosmetic_images SET variants = $2 WHERE id = $1")
        .bind(id)
        .bind(variants)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

fn name_conflict(err: sqlx::Error) -> DbError {
    match unique_violation(&err) {
        Some("badges_name" | "paints_name") => DbError::CosmeticNameTaken,
        _ => err.into(),
    }
}

// Badges

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Badge {
    pub id: Uuid,
    pub name: String,
    pub tooltip: String,
    pub image_id: Uuid,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const BADGE: &str = "id, name, tooltip, image_id, created_by, created_at, updated_at";

pub async fn create_badge(
    conn: &mut PgConnection,
    name: &str,
    tooltip: &str,
    image_id: Uuid,
    created_by: Option<Uuid>,
) -> DbResult<Badge> {
    sqlx::query_as(&format!(
        "INSERT INTO badges (id, name, tooltip, image_id, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {BADGE}"
    ))
    .bind(new_id())
    .bind(name)
    .bind(tooltip)
    .bind(image_id)
    .bind(created_by)
    .fetch_one(conn)
    .await
    .map_err(name_conflict)
}

/// Changes the fields given.
pub async fn update_badge(
    conn: &mut PgConnection,
    id: Uuid,
    name: Option<&str>,
    tooltip: Option<&str>,
    image_id: Option<Uuid>,
) -> DbResult<Badge> {
    sqlx::query_as(&format!(
        "UPDATE badges SET name = coalesce($2, name), tooltip = coalesce($3, tooltip),
                           image_id = coalesce($4, image_id), updated_at = now()
         WHERE id = $1
         RETURNING {BADGE}"
    ))
    .bind(id)
    .bind(name)
    .bind(tooltip)
    .bind(image_id)
    .fetch_optional(conn)
    .await
    .map_err(name_conflict)?
    .ok_or(DbError::NotFound)
}

/// Deletes a badge. Its entitlements go with it, and every selection of it is cleared.
pub async fn delete_badge(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM badges WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn get_badge(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Badge>> {
    Ok(
        sqlx::query_as(&format!("SELECT {BADGE} FROM badges WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Every badge, oldest first (or only `ids` when given).
pub async fn badges(conn: &mut PgConnection, ids: Option<&[Uuid]>) -> DbResult<Vec<Badge>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {BADGE} FROM badges WHERE $1::uuid[] IS NULL OR id = ANY($1) ORDER BY id"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

// Paints

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Paint {
    pub id: Uuid,
    pub name: String,
    /// `linear_gradient`, `radial_gradient` or `image`.
    pub kind: String,
    pub angle: i32,
    pub repeat: bool,
    /// `circle` or `ellipse`.
    pub shape: String,
    pub color: Option<String>,
    pub stops: Value,
    pub shadows: Value,
    pub image_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A paint's fields, checked by the api.
#[derive(Clone, Debug)]
pub struct PaintFields<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub angle: i32,
    pub repeat: bool,
    pub shape: &'a str,
    pub color: Option<&'a str>,
    pub stops: &'a Value,
    pub shadows: &'a Value,
    pub image_id: Option<Uuid>,
}

const PAINT: &str = "id, name, kind, angle, repeat, shape, color, stops, shadows, image_id, \
                     created_by, created_at, updated_at";

pub async fn create_paint(
    conn: &mut PgConnection,
    fields: &PaintFields<'_>,
    created_by: Option<Uuid>,
) -> DbResult<Paint> {
    sqlx::query_as(&format!(
        "INSERT INTO paints (id, name, kind, angle, repeat, shape, color, stops, shadows, image_id, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING {PAINT}"
    ))
    .bind(new_id())
    .bind(fields.name)
    .bind(fields.kind)
    .bind(fields.angle)
    .bind(fields.repeat)
    .bind(fields.shape)
    .bind(fields.color)
    .bind(fields.stops)
    .bind(fields.shadows)
    .bind(fields.image_id)
    .bind(created_by)
    .fetch_one(conn)
    .await
    .map_err(name_conflict)
}

/// Replaces every field of a paint.
pub async fn replace_paint(
    conn: &mut PgConnection,
    id: Uuid,
    fields: &PaintFields<'_>,
) -> DbResult<Paint> {
    sqlx::query_as(&format!(
        "UPDATE paints SET name = $2, kind = $3, angle = $4, repeat = $5, shape = $6, color = $7,
                           stops = $8, shadows = $9, image_id = $10, updated_at = now()
         WHERE id = $1
         RETURNING {PAINT}"
    ))
    .bind(id)
    .bind(fields.name)
    .bind(fields.kind)
    .bind(fields.angle)
    .bind(fields.repeat)
    .bind(fields.shape)
    .bind(fields.color)
    .bind(fields.stops)
    .bind(fields.shadows)
    .bind(fields.image_id)
    .fetch_optional(conn)
    .await
    .map_err(name_conflict)?
    .ok_or(DbError::NotFound)
}

/// Deletes a paint, its entitlements, and every selection of it.
pub async fn delete_paint(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM paints WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn get_paint(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Paint>> {
    Ok(
        sqlx::query_as(&format!("SELECT {PAINT} FROM paints WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Every paint, oldest first (or only `ids` when given).
pub async fn paints(conn: &mut PgConnection, ids: Option<&[Uuid]>) -> DbResult<Vec<Paint>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {PAINT} FROM paints WHERE $1::uuid[] IS NULL OR id = ANY($1) ORDER BY id"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

// Entitlements

/// The entitlement table and its cosmetic column for a kind. Constants only, never input.
fn table(kind: CosmeticKind) -> (&'static str, &'static str) {
    match kind {
        CosmeticKind::Badge => ("badge_entitlements", "badge_id"),
        CosmeticKind::Paint => ("paint_entitlements", "paint_id"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct EntitlementRow {
    pub account_id: Uuid,
    pub cosmetic_id: Uuid,
    pub source: String,
    pub reason: Option<String>,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

/// Gives `account` the cosmetic. False when they had it already (nothing changes then).
pub async fn grant(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
    cosmetic: Uuid,
    source: &str,
    reason: Option<&str>,
    granted_by: Option<Uuid>,
) -> DbResult<bool> {
    let (table, column) = table(kind);
    let done = sqlx::query(&format!(
        "INSERT INTO {table} (account_id, {column}, source, reason, granted_by)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT DO NOTHING"
    ))
    .bind(account)
    .bind(cosmetic)
    .bind(source)
    .bind(reason)
    .bind(granted_by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Takes the cosmetic away and returns what was removed. A selection of it is cleared.
pub async fn revoke(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
    cosmetic: Uuid,
) -> DbResult<Option<EntitlementRow>> {
    let (table, column) = table(kind);
    Ok(sqlx::query_as(&format!(
        "DELETE FROM {table} WHERE account_id = $1 AND {column} = $2
         RETURNING account_id, {column} AS cosmetic_id, source, reason, granted_by, granted_at"
    ))
    .bind(account)
    .bind(cosmetic)
    .fetch_optional(conn)
    .await?)
}

/// The cosmetics of a kind an account owns, oldest grant first.
pub async fn owned(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
) -> DbResult<Vec<Uuid>> {
    let (table, column) = table(kind);
    Ok(sqlx::query_scalar(&format!(
        "SELECT {column} FROM {table} WHERE account_id = $1 ORDER BY granted_at, {column}"
    ))
    .bind(account)
    .fetch_all(conn)
    .await?)
}

pub async fn owns(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
    cosmetic: Uuid,
) -> DbResult<bool> {
    let (table, column) = table(kind);
    Ok(sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM {table} WHERE account_id = $1 AND {column} = $2)"
    ))
    .bind(account)
    .bind(cosmetic)
    .fetch_one(conn)
    .await?)
}

/// Entitlements of a kind, newest first, narrowed to an account and/or a cosmetic, with the total.
pub async fn entitlements(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Option<Uuid>,
    cosmetic: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EntitlementRow>, i64)> {
    let (table, column) = table(kind);
    let filter =
        format!("($1::uuid IS NULL OR account_id = $1) AND ($2::uuid IS NULL OR {column} = $2)");
    let rows = sqlx::query_as(&format!(
        "SELECT account_id, {column} AS cosmetic_id, source, reason, granted_by, granted_at
         FROM {table} WHERE {filter}
         ORDER BY granted_at DESC, account_id
         LIMIT $3 OFFSET $4"
    ))
    .bind(account)
    .bind(cosmetic)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE {filter}"))
        .bind(account)
        .bind(cosmetic)
        .fetch_one(conn)
        .await?;
    Ok((rows, total))
}

// Selections

/// What an account shows: (badge, paint).
pub async fn selection(
    conn: &mut PgConnection,
    account: Uuid,
) -> DbResult<(Option<Uuid>, Option<Uuid>)> {
    Ok(
        sqlx::query_as("SELECT badge_id, paint_id FROM cosmetic_selections WHERE account_id = $1")
            .bind(account)
            .fetch_optional(conn)
            .await?
            .unwrap_or((None, None)),
    )
}

/// Sets what an account shows. Both must be owned (`NotOwned` otherwise).
pub async fn select(
    conn: &mut PgConnection,
    account: Uuid,
    badge: Option<Uuid>,
    paint: Option<Uuid>,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO cosmetic_selections (account_id, badge_id, paint_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (account_id) DO UPDATE
             SET badge_id = EXCLUDED.badge_id, paint_id = EXCLUDED.paint_id, updated_at = now()",
    )
    .bind(account)
    .bind(badge)
    .bind(paint)
    .execute(conn)
    .await
    .map_err(|err| match err.as_database_error() {
        Some(db) if db.is_foreign_key_violation() => DbError::NotOwned,
        _ => err.into(),
    })?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct UserSelection {
    pub platform_user_id: String,
    pub badge_id: Option<Uuid>,
    pub paint_id: Option<Uuid>,
}

/// What the given chatters show, for those showing anything. Blocked and deleted accounts show
/// nothing.
/// What live accounts (not deleted, not blocked) show, for those showing something.
pub async fn selections_for_accounts(
    conn: &mut PgConnection,
    account_ids: &[Uuid],
) -> DbResult<Vec<(Uuid, Option<Uuid>, Option<Uuid>)>> {
    Ok(sqlx::query_as(
        "SELECT s.account_id, s.badge_id, s.paint_id
         FROM cosmetic_selections s
         JOIN accounts a ON a.id = s.account_id AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         WHERE s.account_id = ANY($1) AND (s.badge_id IS NOT NULL OR s.paint_id IS NOT NULL)
         ORDER BY s.account_id",
    )
    .bind(account_ids)
    .fetch_all(conn)
    .await?)
}

pub async fn selections_for_users(
    conn: &mut PgConnection,
    platform: Platform,
    platform_user_ids: &[String],
) -> DbResult<Vec<UserSelection>> {
    Ok(sqlx::query_as(
        "SELECT i.platform_user_id, s.badge_id, s.paint_id
         FROM identities i
         JOIN accounts a ON a.id = i.account_id AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         JOIN cosmetic_selections s ON s.account_id = i.account_id
         WHERE i.platform = $1
           AND i.platform_user_id = ANY($2)
           AND (s.badge_id IS NOT NULL OR s.paint_id IS NOT NULL)
         ORDER BY i.platform_user_id",
    )
    .bind(platform)
    .bind(platform_user_ids)
    .fetch_all(conn)
    .await?)
}
