//! GraphQL: account.

use async_graphql::{Context, Object, Result};
use chrono::SecondsFormat;
use cyex_tv_auth::Caller;
use cyex_tv_db::{DbError, accounts, channels, ids};
use cyex_tv_shared::error::ApiError;
use cyex_tv_shared::model::{Identity, Me, PendingDeletion, Platform};
use sqlx::Connection;

use super::{Restorable, api_error, auth_error, caller, coded, internal, restorable, state};
use crate::{account, catalog, site};
use cyex_tv_shared::model::{AccountCollection, NotificationPref, NotificationPrefInput};

/// The account behind `caller`, as `Me`.
pub(super) async fn me_of(ctx: &Context<'_>, caller: &Caller) -> Result<Me> {
    let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
    let account = accounts::get(&mut conn, caller.account_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| coded("signed_out", "Sign in to use this."))?;
    let identities = accounts::identities(&mut conn, account.id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|i| Identity {
            platform: i.platform,
            platform_user_id: i.platform_user_id,
            login: i.login,
            display_name: i.display_name,
            avatar: i.avatar,
        })
        .collect();
    let permissions = catalog::permissions_of(&mut conn, caller)
        .await
        .map_err(api_error)?;
    let display_platform = accounts::display_platform(&mut conn, account.id)
        .await
        .map_err(internal)?;
    // The owners hold the top role without an assignment row; their own profile lists it, hidden
    // (never public, and `accountProfile` never shows it to anyone else).
    let top = if caller.owner {
        cyex_tv_db::roles::system(&mut conn, cyex_tv_db::roles::SYSTEM_ROOT)
            .await
            .map_err(internal)?
    } else {
        None
    };
    let roles = top
        .into_iter()
        .map(|r| cyex_tv_shared::model::OwnRole {
            id: ids::to_text(r.id),
            name: r.name,
            public: false,
            invisible: true,
        })
        .chain(
            cyex_tv_db::roles::roles_of(&mut conn, account.id)
                .await
                .map_err(internal)?
                .into_iter()
                .map(|r| cyex_tv_shared::model::OwnRole {
                    id: ids::to_text(r.id),
                    name: r.name,
                    public: r.public,
                    invisible: r.invisible,
                }),
        )
        .collect();
    Ok(Me {
        id: ids::to_text(account.id),
        identities,
        permissions,
        roles,
        created_at: account
            .created_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        display_platform,
    })
}

#[derive(Default)]
pub struct AccountQuery;

#[Object]
impl AccountQuery {
    /// The signed-in account.
    async fn me(&self, ctx: &Context<'_>) -> Result<Me> {
        let caller = caller(ctx)?;
        me_of(ctx, caller).await
    }

