//! Roles, their permissions and who holds them (ADR 0010).
//!
//! This module stores and reads. The rules on top (the owner has everything, no escalation, no
//! lock-out) are Phase 2 PR 4, in `crates/auth`; `highest_position` and `holders_of` are what they
//! need from here.

use chrono::{DateTime, Utc};
use cyex_tv_shared::permissions;
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    /// Shown on its holders' public profiles.
    pub public: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct NewRole<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub position: i32,
    pub permissions: &'a [&'a str],
    pub created_by: Option<Uuid>,
}

/// Fields to change. For `description`, `Some(None)` clears it.
#[derive(Clone, Copy, Debug, Default)]
pub struct RolePatch<'a> {
    pub name: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub position: Option<i32>,
}

const ROLE: &str = "id, name, description, position, created_by, created_at, public";

fn check_known(perms: &[&str]) -> DbResult<()> {
    match perms.iter().find(|p| !permissions::is_known(p)) {
        Some(unknown) => Err(DbError::UnknownPermission((*unknown).to_owned())),
        None => Ok(()),
    }
}

fn role_conflict(err: sqlx::Error) -> DbError {
    match unique_violation(&err) {
        Some("roles_name") => DbError::RoleNameTaken,
        _ => err.into(),
    }
}

/// Makes a role with its permissions. Names are unique ignoring case.
pub async fn create(conn: &mut PgConnection, new: &NewRole<'_>) -> DbResult<Role> {
    check_known(new.permissions)?;
    let mut tx = conn.begin().await?;
    let role: Role = sqlx::query_as(&format!(
        "INSERT INTO roles (id, name, description, position, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {ROLE}"
    ))
    .bind(new_id())
    .bind(new.name)
    .bind(new.description)
    .bind(new.position)
    .bind(new.created_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(role_conflict)?;
    insert_permissions(&mut tx, role.id, new.permissions).await?;
    tx.commit().await?;
    Ok(role)
}

async fn insert_permissions(
    conn: &mut PgConnection,
    role_id: Uuid,
    perms: &[&str],
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO role_permissions (role_id, permission)
         SELECT $1, p FROM unnest($2::text[]) AS t (p)
         ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .bind(perms)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update(conn: &mut PgConnection, id: Uuid, patch: &RolePatch<'_>) -> DbResult<Role> {
    sqlx::query_as(&format!(
        "UPDATE roles
         SET name = coalesce($2, name),
             description = CASE WHEN $3 THEN $4 ELSE description END,
             position = coalesce($5, position)
         WHERE id = $1
         RETURNING {ROLE}"
    ))
    .bind(id)
    .bind(patch.name)
    .bind(patch.description.is_some())
    .bind(patch.description.flatten())
    .bind(patch.position)
    .fetch_optional(conn)
    .await
    .map_err(role_conflict)?
    .ok_or(DbError::NotFound)
}

/// Replaces a role's permissions.
pub async fn set_permissions(conn: &mut PgConnection, id: Uuid, perms: &[&str]) -> DbResult<()> {
    check_known(perms)?;
    let mut tx = conn.begin().await?;
    let locked: Option<Uuid> = sqlx::query_scalar("SELECT id FROM roles WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
    if locked.is_none() {
        return Err(DbError::NotFound);
    }
    sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    insert_permissions(&mut tx, id, perms).await?;
    tx.commit().await?;
    Ok(())
}

/// Deletes a role; its holders lose it.
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Role>> {
    Ok(
        sqlx::query_as(&format!("SELECT {ROLE} FROM roles WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Every role, most senior first.
pub async fn list(conn: &mut PgConnection) -> DbResult<Vec<Role>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ROLE} FROM roles ORDER BY position DESC, lower(name)"
    ))
    .fetch_all(conn)
    .await?)
}

/// A role's permissions, sorted.
pub async fn permissions(conn: &mut PgConnection, role_id: Uuid) -> DbResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT permission FROM role_permissions WHERE role_id = $1 ORDER BY permission",
    )
    .bind(role_id)
    .fetch_all(conn)
    .await?)
}

/// Gives an account a role. False if it already had it.
pub async fn assign(
    conn: &mut PgConnection,
    account_id: Uuid,
    role_id: Uuid,
    granted_by: Option<Uuid>,
) -> DbResult<bool> {
    let done = sqlx::query(
        "INSERT INTO account_roles (account_id, role_id, granted_by) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING",
    )
    .bind(account_id)
    .bind(role_id)
    .bind(granted_by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn unassign(conn: &mut PgConnection, account_id: Uuid, role_id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM account_roles WHERE account_id = $1 AND role_id = $2")
        .bind(account_id)
        .bind(role_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// An account's roles, most senior first.
pub async fn roles_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Role>> {
    Ok(sqlx::query_as(
        "SELECT r.id, r.name, r.description, r.position, r.created_by, r.created_at, r.public
         FROM roles r
         JOIN account_roles ar ON ar.role_id = r.id
         WHERE ar.account_id = $1
         ORDER BY r.position DESC, lower(r.name)",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// Shows a role on its holders' public profiles, or stops.
pub async fn set_public(conn: &mut PgConnection, id: Uuid, public: bool) -> DbResult<Role> {
    sqlx::query_as(&format!(
        "UPDATE roles SET public = $2 WHERE id = $1 RETURNING {ROLE}"
    ))
    .bind(id)
    .bind(public)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// An account's public roles, most senior first.
pub async fn public_roles_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Role>> {
    Ok(roles_of(conn, account_id)
        .await?
        .into_iter()
        .filter(|r| r.public)
        .collect())
}

/// The union of an account's roles' permissions, sorted.
pub async fn permissions_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT rp.permission
         FROM role_permissions rp
         JOIN account_roles ar ON ar.role_id = rp.role_id
         WHERE ar.account_id = $1
         ORDER BY rp.permission",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// The position of the account's most senior role, if it has any.
pub async fn highest_position(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Option<i32>> {
    Ok(sqlx::query_scalar(
        "SELECT max(r.position)
         FROM roles r
         JOIN account_roles ar ON ar.role_id = r.id
         WHERE ar.account_id = $1",
    )
    .bind(account_id)
    .fetch_one(conn)
    .await?)
}

/// The accounts that hold a permission through any role, sorted.
pub async fn holders_of(conn: &mut PgConnection, permission: &str) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT DISTINCT ar.account_id
         FROM account_roles ar
         JOIN role_permissions rp ON rp.role_id = ar.role_id
         WHERE rp.permission = $1
         ORDER BY ar.account_id",
    )
    .bind(permission)
    .fetch_all(conn)
    .await?)
}

/// The accounts that hold a role.
pub async fn members(conn: &mut PgConnection, role_id: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT account_id FROM account_roles WHERE role_id = $1 ORDER BY granted_at, account_id",
    )
    .bind(role_id)
    .fetch_all(conn)
    .await?)
}
