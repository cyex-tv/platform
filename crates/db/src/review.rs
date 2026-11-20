//! The review queue's ready reject reasons. See `20261209004001_review_reasons.sql`.

use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// The ready reasons, in the order the reject dialog shows them.
pub async fn reasons(conn: &mut PgConnection) -> DbResult<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT body FROM review_reasons ORDER BY position")
            .fetch_all(conn)
            .await?,
    )
}

/// Replaces the whole list with `reasons` (in order), in one step. The caller checks the words
/// and the count.
pub async fn replace_reasons(
    conn: &mut PgConnection,
    reasons: &[String],
    by: Uuid,
) -> DbResult<()> {
    let mut tx = conn.begin().await?;
    sqlx::query("DELETE FROM review_reasons")
        .execute(&mut *tx)
        .await?;
    let positions: Vec<i32> = (0..reasons.len())
        .map(|i| i32::try_from(i).unwrap_or(i32::MAX))
        .collect();
    sqlx::query(
        "INSERT INTO review_reasons (position, body, updated_by)
         SELECT p, b, $3 FROM unnest($1::int[], $2::text[]) AS t(p, b)",
    )
    .bind(&positions)
    .bind(reasons)
    .bind(by)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
