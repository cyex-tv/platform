//! GraphQL: review.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    AuditEntry, BatchOutcome, DuplicateCandidate, Emote, EmoteNote, EmoteVisibility, LoginMatch,
    ReviewQueue, ReviewQueueFilter, ReviewVerdict,
};
use cyex_tv_shared::permissions;

use super::{api_error, caller, heavy_read, state};
use crate::{idempotency, moderation, review};

#[derive(Default)]
pub struct ReviewQuery;

#[Object]
impl ReviewQuery {
    /// Every permission name a role can hold.
    async fn permissions(&self) -> Vec<String> {
        permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
    }

    /// Pending emotes, oldest first. Needs `emotes.review`. `perPage` is 1 to 100, default 50.
    /// With `filter`: one tab (public, unlisted and private, or resolved), a search by name or
    /// uploader, and an order. A cursor only works for the same filter (`bad_cursor`).
    /// Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
    /// ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
    async fn review_queue(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
        filter: Option<ReviewQueueFilter>,
        after: Option<String>,
    ) -> Result<ReviewQueue> {
        let caller = caller(ctx)?;
        moderation::review_queue(
            state(ctx),
            caller,
            page,
            per_page,
            filter.as_ref(),
            after.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Emotes that may duplicate one in the review queue: the same file (sha256) first, then
    /// similar names, closest first; at most 12, any state but deleted. Needs `emotes.review`.
    async fn emote_duplicates(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
    ) -> Result<Vec<DuplicateCandidate>> {
        let caller = caller(ctx)?;
        review::duplicates(state(ctx), caller, &emote_id)
            .await
            .map_err(api_error)
    }

    /// Public emotes already made from the file with this sha256 (64 hex digits), oldest first,
    /// at most 3: the upload form asks before sending, so it can say "looks like" (it never
    /// blocks). Signed in.
    async fn emotes_with_same_file(&self, ctx: &Context<'_>, sha256: String) -> Result<Vec<Emote>> {
        caller(ctx)?;
        heavy_read(ctx)?;
        review::same_file(state(ctx), &sha256)
            .await
            .map_err(api_error)
    }

    /// The ready reject reasons the review queue offers, in order. Needs `emotes.review` or
    /// `review.settings`.
    async fn review_reasons(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let caller = caller(ctx)?;
        review::reasons(state(ctx), caller).await.map_err(api_error)
    }

    /// Accounts for up to 100 pasted logins (`name`, `@name`, `twitch:name` or `kick:name`), in
    /// order, each with the account or why there is none (`not_found`, `ambiguous`,
    /// `bad_login`). For grants to many at once: needs `cosmetics.grant`, `roles.assign`,
    /// `roles.manage` or `users.view`.
    async fn accounts_by_login(
        &self,
        ctx: &Context<'_>,
        logins: Vec<String>,
    ) -> Result<Vec<LoginMatch>> {
        let caller = caller(ctx)?;
        review::accounts_by_login(state(ctx), caller, &logins)
            .await
            .map_err(api_error)
    }

    /// Reviewers' notes on an emote, newest first (the latest 200). Needs `emotes.review`.
    async fn emote_notes(&self, ctx: &Context<'_>, emote_id: String) -> Result<Vec<EmoteNote>> {
        let caller = caller(ctx)?;
        moderation::notes(state(ctx), caller, &emote_id)
            .await
            .map_err(api_error)
    }

    /// What happened to one emote, newest first: the latest 100 audit entries about it
    /// (upload, edits, reviews, notes, takedowns). Needs `emotes.review`.
    async fn emote_history(&self, ctx: &Context<'_>, emote_id: String) -> Result<Vec<AuditEntry>> {
        let caller = caller(ctx)?;
        moderation::history(state(ctx), caller, &emote_id)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct ReviewMutation;

#[Object]
impl ReviewMutation {
    /// Approve a pending emote. Needs `emotes.review`. Audited. With `visibility`, it is also
    /// made public, unlisted or private in the same step (for example, approved for personal use
    /// only). `idempotencyKey` (optional UUID): a retry with it within 24 hours answers what the
    /// first approval did instead of `not_pending`.
    async fn approve_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        visibility: Option<EmoteVisibility>,
        idempotency_key: Option<String>,
    ) -> Result<Emote> {
        let caller = caller(ctx)?;
        let state = state(ctx);
        let request = serde_json::json!({
            "id": id.trim(),
            "visibility": visibility.map(|v| format!("{v:?}")),
        });
        idempotency::once(
            state,
            caller,
            idempotency::op::APPROVE_EMOTE,
            idempotency_key.as_deref(),
            request,
            || moderation::approve(state, caller, &id, visibility),
        )
        .await
        .map_err(api_error)
    }

    /// Add a reviewers' note to an emote (1 to 2000 characters). Needs `emotes.review`. Audited.
    async fn add_emote_note(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
        body: String,
    ) -> Result<EmoteNote> {
        let caller = caller(ctx)?;
        moderation::add_note(state(ctx), caller, &emote_id, &body)
            .await
            .map_err(api_error)
    }

    /// Delete an emote note: its author may, and the owner. False if it was gone. Audited.
    async fn delete_emote_note(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        moderation::delete_note(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Reject a pending emote with a reason. Needs `emotes.review`. Audited.
    async fn reject_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::reject(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Approve (optionally as `visibility`) or reject (with `reason`) up to 100 pending emotes in
    /// one step. All of it commits together, with one audit row and one notice per emote; emotes
    /// that aren't pending are left alone and listed in `failed`. Needs `emotes.review`.
    /// `idempotencyKey` (optional UUID): a retry with it within 24 hours answers the first
    /// outcome instead of listing every emote as `not_pending`.
    async fn review_emotes(
        &self,
        ctx: &Context<'_>,
        ids: Vec<String>,
        verdict: ReviewVerdict,
        visibility: Option<EmoteVisibility>,
        reason: Option<String>,
        idempotency_key: Option<String>,
    ) -> Result<BatchOutcome> {
        let caller = caller(ctx)?;
        let state = state(ctx);
        let request = serde_json::json!({
            "ids": ids,
            "verdict": format!("{verdict:?}"),
            "visibility": visibility.map(|v| format!("{v:?}")),
            "reason": reason,
        });
        idempotency::once(
            state,
            caller,
            idempotency::op::REVIEW_EMOTES,
            idempotency_key.as_deref(),
            request,
            || {
                moderation::review_batch(
                    state,
                    caller,
                    &ids,
                    verdict,
                    visibility,
                    reason.as_deref(),
                )
            },
        )
        .await
        .map_err(api_error)
    }

    /// Send up to 100 pending emotes to the end of the review queue for every reviewer (an
    /// approval taken back within its undo time comes back last, ADR 0024). Emotes that aren't
    /// pending are listed in `failed`. Needs `emotes.review`. Audited (`emote.requeued`).
    async fn requeue_emotes(&self, ctx: &Context<'_>, ids: Vec<String>) -> Result<BatchOutcome> {
        let caller = caller(ctx)?;
        moderation::requeue_batch(state(ctx), caller, &ids)
            .await
            .map_err(api_error)
    }
}
