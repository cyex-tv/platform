//! GraphQL: entitlements.

use async_graphql::{Context, Object, Result};

use super::{api_error, caller, state};
use crate::staff;

#[derive(Default)]
pub struct EntitlementQuery;

#[Object]
impl EntitlementQuery {
    /// An account's subscriptions: past, current and to come. There are no subscriptions yet,
    /// so this is empty until there are. Needs `billing.view`, `subscriptions.manage` or
    /// `shop.orders`.
    async fn account_subscriptions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::AccountSubscription>> {
        let caller = caller(ctx)?;
        staff::account_subscriptions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// The entitlement graph of an account: every badge, paint and role it holds or held, where
    /// each came from (staff, an order, a redeem code, an event, the User role, the owner list),
    /// who gave it and when, and when and by whom it was taken away. Held ones first. Needs
    /// `entitlements.view` or `cosmetics.grant`.
    async fn entitlement_graph(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::EntitlementGrant>> {
        let caller = caller(ctx)?;
        staff::graph(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// The reverse: who holds or held one badge, paint or role, and how each got it (the latest
    /// 200 of each kind of record). Not for the User and top roles (`implicit_role`). Needs
    /// `entitlements.view` or `cosmetics.grant`.
    async fn entitlement_holders(
        &self,
        ctx: &Context<'_>,
        kind: cyex_tv_shared::model::GrantKind,
        id: String,
    ) -> Result<Vec<cyex_tv_shared::model::EntitlementGrant>> {
        let caller = caller(ctx)?;
        staff::holders(state(ctx), caller, kind, &id)
            .await
            .map_err(api_error)
    }
}
