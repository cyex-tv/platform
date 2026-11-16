//! Emote search (ADR 0004): Postgres with `pg_trgm` for names, prefix matches for tags, and a
//! popularity column. Handlers use [`EmoteSearch`], so moving to Typesense or Meilisearch later
//! is a new implementation of it, not a change to them.
//!
//! Only approved, listed (not private), not deleted emotes are found. NSFW ones only when asked.
//! Ranking: exact name, then name prefix, then name similarity, then popularity.

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
    pub limit: i64,
    pub offset: i64,
}

/// Matching emote IDs in rank order, and how many match in all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hits {
    pub ids: Vec<Uuid>,
    pub total: i64,
}

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
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT e.id, count(*) OVER () AS total
         FROM emotes e
         WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
           AND ($2::boolean IS NULL OR e.animated = $2)
           AND ($3::uuid IS NULL OR e.owner_account_id = $3)
           AND ($4 OR NOT e.nsfw)
           AND ($1 = ''
                OR lower(e.name) LIKE '%' || $5 || '%'
                OR e.name % $1
                OR EXISTS (SELECT 1 FROM emote_tags t
                           WHERE t.emote_id = e.id AND t.tag LIKE $5 || '%'))
         ORDER BY ($1 <> '' AND lower(e.name) = lower($1)) DESC,
                  ($1 <> '' AND lower(e.name) LIKE $5 || '%') DESC,
                  CASE WHEN $1 = '' THEN 0 ELSE similarity(e.name, $1) END DESC,
                  e.popularity DESC,
                  e.id
         LIMIT $6 OFFSET $7",
    )
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
