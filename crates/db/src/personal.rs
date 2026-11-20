//! An account's personal lists: hidden emotes, hidden creators and favourite emotes. See
//! `20261208000920_personal_lists.sql`. Every query takes the account it is about; nothing here
//! reads another account's list.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// One of the three lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    HiddenEmotes,
    HiddenCreators,
    Favourites,
}

impl List {
    fn table(self) -> &'static str {
        match self {
            Self::HiddenEmotes => "hidden_emotes",
            Self::HiddenCreators => "hidden_creators",
            Self::Favourites => "favourite_emotes",
        }
    }

    fn column(self) -> &'static str {
        match self {
            Self::HiddenEmotes | Self::Favourites => "emote_id",
            Self::HiddenCreators => "creator_id",
        }
    }
}

/// One entry: what is listed and when it was added.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// What [`add`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Added {
    /// Newly listed.
    Added,
    /// It was listed already; nothing changed.
    Already,
    /// The list holds `cap` entries already; nothing changed.
    Full,
}

/// Adds `id` to the account's list unless it is there already or the list holds `cap` entries.
/// The account row is locked first, so two adds at once can't pass the cap together.
pub async fn add(
    conn: &mut PgConnection,
    list: List,
    account_id: Uuid,
    id: Uuid,
    cap: i64,
) -> DbResult<Added> {
    let (table, column) = (list.table(), list.column());
    sqlx::query("SELECT 1 FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;
    let listed: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS (SELECT 1 FROM {table} WHERE account_id = $1 AND {column} = $2)"
    ))
    .bind(account_id)
    .bind(id)
    .fetch_one(&mut *conn)
    .await?;
    if listed {
        return Ok(Added::Already);
    }
    if count(conn, list, account_id).await? >= cap {
        return Ok(Added::Full);
    }
    sqlx::query(&format!(
        "INSERT INTO {table} (account_id, {column}) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    ))
    .bind(account_id)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(Added::Added)
}

/// Takes `id` off the account's list. False when it wasn't on it.
pub async fn remove(
    conn: &mut PgConnection,
    list: List,
    account_id: Uuid,
    id: Uuid,
) -> DbResult<bool> {
    let (table, column) = (list.table(), list.column());
    let done = sqlx::query(&format!(
        "DELETE FROM {table} WHERE account_id = $1 AND {column} = $2"
    ))
    .bind(account_id)
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn count(conn: &mut PgConnection, list: List, account_id: Uuid) -> DbResult<i64> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT count(*) FROM {} WHERE account_id = $1",
        list.table()
    ))
    .bind(account_id)
    .fetch_one(conn)
    .await?)
}

/// The whole list, the latest added first.
pub async fn all(conn: &mut PgConnection, list: List, account_id: Uuid) -> DbResult<Vec<Entry>> {
    let (table, column) = (list.table(), list.column());
    Ok(sqlx::query_as(&format!(
        "SELECT {column} AS id, created_at FROM {table} WHERE account_id = $1
         ORDER BY created_at DESC, {column} DESC"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// A page of the account's favourites that still exist and that the account may see (approved,
/// or its own), the latest added first, and how many there are by the same rule; `name` keeps those whose name holds it (any case).
pub async fn favourites_page(
    conn: &mut PgConnection,
    account_id: Uuid,
    name: Option<&str>,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<Uuid>, i64)> {
    let pattern = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", crate::search::like_escape(s)));
    let filter = "f.account_id = $1 AND e.deleted_at IS NULL
         AND (e.state = 'approved' OR e.owner_account_id = $1)
         AND ($2::text IS NULL OR e.name ILIKE $2)";
    let ids = sqlx::query_scalar(&format!(
        "SELECT f.emote_id FROM favourite_emotes f JOIN emotes e ON e.id = f.emote_id
         WHERE {filter} ORDER BY f.created_at DESC, f.emote_id DESC LIMIT $3 OFFSET $4"
    ))
    .bind(account_id)
    .bind(pattern.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM favourite_emotes f JOIN emotes e ON e.id = f.emote_id WHERE {filter}"
    ))
    .bind(account_id)
    .bind(pattern.as_deref())
    .fetch_one(conn)
    .await?;
    Ok((ids, total))
}
