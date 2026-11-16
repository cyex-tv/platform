//! The v1 contract (ADR 0008), drafted in Phase 1 for review. Every handler here answers
//! `501 not_implemented` until the phase that builds it:
//! - Phase 2: auth, accounts, channels, sets, roles
//! - Phase 3: uploads and moderation
//! - Phase 5: events
//! - Phase 6: search
//!
//! Public reads need no auth. Every write needs a cyex.tv session, and the permission named in its
//! description.

use axum::Json;
use axum::http::StatusCode;
use axum::response::Redirect;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::{
    AccountSummary, AddSetEmote, CallbackQuery, Channel, CreateRole, CreateSet, Emote, EmoteSet,
    EventsQuery, LoginQuery, Me, PersonalLookup, PersonalLookupResult, Platform, RejectEmote,
    ReviewQueue, Role, SearchQuery, SearchResults, SetEmote, UpdateChannelSettings, UpdateEmote,
    UpdateRole, UpdateSet, UpdateSetEmote, UploadEmote,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(callback))
        .routes(routes!(logout))
        .routes(routes!(get_me))
        .routes(routes!(unlink_identity))
        .routes(routes!(get_channel))
        .routes(routes!(get_channel_by_login))
        .routes(routes!(update_channel_settings))
        .routes(routes!(get_global_set))
        .routes(routes!(create_set))
        .routes(routes!(get_set, update_set, delete_set))
        .routes(routes!(add_set_emote, update_set_emote, remove_set_emote))
        .routes(routes!(list_set_editors))
        .routes(routes!(add_set_editor, remove_set_editor))
        .routes(routes!(upload_emote))
        .routes(routes!(get_emote, update_emote, delete_emote))
        .routes(routes!(search_emotes))
        .routes(routes!(get_personal_set))
        .routes(routes!(lookup_personal_sets))
        .routes(routes!(review_queue))
        .routes(routes!(approve_emote))
        .routes(routes!(reject_emote))
        .routes(routes!(list_roles, create_role))
        .routes(routes!(update_role, delete_role))
        .routes(routes!(assign_role, unassign_role))
}

// Auth

/// Start signing in with Twitch or Kick.
///
/// Redirects to the platform's consent page. With `link=true` and a signed-in session, the
/// identity is linked to the current account instead.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/login",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), LoginQuery),
    responses(
        (status = 302, description = "Redirect to the platform's sign-in page."),
        (status = 400, description = "Unknown platform or bad `return_to`.", body = ErrorBody)
    )
)]
async fn login() -> ApiResult<Redirect> {
    Err(ApiError::not_implemented())
}

/// Where the platform sends the browser back to.
///
/// Sets the session cookie and redirects to `return_to`.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/callback",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), CallbackQuery),
    responses(
        (status = 302, description = "Signed in. Redirect to `return_to`."),
        (status = 400, description = "Bad or expired `state`, or the user declined.", body = ErrorBody),
        (status = 409, description = "`identity_taken`: linked to another account.", body = ErrorBody)
    )
)]
async fn callback() -> ApiResult<Redirect> {
    Err(ApiError::not_implemented())
}

/// Sign out and end the session.
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    responses((status = 204, description = "Signed out."))
)]
async fn logout() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Accounts

/// The signed-in account.
#[utoipa::path(
    get,
    path = "/v1/me",
    tag = "accounts",
    responses(
        (status = 200, body = Me),
        (status = 401, description = "`signed_out`.", body = ErrorBody)
    )
)]
async fn get_me() -> ApiResult<Json<Me>> {
    Err(ApiError::not_implemented())
}

