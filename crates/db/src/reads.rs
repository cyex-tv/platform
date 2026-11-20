//! Batch reads for building API answers: a whole set with its emotes, files, tags and owners in a
//! few queries instead of a few per emote.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{AccountSummary, EmoteState};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// A set member joined with its emote and the emote's file. Deleted emotes are left out.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct MemberRow {
    pub set_id: Uuid,
    pub emote_id: Uuid,
    /// The name in the set.
    pub name: String,
    pub zero_width: bool,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
    pub emote_owner: Option<Uuid>,
    pub emote_name: String,
    /// The emote's own zero-width flag; `zero_width` is the set's.
    pub emote_zero_width: bool,
    pub private: bool,
    pub unlisted: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    pub emote_created_at: DateTime<Utc>,
    pub sha256: Option<Vec<u8>>,
    pub variants: Option<Value>,
}

/// An emote with its file, as the API shows it.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct EmoteRow {
    pub id: Uuid,
    pub owner: Option<Uuid>,
    pub name: String,
    pub zero_width: bool,
    pub private: bool,
    pub unlisted: bool,
    pub nsfw: bool,
    pub animated: bool,
    pub state: EmoteState,
    pub created_at: DateTime<Utc>,
    pub sha256: Option<Vec<u8>>,
    pub variants: Option<Value>,
}

impl MemberRow {
    /// The emote part of the row.
    pub fn emote(&self) -> EmoteRow {
        EmoteRow {
            id: self.emote_id,
            owner: self.emote_owner,
            name: self.emote_name.clone(),
            zero_width: self.emote_zero_width,
            private: self.private,
            unlisted: self.unlisted,
            nsfw: self.nsfw,
            animated: self.animated,
            state: self.state,
            created_at: self.emote_created_at,
            sha256: self.sha256.clone(),
            variants: self.variants.clone(),
        }
    }
}

const EMOTE_ROW: &str = "SELECT e.id, e.owner_account_id AS owner, e.name, e.zero_width, e.private,
            e.unlisted, e.nsfw, e.animated, e.state, e.created_at, f.sha256, f.variants
     FROM emotes e
     LEFT JOIN files f ON f.id = e.file_id";

/// Emotes by ID, in no particular order. Deleted ones are left out.
pub async fn emotes(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<EmoteRow>> {
    Ok(sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.id = ANY($1) AND e.deleted_at IS NULL"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

/// Emotes in `state`, oldest first (the review queue), and how many there are.
pub async fn emotes_in_state(
    conn: &mut PgConnection,
    state: EmoteState,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let items = sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.state = $1 AND e.deleted_at IS NULL
         ORDER BY e.created_at, e.id LIMIT $2 OFFSET $3"
    ))
    .bind(state)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total =
        sqlx::query_scalar("SELECT count(*) FROM emotes WHERE state = $1 AND deleted_at IS NULL")
            .bind(state)
            .fetch_one(conn)
            .await?;
    Ok((items, total))
}

/// Which part of the review queue a read lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueTab {
    /// Every pending emote.
    #[default]
    Pending,
    /// Pending and public: it would show in search once approved.
    Public,
    /// Pending and unlisted or private: link-only or for its owner's own sets.
    Limited,
    /// Approved or rejected by a reviewer.
    Resolved,
}

/// How a review queue read is ordered. Ties go by upload time, then ID.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueSort {
    #[default]
    Uploaded,
    Reviewed,
    Uploader,
    Name,
}

/// A review queue read: a part, an optional search over the emote's name and its uploader's
/// login and display name, an order.
#[derive(Clone, Copy, Debug, Default)]
pub struct QueueQuery<'a> {
    pub tab: QueueTab,
    pub search: Option<&'a str>,
    pub sort: QueueSort,
    pub descending: bool,
}

/// How many emotes wait in each pending part of the queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, FromRow)]
pub struct QueueCounts {
    pub public: i64,
    pub limited: i64,
}

/// The uploader's name the queue sorts and searches by: the Twitch identity first, as the API
/// shows it.
const UPLOADER: &str =
    "(SELECT i.display_name FROM identities i WHERE i.account_id = e.owner_account_id
       ORDER BY i.platform DESC LIMIT 1)";

