//! The cosmetics shop and redeem codes (ADR 0016), in the API's shapes. Re-exported from `model`.
//! Payments go through cyex.tv's own Paddle account; no card data ever reaches cyex.tv.

use serde::{Deserialize, Serialize};

use crate::cosmetics::{Badge, Paint};
use crate::model::{AccountSummary, Platform};

/// Something staff sell: one or more badges and paints as a bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ShopItem {
    pub id: String,
    pub name: String,
    pub description: String,
    /// The price shown, in cents. Paddle's price (`paddlePriceId`) is what is charged.
    pub price_cents: u32,
    /// ISO 4217, for example `EUR`.
    pub currency: String,
    /// The Paddle price (`pri_…`). Only for holders of `shop.manage`; null for everyone else.
    pub paddle_price_id: Option<String>,
    pub badges: Vec<Badge>,
    pub paints: Vec<Paint>,
    /// On sale from (RFC 3339); none: already.
    pub active_from: Option<String>,
    /// On sale until (RFC 3339); none: no end.
    pub active_to: Option<String>,
    /// Most copies sold; none: no limit.
    pub stock_limit: Option<u32>,
    /// Copies left (pending checkouts of the last hour hold one each); none: no limit.
    pub stock_left: Option<u32>,
    /// Listed in the public shop (still only while on sale).
    pub visible: bool,
    /// Lower first.
    pub sort_order: i32,
    /// Can be bought now: visible, on sale, in stock, has cosmetics.
    pub available: bool,
    /// The caller owns every cosmetic in it (always false signed out). They can still gift it.
    pub owned: bool,
    /// Deleted: only staff see it, in lists with `all: true`.
    pub deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A whole item: `createShopItem` makes one, `updateShopItem` replaces every field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "ShopItemInput"))]
pub struct ShopItemInput {
    /// 1 to 64 characters.
    pub name: String,
    /// Up to 500 characters; new lines allowed.
    pub description: Option<String>,
    /// 0 to 100000 (1000.00).
    pub price_cents: u32,
    /// ISO 4217, default `EUR`.
    pub currency: Option<String>,
    /// The Paddle price (`pri_…`). Without one, checkout says it isn't available.
    pub paddle_price_id: Option<String>,
    /// 1 to 10 cosmetics in all.
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub badge_ids: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub paint_ids: Vec<String>,
    /// RFC 3339.
    pub active_from: Option<String>,
    /// RFC 3339, after `activeFrom`.
    pub active_to: Option<String>,
    /// 1 or more; none: no limit.
    pub stock_limit: Option<u32>,
    /// Default false.
    pub visible: Option<bool>,
    /// Default 0.
    pub sort_order: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Checkout started, not paid yet.
    Pending,
    /// Paid; the recipient owns the cosmetics. Also while a staff refund is being sent to Paddle.
    Paid,
    /// Refunded or charged back; the cosmetics were taken back.
    Refunded,
    /// Cancelled at Paddle, not started there, or not paid within a day (a late payment is still
    /// taken).
    Failed,
}

impl OrderStatus {
    pub const ALL: [Self; 4] = [Self::Pending, Self::Paid, Self::Refunded, Self::Failed];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Refunded => "refunded",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_str() == s)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Order {
    pub id: String,
    /// Who paid. None once their account is gone.
    pub buyer: Option<AccountSummary>,
    /// Who got the cosmetics.
    pub recipient: Option<AccountSummary>,
    /// Bought for another account.
    pub gift: bool,
    pub item_id: String,
    /// The item's name when it was bought.
    pub item_name: String,
    pub status: OrderStatus,
    /// The item's price when the checkout started, in cents.
    pub amount_cents: u32,
    pub currency: String,
    /// What Paddle says was paid, tax included, in the lowest unit. Set once paid.
    pub paid_total_cents: Option<u64>,
    pub paid_currency: Option<String>,
    /// `paddle`.
    pub provider: String,
    /// Paddle's transaction ID (`txn_…`).
    pub provider_transaction_id: Option<String>,
    pub refund_reason: Option<String>,
    pub created_at: String,
    pub paid_at: Option<String>,
    pub refunded_at: Option<String>,
    pub failed_at: Option<String>,
    /// When Paddle rejected this order's refund after the cosmetics were taken back (RFC 3339):
    /// look at it by hand. Only for holders of `shop.orders`.
    pub refund_rejected_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct OrderPage {
    pub items: Vec<Order>,
    pub total: u64,
}

/// What `startCheckout` gives: open Paddle's checkout for this transaction (Paddle.js
/// `Paddle.Checkout.open({ transactionId })` with `clientToken`, or go to `checkoutUrl`). The
/// order turns `PAID` when Paddle's webhook says so; poll `order(id)`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Checkout {
    pub order: Order,
    pub transaction_id: String,
    /// Paddle's hosted checkout link, when the Paddle account has a default payment link.
    pub checkout_url: Option<String>,
    /// The Paddle.js client-side token, when configured.
    pub client_token: Option<String>,
    /// `sandbox` or `production`: pass to `Paddle.Environment.set`.
    pub environment: String,
}

/// Who gets a gift: their Twitch or Kick login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "RecipientInput"))]
pub struct Recipient {
    pub platform: Platform,
    pub login: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CodeBatch {
    pub id: String,
    pub name: String,
    /// Codes look like `PREFIX-XXXX-XXXX-XXXX-XXXX`.
    pub prefix: String,
    /// The item whose cosmetics a code gives, if any.
    pub item_id: Option<String>,
    pub item_name: Option<String>,
    /// Cosmetics a code gives besides the item's.
    pub badge_ids: Vec<String>,
    pub paint_ids: Vec<String>,
    pub code_count: u32,
    pub uses_per_code: u32,
    /// Uses so far, over every code.
    pub redemptions: u64,
    pub expires_at: Option<String>,
    pub disabled_at: Option<String>,
    pub created_by: Option<AccountSummary>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CodeBatchPage {
    pub items: Vec<CodeBatch>,
    pub total: u64,
}

/// A new batch and its codes in plain text. This is the only time they are shown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CreatedCodeBatch {
    pub batch: CodeBatch,
    pub codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CodeBatchInput"))]
pub struct CodeBatchInput {
    /// For staff, 1 to 64 characters.
    pub name: String,
    /// 1 to 12 letters or digits (upper-cased).
    pub prefix: String,
    /// 1 to 1000 codes.
    pub count: u32,
    /// 1 to 10000, default 1. Each account can use a code once.
    pub uses_per_code: Option<u32>,
    /// RFC 3339, in the future.
    pub expires_at: Option<String>,
    /// A shop item whose cosmetics a code gives.
    pub item_id: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub badge_ids: Vec<String>,
    #[serde(default)]
    #[cfg_attr(feature = "graphql", graphql(default))]
    pub paint_ids: Vec<String>,
}

/// What a redeemed code gave.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct RedeemResult {
    /// The batch's name.
    pub name: String,
    /// Newly owned.
    pub badges: Vec<Badge>,
    pub paints: Vec<Paint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_round_trip() {
        for s in OrderStatus::ALL {
            assert_eq!(OrderStatus::parse(s.as_str()), Some(s));
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_str())
            );
        }
        assert_eq!(OrderStatus::parse("PAID"), None);
    }
}
