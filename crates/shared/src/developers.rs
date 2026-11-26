//! The developer platform's API shapes: API keys (ADR 0033) and channel webhooks (ADR 0034).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::model::EventType;

/// One of your developer API keys. The key itself is shown once, when it is made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ApiKey {
    pub id: String,
    /// What you called it.
    pub name: String,
    /// The start of the key (`cyex_live_` and 8 more characters), to tell keys apart.
    pub prefix: String,
    pub created_at: String,
    /// When it last made a request, to the minute or so. `None` until it is used.
    pub last_used_at: Option<String>,
    /// Requests it made per UTC day, the last 30 days, oldest first; days without any are left
    /// out. Written every minute or so, so the last minute may be missing.
    pub usage: Vec<ApiKeyDay>,
}

/// One day of an API key's requests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct ApiKeyDay {
    /// The UTC day, `YYYY-MM-DD`.
    pub day: String,
    pub requests: i64,
}

/// A key just made: the key itself is in this answer only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CreatedApiKey {
    /// Send as `Authorization: Bearer <key>`. It reads public data only.
    pub key: String,
    pub api_key: ApiKey,
}

/// A channel's webhook: where its events are sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct Webhook {
    pub id: String,
    pub url: String,
    /// The event types it is sent.
    #[schema(value_type = Vec<String>)]
    pub events: Vec<EventType>,
    /// Off: nothing is sent. It is turned off by hand, or after 5 deliveries in a row failed.
    pub enabled: bool,
    /// Why it is off: `manual` or `failures`. `None` while it is on.
    pub disabled_reason: Option<String>,
    /// Deliveries in a row that failed after every retry.
    pub failed_deliveries: i32,
    pub created_at: String,
}

/// A webhook just made, or its secret just replaced: the secret is in this answer only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct CreatedWebhook {
    /// Signs every delivery (`X-Cyex-Signature`). Keep it on your server.
    pub secret: String,
    pub webhook: Webhook,
}

/// One delivery of a webhook: what was sent and how it went.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
pub struct WebhookDelivery {
    pub id: String,
    /// The event's type, or `webhook.test` for Send test.
    pub event_type: String,
    /// The outbox event's ID, as the event sockets send it. `None` for a test.
    pub event_id: Option<String>,
    /// `pending`, `delivered` or `failed`.
    pub state: String,
    pub attempts: i32,
    /// The receiver's HTTP status on the last attempt, if it answered.
    pub response_status: Option<i32>,
    /// What went wrong on the last attempt.
    pub error: Option<String>,
    /// How long the last attempt took, in milliseconds.
    pub duration_ms: Option<i32>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

/// A new webhook.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "CreateWebhookInput"))]
pub struct CreateWebhook {
    /// An `https://` URL on a public address.
    pub url: String,
    /// At least one of: `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED`,
    /// `CHANNEL_ACTIVE_SET_CHANGED`, `EMOTE_UPDATED`, `COSMETIC_CREATED`, `COSMETIC_UPDATED`,
    /// `COSMETIC_DELETED`.
    #[schema(value_type = Vec<String>)]
    pub events: Vec<EventType>,
}

/// What to change on a webhook; what is left out stays.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "graphql", derive(async_graphql::InputObject))]
#[cfg_attr(feature = "graphql", graphql(name = "UpdateWebhookInput"))]
pub struct UpdateWebhook {
    pub url: Option<String>,
    #[schema(value_type = Option<Vec<String>>)]
    pub events: Option<Vec<EventType>>,
    /// Turning it on again starts its failure count over.
    pub enabled: Option<bool>,
}
