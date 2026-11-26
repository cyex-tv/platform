//! GraphQL: roles.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{CreateRole, Role, UpdateRole};
use cyex_tv_shared::model::{PermissionDecision, RoleLimit, RoleLimitInput, UploadQuota};

use super::{api_error, caller, state};
use crate::admin;

#[derive(Default)]
pub struct AccountRoleQuery;

#[Object]
impl AccountRoleQuery {
    /// Every permission an account holds or doesn't, and the role that decided each: its most
    /// senior role that allows or denies it (Deny wins at equal positions), the User role last.
    /// Yours, or anyone's with `roles.manage`, `roles.assign` or `users.view`.
    async fn account_permissions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<PermissionDecision>> {
        let caller = caller(ctx)?;
        admin::account_permissions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// An account's roles. Yours, or anyone's with `roles.manage` or `users.view`.
    async fn account_roles(&self, ctx: &Context<'_>, account_id: String) -> Result<Vec<Role>> {
        let caller = caller(ctx)?;
        admin::account_roles(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct RoleQuery;

#[Object]
impl RoleQuery {
    /// All roles, most senior first, each with whether you may edit or assign it. Needs
    /// `roles.manage` or `roles.assign`. The top role is listed to the owners only.
    async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<Role>> {
        let caller = caller(ctx)?;
        admin::list(state(ctx), caller).await.map_err(api_error)
    }

    /// Every per-account limit a role can change, with what everyone gets without one. Needs
    /// `roles.manage`.
    async fn limit_defaults(&self, ctx: &Context<'_>) -> Result<Vec<RoleLimit>> {
        let caller = caller(ctx)?;
        admin::limit_defaults(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// A role's limits: each per-account limit with its default and the role's override. An
    /// account counts under the most generous override among its roles. Needs `roles.manage`.
    async fn role_limits(&self, ctx: &Context<'_>, id: String) -> Result<Vec<RoleLimit>> {
        let caller = caller(ctx)?;
        admin::role_limits(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Where uploads stand: how many emotes wait for review and how many may, and the uploads
    /// this hour. Yours, or with `ownerId` an account you may upload for.
    async fn upload_quota(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
    ) -> Result<UploadQuota> {
        let caller = caller(ctx)?;
        crate::upload::quota(state(ctx), caller, owner_id.as_deref())
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct RoleMutation;

#[Object]
impl RoleMutation {
    /// Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited.
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::create(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change a role below your own highest one that you don't hold. The top role can't be
    /// changed; the User role keeps position 0. Needs `roles.manage`. Audited.
    async fn update_role(&self, ctx: &Context<'_>, id: String, input: UpdateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::update(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Set a role's limits: the overrides listed (a null value or a limit left out keeps the
    /// default), for everyone who holds it. Only on a role you may edit, not the User or top
    /// role. Needs `roles.manage`. Audited.
    async fn set_role_limits(
        &self,
        ctx: &Context<'_>,
        id: String,
        limits: Vec<RoleLimitInput>,
    ) -> Result<Vec<RoleLimit>> {
        let caller = caller(ctx)?;
        admin::set_limits(state(ctx), caller, &id, &limits)
            .await
            .map_err(api_error)
    }

    /// Delete a role below your own highest one that you don't hold; built-in roles stay.
    /// Needs `roles.manage`. Audited.
    async fn delete_role(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::delete(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Give an account a role below your highest one that grants only what you hold. Needs
    /// `roles.manage` or `roles.assign`; the User and top roles can't be given. Audited.
    async fn assign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::assign(state(ctx), caller, &account_id, &role_id)
            .await
            .map_err(api_error)
    }

    /// Take a role from an account. The owner and the last `roles.manage` holder are protected.
    /// Needs `roles.manage` or `roles.assign`. Audited.
    async fn unassign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::unassign(state(ctx), caller, &account_id, &role_id)
            .await
            .map_err(api_error)
    }
}