/// Unlink a platform from the signed-in account. The last identity can't be removed.
#[utoipa::path(
    delete,
    path = "/v1/me/identities/{platform}",
    tag = "accounts",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`.")),
    responses(
        (status = 204, description = "Unlinked."),
        (status = 409, description = "`last_identity`.", body = ErrorBody)
    )
)]
async fn unlink_identity() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Channels

/// A channel by its platform ID.
#[utoipa::path(
    get,
    path = "/v1/channels/{platform}/{id}",
    tag = "channels",
    params(
        ("platform" = Platform, Path, description = "`twitch` or `kick`."),
        ("id" = String, Path, description = "The broadcaster's platform user ID.")
    ),
    responses(
        (status = 200, body = Channel),
        (status = 404, description = "`not_found`: no cyex.tv account has this channel.", body = ErrorBody)
    )
)]
async fn get_channel() -> ApiResult<Json<Channel>> {
    Err(ApiError::not_implemented())
}

/// A channel by its login or slug. Logins change; store the ID, not the login.
#[utoipa::path(
    get,
    path = "/v1/channels/{platform}/by-login/{login}",
    tag = "channels",
    params(
        ("platform" = Platform, Path, description = "`twitch` or `kick`."),
        ("login" = String, Path, description = "Twitch login or Kick slug.")
    ),
    responses((status = 200, body = Channel), (status = 404, body = ErrorBody))
)]
async fn get_channel_by_login() -> ApiResult<Json<Channel>> {
    Err(ApiError::not_implemented())
}

/// Change a channel's active set and settings. Only the channel's owner.
#[utoipa::path(
    patch,
    path = "/v1/channels/{platform}/{id}/settings",
    tag = "channels",
    params(
        ("platform" = Platform, Path, description = "`twitch` or `kick`."),
        ("id" = String, Path, description = "The broadcaster's platform user ID.")
    ),
    request_body = UpdateChannelSettings,
    responses(
        (status = 200, body = Channel),
        (status = 403, description = "`forbidden`.", body = ErrorBody)
    )
)]
async fn update_channel_settings() -> ApiResult<Json<Channel>> {
    Err(ApiError::not_implemented())
}

// Sets

/// The cyex.tv global set, shown in every channel.
#[utoipa::path(
    get,
    path = "/v1/sets/global",
    tag = "sets",
    responses((status = 200, body = EmoteSet))
)]
async fn get_global_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// Create a channel set, or your personal set (one per account).
#[utoipa::path(
    post,
    path = "/v1/sets",
    tag = "sets",
    request_body = CreateSet,
    responses(
        (status = 201, body = EmoteSet),
        (status = 409, description = "`personal_set_exists`.", body = ErrorBody)
    )
)]
async fn create_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// A set with its emotes. Send `If-None-Match` to get `304` when nothing changed.
#[utoipa::path(
    get,
    path = "/v1/sets/{id}",
    tag = "sets",
    params(("id" = String, Path, description = "Set ID.")),
    responses(
        (status = 200, body = EmoteSet),
        (status = 304, description = "Not modified."),
        (status = 404, body = ErrorBody)
    )
)]
async fn get_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// Rename a set or change its capacity. Owner or editor; capacity needs `sets.edit_any`.
#[utoipa::path(
    patch,
    path = "/v1/sets/{id}",
    tag = "sets",
    params(("id" = String, Path, description = "Set ID.")),
    request_body = UpdateSet,
    responses((status = 200, body = EmoteSet), (status = 403, body = ErrorBody))
)]
async fn update_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// Delete a set. Owner only. A channel's active set can't be deleted.
#[utoipa::path(
    delete,
    path = "/v1/sets/{id}",
    tag = "sets",
    params(("id" = String, Path, description = "Set ID.")),
    responses(
        (status = 204, description = "Deleted."),
        (status = 409, description = "`set_in_use`.", body = ErrorBody)
    )
)]
async fn delete_set() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

/// Add an emote to a set. Personal sets take approved emotes only.
#[utoipa::path(
    put,
    path = "/v1/sets/{id}/emotes/{emote_id}",
    tag = "sets",
    params(
        ("id" = String, Path, description = "Set ID."),
        ("emote_id" = String, Path, description = "Emote ID.")
    ),
    request_body = AddSetEmote,
    responses(
        (status = 200, body = SetEmote),
        (status = 409, description = "`set_full`, `name_taken` or `not_approved`.", body = ErrorBody)
    )
)]
async fn add_set_emote() -> ApiResult<Json<SetEmote>> {
    Err(ApiError::not_implemented())
}

/// Rename an emote in a set, or change its zero-width flag there.
#[utoipa::path(
    patch,
    path = "/v1/sets/{id}/emotes/{emote_id}",
    tag = "sets",
    params(
        ("id" = String, Path, description = "Set ID."),
        ("emote_id" = String, Path, description = "Emote ID.")
    ),
    request_body = UpdateSetEmote,
    responses((status = 200, body = SetEmote), (status = 409, body = ErrorBody))
)]
async fn update_set_emote() -> ApiResult<Json<SetEmote>> {
    Err(ApiError::not_implemented())
}

/// Remove an emote from a set.
#[utoipa::path(
    delete,
    path = "/v1/sets/{id}/emotes/{emote_id}",
    tag = "sets",
    params(
        ("id" = String, Path, description = "Set ID."),
        ("emote_id" = String, Path, description = "Emote ID.")
    ),
    responses((status = 204, description = "Removed."))
)]
async fn remove_set_emote() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

/// The people who may edit a set besides its owner.
#[utoipa::path(
    get,
    path = "/v1/sets/{id}/editors",
    tag = "sets",
    params(("id" = String, Path, description = "Set ID.")),
    responses((status = 200, body = Vec<AccountSummary>))
)]
async fn list_set_editors() -> ApiResult<Json<Vec<AccountSummary>>> {
    Err(ApiError::not_implemented())
}

/// Make someone an editor of a set. Owner only.
#[utoipa::path(
    put,
    path = "/v1/sets/{id}/editors/{account_id}",
    tag = "sets",
    params(
        ("id" = String, Path, description = "Set ID."),
        ("account_id" = String, Path, description = "cyex.tv account ID.")
    ),
    responses((status = 204, description = "Added."), (status = 403, body = ErrorBody))
)]
async fn add_set_editor() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

/// Remove an editor. Owner, or the editor themselves.
#[utoipa::path(
    delete,
    path = "/v1/sets/{id}/editors/{account_id}",
    tag = "sets",
    params(
        ("id" = String, Path, description = "Set ID."),
        ("account_id" = String, Path, description = "cyex.tv account ID.")
    ),
    responses((status = 204, description = "Removed."))
)]
async fn remove_set_editor() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Emotes

/// Upload an emote. It starts `pending`: usable in your own sets, hidden from everyone else until a
/// moderator approves it.
///
/// PNG, APNG, GIF, WebP or AVIF; at most 5 MB, 1000×1000 px and 1000 frames.
#[utoipa::path(
    post,
    path = "/v1/emotes",
    tag = "emotes",
    request_body(content = UploadEmote, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; processing.", body = Emote),
        (status = 413, description = "`too_large`.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`.", body = ErrorBody),
        (status = 422, description = "`invalid_image` or `blocked`.", body = ErrorBody)
    )
)]
async fn upload_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

