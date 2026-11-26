//! Channel webhooks (ADR 0034): a channel's owner, and its account editors with the `channels`
//! permission, point a URL at the channel's events. The worker delivers them (`webhook.deliver`).
//!
//! - At most `MAX_PER_CHANNEL` per channel. The URL must be https on a public address
//!   (`webhooks::check_url`); the worker checks every address again before it connects.
//! - The event types are the channel's set and emote events and the cosmetics catalog's
//!   (`webhooks::EVENT_TYPES`).
//! - Each has a secret that signs its deliveries, shown once when it is made or replaced.
//! - Every change is audited on the channel's account; writes and tests count under the
//!   per-account `prefs` limit.

use cyex_tv_auth::Caller;
use cyex_tv_db::{channels, ids, outbox, webhooks as db};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    CreateWebhook, CreatedWebhook, EditorPermission, EventType, Platform, UpdateWebhook, Webhook,
    WebhookDelivery,
};
use cyex_tv_shared::webhooks::{TEST_EVENT, check_url, is_event_type};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};

use crate::editors::has_flag;
use crate::errors::{
    Audit, bad, conflict, db_error, forbidden, limited_as, not_found, parse_id, record_audit,
    sqlx_error,
};
use crate::limits::AccountLimit;
use crate::state::AppState;
use crate::views;

/// Webhooks one channel may have.
pub const MAX_PER_CHANNEL: i64 = 5;

fn secret() -> String {
    format!("whsec_{}", cyex_tv_auth::tokens::random_token())
}

/// The channel, if the caller may manage its webhooks: its owner, or an account editor of the
/// owner with `channels`.
async fn may_manage(
    conn: &mut PgConnection,
    caller: &Caller,
    ch: channels::Channel,
) -> ApiResult<channels::Channel> {
    if ch.account_id == caller.account_id
        || has_flag(
            conn,
            ch.account_id,
            caller.account_id,
            EditorPermission::Channels,
        )
        .await?
    {
        return Ok(ch);
    }
    Err(forbidden(
        "Only the channel's owner and its account editors with the channel settings permission can manage its webhooks.",
    ))
}

async fn channel_by_platform(
    conn: &mut PgConnection,
    caller: &Caller,
    platform: Platform,
    id: &str,
) -> ApiResult<channels::Channel> {
    let ch = channels::find(conn, platform, id.trim())
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    may_manage(conn, caller, ch).await
}

/// The webhook and its channel, if the caller may manage it. Someone who may not gets
/// `not_found`, so webhook IDs say nothing.
async fn webhook_of(
    conn: &mut PgConnection,
    caller: &Caller,
    id: &str,
) -> ApiResult<(db::Webhook, channels::Channel)> {
    let hook = db::get(conn, parse_id(id)?)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let ch = channels::get(conn, hook.channel_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let ch = may_manage(conn, caller, ch)
        .await
        .map_err(|_| not_found())?;
    Ok((hook, ch))
}

fn event_type_of(name: &str) -> Option<EventType> {
    serde_json::from_value(Value::String(name.to_owned())).ok()
}

pub fn view(hook: &db::Webhook) -> Webhook {
    Webhook {
        id: ids::to_text(hook.id),
        url: hook.url.clone(),
        events: hook
            .event_types
            .iter()
            .filter_map(|t| event_type_of(t))
            .collect(),
        enabled: hook.enabled,
        disabled_reason: hook.disabled_reason.clone(),
        failed_deliveries: hook.failed_deliveries,
        created_at: views::time(hook.created_at),
    }
}

fn delivery_view(d: &db::Delivery) -> WebhookDelivery {
    WebhookDelivery {
        id: d.id.to_string(),
        event_type: d.event_type.clone(),
        event_id: d.event_id.map(|id| id.to_string()),
        state: d.state.clone(),
        attempts: d.attempts,
        response_status: d.response_status,
        error: d.error.clone(),
        duration_ms: d.duration_ms,
        created_at: views::time(d.created_at),
        finished_at: d.finished_at.map(views::time),
    }
}

fn url_of(raw: &str) -> ApiResult<String> {
    check_url(raw)
        .map(String::from)
        .map_err(|p| bad("bad_webhook_url", p.message()))
}

/// The stored names of `events`: at least one, each one a webhook may ask for, no repeats.
fn events_of(events: &[EventType]) -> ApiResult<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for e in events {
        let name = outbox::type_name(*e);
        if !is_event_type(&name) {
            return Err(bad(
                "bad_webhook_events",
                format!("Webhooks can't subscribe to {name}."),
            ));
        }
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        return Err(bad("bad_webhook_events", "Pick at least one event type."));
    }
    Ok(names)
}

fn limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::Prefs,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of changes. Try again later.",
    )
}

fn audit_snapshot(hook: &db::Webhook) -> Value {
    json!({
        "url": hook.url,
        "events": hook.event_types,
        "enabled": hook.enabled,
    })
}

