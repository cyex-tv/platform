//! GraphQL: subscription.

use async_graphql::{Context, ErrorExtensions, Result};
use cyex_tv_hub::Message as HubMessage;
use cyex_tv_hub::topics as hub_topics;
use cyex_tv_shared::model::Event;
use futures_util::Stream;
use std::collections::HashMap;

use super::{SocketConnection, api_error, coded, state};

#[derive(Default)]
pub struct Subscription;

#[async_graphql::Subscription]
impl Subscription {
    /// Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
    /// `user:<platform>:<id>` (ADR 0005), and `cosmetics` (ADR 0015). The same events as the event WebSocket.
    async fn events(
        &self,
        ctx: &Context<'_>,
        topics: Vec<String>,
    ) -> Result<impl Stream<Item = Event> + use<>> {
        if topics.len() > hub_topics::MAX_TOPICS {
            return Err(coded("too_many_topics", "At most 500 topics."));
        }
        if let Some(bad) = topics.iter().find(|t| !hub_topics::is_valid(t)) {
            return Err(
                async_graphql::Error::new(format!("{bad:?} is not a topic."))
                    .extend_with(|_, e| e.set("code", "invalid_topic")),
            );
        }
        let slot = match ctx.data_opt::<SocketConnection>() {
            Some(connection) => Some(connection.take_slot().ok_or_else(|| {
                coded(
                    "too_many_subscriptions",
                    "At most 8 subscriptions on one socket.",
                )
            })?),
            None => None,
        };
        // A set named by its old ULID is heard by its Snowflake, as events are stored, and its
        // events come under the name it was asked for (ADR 0027).
        let mut aliases: HashMap<String, String> = HashMap::new();
        let mut stored = Vec::with_capacity(topics.len());
        let legacy = |t: &String| {
            t.strip_prefix("set:")
                .is_some_and(|id| cyex_tv_db::public_ids::parse(id).is_none())
        };
        if topics.iter().any(legacy) {
            // One lookup for every legacy set topic, however many there are.
            let mut conn = crate::errors::conn(state(ctx)).await.map_err(api_error)?;
            let found = cyex_tv_db::outbox::public_topics(&mut conn, &topics)
                .await
                .map_err(|e| api_error(crate::errors::db_error(e)))?;
            drop(conn);
            for topic in &topics {
                let inner = found.get(topic).unwrap_or(topic).clone();
                if &inner != topic {
                    aliases.insert(inner.clone(), topic.clone());
                }
                stored.push(inner);
            }
        } else {
            stored.clone_from(&topics);
        }
        // One socket holds at most `MAX_SOCKET_SUBSCRIPTIONS` streams (ADR 0005).
        let sub = state(ctx).hub.subscriber();
        sub.subscribe(stored.iter().map(String::as_str));
        // The slot lives as long as the stream, so ending it frees the connection's place.
        Ok(futures_util::stream::unfold(
            (sub, slot, aliases),
            |(mut sub, slot, aliases)| async move {
                let event = match sub.recv().await? {
                    HubMessage::Event(d) => {
                        let topic = aliases.get(&d.topic).unwrap_or(&d.topic);
                        cyex_tv_hub::model_event(&d.event, topic)
                    }
                    HubMessage::Lagged => cyex_tv_hub::resync(""),
                };
                Some((event, (sub, slot, aliases)))
            },
        ))
    }
}