/// A page of the review queue and how many match in all.
pub async fn review_queue(
    conn: &mut PgConnection,
    q: &QueueQuery<'_>,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let tab = match q.tab {
        QueueTab::Pending => "e.state = 'pending'",
        QueueTab::Public => "e.state = 'pending' AND NOT e.private AND NOT e.unlisted",
        QueueTab::Limited => "e.state = 'pending' AND (e.private OR e.unlisted)",
        QueueTab::Resolved => "e.state IN ('approved', 'rejected') AND e.reviewed_at IS NOT NULL",
    };
    let dir = if q.descending { "DESC" } else { "ASC" };
    let order = match q.sort {
        QueueSort::Uploaded => format!("e.created_at {dir}, e.id {dir}"),
        QueueSort::Reviewed => {
            format!("e.reviewed_at {dir} NULLS LAST, e.created_at {dir}, e.id {dir}")
        }
        QueueSort::Uploader => {
            format!("lower({UPLOADER}) {dir} NULLS LAST, e.created_at, e.id")
        }
        QueueSort::Name => format!("lower(e.name) {dir}, e.created_at, e.id"),
    };
    let pattern = q
        .search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", crate::search::like_escape(s)));
    let matches = "($1::text IS NULL OR e.name ILIKE $1 OR EXISTS (
            SELECT 1 FROM identities i WHERE i.account_id = e.owner_account_id
              AND (i.login ILIKE $1 OR i.display_name ILIKE $1)))";
    let items = sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE {tab} AND e.deleted_at IS NULL AND {matches}
         ORDER BY {order} LIMIT $2 OFFSET $3"
    ))
    .bind(pattern.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM emotes e WHERE {tab} AND e.deleted_at IS NULL AND {matches}"
    ))
    .bind(pattern.as_deref())
    .fetch_one(conn)
    .await?;
    Ok((items, total))
}

/// How many pending emotes are public, and how many unlisted or private.
pub async fn queue_counts(conn: &mut PgConnection) -> DbResult<QueueCounts> {
    Ok(sqlx::query_as(
        "SELECT count(*) FILTER (WHERE NOT private AND NOT unlisted) AS public,
                count(*) FILTER (WHERE private OR unlisted) AS limited
         FROM emotes WHERE state = 'pending' AND deleted_at IS NULL",
    )
    .fetch_one(conn)
    .await?)
}

/// An account's own emotes in every state, newest first, and how many there are.
pub async fn emotes_by_owner(
    conn: &mut PgConnection,
    owner: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let items = sqlx::query_as(&format!(
        "{EMOTE_ROW} WHERE e.owner_account_id = $1 AND e.deleted_at IS NULL
         ORDER BY e.created_at DESC, e.id DESC LIMIT $2 OFFSET $3"
    ))
    .bind(owner)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(
        "SELECT count(*) FROM emotes WHERE owner_account_id = $1 AND deleted_at IS NULL",
    )
    .bind(owner)
    .fetch_one(conn)
    .await?;
    Ok((items, total))
}

macro_rules! member_columns {
    () => {
        "m.set_id, m.emote_id, m.name, m.zero_width, m.added_by, m.added_at,
                e.owner_account_id AS emote_owner, e.name AS emote_name,
                e.zero_width AS emote_zero_width, e.private, e.unlisted, e.nsfw,
                e.animated, e.state, e.created_at AS emote_created_at,
                f.sha256, f.variants"
    };
}

const MEMBER_ROW: &str = concat!(
    "SELECT ",
    member_columns!(),
    "
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         LEFT JOIN files f ON f.id = e.file_id
"
);

/// Who may see which members: approved ones always; pending ones for a reviewer, in the sets
/// `edit_sets` (the viewer owns or edits) and the viewer's own emotes.
#[derive(Clone, Debug, Default)]
pub struct MemberAccess<'a> {
    pub account: Option<Uuid>,
    pub reviewer: bool,
    pub edit_sets: &'a [Uuid],
}

/// The newest `per_set` members each of the given sets shows `access`, newest first within each
/// set (by when they were added). Only that many rows are read, whatever the sets hold.
pub async fn newest_members(
    conn: &mut PgConnection,
    set_ids: &[Uuid],
    access: &MemberAccess<'_>,
    per_set: i64,
) -> DbResult<Vec<MemberRow>> {
    // Per set, the members in index order (`emote_set_members_set_newest`) until `per_set` pass
    // the access rule, instead of numbering every member of every set.
    Ok(sqlx::query_as(&format!(
        "SELECT t.* FROM (SELECT DISTINCT u.id FROM unnest($1::uuid[]) AS u(id)) s
         CROSS JOIN LATERAL (
             SELECT {}
             FROM emote_set_members m
             JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
             LEFT JOIN files f ON f.id = e.file_id
             WHERE m.set_id = s.id
               AND (e.state = 'approved'
                    OR (e.state = 'pending'
                        AND ($2 OR m.set_id = ANY($3) OR e.owner_account_id = $4)))
             ORDER BY m.added_at DESC, m.emote_id DESC
             LIMIT $5
         ) t
         ORDER BY t.set_id, t.added_at DESC, t.emote_id DESC",
        member_columns!()
    ))
    .bind(set_ids)
    .bind(access.reviewer)
    .bind(access.edit_sets)
    .bind(access.account)
    .bind(per_set)
    .fetch_all(conn)
    .await?)
}

