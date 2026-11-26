//! GraphQL: site.

use async_graphql::{Context, Object, Result};

use super::{api_error, caller, caller_opt, state};
use crate::site;
use cyex_tv_shared::model::{AccountCollection, Announcement, AnnouncementInput, NotificationPref};

#[derive(Default)]
pub struct SiteQuery;

#[Object]
impl SiteQuery {
    /// The site announcement shown under the header right now, or null. Public; shared for a few
    /// seconds, so a change shows within that.
    async fn active_announcement(&self, ctx: &Context<'_>) -> Result<Option<Announcement>> {
        site::active(state(ctx)).await.map_err(api_error)
    }

    /// Every site announcement, newest first (the latest 50), ended ones too. Needs
    /// `site.announce`.
    async fn announcements(&self, ctx: &Context<'_>) -> Result<Vec<Announcement>> {
        let caller = caller(ctx)?;
        site::list(state(ctx), caller).await.map_err(api_error)
    }

    /// Every notification kind and what you chose for it: created at all (`notify`) and popping
    /// up on the website (`toast`). `required` kinds always notify.
    async fn my_notification_prefs(&self, ctx: &Context<'_>) -> Result<Vec<NotificationPref>> {
        let caller = caller(ctx)?;
        site::my_notification_prefs(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// The badges and paints an account owns, with since when and how each came (staff, the
    /// shop, a code, an event); never who gave it, why or what it cost. Null unless the account
    /// shows its collection (`AccountProfile.collectionPublic`); the account itself always sees
    /// its own.
    async fn account_collection(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Option<AccountCollection>> {
        site::collection(state(ctx), caller_opt(ctx), &account_id)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct SiteMutation;

#[Object]
impl SiteMutation {
    /// Put an announcement under the header: one line, a kind, an optional link, emote or badge,
    /// start and end. Needs `site.announce`; audited.
    async fn create_announcement(
        &self,
        ctx: &Context<'_>,
        input: AnnouncementInput,
    ) -> Result<Announcement> {
        let caller = caller(ctx)?;
        site::create(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Replace every field of an announcement that hasn't ended. Needs `site.announce`; audited.
    async fn update_announcement(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: AnnouncementInput,
    ) -> Result<Announcement> {
        let caller = caller(ctx)?;
        site::update(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// End an announcement now. Needs `site.announce`; audited.
    async fn end_announcement(&self, ctx: &Context<'_>, id: String) -> Result<Announcement> {
        let caller = caller(ctx)?;
        site::end(state(ctx), caller, &id).await.map_err(api_error)
    }
}
