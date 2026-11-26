//! GraphQL: audit.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::OrderStatus;
use cyex_tv_shared::model::{AuditEntry, ReviewQueueFilter};

use super::{api_error, caller, state};
use crate::admin;

#[derive(Default)]
pub struct AuditQuery;

#[Object]
impl AuditQuery {
    /// Audited changes, newest first. Needs `audit.view`. Narrow by target, `actorId`, an exact
    /// `action`, and `since` (inclusive) / `until` (exclusive) as RFC 3339 times. Page back with
    /// `before` (an entry ID); `limit` is 1 to 100, default 50.
    #[allow(clippy::too_many_arguments)]
    async fn audit_log(
        &self,
        ctx: &Context<'_>,
        target_kind: Option<String>,
        target_id: Option<String>,
        actor_id: Option<String>,
        action: Option<String>,
        since: Option<String>,
        until: Option<String>,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<AuditEntry>> {
        let caller = caller(ctx)?;
        admin::audit_log(
            state(ctx),
            caller,
            &admin::AuditQuery {
                target_kind: target_kind.as_deref(),
                target_id: target_id.as_deref(),
                actor_id: actor_id.as_deref(),
                action: action.as_deref(),
                since: since.as_deref(),
                until: until.as_deref(),
            },
            before.as_deref(),
            limit,
        )
        .await
        .map_err(api_error)
    }

    /// The audit log's busiest people for the filters `auditLog` takes: its actors (`by: ACTOR`,
    /// the default) or the accounts its entries are about (`by: TARGET`), most entries first
    /// among the newest 2000. Needs `audit.view`. `limit` 1 to 10, default 5.
    #[allow(clippy::too_many_arguments)]
    #[graphql(complexity = "50 + child_complexity")]
    async fn audit_top_people(
        &self,
        ctx: &Context<'_>,
        by: Option<cyex_tv_shared::model::AuditPersonRole>,
        target_kind: Option<String>,
        target_id: Option<String>,
        actor_id: Option<String>,
        action: Option<String>,
        since: Option<String>,
        until: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<cyex_tv_shared::model::PersonCount>> {
        let caller = caller(ctx)?;
        crate::top_people::audit_top_people(
            state(ctx),
            caller,
            &admin::AuditQuery {
                target_kind: target_kind.as_deref(),
                target_id: target_id.as_deref(),
                actor_id: actor_id.as_deref(),
                action: action.as_deref(),
                since: since.as_deref(),
                until: until.as_deref(),
            },
            by.unwrap_or_default(),
            limit,
        )
        .await
        .map_err(api_error)
    }

    /// The buyers with the most orders in a state and of an item. Needs `shop.orders`.
    /// `limit` 1 to 10, default 5.
    async fn order_top_buyers(
        &self,
        ctx: &Context<'_>,
        status: Option<OrderStatus>,
        item_id: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<cyex_tv_shared::model::PersonCount>> {
        let caller = caller(ctx)?;
        crate::top_people::order_top_buyers(state(ctx), caller, status, item_id.as_deref(), limit)
            .await
            .map_err(api_error)
    }

    /// The uploaders with the most emotes in one tab of the review queue (`filter`'s tab and
    /// search; its uploader and order don't matter). Needs `emotes.review`. `limit` 1 to 10,
    /// default 5.
    async fn review_top_uploaders(
        &self,
        ctx: &Context<'_>,
        filter: Option<ReviewQueueFilter>,
        limit: Option<u32>,
    ) -> Result<Vec<cyex_tv_shared::model::PersonCount>> {
        let caller = caller(ctx)?;
        crate::top_people::review_top_uploaders(state(ctx), caller, filter.as_ref(), limit)
            .await
            .map_err(api_error)
    }

    /// The accounts holding the most badges, paints and roles. Needs `entitlements.view` or
    /// `cosmetics.grant`. `limit` 1 to 10, default 5.
    async fn entitlement_top_holders(
        &self,
        ctx: &Context<'_>,
        limit: Option<u32>,
    ) -> Result<Vec<cyex_tv_shared::model::PersonCount>> {
        let caller = caller(ctx)?;
        crate::top_people::entitlement_top_holders(state(ctx), caller, limit)
            .await
            .map_err(api_error)
    }
}