/// A page of one set's members that `access` shows, oldest first (the order `members` reads),
/// narrowed to the ones whose name in the set or own name contains `query` (case-insensitive,
/// literally), with how many match in all.
pub async fn members_page(
    conn: &mut PgConnection,
    set_id: Uuid,
    access: &MemberAccess<'_>,
    query: Option<&str>,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<MemberRow>, i64)> {
    let pattern = query.map(str::trim).filter(|q| !q.is_empty()).map(|q| {
        let escaped = q
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        format!("%{escaped}%")
    });
    const RULE: &str = "m.set_id = $1
               AND (e.state = 'approved'
                    OR (e.state = 'pending'
                        AND ($2 OR m.set_id = ANY($3) OR e.owner_account_id = $4)))
               AND ($5::text IS NULL OR m.name ILIKE $5 OR e.name ILIKE $5)";
    let rows = sqlx::query_as(&format!(
        "SELECT {}
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         LEFT JOIN files f ON f.id = e.file_id
         WHERE {RULE}
         ORDER BY m.added_at, m.emote_id
         LIMIT $6 OFFSET $7",
        member_columns!()
    ))
    .bind(set_id)
    .bind(access.reviewer)
    .bind(access.edit_sets)
    .bind(access.account)
    .bind(pattern.as_deref())
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(&format!(
        "SELECT count(*)
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         WHERE {RULE}"
    ))
    .bind(set_id)
    .bind(access.reviewer)
    .bind(access.edit_sets)
    .bind(access.account)
    .bind(pattern.as_deref())
    .fetch_one(conn)
    .await?;
    Ok((rows, total))
}

/// How many members of each given set `access` shows (the rule of `newest_members`). Sets with
/// none are missing.
pub async fn visible_counts(
    conn: &mut PgConnection,
    set_ids: &[Uuid],
    access: &MemberAccess<'_>,
) -> DbResult<HashMap<Uuid, i64>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT m.set_id, count(*)
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         WHERE m.set_id = ANY($1)
           AND (e.state = 'approved'
                OR (e.state = 'pending'
                    AND ($2 OR m.set_id = ANY($3) OR e.owner_account_id = $4)))
         GROUP BY m.set_id",
    )
    .bind(set_ids)
    .bind(access.reviewer)
    .bind(access.edit_sets)
    .bind(access.account)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// The members of the given sets, oldest first within each set.
pub async fn members(conn: &mut PgConnection, set_ids: &[Uuid]) -> DbResult<Vec<MemberRow>> {
    Ok(sqlx::query_as(&format!(
        "{MEMBER_ROW} WHERE m.set_id = ANY($1) ORDER BY m.set_id, m.added_at, m.emote_id"
    ))
    .bind(set_ids)
    .fetch_all(conn)
    .await?)
}

/// One member of one set, if the emote is in it and not deleted.
pub async fn member(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_id: Uuid,
) -> DbResult<Option<MemberRow>> {
    Ok(sqlx::query_as(&format!(
        "{MEMBER_ROW} WHERE m.set_id = $1 AND m.emote_id = $2"
    ))
    .bind(set_id)
    .bind(emote_id)
    .fetch_optional(conn)
    .await?)
}

/// Tags of each emote, sorted.
pub async fn tags(
    conn: &mut PgConnection,
    emote_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, Vec<String>>> {
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT emote_id, tag FROM emote_tags WHERE emote_id = ANY($1) ORDER BY emote_id, tag",
    )
    .bind(emote_ids)
    .fetch_all(conn)
    .await?;
    let mut out: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, tag) in rows {
        out.entry(id).or_default().push(tag);
    }
    Ok(out)
}

/// How an account is shown to others: the name and avatar of the identity it goes by
/// (`accounts.display_platform`, else Twitch first).
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Summary {
    pub account_id: Uuid,
    pub display_name: String,
    pub avatar: Option<String>,
}

impl From<&Summary> for AccountSummary {
    fn from(s: &Summary) -> Self {
        Self {
            id: crate::ids::to_text(s.account_id),
            display_name: s.display_name.clone(),
            avatar: s.avatar.clone(),
        }
    }
}