/// An emote. Pending ones are visible to their owner and to moderators only.
#[utoipa::path(
    get,
    path = "/v1/emotes/{id}",
    tag = "emotes",
    params(("id" = String, Path, description = "Emote ID.")),
    responses((status = 200, body = Emote), (status = 404, body = ErrorBody))
)]
async fn get_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

/// Change an emote's name, tags or flags. Owner, or `emotes.edit_any`.
#[utoipa::path(
    patch,
    path = "/v1/emotes/{id}",
    tag = "emotes",
    params(("id" = String, Path, description = "Emote ID.")),
    request_body = UpdateEmote,
    responses((status = 200, body = Emote), (status = 403, body = ErrorBody))
)]
async fn update_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

/// Delete your emote. It is removed from every set.
#[utoipa::path(
    delete,
    path = "/v1/emotes/{id}",
    tag = "emotes",
    params(("id" = String, Path, description = "Emote ID.")),
    responses((status = 204, description = "Deleted."), (status = 403, body = ErrorBody))
)]
async fn delete_emote() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Search

/// Search approved, listed emotes by name and tag.
#[utoipa::path(
    get,
    path = "/v1/search/emotes",
    tag = "search",
    params(SearchQuery),
    responses((status = 200, body = SearchResults))
)]
async fn search_emotes() -> ApiResult<Json<SearchResults>> {
    Err(ApiError::not_implemented())
}

// Personal sets

/// One user's personal set.
#[utoipa::path(
    get,
    path = "/v1/users/{platform}/{id}/personal",
    tag = "sets",
    params(
        ("platform" = Platform, Path, description = "`twitch` or `kick`."),
        ("id" = String, Path, description = "Platform user ID.")
    ),
    responses(
        (status = 200, body = EmoteSet),
        (status = 404, description = "No personal set.", body = ErrorBody)
    )
)]
async fn get_personal_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// Personal sets for up to 100 chatters at once, for clients rendering a chat.
#[utoipa::path(
    post,
    path = "/v1/personal-sets/lookup",
    tag = "sets",
    request_body = PersonalLookup,
    responses((status = 200, body = PersonalLookupResult))
)]
async fn lookup_personal_sets() -> ApiResult<Json<PersonalLookupResult>> {
    Err(ApiError::not_implemented())
}

// Moderation

