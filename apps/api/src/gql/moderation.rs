//! GraphQL: moderation.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    AccountReports, AccountStatus, BatchOutcome, Emote, Report, ReportAction, ReportQueue,
};

use super::{api_error, caller, state};
use crate::{abuse, review, staff};

#[derive(Default)]
pub struct ModerationQuery;

#[Object]
impl ModerationQuery {
    /// Open reports, oldest first. Needs `emotes.review` or `emotes.takedown`.
    async fn report_queue(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<ReportQueue> {
        let caller = caller(ctx)?;
        abuse::queue(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// Reports about an account's emotes and the ones it filed, newest first, at most 50 each.
    /// Needs `emotes.review` or `emotes.takedown`.
    async fn account_reports(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<AccountReports> {
        let caller = caller(ctx)?;
        abuse::account_reports(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct ModerationMutation;

#[Object]
impl ModerationMutation {
    /// Dismiss up to 100 open reports with one optional note, in one step, one audit row each.
    /// Reports already resolved are listed in `failed`. Takedowns stay one at a time
    /// (`resolveReport`). Needs `emotes.review`, `emotes.takedown` or `reports.manage`.
    async fn dismiss_reports(
        &self,
        ctx: &Context<'_>,
        ids: Vec<String>,
        note: Option<String>,
    ) -> Result<BatchOutcome> {
        let caller = caller(ctx)?;
        abuse::dismiss_many(state(ctx), caller, &ids, note.as_deref())
            .await
            .map_err(api_error)
    }

    /// Replace the review queue's ready reject reasons (0 to 12, each 1 to 200 characters, in
    /// order). Needs `review.settings`. Audited.
    async fn set_review_reasons(
        &self,
        ctx: &Context<'_>,
        reasons: Vec<String>,
    ) -> Result<Vec<String>> {
        let caller = caller(ctx)?;
        review::set_reasons(state(ctx), caller, &reasons)
            .await
            .map_err(api_error)
    }

    /// Report an emote you can see, with a reason. At most 10 an hour, one open report per emote.
    async fn report_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Report> {
        let caller = caller(ctx)?;
        abuse::report(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Dismiss a report, or take its emote down (`emotes.takedown`). Needs `emotes.review` or
    /// `emotes.takedown`. Audited.
    async fn resolve_report(
        &self,
        ctx: &Context<'_>,
        id: String,
        action: ReportAction,
        note: Option<String>,
    ) -> Result<Report> {
        let caller = caller(ctx)?;
        abuse::resolve(state(ctx), caller, &id, action, note.as_deref())
            .await
            .map_err(api_error)
    }

    /// Take an emote's image down everywhere: out of every set, its file blocked and deleted,
    /// its owners told. Needs `emotes.takedown`. Audited.
    async fn takedown_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        abuse::takedown(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Add a staff note to an account (1 to 2000 characters). Needs `users.notes`. Audited.
    async fn add_staff_note(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        body: String,
    ) -> Result<cyex_tv_shared::model::StaffNote> {
        let caller = caller(ctx)?;
        staff::add_note(state(ctx), caller, &account_id, &body)
            .await
            .map_err(api_error)
    }

    /// Delete a staff note: its author may, and the owner. False if it was gone. Audited.
    async fn delete_staff_note(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        staff::delete_note(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Restrict an account with a reason, for good or `until` a time: no uploads
    /// (`users.restrict_uploads`), no reports or tickets (`users.restrict_tickets`), or a full
    /// block that ends its sessions (`users.block`); `users.restrict` does all three. A running
    /// restriction of the same kind is replaced. The owner, and accounts whose highest role is at
    /// or above yours, are protected. Audited.
    async fn place_restriction(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        input: cyex_tv_shared::model::RestrictionInput,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::place(state(ctx), caller, &account_id, &input)
            .await
            .map_err(api_error)
    }

    /// Lift a restriction early, with an optional reason. Needs the same permission as placing
    /// it. Audited.
    async fn lift_restriction(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        kind: cyex_tv_shared::model::RestrictionKind,
        reason: Option<String>,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::lift(state(ctx), caller, &account_id, kind, reason.as_deref())
            .await
            .map_err(api_error)
    }

    /// Block an account's uploads, or the whole account, for good and without a reason (see
    /// `placeRestriction` for timed ones). Needs `users.restrict`, or `users.restrict_uploads` /
    /// `users.block` for the one you change; the owner is protected. Audited.
    async fn restrict_account(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        uploads_blocked: Option<bool>,
        blocked: Option<bool>,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::restrict(state(ctx), caller, &account_id, uploads_blocked, blocked)
            .await
            .map_err(api_error)
    }
}
