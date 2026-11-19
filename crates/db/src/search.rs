//! Emote search (ADR 0004): Postgres with `pg_trgm` for names, prefix matches for tags, and a
//! popularity column. Handlers use [`EmoteSearch`], so moving to Typesense or Meilisearch later
//! is a new implementation of it, not a change to them.
//!
//! Only approved, public (not private or unlisted), not deleted emotes are found, and none of an
//! account whose deletion is pending (ADR 0019). NSFW ones only when asked.
//! Ranking: exact name, then name prefix, then name similarity, then popularity.

use cyex_tv_shared::model::EmoteSort;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::error::DbResult;

/// What to look for. `query` is trimmed and at most 100 characters.
#[derive(Clone, Debug, Default)]
pub struct Query<'a> {
    pub text: &'a str,
    pub animated: Option<bool>,
    pub owner: Option<Uuid>,
    pub nsfw: bool,
    pub sort: EmoteSort,
    pub limit: i64,
    pub offset: i64,
}

/// Matching emote IDs in rank order, and how many match in all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hits {
    pub ids: Vec<Uuid>,
    pub total: i64,
    /// `total` stopped counting at `ACCOUNT_TOTAL_CAP` (account search only).
    pub capped: bool,
}

/// `accounts` counts matches up to this many; more are reported as this, `capped`.
pub const ACCOUNT_TOTAL_CAP: i64 = 1000;
/// Shorter queries (in characters, after trimming) match nobody: one letter is every account.
pub const ACCOUNT_QUERY_MIN: usize = 2;

#[allow(async_fn_in_trait)]
pub trait EmoteSearch {
    async fn search(&self, query: &Query<'_>) -> DbResult<Hits>;
}

/// The Postgres implementation.
pub struct PgSearch {
    pub pool: PgPool,
}

/// `%` and `_` mean something in `LIKE`; a search for them means the characters.
fn like_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub async fn search(conn: &mut PgConnection, q: &Query<'_>) -> DbResult<Hits> {
    let text: String = q.text.trim().chars().take(100).collect();
    let pattern = like_escape(&text.to_lowercase());
    // The order is one of three fixed strings, never user text.
    let order = match q.sort {
        EmoteSort::Relevance => {
            "($1 <> '' AND lower(e.name) = lower($1)) DESC,
                  ($1 <> '' AND lower(e.name) LIKE $5 || '%') DESC,
                  CASE WHEN $1 = '' THEN 0 ELSE similarity(e.name, $1) END DESC,
                  e.popularity DESC,
                  e.id"
        }
        EmoteSort::Top => "e.popularity DESC, lower(e.name), e.id",
        EmoteSort::New => "e.created_at DESC, e.id DESC",
    };
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
        "SELECT e.id, count(*) OVER () AS total
         FROM emotes e
         WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
           AND NOT e.unlisted
           AND NOT EXISTS (SELECT 1 FROM accounts oa
                           WHERE oa.id = e.owner_account_id AND oa.deleted_at IS NOT NULL)
           AND ($2::boolean IS NULL OR e.animated = $2)
           AND ($3::uuid IS NULL OR e.owner_account_id = $3)
           AND ($4 OR NOT e.nsfw)
           AND ($1 = ''
                OR lower(e.name) LIKE '%' || $5 || '%'
                OR e.name % $1
                OR EXISTS (SELECT 1 FROM emote_tags t
                           WHERE t.emote_id = e.id AND t.tag LIKE $5 || '%'))
         ORDER BY {order}
         LIMIT $6 OFFSET $7"
    ))
    .bind(&text)
    .bind(q.animated)
    .bind(q.owner)
    .bind(q.nsfw)
    .bind(&pattern)
    .bind(q.limit)
    .bind(q.offset)
    .fetch_all(conn)
    .await?;
    let total = rows.first().map_or(0, |(_, t)| *t);
    Ok(Hits {
        ids: rows.into_iter().map(|(id, _)| id).collect(),
        total,
        capped: false,
    })
}

/// The header search: up to `limit` emote IDs for `text` (trimmed, at most 100 characters; empty
/// finds nothing), best first. Same visibility as [`search`] (approved, listed, not deleted, no
/// NSFW). Tiers: exact name, same ignoring case, name prefix, word or initials prefix
/// (`PepePoint` is found by `point` and `pp`), substring, then, from 3 characters, similar names.
/// Within a tier: most used, shorter name, name, ID; similar names by similarity first.
pub async fn quick(conn: &mut PgConnection, text: &str, limit: i64) -> DbResult<Vec<Uuid>> {
    let text: String = text.trim().chars().take(100).collect();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let lower = text.to_lowercase();
    let pattern = like_escape(&lower);
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM (
             SELECT e.id, e.name, e.popularity,
                    CASE WHEN e.name = $1 THEN 1
                         WHEN lower(e.name) = $2 THEN 2
                         WHEN e.name ILIKE $3 || '%' THEN 3
                         WHEN (' ' || e.name_words) LIKE '% ' || $3 || '%'
                              OR e.name_initials LIKE $3 || '%' THEN 4
                         WHEN e.name ILIKE '%' || $3 || '%' THEN 5
                         ELSE 6 END AS tier,
                    similarity(e.name, $1) AS sim
             FROM emotes e
             WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
               AND NOT e.nsfw
               AND (e.name ILIKE '%' || $3 || '%'
                    OR (' ' || e.name_words) LIKE '% ' || $3 || '%'
                    OR e.name_initials LIKE $3 || '%'
                    OR ($4 AND e.name % $1))
         ) r
         ORDER BY tier,
                  CASE WHEN tier = 6 THEN sim END DESC NULLS LAST,
                  popularity DESC,
                  CASE WHEN tier < 6 THEN char_length(name) END,
                  CASE WHEN tier < 6 THEN lower(name) END,
                  id
         LIMIT $5",
    )
    .bind(&text)
    .bind(&lower)
    .bind(&pattern)
    .bind(text.chars().count() >= 3)
    .bind(limit)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Live accounts (not deleted, not blocked) whose Twitch or Kick login or display name matches
