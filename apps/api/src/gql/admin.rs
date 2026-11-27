//! GraphQL: admin.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{AccountSession, AccountStatus, AdminAccount, EmotePage, Notification};

use super::{api_error, caller, state};
use crate::{abuse, account, moderation, staff};

#[derive(Default)]
pub struct AdminQuery;

#[Object]
impl AdminQuery {
    /// Where you are signed in: your live sessions, the most recently used first, with the one
    /// this request came with marked `current`. Never shows tokens.
    async fn my_sessions(&self, ctx: &Context<'_>) -> Result<Vec<AccountSession>> {
        let caller = caller(ctx)?;
        account::my_sessions(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// An account as staff see it: identities, created time and restrictions. Needs
    /// `users.view` or `users.restrict`.
    async fn admin_account(&self, ctx: &Context<'_>, account_id: String) -> Result<AdminAccount> {
        let caller = caller(ctx)?;
        abuse::admin_account(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// How many of your notifications are unread.
    async fn unread_notification_count(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        account::unread_count(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// An account's emotes in every state, newest first. Needs `users.view`, `emotes.review`,
    /// `emotes.takedown`, `emotes.edit_any`, `emotes.delete_any` or `emotes.flags`, or to be the account or its
    /// editor with `EMOTE_MANAGE` on every set. `perPage` is 1 to 100, default 50.
    /// Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
    /// ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
    async fn account_emotes(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        page: Option<u32>,
        per_page: Option<u32>,
        after: Option<String>,
    ) -> Result<EmotePage> {
        let caller = caller(ctx)?;
        moderation::account_emotes(
            state(ctx),
            caller,
            &account_id,
            page,
            per_page,
            after.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Your notifications, newest first (the latest 100). `kind` is one of `emote.approved`,
    /// `emote.rejected`, `emote.approvals_changed` (a reviewer decided its public listing or
    /// personal use, ADR 0039: `public_listing`, `personal_use`, `reason`), `emote.transferred`,
    /// `emote.taken_down`, `emote.visibility_changed` (staff changed its
    /// visibility, ADR 0024: `visibility` and `was`), `editor.added`, `editor.changed`, `editor.removed`,
    /// `cosmetic.granted` and `cosmetic.revoked`; `payload` is JSON with the IDs and names it is
    /// about (see the API reference). Cosmetics from the shop and from codes carry `source`
    /// (`purchase` or `code`), and from orders `order_id`.
    async fn my_notifications(
        &self,
        ctx: &Context<'_>,
        unread_only: Option<bool>,
    ) -> Result<Vec<Notification>> {
        let caller = caller(ctx)?;
        abuse::my_notifications(state(ctx), caller, unread_only.unwrap_or(false))
            .await
            .map_err(api_error)
    }

    /// Every restriction placed on an account, newest first (the latest 100), running or over,
    /// with its reason and end. Needs `users.view`, `users.restrict` or a narrower restrict
    /// permission.
    async fn account_restrictions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::Restriction>> {
        let caller = caller(ctx)?;
        abuse::restrictions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// What an account may do now: uploads, reports and sign-in, each with its end when timed.
    /// Needs `users.view`, `users.restrict` or a narrower restrict permission.
    async fn account_status(&self, ctx: &Context<'_>, account_id: String) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::status(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Staff notes on an account, newest first (the latest 200). Needs `users.notes`.
    async fn staff_notes(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::StaffNote>> {
        let caller = caller(ctx)?;
        staff::notes(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Where an account is signed in, the most recently used first. Never tokens. Needs
    /// `users.sessions`.
    async fn account_sessions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<AccountSession>> {
        let caller = caller(ctx)?;
        staff::account_sessions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }
}
