//! Roles, their permissions and who holds them (ADR 0010).
//!
//! This module stores and reads. The rules on top (the owner has everything, no escalation, no
//! lock-out) are in `crates/auth`; `highest_position` and `holders_of` are what they need from
//! here.

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

use cyex_tv_shared::permissions::{self, Decision, Effect};
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
    /// A built-in role: `user`, `admin` or `root` (see the migration `default_roles`).
    pub system: Option<String>,
    /// Never shown publicly, even when `public`, and left out wherever roles rank people
    /// (migration `role_deny_invisible`).
    pub invisible: bool,
    /// Optional `#rrggbb` (lower case; migration `role_color`).
    pub color: Option<String>,
}

impl Role {
    pub fn is(&self, system: &str) -> bool {
        self.system.as_deref() == Some(system)
    }
}

/// The built-in role every account holds without an assignment.
pub const SYSTEM_USER: &str = "user";
/// The built-in "Admin" role.
pub const SYSTEM_ADMIN: &str = "admin";
/// The top role: the owners hold it, nobody else sees it.
pub const SYSTEM_ROOT: &str = "root";
/// The top role's position: the highest the column allows. No other role reaches it.
pub const ROOT_POSITION: i32 = 1_000_000;

#[derive(Clone, Copy, Debug)]
pub struct NewRole<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub position: i32,
    /// What it allows (`set_effects` adds denies).
    pub permissions: &'a [&'a str],
    pub created_by: Option<Uuid>,
}

/// Fields to change. For `description`, `Some(None)` clears it.
#[derive(Clone, Copy, Debug, Default)]
pub struct RolePatch<'a> {
    pub name: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub position: Option<i32>,
    /// `Some(None)` clears the colour.
    pub color: Option<Option<&'a str>>,
}

const ROLE: &str =
    "id, name, description, position, created_by, created_at, public, system, invisible, color";

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
    insert_permissions(&mut tx, role.id, new.permissions, Effect::Allow).await?;
    tx.commit().await?;
    Ok(role)
}

