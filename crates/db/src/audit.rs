//! The audit log: every change to roles, grants and moderation state (ADR 0010). Write it in the
//! same transaction as the change. Rows are never changed afterwards.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub actor_account_id: Option<Uuid>,
    /// For example `roles.create` or `emotes.approve`.
    pub action: String,
    /// For example `role`, `emote`, `account`.
    pub target_kind: String,
    pub target_id: String,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewEntry<'a> {
    /// `None` for changes the system makes on its own.
    pub actor: Option<Uuid>,
    pub action: &'a str,
    pub target_kind: &'a str,
    pub target_id: &'a str,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Narrows `list`. Empty fields match everything.
#[derive(Clone, Debug, Default)]
pub struct Filter<'a> {
    pub actor: Option<Uuid>,
    pub target_kind: Option<&'a str>,
    pub target_id: Option<&'a str>,
    /// An exact action name, for example `emotes.approve`.
    pub action: Option<&'a str>,
    /// Only entries at or after this time.
    pub since: Option<DateTime<Utc>>,
    /// Only entries before this time.
    pub until: Option<DateTime<Utc>>,
    /// Paging: only entries older than this one.
    pub before_id: Option<Uuid>,
}

const ENTRY: &str = "id, actor_account_id, action, target_kind, target_id, before_json, \
                     after_json, created_at";

pub async fn write(conn: &mut PgConnection, entry: &NewEntry<'_>) -> DbResult<Uuid> {
    write_for(conn, entry, None).await
}

/// Like `write`, for a change the account `subject` can see on its public activity feed. Only
/// changes the owner or its account editors made themselves, never staff actions.
pub async fn write_for(
    conn: &mut PgConnection,
    entry: &NewEntry<'_>,
    subject: Option<Uuid>,
) -> DbResult<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO audit_log
             (id, actor_account_id, action, target_kind, target_id, before_json, after_json,
              subject_account_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(new_id())
    .bind(entry.actor)
    .bind(entry.action)
    .bind(entry.target_kind)
    .bind(entry.target_id)
    .bind(&entry.before)
    .bind(&entry.after)
    .bind(subject)
    .fetch_one(conn)
    .await?)
}

/// Entries newest first. IDs are ULIDs, so `before_id` pages back in time.
pub async fn list(
    conn: &mut PgConnection,
    filter: &Filter<'_>,
    limit: i64,
) -> DbResult<Vec<Entry>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ENTRY} FROM audit_log
         WHERE ($1::uuid IS NULL OR actor_account_id = $1)
           AND ($2::text IS NULL OR target_kind = $2)
           AND ($3::text IS NULL OR target_id = $3)
           AND ($4::uuid IS NULL OR id < $4)
           AND ($6::text IS NULL OR action = $6)
           AND ($7::timestamptz IS NULL OR created_at >= $7)
           AND ($8::timestamptz IS NULL OR created_at < $8)
         ORDER BY id DESC
         LIMIT $5"
    ))
    .bind(filter.actor)
    .bind(filter.target_kind)
    .bind(filter.target_id)
    .bind(filter.before_id)
    .bind(limit)
    .bind(filter.action)
    .bind(filter.since)
    .bind(filter.until)
    .fetch_all(conn)
    .await?)
}

/// The actions the public activity feed is built from (see `apps/api/src/activity.rs`).
pub const ACTIVITY_ACTIONS: [&str; 10] = [
    "set.created",
    "set.renamed",
    "set.emote_added",
    "set.emote_removed",
    "set.emote_updated",
    "channel.settings_changed",
    "cosmetics.selected",
    "emote.approved",
    "account.editor_added",
    "account.editor_removed",
];

/// Narrows an account's activity rows. Empty fields match everything.
#[derive(Clone, Debug, Default)]
pub struct ActivityFilter {
    /// Only these actions (each one of `ACTIVITY_ACTIONS`); `None` for all of them.
    pub actions: Option<Vec<&'static str>>,
    /// Only lines shown as by this account: changes it made, and (for the account itself) its
    /// approved uploads.
    pub actor: Option<Uuid>,
    /// Only rows at or after this time.
    pub since: Option<DateTime<Utc>>,
    /// Only rows before this time.
    pub until: Option<DateTime<Utc>>,
}

/// An activity row with the account whose feed it belongs to.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct SubjectEntry {
    #[sqlx(flatten)]
    pub entry: Entry,
    pub subject_account_id: Uuid,
}

/// The set actions of the feed: they only show for a set its subject owns and lists
/// (ADR 0017), checked in SQL so hidden rows don't use up the api's read rounds.
const LISTED_SET_ONLY: &str = "(a.action NOT IN ('set.created', 'set.renamed', 'set.emote_added',
                                              'set.emote_removed', 'set.emote_updated')
               OR EXISTS (SELECT 1 FROM emote_sets st
                          WHERE st.id = ulid_to_uuid(a.target_id)
                            AND st.owner_account_id = a.subject_account_id
                            AND st.visibility = 'listed'))";

