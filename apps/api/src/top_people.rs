//! "Most important people" for the site's person filters (the `UserFilter` field): who has the
//! most entries in one list, so a focused, empty filter can offer them before anything is typed.
//! Each read counts what its list shows and is visible exactly as that list is:
//!
//! - `activityTopActors`: an account's or an emote's public activity feed (public, a heavy read).
//! - `auditTopPeople`: the audit log's actors or account targets for its filters (`audit.view`).
//! - `orderTopBuyers`: the shop's orders for a state and item (`shop.orders`).
//! - `reviewTopUploaders`: one tab of the review queue (`emotes.review`).
//! - `entitlementTopHolders`: accounts holding the most badges, paints and roles
//!   (`entitlements.view` or `cosmetics.grant`, as the entitlement graph).
//!
//! Logs and feeds count their newest 2000 rows (`audit::TOP_WINDOW`), so a long history reads a
//! bounded page. `limit` is 1 to 10, default 5. Deleted accounts never show.

use cyex_tv_auth::Caller;
use cyex_tv_db::audit::{self, AuditSide};
use cyex_tv_db::{accounts, ids, reads, shop, staff};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    ActivityFilter, AuditPersonRole, OrderStatus, PersonCount, ReviewQueueFilter,
};
use cyex_tv_shared::permissions;
use sqlx::PgConnection;

use crate::activity::sql_filter;
use crate::admin::{AuditQuery, audit_filter_parts};
use crate::catalog::{db_error, parse_id, public_emote, require_any};
use crate::errors::{bad, conn};
use crate::moderation::QueueRead;
use crate::search::account_hits;
use crate::state::AppState;

pub const TOP_DEFAULT: u32 = 5;
pub const TOP_MAX: u32 = 10;

fn pick(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|t| !t.is_empty())
}

fn limit_of(limit: Option<u32>) -> i64 {
    i64::from(limit.unwrap_or(TOP_DEFAULT).clamp(1, TOP_MAX))
}

/// The counted accounts as people: summary, logins and count, in the order counted.
async fn people(
    conn: &mut PgConnection,
    counts: Vec<audit::PersonCount>,
) -> ApiResult<Vec<PersonCount>> {
    let ids: Vec<_> = counts.iter().map(|c| c.account_id).collect();
    let hits = account_hits(conn, &ids).await?;
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let id = ids::from_text(&hit.account.id)?;
            let n = counts.iter().find(|c| c.account_id == id)?.count;
            Some(PersonCount {
                account: hit.account,
                identities: hit.identities,
                count: u64::try_from(n).unwrap_or(0),
            })
        })
        .collect())
}

/// Who an account's (`account_id`) or an emote's (`emote_id`, exactly one of the two) public
/// activity shows most often as by. For an account, `filter`'s groups and time range apply (its
/// actor doesn't). Empty when the feed is: an unknown, deleted or blocked account, or an emote
/// that isn't public.
pub async fn activity_top_actors(
    state: &AppState,
    account_id: Option<&str>,
    emote_id: Option<&str>,
    filter: Option<&ActivityFilter>,
    limit: Option<u32>,
) -> ApiResult<Vec<PersonCount>> {
    let limit = limit_of(limit);
    let mut conn = conn(state).await?;
    let counts = match (pick(account_id), pick(emote_id)) {
        (Some(account), None) => {
            let wanted = sql_filter(filter)?;
            let Some(owner) = ids::from_text(account) else {
                return Ok(Vec::new());
            };
            let Some(row) = accounts::get(&mut conn, owner).await.map_err(db_error)? else {
                return Ok(Vec::new());
            };
            if row.deleted_at.is_some() || row.blocked_at.is_some() {
                return Ok(Vec::new());
            }
            audit::activity_top_actors(&mut conn, owner, &wanted, limit)
                .await
                .map_err(db_error)?
        }
        (None, Some(emote)) => {
            let Some(emote) = ids::from_text(emote) else {
                return Ok(Vec::new());
            };
            if !public_emote(&mut conn, emote).await? {
                return Ok(Vec::new());
            }
            audit::emote_top_actors(&mut conn, &ids::to_text(emote), limit)
                .await
                .map_err(db_error)?
        }
        _ => {
            return Err(bad(
                "bad_scope",
                "Give either `accountId` or `emoteId`, not both.",
            ));
        }
    };
    people(&mut conn, counts).await
}

/// The audit log's busiest actors (or account targets) for the filters `auditLog` takes.
/// Needs `audit.view`.
pub async fn audit_top_people(
    state: &AppState,
    caller: &Caller,
    q: &AuditQuery<'_>,
    by: AuditPersonRole,
    limit: Option<u32>,
) -> ApiResult<Vec<PersonCount>> {
    let limit = limit_of(limit);
    let mut conn = conn(state).await?;
    require_any(&mut conn, caller, &[permissions::AUDIT_VIEW]).await?;
    let parts = audit_filter_parts(q)?;
    let side = match by {
        AuditPersonRole::Actor => AuditSide::Actor,
        AuditPersonRole::Target => AuditSide::Target,
    };
    let counts = audit::top_people(&mut conn, &parts.filter(), side, limit)
        .await
        .map_err(db_error)?;
    people(&mut conn, counts).await
}

/// The buyers with the most orders in a state and of an item. Needs `shop.orders`.
pub async fn order_top_buyers(
    state: &AppState,
    caller: &Caller,
    status: Option<OrderStatus>,
    item_id: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<Vec<PersonCount>> {
    let limit = limit_of(limit);
    let item = item_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    let mut conn = conn(state).await?;
    require_any(&mut conn, caller, &[permissions::SHOP_ORDERS]).await?;
    let counts = shop::top_buyers(&mut conn, status.map(OrderStatus::as_str), item, limit)
        .await
        .map_err(db_error)?;
    people(&mut conn, counts).await
}

/// The uploaders with the most emotes in one tab of the review queue (its search applies).
/// Needs `emotes.review`.
pub async fn review_top_uploaders(
    state: &AppState,
    caller: &Caller,
    filter: Option<&ReviewQueueFilter>,
    limit: Option<u32>,
) -> ApiResult<Vec<PersonCount>> {
    let limit = limit_of(limit);
    let read = QueueRead::from_filter(filter.cloned().unwrap_or_default())?;
    let mut conn = conn(state).await?;
    require_any(
        &mut conn,
        caller,
        crate::moderation::queue_permissions(read.tab),
    )
    .await?;
    let counts = reads::queue_top_uploaders(&mut conn, read.tab, read.search.as_deref(), limit)
        .await
        .map_err(db_error)?;
    people(&mut conn, counts).await
}

/// The accounts holding the most badges, paints and roles. Needs `entitlements.view` or
/// `cosmetics.grant`.
pub async fn entitlement_top_holders(
    state: &AppState,
    caller: &Caller,
    limit: Option<u32>,
) -> ApiResult<Vec<PersonCount>> {
    let limit = limit_of(limit);
    let mut conn = conn(state).await?;
    require_any(
        &mut conn,
        caller,
        &[permissions::ENTITLEMENTS_VIEW, permissions::COSMETICS_GRANT],
    )
    .await?;
    let counts = staff::top_holders(&mut conn, limit)
        .await
        .map_err(db_error)?;
    people(&mut conn, counts).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_stay_between_one_and_ten() {
        assert_eq!(limit_of(None), 5);
        assert_eq!(limit_of(Some(0)), 1);
        assert_eq!(limit_of(Some(3)), 3);
        assert_eq!(limit_of(Some(500)), 10);
    }
}
