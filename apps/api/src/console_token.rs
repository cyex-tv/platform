//! Console tokens (ADR 0028): a personal Bearer token a signed-in person makes on the website, so
//! they can use the API as themselves (a GraphQL playground, a script, moving things between their
//! own accounts). The website's own session stays an `HttpOnly` cookie that no script can read;
//! this is the one way to hold a token, and only by asking for it.
//!
//! - Made only with the website's cookie session (and its `Origin`, checked by `MaybeCaller`):
//!   a Bearer token, a console token included, can't make another (`website_only`).
//! - One per account: making one signs the last one out, in the same transaction.
//! - A `console` session: 3 days from when it was made, never slid (ADR 0029; the database holds
//!   it to that); listed in `mySessions` as `Console token`, and ended there, by signing out any
//!   session or everywhere (ADR 0029), deletion and blocking.
//! - It acts with the account's own rights: its roles count (staff ones too), the owner's
//!   all-permissions don't (owner-only work stays on the website), and website-only actions
//!   (deleting the account, the data copy, signing other sessions out) stay website only.
//! - 5 an hour per account (`console_tokens`), on top of the `/v1/auth/*` limit per IP. Audited
//!   (`account.console_token_created`), never with the token.

use axum::http::StatusCode;
use cyex_tv_auth::session::ttl;
use cyex_tv_auth::{Caller, Via, tokens};
use cyex_tv_db::ids;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::ConsoleToken;
use serde_json::json;
use sqlx::Connection;

use crate::errors::{Audit, db_error, limited, record_audit, sqlx_error};
use crate::state::AppState;
use crate::views;

/// The audit action for a new console token.
pub const AUDIT_ACTION: &str = "account.console_token_created";

/// Makes a console token for the caller, signing out the one they had. `user_agent` is kept like
/// any session's, though the token is named `Console token` wherever it is used.
pub async fn mint(
    state: &AppState,
    caller: &Caller,
    user_agent: Option<&str>,
) -> ApiResult<ConsoleToken> {
    if caller.via != Via::Cookie || caller.kind != SessionKind::Site {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "website_only",
            "Make an API token on cyex.tv, signed in there.",
        ));
    }
    let account = ids::to_text(caller.account_id);
    limited(
        &state.limits.console_tokens,
        &account,
        "You made a few API tokens already. Try again in an hour.",
    )?;

    let token = tokens::random_token();
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (session, replaced) = sessions::replace_console(
        &mut tx,
        &tokens::hash(&token),
        caller.account_id,
        ttl(SessionKind::Console),
        user_agent,
    )
    .await
    .map_err(db_error)?;
    let session_id = ids::to_text(session.id);
    let expires_at = views::time(session.expires_at);
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(AUDIT_ACTION, "account", &account).change(
            replaced.map(|id| json!({ "session_id": ids::to_text(id) })),
            Some(json!({ "session_id": session_id, "expires_at": expires_at })),
        ),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;

    Ok(ConsoleToken {
        token,
        token_type: "Bearer".to_owned(),
        expires_at,
        session_id,
    })
}
