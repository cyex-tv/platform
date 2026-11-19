//! Emote search (ADR 0004): Postgres with `pg_trgm` for names, prefix matches for tags, and a
//! popularity column. Handlers use [`EmoteSearch`], so moving to Typesense or Meilisearch later
//! is a new implementation of it, not a change to them.
//!
//! Only approved, public (not private or unlisted), not deleted emotes are found, and none of an
//! account whose deletion is pending (ADR 0019). NSFW ones only when asked.
//! Ranking: exact name, then name prefix, then name similarity, then popularity.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EmoteFlag, EmoteSort, TrendingWindow};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids;

/// What to look for. `query` is trimmed and at most 100 characters.
#[derive(Clone, Debug, Default)]
pub struct Query<'a> {
    pub text: &'a str,
    pub animated: Option<bool>,
    pub owner: Option<Uuid>,
    pub nsfw: bool,
    /// Every listed flag must be set. Duplicates are ignored.
    pub flags: &'a [EmoteFlag],
    /// Every tag must be on the emote (exact, after trim, lowercase and a leading `#` removed).
    /// Empty ones are ignored and only the first 10 count.
    pub tags: &'a [String],
    /// The name must equal `text` (any case) instead of matching it roughly.
    pub exact: bool,
    /// Only for `EmoteSort::Trending`.
    pub window: TrendingWindow,
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
pub(crate) fn like_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Most tags `search` looks at.
pub const TAGS_MAX: usize = 10;
/// A tag's length at most, in characters (as uploads take them).
pub const TAG_LEN_MAX: usize = 32;

/// The tags as stored: trimmed, lowercase, no leading `#`, no empties or repeats, the first 10.
/// None when one can never be on an emote (they are 1 to 32 letters, digits, `-` or `_`).
fn clean_tags(tags: &[String]) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for raw in tags {
        let tag = raw.trim().to_lowercase();
        let tag = tag.trim_start_matches('#');
        if tag.is_empty() || out.iter().any(|t| t == tag) {
            continue;
        }
        if out.len() == TAGS_MAX {
            break;
        }
        out.push(tag.to_owned());
    }
    // The same rule uploads and edits store tags by (1 to 32 letters, digits, `-` or `_`), so
    // every stored tag can be searched for.
    let valid = |t: &String| {
        t.chars().count() <= TAG_LEN_MAX
            && t.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    };
    out.iter().all(valid).then_some(out)
}

/// Net channel adds per emote in the last `days`, only those above zero.
fn net_adds(days: i32) -> String {
    format!(
        "SELECT emote_id, sum(adds - removes)::bigint AS net
         FROM emote_channel_adds
         WHERE hour > now() - make_interval(days => {days})
         GROUP BY emote_id
         HAVING sum(adds - removes) > 0"
    )
}

pub async fn search(conn: &mut PgConnection, q: &Query<'_>) -> DbResult<Hits> {
    let Some(tags) = clean_tags(q.tags) else {
        return Ok(Hits {
            ids: Vec::new(),
            total: 0,
            capped: false,
        });
    };
    let wants = |f: EmoteFlag| q.flags.contains(&f);
    let text: String = q.text.trim().chars().take(100).collect();
    let pattern = like_escape(&text.to_lowercase());
    // The order and the join are fixed strings, never user text.
    let mut join = String::new();
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
        EmoteSort::Trending => {
            join = format!(
                "JOIN ({}) tr ON tr.emote_id = e.id",
                net_adds(q.window.days())
            );
            "tr.net DESC, e.popularity DESC, e.id"
        }
    };
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
        "SELECT e.id, count(*) OVER () AS total
         FROM emotes e
         {join}
         WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
           AND NOT e.unlisted
           AND NOT EXISTS (SELECT 1 FROM accounts oa
                           WHERE oa.id = e.owner_account_id AND oa.deleted_at IS NOT NULL)
           AND ($2::boolean IS NULL OR e.animated = $2)
           AND ($3::uuid IS NULL OR e.owner_account_id = $3)
           AND ($4 OR $10 OR NOT e.nsfw)
           AND (NOT $10 OR e.nsfw)
           AND (NOT $8 OR e.zero_width)
           AND (NOT $9 OR EXISTS (SELECT 1 FROM files f
                                  WHERE f.id = e.file_id AND f.width * 4 >= f.height * 7))
           AND (SELECT count(*) FROM emote_tags t
                WHERE t.emote_id = e.id AND t.tag = ANY($11::text[])) = cardinality($11::text[])
           AND ($1 = ''
                OR ($12 AND lower(e.name) = lower($1))
                OR (NOT $12
                    AND (lower(e.name) LIKE '%' || $5 || '%'
                         OR e.name % $1
                         OR EXISTS (SELECT 1 FROM emote_tags t
                                    WHERE t.emote_id = e.id AND t.tag LIKE $5 || '%'))))
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
    .bind(wants(EmoteFlag::ZeroWidth))
    .bind(wants(EmoteFlag::Wide))
    .bind(wants(EmoteFlag::Nsfw))
    .bind(&tags)
    .bind(q.exact)
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