/// Public activity rows about one emote, newest first: its approval (once) and the set changes
/// naming it, written by the owner of the set or their account editors. Rows about accounts that
/// are blocked or deleted, rows by blocked or deleted actors and rows about sets that aren't
/// listed are left out. The api drops what is still private in ways SQL can't tell (deleted sets).
pub async fn emote_activity_rows(
    conn: &mut PgConnection,
    emote_id: &str,
    before_id: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<SubjectEntry>> {
    Ok(sqlx::query_as(&format!(
        "SELECT a.id, a.actor_account_id, a.action, a.target_kind, a.target_id, a.before_json,
                a.after_json, a.created_at, a.subject_account_id
         FROM audit_log a
         JOIN accounts s ON s.id = a.subject_account_id
                        AND s.deleted_at IS NULL AND (s.blocked_at IS NULL OR s.blocked_until <= now())
         LEFT JOIN accounts act ON act.id = a.actor_account_id
         WHERE a.subject_account_id IS NOT NULL
           AND ($2::uuid IS NULL OR a.id < $2)
           AND (
               (a.action = 'emote.approved' AND a.target_kind = 'emote' AND a.target_id = $1
                AND NOT EXISTS (SELECT 1 FROM audit_log p
                                WHERE p.action = 'emote.approved' AND p.target_kind = 'emote'
                                  AND p.target_id = a.target_id AND p.id < a.id))
               OR (a.action IN ('set.emote_added', 'set.emote_removed', 'set.emote_updated')
                   AND coalesce(a.after_json ->> 'emote_id', a.before_json ->> 'emote_id') = $1
                   AND act.id IS NOT NULL AND (act.blocked_at IS NULL OR act.blocked_until <= now()) AND act.deleted_at IS NULL
                   AND (a.actor_account_id = a.subject_account_id
                        OR a.after_json ->> 'via' = 'account_editor'))
           )
           AND {LISTED_SET_ONLY}
         ORDER BY a.id DESC
         LIMIT $3"
    ))
    .bind(emote_id)
    .bind(before_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Candidate rows for `subject`'s public activity feed, newest first. The rows pass the checks
/// that fit in SQL: they are about the account, are one of `ACTIVITY_ACTIONS`, were made by the
/// account itself or through an account editor grant (never by staff acting on their own
/// permissions), by an actor who isn't blocked or deleted, an emote's approval counts once, and
/// set changes are about a set the account lists. The api drops what's still private (pending
/// emotes, deleted sets and so on).
pub async fn activity_rows(
    conn: &mut PgConnection,
    subject: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<Entry>> {
    activity_rows_where(conn, subject, before_id, limit, &ActivityFilter::default()).await
}

/// `activity_rows`, narrowed by `filter` (kind, who, when).
pub async fn activity_rows_where(
    conn: &mut PgConnection,
    subject: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
    filter: &ActivityFilter,
) -> DbResult<Vec<Entry>> {
    let actions: Vec<&str> = match &filter.actions {
        Some(only) => ACTIVITY_ACTIONS
            .iter()
            .copied()
            .filter(|a| only.contains(a))
            .collect(),
        None => ACTIVITY_ACTIONS.to_vec(),
    };
    Ok(sqlx::query_as(&format!(
        "SELECT a.id, a.actor_account_id, a.action, a.target_kind, a.target_id, a.before_json,
                a.after_json, a.created_at
         FROM audit_log a
         LEFT JOIN accounts act ON act.id = a.actor_account_id
         WHERE a.subject_account_id = $1
           AND ($2::uuid IS NULL OR a.id < $2)
           AND a.action = ANY($3)
           AND (
               (a.action = 'emote.approved'
                AND NOT EXISTS (SELECT 1 FROM audit_log p
                                WHERE p.action = 'emote.approved' AND p.target_kind = 'emote'
                                  AND p.target_id = a.target_id AND p.id < a.id))
               OR (a.action <> 'emote.approved'
                   AND act.id IS NOT NULL AND (act.blocked_at IS NULL OR act.blocked_until <= now()) AND act.deleted_at IS NULL
                   AND (a.actor_account_id = a.subject_account_id
                        OR a.after_json ->> 'via' = 'account_editor'))
           )
           AND {LISTED_SET_ONLY}
           AND ($5::uuid IS NULL
                OR (a.action = 'emote.approved' AND a.subject_account_id = $5)
                OR (a.action <> 'emote.approved' AND a.actor_account_id = $5))
           AND ($6::timestamptz IS NULL OR a.created_at >= $6)
           AND ($7::timestamptz IS NULL OR a.created_at < $7)
         ORDER BY a.id DESC
         LIMIT $4"
    ))
    .bind(subject)
    .bind(before_id)
    .bind(&actions)
    .bind(limit)
    .bind(filter.actor)
    .bind(filter.since)
    .bind(filter.until)
    .fetch_all(conn)
    .await?)
}
