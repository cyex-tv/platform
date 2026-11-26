//! GraphQL: editors.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    AccountEditor, AccountSummary, AddAccountEditor, Channel, EditableChannel, EditedAccount,
    EditorPermission, EmoteSet, EmoteTransferRequest, Platform,
};

use super::{api_error, caller, state};
use crate::{catalog, editors, transfers};

#[derive(Default)]
pub struct EditorQuery;

#[Object]
impl EditorQuery {
    /// A set's editors. For its owner, its editors and `sets.edit_any`.
    async fn set_editors(&self, ctx: &Context<'_>, set_id: String) -> Result<Vec<AccountSummary>> {
        let caller = caller(ctx)?;
        catalog::set_editors(state(ctx), caller, &set_id)
            .await
            .map_err(api_error)
    }

    /// Your channels: one per linked platform you have signed in with.
    async fn my_channels(&self, ctx: &Context<'_>) -> Result<Vec<Channel>> {
        let caller = caller(ctx)?;
        catalog::my_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Channels you can manage: yours (`OWNER`), those of accounts you are an account editor of
    /// (`ACCOUNT_EDITOR`), and those of accounts where you edit at least one set (`SET_EDITOR`).
    /// Each channel once, under your highest role; sorted by role (`OWNER`, `ACCOUNT_EDITOR`,
    /// `SET_EDITOR`), then login (ignoring case), then platform. Signed-in only.
    async fn my_editable_channels(&self, ctx: &Context<'_>) -> Result<Vec<EditableChannel>> {
        let caller = caller(ctx)?;
        editors::my_editable_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// A channel, by its platform ID (`id`) or its login (`login`; one of the two), with the sets
    /// of its account you may add, rename and remove emotes in, your role on it and whether its
    /// active set is among them. The same rights check as `addSetEmote`, `updateSetEmote` and
    /// `removeSetEmote` (owner, account editors with `EMOTES` covering the set, set editors,
    /// `sets.edit_any`). Null when the channel is unknown. Signed-in only.
    async fn channel_editable_sets(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: Option<String>,
        login: Option<String>,
    ) -> Result<Option<cyex_tv_shared::model::ChannelEditableSets>> {
        let caller = caller(ctx)?;
        catalog::channel_editable_sets(
            state(ctx),
            caller,
            platform,
            id.as_deref(),
            login.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// An account's editors (ADR 0014): yours, or, with `accountId`, those of an account you edit
    /// with the `EDITORS` permission, or anyone's with `editors.manage_any`.
    async fn account_editors(
        &self,
        ctx: &Context<'_>,
        account_id: Option<String>,
    ) -> Result<Vec<AccountEditor>> {
        let caller = caller(ctx)?;
        editors::account_editors(state(ctx), caller, account_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// Open emote transfer requests you take part in (ADR 0014). Editors can't ask for a transfer
    /// any more (amendment 2026/11/22) and the open ones were withdrawn, so this is empty.
    #[graphql(
        deprecation = "Retired: editors can't ask for a transfer any more; only an emote's owner gives it away."
    )]
    async fn emote_transfer_requests(
        &self,
        ctx: &Context<'_>,
        emote_id: Option<String>,
    ) -> Result<Vec<EmoteTransferRequest>> {
        let caller = caller(ctx)?;
        transfers::open_requests(state(ctx), caller, emote_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// The accounts you edit, with what you may do on each.
    async fn editor_of(&self, ctx: &Context<'_>) -> Result<Vec<EditedAccount>> {
        let caller = caller(ctx)?;
        editors::editor_of(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// The sets of the accounts you edit. Each set names its owner.
    async fn editor_sets(&self, ctx: &Context<'_>) -> Result<Vec<EmoteSet>> {
        let caller = caller(ctx)?;
        editors::editor_sets(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// The channels of the accounts you edit.
    async fn editor_channels(&self, ctx: &Context<'_>) -> Result<Vec<Channel>> {
        let caller = caller(ctx)?;
        editors::editor_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct EditorMutation;

#[Object]
impl EditorMutation {
    /// Make someone an editor of your account, of an account you edit with `EDITORS` (ADR
    /// 0014), or of anyone's with `editors.manage_any` (audited with `via`). Give their account ID, or a platform and login. You can only grant permissions you
    /// hold. `setIds` limits them to some of the account's sets; leave it out for every set, new
    /// ones included. At most 50 editors per account (`too_many_editors`). Audited.
    async fn add_account_editor(
        &self,
        ctx: &Context<'_>,
        input: AddAccountEditor,
    ) -> Result<AccountEditor> {
        let caller = caller(ctx)?;
        editors::add(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change an account editor's permissions. `ownerId` is the account edited; empty for yours.
    /// The owner, an editor with `EDITORS`, or `editors.manage_any`.
    /// `setIds` changes which sets they work on: empty for every set, left out to keep them.
    /// Audited.
    async fn update_account_editor(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
        account_id: String,
        permissions: Vec<EditorPermission>,
        set_ids: Option<Vec<String>>,
    ) -> Result<AccountEditor> {
        let caller = caller(ctx)?;
        editors::update(
            state(ctx),
            caller,
            owner_id.as_deref(),
            &account_id,
            &permissions,
            set_ids.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Remove an account editor: the owner, an editor with `EDITORS`, `editors.manage_any`, or the editor themselves
    /// (with `ownerId` set to the account they leave). False when they weren't one. Audited.
    async fn remove_account_editor(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
        account_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        editors::remove(state(ctx), caller, owner_id.as_deref(), &account_id)
            .await
            .map_err(api_error)
    }
}