/// `text` (lowercase, trimmed, at most 64 characters, at least `ACCOUNT_QUERY_MIN`), in rank order,
/// and how many match (counted up to `ACCOUNT_TOTAL_CAP`).
/// Ranking: exact login, then login prefix, then display name prefix, then similarity.
pub async fn accounts(
    conn: &mut PgConnection,
    text: &str,
    limit: i64,
    offset: i64,
) -> DbResult<Hits> {
    let hits = accounts_window(conn, text, limit, offset).await?;
    // A page past the last one has no rows to carry the count; ask for the first row instead.
    if hits.ids.is_empty() && offset > 0 {
        let first = accounts_window(conn, text, 1, 0).await?;
        return Ok(Hits {
            ids: Vec::new(),
            ..first
        });
    }
    Ok(hits)
}

async fn accounts_window(
    conn: &mut PgConnection,
    text: &str,
    limit: i64,
    offset: i64,
) -> DbResult<Hits> {
    let text: String = text.trim().to_lowercase().chars().take(64).collect();
    if text.chars().count() < ACCOUNT_QUERY_MIN {
        return Ok(Hits {
            ids: Vec::new(),
            total: 0,
            capped: false,
        });
    }
    let pattern = like_escape(&text);
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "WITH hits AS (
             SELECT i.account_id,
                    max(CASE WHEN lower(i.login) = $1 THEN 3
                             WHEN lower(i.login) LIKE $2 || '%' THEN 2
                             WHEN lower(i.display_name) LIKE $2 || '%' THEN 1
                             ELSE 0 END) AS tier,
                    max(greatest(similarity(lower(i.login), $1),
                                 similarity(lower(i.display_name), $1))) AS score
             FROM identities i
             JOIN accounts a ON a.id = i.account_id
                            AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())
             WHERE lower(i.login) LIKE '%' || $2 || '%'
                OR lower(i.display_name) LIKE '%' || $2 || '%'
                OR lower(i.login) % $1
                OR lower(i.display_name) % $1
             GROUP BY i.account_id
         )
         , ranked AS (
             SELECT account_id, tier, score FROM hits
             ORDER BY tier DESC, score DESC, account_id
             LIMIT $5
         )
         SELECT account_id, count(*) OVER () AS total
         FROM ranked
         ORDER BY tier DESC, score DESC, account_id
         LIMIT $3 OFFSET $4",
    )
    .bind(&text)
    .bind(&pattern)
    .bind(limit)
    .bind(offset)
    .bind(ACCOUNT_TOTAL_CAP + 1)
    .fetch_all(conn)
    .await?;
    let counted = rows.first().map_or(0, |(_, t)| *t);
    Ok(Hits {
        ids: rows.into_iter().map(|(id, _)| id).collect(),
        total: counted.min(ACCOUNT_TOTAL_CAP),
        capped: counted > ACCOUNT_TOTAL_CAP,
    })
}

impl EmoteSearch for PgSearch {
    async fn search(&self, query: &Query<'_>) -> DbResult<Hits> {
        let mut conn = self.pool.acquire().await?;
        search(&mut conn, query).await
    }
}

/// Recomputes `popularity`: channels whose active set holds the emote. Returns rows changed.
pub async fn refresh_popularity(conn: &mut PgConnection) -> DbResult<u64> {
    let done = sqlx::query(
        "WITH used AS (
             SELECT m.emote_id, count(DISTINCT c.id)::integer AS n
             FROM channels c
             JOIN emote_set_members m ON m.set_id = c.active_set_id
             GROUP BY m.emote_id
         )
         UPDATE emotes e SET popularity = coalesce(u.n, 0)
         FROM emotes e2
         LEFT JOIN used u ON u.emote_id = e2.id
         WHERE e.id = e2.id AND e.popularity <> coalesce(u.n, 0)",
    )
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_patterns_are_escaped() {
        assert_eq!(like_escape("50%_off"), "50\\%\\_off");
        assert_eq!(like_escape("a\\b"), "a\\\\b");
        assert_eq!(like_escape("pog"), "pog");
    }
}
