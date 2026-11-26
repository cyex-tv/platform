//! Batch reads for building API answers: a whole set with its emotes, files, tags and owners in a
//! few queries instead of a few per emote.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{AccountSummary, EmoteState};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::keyset::{self, Key, KeyType};

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
    /// The file's transcode failed for good.
    pub processing_failed: bool,
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
    /// The file's transcode failed for good.
    pub processing_failed: bool,
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
            processing_failed: self.processing_failed,
        }
    }
}

const EMOTE_ROW: &str = "SELECT e.id, e.owner_account_id AS owner, e.name, e.zero_width, e.private,
            e.unlisted, e.nsfw, e.animated, e.state, e.created_at, f.sha256, f.variants,
            (f.processing_failed_at IS NOT NULL) AS processing_failed
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
    /// Only this uploader's emotes.
    pub uploader: Option<Uuid>,
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

/// The keys a review queue read is ordered by, for cursor paging: the order of its numbered
/// pages, with `NULLS LAST` spelled as an "is missing" flag first so no key is ever NULL.
pub fn queue_keys(q: &QueueQuery<'_>) -> Vec<Key> {
    let way = |expr: String, ty: KeyType| {
        if q.descending {
            Key::desc(expr, ty)
        } else {
            Key::asc(expr, ty)
        }
    };
    match q.sort {
        QueueSort::Uploaded => vec![
            way(
                "coalesce(e.requeued_at, e.created_at)".into(),
                KeyType::Time,
            ),
            way("e.id".into(), KeyType::Uuid),
        ],
        QueueSort::Reviewed => vec![
            Key::asc("(e.reviewed_at IS NULL)", KeyType::Bool),
            way(
                "coalesce(e.reviewed_at, 'epoch'::timestamptz)".into(),
                KeyType::Time,
            ),
            way("e.created_at".into(), KeyType::Time),
            way("e.id".into(), KeyType::Uuid),
        ],
        QueueSort::Uploader => vec![
            Key::asc(format!("(lower({UPLOADER}) IS NULL)"), KeyType::Bool),
            way(format!("coalesce(lower({UPLOADER}), '')"), KeyType::Text),
            Key::asc("e.created_at", KeyType::Time),
            Key::asc("e.id", KeyType::Uuid),
        ],
        QueueSort::Name => vec![
            way("lower(e.name)".into(), KeyType::Text),
            Key::asc("e.created_at", KeyType::Time),
            Key::asc("e.id", KeyType::Uuid),
        ],
    }
}

/// [`EMOTE_ROW`] with `cols` added to its select list.
fn emote_row_with(cols: &str) -> String {
    EMOTE_ROW.replacen(
        "
     FROM emotes e",
        &format!(
            "{cols}
     FROM emotes e"
        ),
        1,
    )
}

/// A page of the review queue and how many match in all: from `offset`, or after the position
/// `after` ([`queue_keys`]) when set; the page says where the next one starts.
pub async fn review_queue_page(
    conn: &mut PgConnection,
    q: &QueueQuery<'_>,
    limit: i64,
    offset: i64,
    after: Option<&[keyset::Value]>,
) -> DbResult<(keyset::Page<EmoteRow>, i64)> {
    let keys = queue_keys(q);
    if after.is_some_and(|a| !keyset::fits(&keys, a)) {
        return Err(crate::error::DbError::NotFound);
    }
    let tab = queue_tab(q.tab);
    let dir = if q.descending { "DESC" } else { "ASC" };
    let order = match q.sort {
        QueueSort::Uploaded => {
            format!("coalesce(e.requeued_at, e.created_at) {dir}, e.id {dir}")
        }
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
    let (position, fetch, order) = match after {
        Some(_) => (keyset::after(&keys, 5), limit + 1, keyset::order(&keys)),
        None => ("TRUE".to_owned(), limit, order),
    };
    let sql = format!(
        "{} WHERE {tab} AND e.deleted_at IS NULL AND {QUEUE_MATCHES} AND {position}
         ORDER BY {order} LIMIT $2 OFFSET $3",
        emote_row_with(&keyset::columns(&keys))
    );
    let query = sqlx::query(&sql)
        .bind(pattern.as_deref())
        .bind(fetch)
        .bind(if after.is_some() { 0 } else { offset })
        .bind(q.uploader);
    let rows = keyset::bind(query, after.unwrap_or_default())
        .fetch_all(&mut *conn)
        .await?;
    let rows = keyset::decode::<EmoteRow>(&rows, &keys)?;
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM emotes e WHERE {tab} AND e.deleted_at IS NULL AND {QUEUE_MATCHES}"
    ))
    .bind(pattern.as_deref())
    .bind(None::<i64>)
    .bind(None::<i64>)
    .bind(q.uploader)
    .fetch_one(conn)
    .await?;
    let page = match after {
        Some(_) => keyset::split(rows, limit),
        None => {
            let more = offset + (rows.len() as i64) < total;
            keyset::finish(rows, more)
        }
    };
    Ok((page, total))
}