async fn insert_permissions(
    conn: &mut PgConnection,
    role_id: Uuid,
    perms: &[&str],
    effect: Effect,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO role_permissions (role_id, permission, effect)
         SELECT $1, p, $3 FROM unnest($2::text[]) AS t (p)
         ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .bind(perms)
    .bind(effect.as_str())
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn update(conn: &mut PgConnection, id: Uuid, patch: &RolePatch<'_>) -> DbResult<Role> {
    sqlx::query_as(&format!(
        "UPDATE roles
         SET name = coalesce($2, name),
             description = CASE WHEN $3 THEN $4 ELSE description END,
             position = coalesce($5, position),
             color = CASE WHEN $6 THEN $7 ELSE color END
         WHERE id = $1
         RETURNING {ROLE}"
    ))
    .bind(id)
    .bind(patch.name)
    .bind(patch.description.is_some())
    .bind(patch.description.flatten())
    .bind(patch.position)
    .bind(patch.color.is_some())
    .bind(patch.color.flatten())
    .fetch_optional(conn)
    .await
    .map_err(role_conflict)?
    .ok_or(DbError::NotFound)
}

/// Replaces a role's permissions: it allows `perms` and leaves the rest to the roles below.
pub async fn set_permissions(conn: &mut PgConnection, id: Uuid, perms: &[&str]) -> DbResult<()> {
    set_effects(conn, id, perms, &[]).await
}

/// Replaces what a role allows and denies; it leaves everything else to the roles below. The api
/// refuses a name in both lists before it gets here.
pub async fn set_effects(
    conn: &mut PgConnection,
    id: Uuid,
    allows: &[&str],
    denies: &[&str],
) -> DbResult<()> {
    check_known(allows)?;
    check_known(denies)?;
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
    insert_permissions(&mut tx, id, allows, Effect::Allow).await?;
    insert_permissions(&mut tx, id, denies, Effect::Deny).await?;
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

/// What a role allows, sorted.
pub async fn permissions(conn: &mut PgConnection, role_id: Uuid) -> DbResult<Vec<String>> {
    with_effect(conn, role_id, Effect::Allow).await
}

/// What a role denies, sorted.
pub async fn denies(conn: &mut PgConnection, role_id: Uuid) -> DbResult<Vec<String>> {
    with_effect(conn, role_id, Effect::Deny).await
}

async fn with_effect(
    conn: &mut PgConnection,
    role_id: Uuid,
    effect: Effect,
) -> DbResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT permission FROM role_permissions WHERE role_id = $1 AND effect = $2
         ORDER BY permission",
    )
    .bind(role_id)
    .bind(effect.as_str())
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
        "SELECT r.id, r.name, r.description, r.position, r.created_by, r.created_at, r.public,
                r.system, r.invisible, r.color
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

/// Hides a role from everyone but staff and its holders, or stops.
pub async fn set_invisible(conn: &mut PgConnection, id: Uuid, invisible: bool) -> DbResult<Role> {
    sqlx::query_as(&format!(
        "UPDATE roles SET invisible = $2 WHERE id = $1 RETURNING {ROLE}"
    ))
    .bind(id)
    .bind(invisible)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// An account's public roles, most senior first: marked public and not invisible.
pub async fn public_roles_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Role>> {
    Ok(roles_of(conn, account_id)
        .await?
        .into_iter()
        .filter(shown_publicly)
        .collect())
}

/// Whether a role may be shown to anyone, or rank its holders where anyone sees the order.
/// Invisible roles never are, whatever `public` says.
pub fn shown_publicly(role: &Role) -> bool {
    role.public && !role.invisible
}

/// The most senior role an account shows publicly: what any public ordering by role must use, so
/// invisible (and non-public) roles never move anyone up.
pub fn display_rank(roles: &[Role]) -> Option<i32> {
    roles
        .iter()
        .filter(|r| shown_publicly(r))
        .map(|r| r.position)
        .max()
}

/// The ring colour of an account's profile picture: the `#rrggbb` of its most senior role that has
/// a colour and is shown publicly (so the profile looks the same to everyone, and an invisible
/// or hidden role never colours it). At equal positions the name first alphabetically wins.
pub fn ring_color(roles: &[Role]) -> Option<&str> {
    roles
        .iter()
        .filter(|r| shown_publicly(r) && r.color.is_some())
        .min_by_key(|r| (std::cmp::Reverse(r.position), r.name.to_lowercase()))
        .and_then(|r| r.color.as_deref())
}

/// The built-in role `system` (`user`, `admin` or `root`).
pub async fn system(conn: &mut PgConnection, system: &str) -> DbResult<Option<Role>> {
    Ok(
        sqlx::query_as(&format!("SELECT {ROLE} FROM roles WHERE system = $1"))
            .bind(system)
            .fetch_optional(conn)
            .await?,
    )
}

/// One explicit Allow or Deny held by one of an account's roles.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Grant {
    pub role_id: Uuid,
    pub role_name: String,
    pub position: i32,
    pub permission: String,
    /// `allow` or `deny`.
    pub effect: String,
}

/// Every Allow and Deny of the account's roles, the built-in User role's included (every account
/// holds it), most senior first.
pub async fn grants_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Grant>> {
    Ok(sqlx::query_as(
        "SELECT r.id AS role_id, r.name AS role_name, r.position, rp.permission, rp.effect
         FROM role_permissions rp
         JOIN roles r ON r.id = rp.role_id
         WHERE r.system = 'user'
            OR r.id IN (SELECT role_id FROM account_roles WHERE account_id = $1)
         ORDER BY r.position DESC, rp.permission",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// `permissions::resolve` over stored rows. Each decision names the grant that made it, as an
/// index into `grants`.
pub fn resolve_grants(grants: &[Grant]) -> BTreeMap<String, Decision<usize>> {
    permissions::resolve(grants.iter().enumerate().filter_map(|(i, g)| {
        Some((
            i,
            g.position,
            g.permission.clone(),
            Effect::parse(&g.effect)?,
        ))
    }))
}

/// What an account may do, sorted: for each permission, its most senior role that allows or
/// denies it decides (Deny wins at equal positions), the built-in User role last; a permission
/// no role names is not granted (ADR 0010, amendment 2026/11/22). Every permission check in the
/// api goes through this (`catalog::permissions_of`). The owners don't: they hold everything.
pub async fn permissions_of(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<String>> {
    let grants = grants_of(conn, account_id).await?;
    Ok(permissions::granted(&resolve_grants(&grants)))
}

/// The permissions every account holds: what the built-in User role allows.
pub async fn everyones_permissions(conn: &mut PgConnection) -> DbResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT rp.permission
         FROM role_permissions rp
         JOIN roles r ON r.id = rp.role_id AND r.system = 'user'
         WHERE rp.effect = 'allow'
         ORDER BY 1",
    )
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

/// The accounts with an assigned role that hold a permission, sorted, decided as
/// `permissions_of` decides it: the most senior role naming it (assigned, else the User role)
/// allows it. A Deny on a more senior role takes an account out. Accounts with no assigned role
/// are left out even when the User role allows it (the lock-out rule counts staff, not everyone).
pub async fn holders_of(conn: &mut PgConnection, permission: &str) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "WITH named AS (
             SELECT ar.account_id, r.position, rp.effect
             FROM account_roles ar
             JOIN roles r ON r.id = ar.role_id
             JOIN role_permissions rp ON rp.role_id = r.id AND rp.permission = $1
             UNION ALL
             SELECT held.account_id, r.position, rp.effect
             FROM (SELECT DISTINCT account_id FROM account_roles) held,
                  roles r
             JOIN role_permissions rp ON rp.role_id = r.id AND rp.permission = $1
             WHERE r.system = 'user'
         ),
         decided AS (
             SELECT DISTINCT ON (account_id) account_id, effect
             FROM named
             ORDER BY account_id, position DESC, (effect = 'deny') DESC
         )
         SELECT account_id FROM decided WHERE effect = 'allow' ORDER BY account_id",
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

/// A role's limit overrides: an object from limit key to count, empty when it has none.
pub async fn limits(conn: &mut PgConnection, id: Uuid) -> DbResult<serde_json::Value> {
    sqlx::query_scalar("SELECT limits FROM roles WHERE id = $1")
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(DbError::NotFound)
}

/// Replaces a role's limit overrides (an object; the api checks keys and values).
pub async fn set_limits(
    conn: &mut PgConnection,
    id: Uuid,
    limits: &serde_json::Value,
) -> DbResult<()> {
    let done = sqlx::query("UPDATE roles SET limits = $2 WHERE id = $1")
        .bind(id)
        .bind(limits)
        .execute(conn)
        .await?;
    if done.rows_affected() == 0 {
        return Err(DbError::NotFound);
    }
    Ok(())
}

/// The limit overrides of every role an account holds that has any (assigned roles only: the
/// built-in User role's limits are the defaults).
pub async fn limit_overrides_of(
    conn: &mut PgConnection,
    account_id: Uuid,
) -> DbResult<Vec<serde_json::Value>> {
    Ok(sqlx::query_scalar(
        "SELECT r.limits
         FROM roles r
         JOIN account_roles ar ON ar.role_id = r.id
         WHERE ar.account_id = $1 AND r.limits <> '{}'::jsonb",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(position: i32, public: bool, invisible: bool) -> Role {
        Role {
            id: Uuid::from_u128(u128::try_from(position).unwrap()),
            name: format!("r{position}"),
            description: None,
            position,
            created_by: None,
            created_at: Utc::now(),
            public,
            system: None,
            invisible,
            color: None,
        }
    }

    #[test]
    fn invisible_roles_are_never_shown_or_ranked() {
        assert!(shown_publicly(&role(5, true, false)));
        assert!(!shown_publicly(&role(5, true, true)));
        assert!(!shown_publicly(&role(5, false, false)));
        // A senior invisible role doesn't move its holder up; the public one below it ranks.
        let held = [
            role(900, true, true),
            role(40, true, false),
            role(70, false, false),
        ];
        assert_eq!(display_rank(&held), Some(40));
        assert_eq!(display_rank(&[role(900, true, true)]), None);
    }

    fn coloured(position: i32, name: &str, color: &str, public: bool, invisible: bool) -> Role {
        Role {
            name: name.to_owned(),
            color: Some(color.to_owned()),
            ..role(position, public, invisible)
        }
    }

    #[test]
    fn the_ring_is_the_most_senior_coloured_public_role() {
        assert_eq!(ring_color(&[]), None);
        // Uncoloured roles never count, however senior.
        assert_eq!(ring_color(&[role(900, true, false)]), None);
        let held = [
            role(90, true, false),
            coloured(10, "low", "#111111", true, false),
            coloured(50, "mid", "#222222", true, false),
            // Senior but hidden or invisible: never the ring.
            coloured(80, "hidden", "#333333", false, false),
            coloured(85, "unseen", "#444444", true, true),
        ];
        assert_eq!(ring_color(&held), Some("#222222"));
        // At one position the name first alphabetically wins, whatever the order held.
        let tie = [
            coloured(20, "Zed", "#aaaaaa", true, false),
            coloured(20, "amy", "#bbbbbb", true, false),
        ];
        assert_eq!(ring_color(&tie), Some("#bbbbbb"));
    }

    fn grant(role: u128, position: i32, permission: &str, effect: &str) -> Grant {
        Grant {
            role_id: Uuid::from_u128(role),
            role_name: format!("r{role}"),
            position,
            permission: permission.to_owned(),
            effect: effect.to_owned(),
        }
    }

    #[test]
    fn stored_rows_resolve_by_the_one_rule() {
        let rows = [
            grant(5, 5, permissions::EMOTES_UPLOAD, "deny"),
            grant(0, 0, permissions::EMOTES_UPLOAD, "allow"),
            grant(0, 0, permissions::REPORTS_FILE, "allow"),
            grant(7, 7, permissions::AUDIT_VIEW, "nonsense"),
        ];
        let d = resolve_grants(&rows);
        assert_eq!(d[permissions::EMOTES_UPLOAD].effect, Effect::Deny);
        assert_eq!(rows[d[permissions::EMOTES_UPLOAD].role].role_name, "r5");
        assert_eq!(permissions::granted(&d), [permissions::REPORTS_FILE]);
        // An unknown effect is ignored, never read as an Allow.
        assert!(!d.contains_key(permissions::AUDIT_VIEW));
    }

    #[test]
    fn the_migration_gives_the_user_role_the_everyday_permissions() {
        let sql = include_str!("../migrations/20261213004101_role_deny_invisible.sql");
        for p in permissions::USER_ROLE_ALLOWED {
            assert!(sql.contains(&format!("'{p}'")), "{p}");
        }
    }

    #[test]
    fn the_admin_seed_names_only_known_permissions() {
        let sql = include_str!("../migrations/20261207000020_default_roles.sql");
        let list = sql.split("unnest(ARRAY[").nth(1).unwrap();
        let list = list.split("])").next().unwrap();
        let names: Vec<&str> = list
            .split(',')
            .map(|p| p.trim().trim_matches('\''))
            .collect();
        for name in &names {
            assert!(cyex_tv_shared::permissions::is_known(name), "{name}");
        }
        // Every permission that existed when the seed was written.
        assert_eq!(names.len(), 24);
    }
}