/// The channel's webhooks, oldest first.
pub async fn list(
    state: &AppState,
    caller: &Caller,
    platform: Platform,
    id: &str,
) -> ApiResult<Vec<Webhook>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let ch = channel_by_platform(&mut conn, caller, platform, id).await?;
    Ok(db::list_for_channel(&mut conn, ch.id)
        .await
        .map_err(db_error)?
        .iter()
        .map(view)
        .collect())
}

pub async fn create(
    state: &AppState,
    caller: &Caller,
    platform: Platform,
    id: &str,
    input: &CreateWebhook,
) -> ApiResult<CreatedWebhook> {
    let url = url_of(&input.url)?;
    let events = events_of(&input.events)?;
    limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let ch = channel_by_platform(&mut tx, caller, platform, id).await?;
    let secret = secret();
    let topic = outbox::channel_topic(ch.platform, &ch.platform_channel_id);
    let hook = db::create(
        &mut tx,
        ch.id,
        &topic,
        &url,
        &events,
        &secret,
        caller.account_id,
        MAX_PER_CHANNEL,
    )
    .await
    .map_err(db_error)?
    .ok_or_else(|| {
        conflict(
            "too_many_webhooks",
            "This channel has 5 webhooks, the most it may. Delete one first.",
        )
    })?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("channel.webhook_created", "webhook", &ids::to_text(hook.id))
            .change(None, Some(audit_snapshot(&hook)))
            .subject((ch.account_id != caller.account_id).then_some(ch.account_id)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(CreatedWebhook {
        secret,
        webhook: view(&hook),
    })
}

pub async fn update(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &UpdateWebhook,
) -> ApiResult<Webhook> {
    let url = input.url.as_deref().map(url_of).transpose()?;
    let events = input.events.as_deref().map(events_of).transpose()?;
    limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (before, ch) = webhook_of(&mut tx, caller, id).await?;
    let hook = db::update(
        &mut tx,
        before.id,
        url.as_deref(),
        events.as_deref(),
        input.enabled,
    )
    .await
    .map_err(db_error)?
    .ok_or_else(not_found)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("channel.webhook_updated", "webhook", &ids::to_text(hook.id))
            .change(Some(audit_snapshot(&before)), Some(audit_snapshot(&hook)))
            .subject((ch.account_id != caller.account_id).then_some(ch.account_id)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(view(&hook))
}

pub async fn delete(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (hook, ch) = webhook_of(&mut tx, caller, id).await?;
    let done = db::delete(&mut tx, hook.id).await.map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("channel.webhook_deleted", "webhook", &ids::to_text(hook.id))
            .change(Some(audit_snapshot(&hook)), None)
            .subject((ch.account_id != caller.account_id).then_some(ch.account_id)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(done)
}

/// Replaces the secret; the old one stops signing at once.
pub async fn rotate_secret(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<CreatedWebhook> {
    limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (hook, ch) = webhook_of(&mut tx, caller, id).await?;
    let secret = secret();
    let hook = db::set_secret(&mut tx, hook.id, &secret)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(
            "channel.webhook_secret_rotated",
            "webhook",
            &ids::to_text(hook.id),
        )
        .subject((ch.account_id != caller.account_id).then_some(ch.account_id)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(CreatedWebhook {
        secret,
        webhook: view(&hook),
    })
}

/// Queues a `webhook.test` delivery, sent once, even while the webhook is off.
pub async fn send_test(state: &AppState, caller: &Caller, id: &str) -> ApiResult<WebhookDelivery> {
    limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (hook, _) = webhook_of(&mut tx, caller, id).await?;
    let body = json!({
        "id": "0",
        "type": TEST_EVENT,
        "created_at": views::time(chrono::Utc::now()),
        "data": { "message": "A test delivery from cyex.tv." },
    });
    let delivery = db::queue_test(&mut tx, hook.id, &body)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(delivery_view(&delivery))
}

/// The webhook's latest deliveries, newest first (50 are kept).
pub async fn deliveries(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<Vec<WebhookDelivery>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let (hook, _) = webhook_of(&mut conn, caller, id).await?;
    Ok(db::deliveries(&mut conn, hook.id, db::KEEP_DELIVERIES)
        .await
        .map_err(db_error)?
        .iter()
        .map(delivery_view)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_types_are_checked() {
        assert_eq!(
            events_of(&[
                EventType::SetEmoteAdded,
                EventType::SetEmoteAdded,
                EventType::CosmeticUpdated
            ])
            .unwrap(),
            ["set.emote_added", "cosmetic.updated"]
        );
        for refused in [
            vec![],
            vec![EventType::PersonalUpdated],
            vec![EventType::Resync],
            vec![EventType::SetCreated],
            vec![EventType::UserCosmeticsUpdated],
        ] {
            assert_eq!(events_of(&refused).unwrap_err().code, "bad_webhook_events");
        }
        assert_eq!(
            event_type_of("emote.updated"),
            Some(EventType::EmoteUpdated)
        );
        assert!(url_of("http://example.com").is_err());
        assert!(secret().starts_with("whsec_") && secret().len() == 49);
    }
}
