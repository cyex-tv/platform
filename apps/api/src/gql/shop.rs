//! GraphQL: shop.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    Checkout, CodeBatch, CodeBatchInput, CodeBatchPage, CreatedCodeBatch, Order, OrderPage,
    OrderStatus, Recipient, RedeemResult, ShopItem, ShopItemInput,
};

use super::{api_error, caller, caller_opt, state};
use crate::{codes, idempotency, shop};

#[derive(Default)]
pub struct ShopQuery;

#[Object]
impl ShopQuery {
    /// Shop items on sale now, by sort order (ADR 0016). Public; signed in, `owned` says whether
    /// you own everything in each. With `all: true`, every item that isn't deleted, hidden and
    /// scheduled ones too (needs `shop.manage`).
    async fn shop_items(&self, ctx: &Context<'_>, all: Option<bool>) -> Result<Vec<ShopItem>> {
        shop::items(state(ctx), caller_opt(ctx), all.unwrap_or(false))
            .await
            .map_err(api_error)
    }

    /// One shop item: on sale for everyone; any, deleted ones too, with `shop.manage`.
    async fn shop_item(&self, ctx: &Context<'_>, id: String) -> Result<Option<ShopItem>> {
        shop::item(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// Orders you bought, and gifts you got once paid (without what the buyer paid), newest
    /// first. `perPage` 1 to 100, default 50.
    async fn my_orders(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<OrderPage> {
        let caller = caller(ctx)?;
        shop::my_orders(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// One order, for its buyer, its recipient (once paid, without what the buyer paid), or
    /// `shop.orders`. Poll it after `startCheckout` until it is `PAID`.
    async fn order(&self, ctx: &Context<'_>, id: String) -> Result<Option<Order>> {
        let caller = caller(ctx)?;
        shop::order(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Every order, newest first; narrow by status, account (bought or received) and item.
    /// Needs `shop.orders`, or `billing.view` narrowed to one account.
    #[allow(clippy::too_many_arguments)]
    async fn orders(
        &self,
        ctx: &Context<'_>,
        status: Option<OrderStatus>,
        account_id: Option<String>,
        item_id: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<OrderPage> {
        let caller = caller(ctx)?;
        shop::orders(
            state(ctx),
            caller,
            status,
            account_id.as_deref(),
            item_id.as_deref(),
            page,
            per_page,
        )
        .await
        .map_err(api_error)
    }

    /// Redeem code batches, newest first. Needs `codes.manage`. The codes themselves are shown
    /// only once, by `createCodeBatch`.
    async fn code_batches(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<CodeBatchPage> {
        let caller = caller(ctx)?;
        codes::batches(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct ShopMutation;

#[Object]
impl ShopMutation {
    /// Make a shop item: a bundle of 1 to 10 badges and paints. Needs `shop.manage`. Audited.
    async fn create_shop_item(&self, ctx: &Context<'_>, input: ShopItemInput) -> Result<ShopItem> {
        let caller = caller(ctx)?;
        shop::create_item(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Replace every field of a shop item. Orders made already keep their price. Needs
    /// `shop.manage`. Audited.
    async fn update_shop_item(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: ShopItemInput,
    ) -> Result<ShopItem> {
        let caller = caller(ctx)?;
        shop::update_item(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a shop item: never listed or sold again; its orders stay. False when there was
    /// none. Needs `shop.manage`. Audited.
    async fn delete_shop_item(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        shop::delete_item(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Start buying an item for yourself, or as a gift with `recipient`. Answers the Paddle
    /// transaction to open in Paddle's checkout; the order turns `PAID` once Paddle confirms.
    /// `checkout_unavailable` while Paddle isn't set up or the item has no Paddle price. Asking
    /// again for the same item and recipient within the hour hands out the same transaction
    /// (one pending order each); `checkout_busy` while another request is starting it.
    ///
    /// `idempotencyKey` (optional, a UUID the client makes once per purchase and sends again on
    /// a retry): a repeat within 24 hours answers what the first one did (as it was then; poll
    /// `order(id)` for where it stands) instead of starting another order.
    /// `idempotency_key_reused` when the key went with another item or recipient,
    /// `idempotency_in_progress` while the first is still running. A request that failed gives
    /// its key back.
    async fn start_checkout(
        &self,
        ctx: &Context<'_>,
        item_id: String,
        recipient: Option<Recipient>,
        idempotency_key: Option<String>,
    ) -> Result<Checkout> {
        let caller = caller(ctx)?;
        let state = state(ctx);
        let request = serde_json::json!({ "itemId": item_id.trim(), "recipient": recipient });
        idempotency::once(
            state,
            caller,
            idempotency::op::START_CHECKOUT,
            idempotency_key.as_deref(),
            request,
            || shop::start_checkout(state, caller, &item_id, recipient.as_ref()),
        )
        .await
        .map_err(api_error)
    }

    /// Refund a paid order: asks Paddle for a full refund when it is set up, and takes the
    /// cosmetics back at once (only those no other paid order, code or staff grant covers).
    /// `already_refunded` when Paddle says it was refunded already; `refund_in_progress` while
    /// another refund of it runs (it may be tried again after 10 minutes); `refund_unavailable`
    /// when the order has a Paddle transaction and Paddle isn't set up. Needs `shop.orders`.
    /// Audited.
    async fn refund_order(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Order> {
        let caller = caller(ctx)?;
        shop::refund_order(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Redeem a code (case, spaces and dashes don't matter). Wrong codes lock you out for a
    /// while after a few tries (`locked_out`). Each cosmetic it gives sends a `cosmetic.granted`
    /// notification (`source` `code`).
    ///
    /// `idempotencyKey` (optional UUID, one per attempt, sent again on a retry): a repeat within
    /// 24 hours answers what the first redeem gave instead of `already_redeemed`.
    /// `idempotency_key_reused` when the key went with another code.
    async fn redeem_code(
        &self,
        ctx: &Context<'_>,
        code: String,
        idempotency_key: Option<String>,
    ) -> Result<RedeemResult> {
        let caller = caller(ctx)?;
        let state = state(ctx);
        let ip = ctx
            .data_opt::<crate::limits::ClientIp>()
            .map(|i| i.0.as_str());
        // Hashed, never stored: the same code however it was typed.
        let request = serde_json::json!({ "code": codes::normalize(&code) });
        idempotency::once(
            state,
            caller,
            idempotency::op::REDEEM_CODE,
            idempotency_key.as_deref(),
            request,
            || codes::redeem(state, caller, ip, &code),
        )
        .await
        .map_err(api_error)
    }

    /// Make a batch of 1 to 1000 redeem codes. The answer holds the codes in plain text; this is
    /// the only time they are shown. Needs `codes.manage`. Audited.
    async fn create_code_batch(
        &self,
        ctx: &Context<'_>,
        input: CodeBatchInput,
    ) -> Result<CreatedCodeBatch> {
        let caller = caller(ctx)?;
        codes::create_batch(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Turn off every code of a batch. Needs `codes.manage`. Audited.
    async fn disable_code_batch(&self, ctx: &Context<'_>, id: String) -> Result<CodeBatch> {
        let caller = caller(ctx)?;
        codes::disable_batch(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }
}