/// Summaries of live accounts. Deleted accounts and accounts with no identity are left out.
pub async fn summaries(
    conn: &mut PgConnection,
    account_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, Summary>> {
    let rows: Vec<Summary> = sqlx::query_as(
        "SELECT DISTINCT ON (i.account_id) i.account_id, i.display_name, i.avatar
         FROM identities i
         JOIN accounts a ON a.id = i.account_id AND a.deleted_at IS NULL
         WHERE i.account_id = ANY($1)
         ORDER BY i.account_id, i.platform IS NOT DISTINCT FROM a.display_platform DESC,
                  i.platform DESC",
    )
    .bind(account_ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(|s| (s.account_id, s)).collect())
}

/// How many channels have each emote in their active set. Channels of deleted or blocked
/// accounts don't count. Emotes in no channel are missing from the map. One query for any
/// number of emotes.
pub async fn channel_counts(
    conn: &mut PgConnection,
    emote_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, u32>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT m.emote_id, count(DISTINCT c.id)
         FROM emote_set_members m
         JOIN channels c ON c.active_set_id = m.set_id
         JOIN accounts a ON a.id = c.account_id AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         WHERE m.emote_id = ANY($1)
         GROUP BY m.emote_id",
    )
    .bind(emote_ids)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, n)| (id, u32::try_from(n).unwrap_or(u32::MAX)))
        .collect())
}

/// Channels whose active set holds the emote, the most recently added first, `offset` rows in.
/// Channels of deleted or blocked accounts are left out.
pub async fn emote_channels(
    conn: &mut PgConnection,
    emote_id: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<crate::channels::Channel>> {
    Ok(sqlx::query_as(
        "SELECT c.id, c.account_id, c.platform, c.platform_channel_id, c.login, c.active_set_id,
                c.allow_personal_emotes, c.created_at, c.updated_at
         FROM emote_set_members m
         JOIN channels c ON c.active_set_id = m.set_id
         JOIN accounts a ON a.id = c.account_id AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         WHERE m.emote_id = $1
         ORDER BY m.added_at DESC, c.id
         LIMIT $2 OFFSET $3",
    )
    .bind(emote_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(conn)
    .await?)
}

/// How close a name has to be (pg_trgm similarity, 0 to 1) to count as a possible duplicate.
pub const DUPLICATE_NAME_SIMILARITY: f32 = 0.5;

/// Another emote that may be the same as the one under review.
#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct DuplicateHit {
    pub id: Uuid,
    /// Made from the very same file (the same sha256).
    pub same_file: bool,
    /// How alike the names are, 0 to 1 (1 when they only differ in case).
    pub similarity: f32,
}

/// Emotes that may duplicate `id`: the same file first, then names at least
/// [`DUPLICATE_NAME_SIMILARITY`] alike, closest first. Any state but deleted, never `id` itself.
/// Two index scans (`emotes_file_id`, `emotes_name_trgm`), so it costs the same on a big table.
pub async fn duplicates(
    conn: &mut PgConnection,
    id: Uuid,
    limit: i64,
) -> DbResult<Vec<DuplicateHit>> {
    Ok(sqlx::query_as(
        "WITH me AS (SELECT id, name, file_id FROM emotes WHERE id = $1),
         hits AS (
             SELECT e.id FROM emotes e, me
             WHERE me.file_id IS NOT NULL AND e.file_id = me.file_id
               AND e.id <> me.id AND e.deleted_at IS NULL
             UNION
             SELECT e.id FROM emotes e, me
             WHERE e.name % me.name AND e.id <> me.id AND e.deleted_at IS NULL
         )
         SELECT e.id,
                (me.file_id IS NOT NULL AND e.file_id IS NOT DISTINCT FROM me.file_id) AS same_file,
                CASE WHEN lower(e.name) = lower(me.name) THEN 1::real
                     ELSE similarity(e.name, me.name) END AS similarity
         FROM hits h JOIN emotes e ON e.id = h.id, me
         WHERE (me.file_id IS NOT NULL AND e.file_id IS NOT DISTINCT FROM me.file_id)
            OR lower(e.name) = lower(me.name)
            OR similarity(e.name, me.name) >= $3
         ORDER BY 2 DESC, 3 DESC, e.created_at, e.id
         LIMIT $2",
    )
    .bind(id)
    .bind(limit)
    .bind(DUPLICATE_NAME_SIMILARITY)
    .fetch_all(conn)
    .await?)
}

/// Public emotes made from the file with this sha256: approved, neither unlisted nor private, not
/// deleted, and not of an account being deleted. Oldest first.
pub async fn public_by_sha256(
    conn: &mut PgConnection,
    sha256: &[u8],
    limit: i64,
) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT e.id FROM files f
         JOIN emotes e ON e.file_id = f.id
         JOIN accounts a ON a.id = e.owner_account_id AND a.deleted_at IS NULL
         WHERE f.sha256 = $1 AND e.deleted_at IS NULL AND e.state = 'approved'
           AND NOT e.private AND NOT e.unlisted
         ORDER BY e.created_at, e.id
         LIMIT $2",
    )
    .bind(sha256)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
