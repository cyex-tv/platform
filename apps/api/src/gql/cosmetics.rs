//! GraphQL: cosmetics.

use async_graphql::{Context, MaybeUndefined, Object, Result};
use cyex_tv_shared::model::{
    Badge, CosmeticCatalog, CosmeticImage, CosmeticKind, CreateBadge, Entitlement, EntitlementPage,
    GrantCosmetic, OwnedCosmetics, Paint, PaintInput, Platform, UpdateBadge, UserCosmetics,
};

use super::{api_error, caller, caller_opt, state};
use crate::cosmetics;

#[derive(Default)]
pub struct CosmeticQuery;

#[Object]
impl CosmeticQuery {
    /// The cyex.tv chat badges and name paints that are for sale in the shop or held by at least
    /// one account (ADR 0015). Public; unreleased ones are left out, except for staff with a
    /// `cosmetics.*` permission. Clients cache it (`GET /v1/cosmetics` has an ETag) and follow
    /// the `cosmetics` topic.
    async fn cosmetics(&self, ctx: &Context<'_>) -> Result<CosmeticCatalog> {
        cosmetics::catalog_for(state(ctx), caller_opt(ctx))
            .await
            .map_err(api_error)
    }

    /// The badge and paint up to 100 chatters show, as catalog IDs. Only chatters showing
    /// something are listed.
    async fn user_cosmetics(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        user_ids: Vec<String>,
    ) -> Result<Vec<UserCosmetics>> {
        cosmetics::user_cosmetics(state(ctx), platform, &user_ids)
            .await
            .map_err(api_error)
    }

    /// The cosmetics you own and show; with `accountId`, those of an account you edit with
    /// `PROFILE`.
    async fn my_cosmetics(
        &self,
        ctx: &Context<'_>,
        account_id: Option<String>,
    ) -> Result<OwnedCosmetics> {
        let caller = caller(ctx)?;
        cosmetics::my_cosmetics(state(ctx), caller, account_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// A badge or paint picture, to see whether its files are made (`ready`). Needs
    /// `cosmetics.manage`.
    async fn cosmetic_image(&self, ctx: &Context<'_>, id: String) -> Result<Option<CosmeticImage>> {
        let caller = caller(ctx)?;
        cosmetics::image(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Who owns which badges or paints, newest first; narrow it with `accountId` and/or
    /// `cosmeticId`. `perPage` is 1 to 100, default 50. Needs `cosmetics.grant`.
    #[allow(clippy::too_many_arguments)]
    async fn cosmetic_entitlements(
        &self,
        ctx: &Context<'_>,
        kind: CosmeticKind,
        account_id: Option<String>,
        cosmetic_id: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<EntitlementPage> {
        let caller = caller(ctx)?;
        cosmetics::entitlements(
            state(ctx),
            caller,
            kind,
            account_id.as_deref(),
            cosmetic_id.as_deref(),
            page,
            per_page,
        )
        .await
        .map_err(api_error)
    }
}

#[derive(Default)]
pub struct CosmeticMutation;

#[Object]
impl CosmeticMutation {
    /// Pick the badge and paint you show, from what you own (ADR 0015). Leave a field out to keep
    /// it; send null or an empty string to show none. With `ownerId`, for an account you edit
    /// with `PROFILE`.
    async fn select_cosmetics(
        &self,
        ctx: &Context<'_>,
        badge_id: MaybeUndefined<String>,
        paint_id: MaybeUndefined<String>,
        owner_id: Option<String>,
    ) -> Result<OwnedCosmetics> {
        let caller = caller(ctx)?;
        let pick = |m: MaybeUndefined<String>| match m {
            MaybeUndefined::Undefined => cosmetics::Pick::Keep,
            MaybeUndefined::Null => cosmetics::Pick::Clear,
            MaybeUndefined::Value(v) => cosmetics::Pick::Set(v),
        };
        cosmetics::select(
            state(ctx),
            caller,
            owner_id.as_deref(),
            &pick(badge_id),
            &pick(paint_id),
        )
        .await
        .map_err(api_error)
    }

    /// Make a badge from a ready badge picture (`POST /v1/cosmetics/images`). Needs
    /// `cosmetics.manage`. Audited; publishes `cosmetic.created`.
    async fn create_badge(&self, ctx: &Context<'_>, input: CreateBadge) -> Result<Badge> {
        let caller = caller(ctx)?;
        cosmetics::create_badge(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change a badge's name, tooltip or picture. Needs `cosmetics.manage`. Audited.
    async fn update_badge(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateBadge,
    ) -> Result<Badge> {
        let caller = caller(ctx)?;
        cosmetics::update_badge(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a badge: everyone loses it and stops showing it. False when there was none. Needs
    /// `cosmetics.manage`. Audited.
    async fn delete_badge(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::delete_badge(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Make a badge or paint picture's files again from its stored original: a badge made
    /// before the 128, 256 and 512 px sizes gets them. Its files stay in use until the new set
    /// is made; read `cosmeticImage(id)` to see it. `original_missing` when the original is
    /// gone (upload it again). Needs `cosmetics.manage`. Audited.
    async fn remake_cosmetic_image(&self, ctx: &Context<'_>, id: String) -> Result<CosmeticImage> {
        let caller = caller(ctx)?;
        cosmetics::remake_image(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Make a paint. Needs `cosmetics.manage`. Audited; publishes `cosmetic.created`.
    async fn create_paint(&self, ctx: &Context<'_>, input: PaintInput) -> Result<Paint> {
        let caller = caller(ctx)?;
        cosmetics::create_paint(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Replace every field of a paint. Needs `cosmetics.manage`. Audited.
    async fn update_paint(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: PaintInput,
    ) -> Result<Paint> {
        let caller = caller(ctx)?;
        cosmetics::update_paint(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a paint: everyone loses it and stops showing it. False when there was none. Needs
    /// `cosmetics.manage`. Audited.
    async fn delete_paint(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::delete_paint(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Give an account a badge or paint, with a reason. Needs `cosmetics.grant`. Audited.
    async fn grant_cosmetic(&self, ctx: &Context<'_>, input: GrantCosmetic) -> Result<Entitlement> {
        let caller = caller(ctx)?;
        cosmetics::grant(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Take a badge or paint away; if they showed it, it stops. False when they didn't own it.
    /// Needs `cosmetics.grant`. Audited.
    async fn revoke_cosmetic(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        kind: CosmeticKind,
        cosmetic_id: String,
        reason: Option<String>,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::revoke(
            state(ctx),
            caller,
            &account_id,
            kind,
            &cosmetic_id,
            reason.as_deref(),
        )
        .await
        .map_err(api_error)
    }
}
