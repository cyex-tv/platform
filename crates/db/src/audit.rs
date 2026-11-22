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

/// The statement [`emote_activity_rows`] runs (`$1` emote ID text, `$2` rows before this ID,
/// `$3` limit, `$4` only lines shown as by this account). Public so the scale test can ask
/// Postgres for its plan.
#[doc(hidden)]
pub fn emote_activity_sql() -> String {
    // Two branches, each read newest first from its own index (the approval by target, the set
    // changes by `audit_log_emote_activity`) and cut at `limit`, so an emote in thousands of
    // channels reads a page, not every row naming it. No row is in both (the actions differ).
    const ROW: &str = "SELECT a.id, a.actor_account_id, a.action, a.target_kind, a.target_id,
                a.before_json, a.after_json, a.created_at, a.subject_account_id
         FROM audit_log a
         JOIN accounts s ON s.id = a.subject_account_id
                        AND s.deleted_at IS NULL AND (s.blocked_at IS NULL OR s.blocked_until <= now())
         LEFT JOIN accounts act ON act.id = a.actor_account_id
         WHERE a.subject_account_id IS NOT NULL
           AND ($2::uuid IS NULL OR a.id < $2)";
    format!(
        "SELECT * FROM (
             ({ROW}
               AND a.action = 'emote.approved' AND a.target_kind = 'emote' AND a.target_id = $1
               AND NOT EXISTS (SELECT 1 FROM audit_log p
                               WHERE p.action = 'emote.approved' AND p.target_kind = 'emote'
                                 AND p.target_id = a.target_id AND p.id < a.id)
               AND {LISTED_SET_ONLY}
               AND ($4::uuid IS NULL OR a.subject_account_id = $4)
             ORDER BY a.id DESC
             LIMIT $3)
             UNION ALL
             ({ROW}
               AND a.action IN ('set.emote_added', 'set.emote_removed', 'set.emote_updated')
               AND coalesce(a.after_json ->> 'emote_id', a.before_json ->> 'emote_id') = $1
               AND act.id IS NOT NULL AND (act.blocked_at IS NULL OR act.blocked_until <= now()) AND act.deleted_at IS NULL
               AND (a.actor_account_id = a.subject_account_id
                    OR a.after_json ->> 'via' = 'account_editor')
               AND {LISTED_SET_ONLY}
               AND ($4::uuid IS NULL OR a.actor_account_id = $4)
             ORDER BY a.id DESC
             LIMIT $3)
         ) r
         ORDER BY id DESC
         LIMIT $3"
    )
}