/// The review queue's search (`$1`, a LIKE pattern) and uploader (`$4`).
const QUEUE_MATCHES: &str = "($1::text IS NULL OR e.name ILIKE $1 OR EXISTS (
            SELECT 1 FROM identities i WHERE i.account_id = e.owner_account_id
              AND (i.login ILIKE $1 OR i.display_name ILIKE $1)))
         AND ($4::uuid IS NULL OR e.owner_account_id = $4)";

fn queue_tab(tab: QueueTab) -> &'static str {
    match tab {
        QueueTab::Pending => "e.state = 'pending'",
        QueueTab::Public => "e.state = 'pending' AND NOT e.private AND NOT e.unlisted",
        QueueTab::Limited => "e.state = 'pending' AND (e.private OR e.unlisted)",
        QueueTab::Resolved => "e.state IN ('approved', 'rejected') AND e.reviewed_at IS NOT NULL",
    }
}

/// The uploaders with the most emotes in one tab of the review queue (its search applies, the
/// uploader filter doesn't), most first, then the newest upload.
pub async fn queue_top_uploaders(
    conn: &mut PgConnection,
    tab: QueueTab,
    search: Option<&str>,
    limit: i64,
) -> DbResult<Vec<crate::audit::PersonCount>> {
    let tab = queue_tab(tab);
    let pattern = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", crate::search::like_escape(s)));
    Ok(sqlx::query_as(&format!(
        "SELECT e.owner_account_id AS account_id, count(*) AS count
         FROM emotes e
         JOIN accounts x ON x.id = e.owner_account_id AND x.deleted_at IS NULL
         WHERE {tab} AND e.deleted_at IS NULL AND {QUEUE_MATCHES}
         GROUP BY e.owner_account_id
         ORDER BY count(*) DESC, max(e.created_at) DESC
         LIMIT $2"
    ))
    .bind(pattern.as_deref())
    .bind(limit)
    .bind(None::<i64>)
    .bind(None::<Uuid>)
    .fetch_all(conn)
    .await?)
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

/// The keys an account's own emotes are ordered by: newest first, then ID.
pub fn owner_keys() -> Vec<Key> {
    vec![
        Key::desc("e.created_at", KeyType::Time),
        Key::desc("e.id", KeyType::Uuid),
    ]
}

/// An account's own emotes in every state, newest first, and how many there are.
pub async fn emotes_by_owner(
    conn: &mut PgConnection,
    owner: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<EmoteRow>, i64)> {
    let (page, total) = emotes_by_owner_page(conn, owner, limit, offset, None).await?;
    Ok((page.rows, total))
}

/// `emotes_by_owner` from `offset`, or after the position `after` ([`owner_keys`]) when set.
pub async fn emotes_by_owner_page(
    conn: &mut PgConnection,
    owner: Uuid,
    limit: i64,
    offset: i64,
    after: Option<&[keyset::Value]>,
) -> DbResult<(keyset::Page<EmoteRow>, i64)> {
    let keys = owner_keys();
    if after.is_some_and(|a| !keyset::fits(&keys, a)) {
        return Err(crate::error::DbError::NotFound);
    }
    let (position, fetch) = match after {
        Some(_) => (keyset::after(&keys, 4), limit + 1),
        None => ("TRUE".to_owned(), limit),
    };
    let sql = format!(
        "{} WHERE e.owner_account_id = $1 AND e.deleted_at IS NULL AND {position}
         ORDER BY e.created_at DESC, e.id DESC LIMIT $2 OFFSET $3",
        emote_row_with(&keyset::columns(&keys))
    );
    let query = sqlx::query(&sql)
        .bind(owner)
        .bind(fetch)
        .bind(if after.is_some() { 0 } else { offset });
    let rows = keyset::bind(query, after.unwrap_or_default())
        .fetch_all(&mut *conn)
        .await?;
    let rows = keyset::decode::<EmoteRow>(&rows, &keys)?;
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM emotes WHERE owner_account_id = $1 AND deleted_at IS NULL",
    )
    .bind(owner)
    .fetch_one(conn)
    .await?;
    let page = match after {
        Some(_) => keyset::split(rows, limit),
        None => {
            let more = offset + (rows.len() as i64) < total;
            keyset::finish(rows, more)
        }
    };
    Ok((page, total))
}

