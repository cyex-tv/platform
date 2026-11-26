//! GraphQL: the developer platform, API keys (ADR 0033) and channel webhooks (ADR 0034).

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    ApiKey, CreateWebhook, CreatedApiKey, CreatedWebhook, Platform, UpdateWebhook, Webhook,
    WebhookDelivery,
};

use super::{api_error, caller, state};
use crate::{developers, webhooks};

#[derive(Default)]
pub struct DeveloperQuery;

#[Object]
impl DeveloperQuery {
    /// Your developer API keys, oldest first, each with its requests per day for the last 30
    /// days. Never the keys themselves.
    async fn my_api_keys(&self, ctx: &Context<'_>) -> Result<Vec<ApiKey>> {
        developers::my_keys(state(ctx), caller(ctx)?)
            .await
            .map_err(api_error)
    }

    /// A channel's webhooks, for its owner and its account editors with the channel settings
    /// permission (`forbidden` for anyone else).
    async fn channel_webhooks(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
    ) -> Result<Vec<Webhook>> {
        webhooks::list(state(ctx), caller(ctx)?, platform, &id)
            .await
            .map_err(api_error)
    }

    /// A webhook's latest deliveries, newest first (the last 50 are kept).
    async fn webhook_deliveries(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Vec<WebhookDelivery>> {
        webhooks::deliveries(state(ctx), caller(ctx)?, &id)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct DeveloperMutation;

#[Object]
impl DeveloperMutation {
    /// Make a developer API key: a Bearer token for bots and tools that reads public data at a
    /// higher rate than signed-out requests. The key is in this answer only. At most 10 per
    /// account (`too_many_api_keys`); made on cyex.tv only (`website_only` with a token).
    async fn create_api_key(&self, ctx: &Context<'_>, name: String) -> Result<CreatedApiKey> {
        developers::create(state(ctx), caller(ctx)?, &name)
            .await
            .map_err(api_error)
    }

    /// Revoke one of your API keys; it stops working at once. False when there was no such key.
    async fn revoke_api_key(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        developers::revoke(state(ctx), caller(ctx)?, &id)
            .await
            .map_err(api_error)
    }

    /// Point a channel's events at an https URL. The secret that signs every delivery is in this
    /// answer only. At most 5 per channel (`too_many_webhooks`).
    async fn create_webhook(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
        input: CreateWebhook,
    ) -> Result<CreatedWebhook> {
        webhooks::create(state(ctx), caller(ctx)?, platform, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Change a webhook's URL or events, or turn it off or on again.
    async fn update_webhook(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateWebhook,
    ) -> Result<Webhook> {
        webhooks::update(state(ctx), caller(ctx)?, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a webhook and its delivery log.
    async fn delete_webhook(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        webhooks::delete(state(ctx), caller(ctx)?, &id)
            .await
            .map_err(api_error)
    }

    /// Replace a webhook's secret. The new one is in this answer only; the old one stops at once.
    async fn rotate_webhook_secret(&self, ctx: &Context<'_>, id: String) -> Result<CreatedWebhook> {
        webhooks::rotate_secret(state(ctx), caller(ctx)?, &id)
            .await
            .map_err(api_error)
    }

    /// Send a `webhook.test` delivery now (once, no retries), even while the webhook is off.
    async fn test_webhook(&self, ctx: &Context<'_>, id: String) -> Result<WebhookDelivery> {
        webhooks::send_test(state(ctx), caller(ctx)?, &id)
            .await
            .map_err(api_error)
    }
}