    /// The deletion you can still undo: set only for the cookie session that signing in to a
    /// deleted account within its 30 days gave (`restoreAccount` undoes it). Null otherwise,
    /// including for every normal session.
    async fn pending_deletion(&self, ctx: &Context<'_>) -> Result<Option<PendingDeletion>> {
        let Some(pending) = ctx.data_opt::<Restorable>().and_then(|r| r.0.as_ref()) else {
            return Ok(None);
        };
        account::pending_deletion(state(ctx), pending)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct AccountMutation;

#[Object]
impl AccountMutation {
    /// Unlink a platform from your account. The last identity can't be removed, and a
    /// restricted or blocked account can't unlink (`restricted`).
    async fn unlink_identity(&self, ctx: &Context<'_>, platform: Platform) -> Result<Me> {
        let caller = caller(ctx)?;
        let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
        let mut tx = conn.begin().await.map_err(internal)?;
        // Locked, so a restriction landing at the same time is seen. Signing in again with the
        // dropped identity would otherwise make a fresh, unrestricted account.
        let restricted: Option<bool> = sqlx::query_scalar(
            "SELECT (restricted_at IS NOT NULL AND (restricted_until IS NULL OR restricted_until > now()))
                 OR (blocked_at IS NOT NULL AND (blocked_until IS NULL OR blocked_until > now()))
                 OR (tickets_blocked_at IS NOT NULL
                     AND (tickets_blocked_until IS NULL OR tickets_blocked_until > now()))
               FROM accounts WHERE id = $1 FOR SHARE",
        )
        .bind(caller.account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;
        if restricted == Some(true) {
            return Err(api_error(ApiError::new(
                axum::http::StatusCode::FORBIDDEN,
                "restricted",
                "A restricted account can't unlink a platform. Write to us if you need it changed.",
            )));
        }
        match accounts::unlink(&mut tx, caller.account_id, platform).await {
            Ok(_) => {}
            Err(DbError::NotFound) => {
                return Err(coded(
                    "not_found",
                    "That platform isn't linked to your account.",
                ));
            }
            Err(err) => return Err(auth_error(err.into())),
        }
        // The account no longer proves it owns that platform's channel.
        channels::delete_for(&mut tx, caller.account_id, platform)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        drop(conn);
        me_of(ctx, caller).await
    }

    /// Turn notification kinds on or off (`notify`: off, the server doesn't create them at all)
    /// and choose which pop up (`toast`). Kinds left out keep their setting. `unknown_kind` for a
    /// kind that doesn't exist, `required_kind` for turning off one that must stay on.
    async fn set_notification_prefs(
        &self,
        ctx: &Context<'_>,
        prefs: Vec<NotificationPrefInput>,
    ) -> Result<Vec<NotificationPref>> {
        let caller = caller(ctx)?;
        site::set_notification_prefs(state(ctx), caller, &prefs)
            .await
            .map_err(api_error)
    }

    /// Show (or hide) the badges and paints you own on your public profile. Off by default.
    async fn set_collection_public(
        &self,
        ctx: &Context<'_>,
        public: bool,
    ) -> Result<AccountCollection> {
        let caller = caller(ctx)?;
        site::set_collection_public(state(ctx), caller, public)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct AccountSessionMutation;

#[Object]
impl AccountSessionMutation {
    /// Sign out one of your sessions, the current one too (then the cookie is cleared). False
    /// when you have none with that ID.
    async fn revoke_session(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        let (done, current) = account::revoke_session(state(ctx), caller, &id)
            .await
            .map_err(api_error)?;
        if current && caller.via == cyex_tv_auth::Via::Cookie {
            ctx.insert_http_header(
                axum::http::header::SET_COOKIE,
                cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
            );
        }
        Ok(done)
    }

    /// Sign out everywhere, here too: every site and app session ends and the cookie is cleared.
    /// Returns how many sessions ended. With an app token, only that app session ends.
    async fn sign_out_everywhere(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        let n = account::revoke_all_sessions(state(ctx), caller)
            .await
            .map_err(api_error)?;
        if caller.via == cyex_tv_auth::Via::Cookie {
            ctx.insert_http_header(
                axum::http::header::SET_COOKIE,
                cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
            );
        }
        Ok(n)
    }

    /// Sign out everywhere but here. Returns how many sessions ended. Website only
    /// (`website_only` with an app token).
    async fn revoke_other_sessions(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        account::revoke_other_sessions(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// A copy of everything about your account, as JSON text (`format`, `version`, `account`,
    /// `identities`, `sessions`, `channels`, `emotes`, `sets`, `set_editors`, `account_editors`,
    /// `editor_of`, `cosmetics`, `roles`, `reports_filed`, `notifications`, `orders`,
    /// `hidden_emotes`, `hidden_creators`, `favourite_emotes`). Website
    /// only (`website_only` with an app token). 5 an hour (`rate_limited`). Audited.
    async fn export_my_data(&self, ctx: &Context<'_>) -> Result<String> {
        let caller = caller(ctx)?;
        account::export(state(ctx), caller).await.map_err(api_error)
    }

    /// Make a personal API token (a console token, ADR 0028): a Bearer token that acts as you,
    /// for a GraphQL playground or a script. It has your account's own rights (your roles count;
    /// owner-only and website-only actions don't work with it), lasts 90 days and shows in
    /// `mySessions` as `Console token`. Only one at a time: making one signs the last one out.
    /// The token is in this answer only. Website only (`website_only` with a Bearer token). 5 an
    /// hour (`rate_limited`). Audited. Same as `POST /v1/auth/token/export`.
    async fn create_console_token(
        &self,
        ctx: &Context<'_>,
    ) -> Result<cyex_tv_shared::model::ConsoleToken> {
        let caller = caller(ctx)?;
        crate::console_token::mint(state(ctx), caller, None)
            .await
            .map_err(api_error)
    }

    /// Delete your account. `confirm` is one of your Twitch or Kick logins. Website only
    /// (`website_only` with an app token); the owner's account can't be (`owner_protected`).
    /// The account is hidden at once (profile, sets, emotes and channels read as gone, every
    /// session ends, the cookie is cleared) and erased for good 30 days later: then your
    /// identities, channels, sets, editor grants, cosmetics and notifications go, your approved
    /// emotes that are in other people's sets stay, shown as by a deleted user, and your other
    /// emotes are deleted. Signing in again before then offers to undo it (`pendingDeletion`,
    /// `restoreAccount`); after it, signing in makes a new account. Audited.
    /// `confirm_mismatch` when `confirm` isn't one of your logins; `restricted` while the account's
    /// uploads or sign-in are blocked. 3 tries an hour (`rate_limited`).
    async fn delete_account(&self, ctx: &Context<'_>, confirm: String) -> Result<bool> {
        let caller = caller(ctx)?;
        account::delete_account(state(ctx), caller, &confirm)
            .await
            .map_err(api_error)?;
        ctx.insert_http_header(
            axum::http::header::SET_COOKIE,
            cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
        );
        Ok(true)
    }

    /// Undo `deleteAccount` within 30 days, signed in with the cookie that signing in to the
    /// deleted account gave (the website's restore page). Everything shows again and the session
    /// works as before. `signed_out` without such a session; `nothing_to_restore` when the window
    /// has passed. Audited.
    async fn restore_account(&self, ctx: &Context<'_>) -> Result<bool> {
        let pending = restorable(ctx)?;
        account::restore_account(state(ctx), pending)
            .await
            .map_err(api_error)?;
        Ok(true)
    }
}