/// Pending emotes, oldest first. Needs `emotes.review`.
#[utoipa::path(
    get,
    path = "/v1/mod/queue",
    tag = "moderation",
    responses((status = 200, body = ReviewQueue), (status = 403, body = ErrorBody))
)]
async fn review_queue() -> ApiResult<Json<ReviewQueue>> {
    Err(ApiError::not_implemented())
}

/// Approve an emote. Needs `emotes.review`. Audited.
#[utoipa::path(
    post,
    path = "/v1/mod/emotes/{id}/approve",
    tag = "moderation",
    params(("id" = String, Path, description = "Emote ID.")),
    responses((status = 200, body = Emote), (status = 403, body = ErrorBody))
)]
async fn approve_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

/// Reject an emote with a reason. Needs `emotes.review`. Audited.
#[utoipa::path(
    post,
    path = "/v1/mod/emotes/{id}/reject",
    tag = "moderation",
    params(("id" = String, Path, description = "Emote ID.")),
    request_body = RejectEmote,
    responses((status = 200, body = Emote), (status = 403, body = ErrorBody))
)]
async fn reject_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

// Roles

/// All roles. Needs `roles.manage`.
#[utoipa::path(
    get,
    path = "/v1/roles",
    tag = "roles",
    responses((status = 200, body = Vec<Role>), (status = 403, body = ErrorBody))
)]
async fn list_roles() -> ApiResult<Json<Vec<Role>>> {
    Err(ApiError::not_implemented())
}

/// Create a role. You can only grant permissions you hold. Audited.
#[utoipa::path(
    post,
    path = "/v1/roles",
    tag = "roles",
    request_body = CreateRole,
    responses(
        (status = 201, body = Role),
        (status = 403, description = "`forbidden` or `escalation`.", body = ErrorBody),
        (status = 422, description = "`unknown_permission`.", body = ErrorBody)
    )
)]
async fn create_role() -> ApiResult<Json<Role>> {
    Err(ApiError::not_implemented())
}

/// Change a role below your own highest one. Audited.
#[utoipa::path(
    patch,
    path = "/v1/roles/{id}",
    tag = "roles",
    params(("id" = String, Path, description = "Role ID.")),
    request_body = UpdateRole,
    responses((status = 200, body = Role), (status = 403, body = ErrorBody))
)]
async fn update_role() -> ApiResult<Json<Role>> {
    Err(ApiError::not_implemented())
}

/// Delete a role below your own highest one. Audited.
#[utoipa::path(
    delete,
    path = "/v1/roles/{id}",
    tag = "roles",
    params(("id" = String, Path, description = "Role ID.")),
    responses((status = 204, description = "Deleted."), (status = 403, body = ErrorBody))
)]
async fn delete_role() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

/// Give an account a role. Audited.
#[utoipa::path(
    put,
    path = "/v1/accounts/{account_id}/roles/{role_id}",
    tag = "roles",
    params(
        ("account_id" = String, Path, description = "cyex.tv account ID."),
        ("role_id" = String, Path, description = "Role ID.")
    ),
    responses((status = 204, description = "Assigned."), (status = 403, body = ErrorBody))
)]
async fn assign_role() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

/// Take a role from an account. The owner and the last `roles.manage` holder are protected.
/// Audited.
#[utoipa::path(
    delete,
    path = "/v1/accounts/{account_id}/roles/{role_id}",
    tag = "roles",
    params(
        ("account_id" = String, Path, description = "cyex.tv account ID."),
        ("role_id" = String, Path, description = "Role ID.")
    ),
    responses(
        (status = 204, description = "Removed."),
        (status = 409, description = "`last_manager` or `owner`.", body = ErrorBody)
    )
)]
async fn unassign_role() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Events

/// Live updates over SSE (ADR 0005).
///
/// Served by the events service at the same host; listed here so the contract is in one document.
///
/// Event types:
/// - `set.emote_added`
/// - `set.emote_removed`
/// - `set.emote_renamed`
/// - `channel.active_set_changed`
/// - `personal.updated`
/// - `emote.updated`
/// - `resync`
///
/// Resume with `Last-Event-ID` (24 h window). A comment line arrives every 20 s.
#[utoipa::path(
    get,
    path = "/v1/events",
    tag = "events",
    params(EventsQuery),
    responses(
        (status = 200, description = "An event stream.", content_type = "text/event-stream", body = String),
        (status = 400, description = "`bad_subscription` or `too_many_subscriptions`.", body = ErrorBody)
    )
)]
#[allow(dead_code)] // Documentation only: the events service serves this path.
pub(crate) async fn events() {}