/// Public activity rows about one emote, newest first: its approval (once) and the set changes
/// naming it, written by the owner of the set or their account editors. Rows about accounts that
/// are blocked or deleted, rows by blocked or deleted actors and rows about sets that aren't
/// listed are left out. The api drops what is still private in ways SQL can't tell (deleted sets).
/// With `actor`, only the lines shown as by that account (an approval is the emote owner's).
pub async fn emote_activity_rows(
    conn: &mut PgConnection,
    emote_id: &str,
    before_id: Option<Uuid>,
    limit: i64,
    actor: Option<Uuid>,
) -> DbResult<Vec<SubjectEntry>> {
    Ok(sqlx::query_as(&emote_activity_sql())
        .bind(emote_id)
        .bind(before_id)
        .bind(limit)
        .bind(actor)
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

/// The actions `filter` keeps, in `ACTIVITY_ACTIONS` order.
fn activity_actions(filter: &ActivityFilter) -> Vec<&'static str> {
    match &filter.actions {
        Some(only) => ACTIVITY_ACTIONS
            .iter()
            .copied()
            .filter(|a| only.contains(a))
            .collect(),
        None => ACTIVITY_ACTIONS.to_vec(),
    }
}

/// The statement behind [`activity_rows_where`] (`$1` subject, `$2` rows before this ID, `$3`
/// actions, `$4` limit, `$5` actor, `$6` since, `$7` until), selecting `columns` from `audit_log
/// a`. The account a line is shown as by is `ACTIVITY_SHOWN_ACTOR`.
fn activity_sql(columns: &str) -> String {
    format!(
        "SELECT {columns}
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
           AND ($5::uuid IS NULL OR {ACTIVITY_SHOWN_ACTOR} = $5)
           AND ($6::timestamptz IS NULL OR a.created_at >= $6)
           AND ($7::timestamptz IS NULL OR a.created_at < $7)
         ORDER BY a.id DESC
         LIMIT $4"
    )
}

/// Who a feed line is shown as by: an approved upload is the subject's own (the feed's owner),
/// every other line is by whoever made the change.
const ACTIVITY_SHOWN_ACTOR: &str =
    "(CASE WHEN a.action = 'emote.approved' THEN a.subject_account_id ELSE a.actor_account_id END)";

/// `activity_rows`, narrowed by `filter` (kind, who, when).
pub async fn activity_rows_where(
    conn: &mut PgConnection,
    subject: Uuid,
    before_id: Option<Uuid>,
    limit: i64,
    filter: &ActivityFilter,
) -> DbResult<Vec<Entry>> {
    let actions = activity_actions(filter);
    Ok(sqlx::query_as(&activity_sql(
        "a.id, a.actor_account_id, a.action, a.target_kind, a.target_id, a.before_json,
                a.after_json, a.created_at",
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

/// How many of the newest rows a "top people" count looks at: enough to say who is busy there
/// lately, few enough that a long history reads a bounded page from its index.
pub const TOP_WINDOW: i64 = 2000;

/// One person and how many rows they have in a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromRow)]
pub struct PersonCount {
    pub account_id: Uuid,
    pub count: i64,
}

/// The people `subject`'s public feed shows most often as by, among its newest `TOP_WINDOW`
/// candidate rows that pass `filter` (its `actor` is ignored: this is what picks one), most
/// lines first, then the newest. The same SQL checks as [`activity_rows_where`].
pub async fn activity_top_actors(
    conn: &mut PgConnection,
    subject: Uuid,
    filter: &ActivityFilter,
    limit: i64,
) -> DbResult<Vec<PersonCount>> {
    let actions = activity_actions(filter);
    let rows = activity_sql(&format!(
        "{ACTIVITY_SHOWN_ACTOR} AS who, a.id, a.created_at"
    ));
    Ok(sqlx::query_as(&format!(
        "SELECT r.who AS account_id, count(*) AS count FROM ({rows}) r
         JOIN accounts x ON x.id = r.who AND x.deleted_at IS NULL
         GROUP BY r.who ORDER BY count(*) DESC, max(r.created_at) DESC, r.who LIMIT $8"
    ))
    .bind(subject)
    .bind(None::<Uuid>)
    .bind(&actions)
    .bind(TOP_WINDOW)
    .bind(None::<Uuid>)
    .bind(filter.since)
    .bind(filter.until)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// The people one emote's public feed shows most often as by (an approval is its owner's), among
/// its newest `TOP_WINDOW` rows, most lines first. The same checks as [`emote_activity_rows`].
pub async fn emote_top_actors(
    conn: &mut PgConnection,
    emote_id: &str,
    limit: i64,
) -> DbResult<Vec<PersonCount>> {
    let rows = emote_activity_sql();
    Ok(sqlx::query_as(&format!(
        "SELECT r.who AS account_id, count(*) AS count FROM (
             SELECT CASE WHEN f.action = 'emote.approved' THEN f.subject_account_id
                         ELSE f.actor_account_id END AS who, f.id, f.created_at
             FROM ({rows}) f) r
         JOIN accounts x ON x.id = r.who AND x.deleted_at IS NULL
         GROUP BY r.who ORDER BY count(*) DESC, max(r.created_at) DESC, r.who LIMIT $5"
    ))
    .bind(emote_id)
    .bind(None::<Uuid>)
    .bind(TOP_WINDOW)
    .bind(None::<Uuid>)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Which side of an audit entry [`top_people`] counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditSide {
    /// Who made the changes.
    Actor,
    /// The accounts the changes were about (`target_kind = 'account'`).
    Target,
}

/// The accounts with the most entries among the newest `TOP_WINDOW` that pass `filter` (its
/// `before_id` is ignored), as actors or as account targets, most entries first, then the newest.
pub async fn top_people(
    conn: &mut PgConnection,
    filter: &Filter<'_>,
    side: AuditSide,
    limit: i64,
) -> DbResult<Vec<PersonCount>> {
    let (who, kind) = match side {
        AuditSide::Actor => ("actor_account_id", filter.target_kind),
        AuditSide::Target => ("ulid_to_uuid(target_id)", Some("account")),
    };
    Ok(sqlx::query_as(&format!(
        "SELECT r.who AS account_id, count(*) AS count FROM (
             SELECT {who} AS who, id, created_at FROM audit_log
             WHERE ($1::uuid IS NULL OR actor_account_id = $1)
               AND ($2::text IS NULL OR target_kind = $2)
               AND ($3::text IS NULL OR target_id = $3)
               AND ($4::text IS NULL OR action = $4)
               AND ($5::timestamptz IS NULL OR created_at >= $5)
               AND ($6::timestamptz IS NULL OR created_at < $6)
             ORDER BY id DESC
             LIMIT $7) r
         JOIN accounts x ON x.id = r.who AND x.deleted_at IS NULL
         GROUP BY r.who ORDER BY count(*) DESC, max(r.created_at) DESC, r.who LIMIT $8"
    ))
    .bind(filter.actor)
    .bind(kind)
    .bind(filter.target_id)
    .bind(filter.action)
    .bind(filter.since)
    .bind(filter.until)
    .bind(TOP_WINDOW)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