/// Adds the hourly buckets of `emote_channel_adds` up from the events outbox: how many distinct
/// channels added and removed each emote in each hour. It redoes the hour of the newest bucket
/// onwards (the whole outbox when the table is empty), so it can run any time and twice gives the
/// same table. Run it before the outbox is pruned. Returns the buckets written.
pub async fn rollup_channel_adds(conn: &mut PgConnection) -> DbResult<u64> {
    let mut tx = conn.begin().await?;
    // Never earlier than the outbox still reaches, or a bucket would be redone from half its events.
    let since: DateTime<Utc> = sqlx::query_scalar(
        "SELECT CASE WHEN max(hour) IS NULL THEN date_trunc('hour', now() - interval '48 hours')
                     ELSE greatest(max(hour), date_trunc('hour', now()) - interval '47 hours')
                END
         FROM emote_channel_adds",
    )
    .fetch_one(&mut *tx)
    .await?;
    let rows: Vec<(Option<String>, DateTime<Utc>, i64, i64)> = sqlx::query_as(
        "SELECT o.payload->>'emote_id', date_trunc('hour', o.created_at) AS hour,
                count(DISTINCT t.topic) FILTER (WHERE o.type = 'set.emote_added'),
                count(DISTINCT t.topic) FILTER (WHERE o.type = 'set.emote_removed')
         FROM events_outbox o, unnest(o.topics) AS t(topic)
         WHERE o.type IN ('set.emote_added', 'set.emote_removed')
           AND o.created_at >= $1
           AND t.topic LIKE 'channel:%'
         GROUP BY 1, 2",
    )
    .bind(since)
    .fetch_all(&mut *tx)
    .await?;
    // Two spellings of one ID land in one bucket.
    let mut buckets: HashMap<(Uuid, DateTime<Utc>), (i32, i32)> = HashMap::new();
    for (emote, hour, adds, removes) in rows {
        let Some(id) = emote.as_deref().and_then(ids::from_text) else {
            continue;
        };
        let b = buckets.entry((id, hour)).or_default();
        b.0 = b.0.saturating_add(i32::try_from(adds).unwrap_or(i32::MAX));
        b.1 =
            b.1.saturating_add(i32::try_from(removes).unwrap_or(i32::MAX));
    }
    let (mut emote_ids, mut hours, mut adds, mut removes) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for ((id, hour), (a, r)) in buckets {
        emote_ids.push(id);
        hours.push(hour);
        adds.push(a);
        removes.push(r);
    }
    sqlx::query("DELETE FROM emote_channel_adds WHERE hour >= $1")
        .bind(since)
        .execute(&mut *tx)
        .await?;
    let done = sqlx::query(
        "INSERT INTO emote_channel_adds (emote_id, hour, adds, removes)
         SELECT u.emote_id, u.hour, u.adds, u.removes
         FROM unnest($1::uuid[], $2::timestamptz[], $3::int[], $4::int[])
              AS u(emote_id, hour, adds, removes)
         JOIN emotes e ON e.id = u.emote_id",
    )
    .bind(&emote_ids)
    .bind(&hours)
    .bind(&adds)
    .bind(&removes)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(done.rows_affected())
}

/// Where an emote stands in `TRENDING` over the last `window_days`: its 1-based rank and net
/// channel adds. Among approved, public (not private or unlisted), not deleted emotes with net adds above zero, ordered as
/// the `TRENDING` sort is; NSFW emotes count only when this one is NSFW. None when it isn't among
/// them or ranks below `top`.
pub async fn trending_rank(
    conn: &mut PgConnection,
    emote_id: Uuid,
    window_days: i32,
    top: i64,
) -> DbResult<Option<(i64, i64)>> {
    let net = net_adds(window_days);
    Ok(sqlx::query_as(&format!(
        "SELECT rank, net FROM (
             SELECT e.id, tr.net,
                    row_number() OVER (ORDER BY tr.net DESC, e.popularity DESC, e.id) AS rank
             FROM emotes e
             JOIN ({net}) tr ON tr.emote_id = e.id
             WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
               AND NOT e.unlisted
               AND NOT EXISTS (SELECT 1 FROM accounts oa
                               WHERE oa.id = e.owner_account_id AND oa.deleted_at IS NOT NULL)
               AND (NOT e.nsfw OR coalesce((SELECT nsfw FROM emotes WHERE id = $1), false))
         ) ranked
         WHERE id = $1 AND rank <= $2"
    ))
    .bind(emote_id)
    .bind(top)
    .fetch_optional(conn)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| (*t).to_owned()).collect()
    }

    #[test]
    fn tags_are_cleaned_and_bad_ones_match_nothing() {
        assert_eq!(
            clean_tags(&tags(&[" #Pog ", "pog", "", "#", "a_1"])),
            Some(tags(&["pog", "a_1"]))
        );
        assert_eq!(clean_tags(&tags(&["a b"])), None);
        assert_eq!(clean_tags(&tags(&["a!"])), None);
        assert_eq!(clean_tags(&["a".repeat(33)]), None);
        // Every tag an upload may carry can be searched for: dashes, non-Latin letters, 32 long.
        assert_eq!(
            clean_tags(&tags(&["Pepe-the-frog", "猫ダンス", &"a".repeat(32)])),
            Some(tags(&["pepe-the-frog", "猫ダンス", &"a".repeat(32)]))
        );
        assert_eq!(clean_tags(&[]), Some(Vec::new()));
        let many: Vec<String> = (0..15).map(|i| format!("t{i}")).collect();
        assert_eq!(clean_tags(&many).map(|t| t.len()), Some(TAGS_MAX));
    }

    #[test]
    fn like_patterns_are_escaped() {
        assert_eq!(like_escape("50%_off"), "50\\%\\_off");
        assert_eq!(like_escape("a\\b"), "a\\\\b");
        assert_eq!(like_escape("pog"), "pog");
    }
}
