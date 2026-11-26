//! GraphQL extensions: app scopes, and the socket rules.

use cyex_tv_auth::Caller;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::coded;

/// App sign-in sessions (ADR 0021) run only the mutations their scopes name and the queries
/// `app::APP_QUERIES` lists (no staff reads). Checked per resolved top-level field, so aliases
/// and fragments change nothing.
pub(super) struct AppScopes;

impl async_graphql::extensions::ExtensionFactory for AppScopes {
    fn create(&self) -> Arc<dyn async_graphql::extensions::Extension> {
        Arc::new(AppScopes)
    }
}

#[async_trait::async_trait]
impl async_graphql::extensions::Extension for AppScopes {
    async fn resolve(
        &self,
        ctx: &async_graphql::extensions::ExtensionContext<'_>,
        info: async_graphql::extensions::ResolveInfo<'_>,
        next: async_graphql::extensions::NextResolve<'_>,
    ) -> async_graphql::ServerResult<Option<async_graphql::Value>> {
        let client = ctx
            .data_opt::<Option<Caller>>()
            .and_then(Option::as_ref)
            .and_then(|c| c.app_client.as_deref());
        let refused = client.is_some_and(|client| match info.parent_type {
            "Mutation" => !cyex_tv_auth::app::allows_mutation(client, info.name),
            "Query" => !cyex_tv_auth::app::allows_query(client, info.name),
            _ => false,
        });
        if refused {
            return Err(coded(
                "insufficient_scope",
                "This app may not do that. Use cyex.tv for it.",
            )
            .into_server_error(async_graphql::Pos::default()));
        }
        next.run(ctx, info).await
    }
}

/// Subscription streams one `/v1/gql/ws` connection may hold at once.
pub const MAX_SOCKET_SUBSCRIPTIONS: usize = 8;

/// Connection data for one `/v1/gql/ws` socket: marks requests as coming over the socket and
/// counts the subscription streams it holds.
#[derive(Default)]
pub struct SocketConnection {
    streams: Arc<AtomicUsize>,
}

impl SocketConnection {
    /// Take a stream slot, or `None` when the connection holds the most it may.
    pub(super) fn take_slot(&self) -> Option<StreamSlot> {
        let slot = StreamSlot(self.streams.clone());
        // Over the limit, the slot is given straight back as it drops.
        (self.streams.fetch_add(1, Ordering::AcqRel) < MAX_SOCKET_SUBSCRIPTIONS).then_some(slot)
    }
}

/// One subscription stream's place in its connection's count, given back when the stream ends.
pub(super) struct StreamSlot(Arc<AtomicUsize>);

impl Drop for StreamSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// The socket runs subscriptions only. async-graphql would otherwise run a query or mutation sent
/// as a `subscribe` message once, outside the `POST /v1/gql` rate limits. A document sent over the
/// socket may hold subscription operations only; over HTTP there is no [`SocketConnection`], so
/// everything passes.
pub(super) struct SocketRunsSubscriptionsOnly;

impl async_graphql::extensions::ExtensionFactory for SocketRunsSubscriptionsOnly {
    fn create(&self) -> Arc<dyn async_graphql::extensions::Extension> {
        Arc::new(SocketRunsSubscriptionsOnly)
    }
}

#[async_trait::async_trait]
impl async_graphql::extensions::Extension for SocketRunsSubscriptionsOnly {
    async fn parse_query(
        &self,
        ctx: &async_graphql::extensions::ExtensionContext<'_>,
        query: &str,
        variables: &async_graphql::Variables,
        next: async_graphql::extensions::NextParseQuery<'_>,
    ) -> async_graphql::ServerResult<async_graphql::parser::types::ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        if ctx.data_opt::<SocketConnection>().is_some()
            && let Some((_, op)) = document.operations.iter().find(|(_, op)| {
                op.node.ty != async_graphql::parser::types::OperationType::Subscription
            })
        {
            return Err(coded(
                "subscriptions_only",
                "The socket runs subscriptions only. Send queries and mutations to POST /v1/gql.",
            )
            .into_server_error(op.pos));
        }
        Ok(document)
    }
}
