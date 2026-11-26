//! GraphQL extensions: app scopes, the root-field cap, and the socket rules.

use cyex_tv_auth::Caller;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use super::coded;

/// Requests with a developer API key run no mutations (ADR 0033). App sign-in sessions (ADR 0021) run only the mutations their scopes name and the queries
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
        // A developer API key (ADR 0033) only reads: it runs signed out, and no mutation runs
        // with it, not even one a signed-out person could.
        let keyed = ctx
            .data_opt::<Option<crate::developers::KeyUse>>()
            .is_some_and(Option::is_some);
        if keyed && info.parent_type == "Mutation" {
            return Err(coded(
                "api_key_read_only",
                "API keys read public data only. Sign in on cyex.tv for anything else.",
            )
            .into_server_error(async_graphql::Pos::default()));
        }
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

/// Top-level fields one operation may select, aliases and fragments counted. A backstop behind
/// the complexity limit: no client of ours sends more than a handful.
pub const MAX_ROOT_FIELDS: usize = 50;

/// Refuses an operation selecting more than [`MAX_ROOT_FIELDS`] top-level fields
/// (`too_many_fields`), so one request can't alias the same read hundreds of times.
pub(super) struct RootFieldCap;

impl async_graphql::extensions::ExtensionFactory for RootFieldCap {
    fn create(&self) -> Arc<dyn async_graphql::extensions::Extension> {
        Arc::new(RootFieldCap)
    }
}

/// The fields `set` selects, through fragment spreads and inline fragments; stops counting past
/// `MAX_ROOT_FIELDS`. `seen` keeps a fragment spread twice from being walked twice in one path.
fn root_fields(
    set: &async_graphql::parser::types::SelectionSet,
    fragments: &std::collections::HashMap<
        async_graphql::Name,
        async_graphql::Positioned<async_graphql::parser::types::FragmentDefinition>,
    >,
    depth: usize,
) -> usize {
    use async_graphql::parser::types::Selection;
    // Fragments can't cycle in a valid document; the depth stops one that does before validation
    // reports it.
    if depth > 16 {
        return 0;
    }
    let mut count = 0;
    for item in &set.items {
        count += match &item.node {
            Selection::Field(_) => 1,
            Selection::InlineFragment(f) => {
                root_fields(&f.node.selection_set.node, fragments, depth + 1)
            }
            Selection::FragmentSpread(f) => {
                fragments.get(&f.node.fragment_name.node).map_or(0, |d| {
                    root_fields(&d.node.selection_set.node, fragments, depth + 1)
                })
            }
        };
        if count > MAX_ROOT_FIELDS {
            break;
        }
    }
    count
}

#[async_trait::async_trait]
impl async_graphql::extensions::Extension for RootFieldCap {
    async fn parse_query(
        &self,
        ctx: &async_graphql::extensions::ExtensionContext<'_>,
        query: &str,
        variables: &async_graphql::Variables,
        next: async_graphql::extensions::NextParseQuery<'_>,
    ) -> async_graphql::ServerResult<async_graphql::parser::types::ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        if let Some((_, op)) = document.operations.iter().find(|(_, op)| {
            root_fields(&op.node.selection_set.node, &document.fragments, 0) > MAX_ROOT_FIELDS
        }) {
            return Err(coded(
                "too_many_fields",
                "That asks for too much at once. Split it into smaller requests.",
            )
            .into_server_error(op.pos));
        }
        Ok(document)
    }
}

/// The largest message (and frame) `/v1/gql/ws` reads. A subscribe message is a small document
/// and its variables; anything larger closes the socket.
pub const MAX_SOCKET_MESSAGE: usize = 64 * 1024;
/// A socket that hasn't sent `connection_init` this long after opening is closed (`4408`, as
/// `graphql-transport-ws` says).
pub const SOCKET_INIT_DEADLINE: Duration = Duration::from_secs(10);
/// The server pings every socket this often; browsers and socket libraries answer on their own.
pub const SOCKET_PING_EVERY: Duration = Duration::from_secs(30);
/// A socket the server has heard nothing from (no message, no pong) this long is closed.
pub const SOCKET_IDLE: Duration = Duration::from_secs(90);

/// Serves one `/v1/gql/ws` socket: the GraphQL protocol, plus the connection rules above
/// (`connection_init` deadline, server pings, idle timeout).
pub async fn serve_socket(
    socket: axum::extract::ws::WebSocket,
    schema: super::ApiSchema,
    protocol: async_graphql_axum::GraphQLProtocol,
) {
    use axum::extract::ws::{CloseFrame, Message};
    use futures_util::{SinkExt, StreamExt};

    let (mut out, input) = socket.split();
    let heard = Arc::new(Mutex::new(Instant::now()));
    let input = {
        let heard = heard.clone();
        input.inspect(move |msg| {
            if msg.is_ok() {
                *heard.lock().unwrap_or_else(PoisonError::into_inner) = Instant::now();
            }
        })
    };
    // The protocol writes into a channel; this loop owns the socket's sending half, so it can
    // also ping and close.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(32);
    let sink = futures_util::sink::unfold(tx, |tx, msg: Message| async move {
        tx.send(msg).await.map(|()| tx)
    });
    let initialised = Arc::new(AtomicBool::new(false));
    let mut data = async_graphql::Data::default();
    data.insert(SocketConnection::default());
    let graphql = {
        let initialised = initialised.clone();
        async_graphql_axum::GraphQLWebSocket::new_with_pair(sink, input, schema, protocol)
            .with_data(data)
            .on_connection_init(move |_| {
                initialised.store(true, Ordering::Release);
                std::future::ready(Ok(async_graphql::Data::default()))
            })
            .serve()
    };
    tokio::pin!(graphql);
    let deadline = tokio::time::sleep(SOCKET_INIT_DEADLINE);
    tokio::pin!(deadline);
    let mut ping = tokio::time::interval_at(
        tokio::time::Instant::now() + SOCKET_PING_EVERY,
        SOCKET_PING_EVERY,
    );
    let close = loop {
        tokio::select! {
            () = &mut graphql => break None,
            Some(msg) = rx.recv() => {
                if out.send(msg).await.is_err() {
                    return;
                }
            }
            () = &mut deadline, if !initialised.load(Ordering::Acquire) => {
                break Some((4408, "Connection initialisation timeout"));
            }
            _ = ping.tick() => {
                let quiet = heard.lock().unwrap_or_else(PoisonError::into_inner).elapsed();
                if quiet >= SOCKET_IDLE {
                    break Some((3008, "timeout"));
                }
                if out.send(Message::Ping(Default::default())).await.is_err() {
                    return;
                }
            }
        }
    };
    // What the protocol wrote last (its own close frame, say) is still in the channel.
    while let Ok(msg) = rx.try_recv() {
        if out.send(msg).await.is_err() {
            return;
        }
    }
    if let Some((code, reason)) = close {
        let _ = out
            .send(Message::Close(Some(CloseFrame {
                code,
                reason: reason.into(),
            })))
            .await;
    }
    let _ = out.close().await;
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