macro_rules! member_columns {
    () => {
        "m.set_id, m.emote_id, m.name, m.zero_width, m.added_by, m.added_at,
                e.owner_account_id AS emote_owner, e.name AS emote_name,
                e.zero_width AS emote_zero_width, e.private, e.unlisted, e.nsfw,
                e.animated, e.state, e.created_at AS emote_created_at,
                f.sha256, f.variants, (f.processing_failed_at IS NOT NULL) AS processing_failed"
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

/// The statement `newest_members` runs, with its binds `$1` to `$5` in that order. Public so the
/// scale test can ask Postgres for its plan.
#[doc(hidden)]
pub fn newest_members_sql() -> String {
    // Per set, the members in index order (`emote_set_members_set_newest`) until `per_set` pass
    // the access rule, instead of numbering every member of every set.
    format!(
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
    )
}

/// The newest `per_set` members each of the given sets shows `access`, newest first within each
/// set (by when they were added). Only that many rows are read, whatever the sets hold.
pub async fn newest_members(
    conn: &mut PgConnection,
    set_ids: &[Uuid],
    access: &MemberAccess<'_>,
    per_set: i64,
) -> DbResult<Vec<MemberRow>> {
    Ok(sqlx::query_as(&newest_members_sql())
        .bind(set_ids)
        .bind(access.reviewer)
        .bind(access.edit_sets)
        .bind(access.account)
        .bind(per_set)
        .fetch_all(conn)
        .await?)
}

/// The keys a set's members are ordered by: oldest added first, then emote ID.
pub fn member_keys() -> Vec<Key> {
    vec![
        Key::asc("m.added_at", KeyType::Time),
        Key::asc("m.emote_id", KeyType::Uuid),
    ]
}

/// A page of one set's members that `access` shows, oldest first (the order `members` reads),
/// narrowed to the ones whose name in the set or own name contains `query` (case-insensitive,
/// literally), with how many match in all. From `offset`, or after the position `after`
/// ([`member_keys`]) when set; the page says where the next one starts.
pub async fn members_page(
    conn: &mut PgConnection,
    set_id: Uuid,
    access: &MemberAccess<'_>,
    query: Option<&str>,
    limit: i64,
    offset: i64,
    after: Option<&[keyset::Value]>,
) -> DbResult<(keyset::Page<MemberRow>, i64)> {
    let keys = member_keys();
    if after.is_some_and(|a| !keyset::fits(&keys, a)) {
        return Err(crate::error::DbError::NotFound);
    }
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
    let (position, fetch) = match after {
        Some(_) => (keyset::after(&keys, 8), limit + 1),
        None => ("TRUE".to_owned(), limit),
    };
    let sql = format!(
        "SELECT {}{}
         FROM emote_set_members m
         JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
         LEFT JOIN files f ON f.id = e.file_id
         WHERE {RULE} AND {position}
         ORDER BY m.added_at, m.emote_id
         LIMIT $6 OFFSET $7",
        member_columns!(),
        keyset::columns(&keys)
    );
    let rows = sqlx::query(&sql)
        .bind(set_id)
        .bind(access.reviewer)
        .bind(access.edit_sets)
        .bind(access.account)
        .bind(pattern.as_deref())
        .bind(fetch)
        .bind(if after.is_some() { 0 } else { offset });
    let rows = keyset::bind(rows, after.unwrap_or_default())
        .fetch_all(&mut *conn)
        .await?;
    let rows = keyset::decode::<MemberRow>(&rows, &keys)?;
    let total: i64 = sqlx::query_scalar(&format!(
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
    let page = match after {
        Some(_) => keyset::split(rows, limit),
        None => {
            let more = offset + (rows.len() as i64) < total;
            keyset::finish(rows, more)
        }
    };
    Ok((page, total))
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

/// The members of one set among `emote_ids` (deleted emotes left out), oldest first: what a
/// delta read (ADR 0035) shows of the members that changed.
pub async fn members_among(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_ids: &[Uuid],
) -> DbResult<Vec<MemberRow>> {
    if emote_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as(&format!(
        "{MEMBER_ROW} WHERE m.set_id = $1 AND m.emote_id = ANY($2) ORDER BY m.added_at, m.emote_id"
    ))
    .bind(set_id)
    .bind(emote_ids)
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

/// The statement [`channel_counts`] runs (`$1`: the emote IDs). Public for the scale test's plan.
#[doc(hidden)]
pub const CHANNEL_COUNTS_SQL: &str =
    "SELECT id, channel_count FROM emotes WHERE id = ANY($1) AND channel_count > 0";

/// How many channels have each emote in their active set. Channels of deleted or blocked
/// accounts don't count. Emotes in no channel are missing from the map. The stored count
/// (`emotes.channel_count`, see [`crate::channel_counts`]): at most about a minute old, one
/// primary-key read per emote.
pub async fn channel_counts(
    conn: &mut PgConnection,
    emote_ids: &[Uuid],
) -> DbResult<HashMap<Uuid, u32>> {
    let rows: Vec<(Uuid, i32)> = sqlx::query_as(CHANNEL_COUNTS_SQL)
        .bind(emote_ids)
        .fetch_all(conn)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, n)| (id, u32::try_from(n).unwrap_or(0)))
        .collect())
}

/// The statement [`emote_channels`] runs (`$1` emote, `$2` limit, `$3` offset). Public for the
/// scale test's plan.
#[doc(hidden)]
pub const EMOTE_CHANNELS_SQL: &str =
    "SELECT c.id, c.account_id, c.platform, c.platform_channel_id, c.login, c.active_set_id,
                c.allow_personal_emotes, c.created_at, c.updated_at
         FROM emote_set_members m
         JOIN channels c ON c.active_set_id = m.set_id
         JOIN accounts a ON a.id = c.account_id AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         WHERE m.emote_id = $1
         ORDER BY m.added_at DESC, c.id
         LIMIT $2 OFFSET $3";

/// Channels whose active set holds the emote, the most recently added first, `offset` rows in.
/// Channels of deleted or blocked accounts are left out.
pub async fn emote_channels(
    conn: &mut PgConnection,
    emote_id: Uuid,
    limit: i64,
    offset: i64,
) -> DbResult<Vec<crate::channels::Channel>> {
    Ok(sqlx::query_as(EMOTE_CHANNELS_SQL)
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

/// Public emotes made from the file with this sha256: listed by the rule every emote search
/// shares ([`crate::search::LISTED_EMOTE`]), so the uploader is never told about an emote search
/// wouldn't show. Oldest first.
pub async fn public_by_sha256(
    conn: &mut PgConnection,
    sha256: &[u8],
    limit: i64,
) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT e.id FROM files f
         JOIN emotes e ON e.file_id = f.id
         WHERE f.sha256 = $1 AND e.owner_account_id IS NOT NULL AND {}
         ORDER BY e.created_at, e.id
         LIMIT $2",
        crate::search::LISTED_EMOTE
    ))
    .bind(sha256)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_columns_go_into_the_emote_select_list() {
        let sql = emote_row_with(", 1 AS keyset_0");
        assert!(
            sql.contains("AS processing_failed, 1 AS keyset_0\n     FROM emotes e"),
            "{sql}"
        );
        assert_eq!(emote_row_with(""), EMOTE_ROW);
    }
}
