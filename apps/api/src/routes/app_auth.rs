//! App sign-in (ADR 0021): `/v1/auth/app/*`, the authorization code flow with PKCE and a loopback
//! redirect for desktop apps such as cyex deck. The logic is in `cyex_tv_auth::app`.

use axum::Json;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, LOCATION, PRAGMA, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use cyex_tv_auth::app::{self, AppAuthError, Authorize, AuthorizeRequest, TokenRequest};
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::{AppState, MaybeCaller};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(authorize))
        .routes(routes!(get_client))
        .routes(routes!(consent))
        .routes(routes!(token))
        .routes(routes!(revoke))
}

/// The authorize request (RFC 6749 4.1.1, RFC 7636 4.3).
#[derive(Clone, Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AuthorizeQuery {
    /// `code`.
    pub response_type: Option<String>,
    /// `cyex-deck`.
    pub client_id: Option<String>,
    /// `http://127.0.0.1:<port>/callback` or `http://[::1]:<port>/callback`; any port.
    pub redirect_uri: Option<String>,
    /// The S256 challenge: base64url(sha256(verifier)), 43 characters.
    pub code_challenge: Option<String>,
    /// `S256` (`plain` is refused).
    pub code_challenge_method: Option<String>,
    /// Echoed back to the redirect; 1 to 512 characters.
    pub state: Option<String>,
}

impl AuthorizeQuery {
    fn as_request(&self) -> AuthorizeRequest<'_> {
        AuthorizeRequest {
            response_type: self.response_type.as_deref(),
            client_id: self.client_id.as_deref(),
            redirect_uri: self.redirect_uri.as_deref(),
            code_challenge: self.code_challenge.as_deref(),
            code_challenge_method: self.code_challenge_method.as_deref(),
            state: self.state.as_deref(),
        }
    }
}

fn api_error(err: AppAuthError) -> ApiError {
    if let AppAuthError::Auth(inner) = err {
        return inner.into();
    }
    let status = match err {
        AppAuthError::NeedsSiteSession => StatusCode::UNAUTHORIZED,
        _ => StatusCode::BAD_REQUEST,
    };
    ApiError::new(status, err.api_code(), err.message())
}

fn see_other(location: &str) -> ApiResult<Response> {
    let mut response = StatusCode::SEE_OTHER.into_response();
    let headers = response.headers_mut();
    headers.insert(
        LOCATION,
        HeaderValue::from_str(location).map_err(|_| ApiError::internal())?,
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

/// Start an app sign-in (open this in the browser).
///
/// Checks the client and the loopback `redirect_uri`, then sends the browser to the cyex.tv
/// consent page, where the user signs in with Twitch or Kick if needed and allows or denies.
/// A bad client or redirect answers 400 here and is never redirected to; other mistakes go back
/// to the redirect as `error=invalid_request` with `state`.
#[utoipa::path(
    get,
    path = "/v1/auth/app/authorize",
    tag = "auth",
    params(AuthorizeQuery),
    responses(
        (status = 303, description = "To the consent page on cyex.tv, or back to `redirect_uri` with `error=invalid_request`."),
        (status = 400, description = "`unknown_client` or `bad_redirect_uri`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody)
    )
)]
async fn authorize(
    State(state): State<AppState>,
    Query(query): Query<AuthorizeQuery>,
) -> ApiResult<Response> {
    match state
        .auth
        .app_authorize(&query.as_request())
        .map_err(api_error)?
    {
        Authorize::Consent(location) | Authorize::Back(location) => see_other(&location),
    }
}

/// A registered app and what it asks for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppClientInfo {
    pub client_id: String,
    pub name: String,
    pub scopes: Vec<AppScopeInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AppScopeInfo {
    pub id: String,
    /// For people: "Manage emotes in sets you can edit".
    pub description: String,
}

/// A registered app: its name and scopes, for the consent page.
#[utoipa::path(
    get,
    path = "/v1/auth/app/clients/{client_id}",
    tag = "auth",
    params(("client_id" = String, Path, description = "`cyex-deck`.")),
    responses(
        (status = 200, description = "The app.", body = AppClientInfo),
        (status = 404, description = "`unknown_client`.", body = ErrorBody)
    )
)]
async fn get_client(Path(id): Path<String>) -> ApiResult<Json<AppClientInfo>> {
    let client = app::client(&id).ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "unknown_client",
            "Unknown client_id.",
        )
    })?;
    Ok(Json(AppClientInfo {
        client_id: client.id.to_owned(),
        name: client.name.to_owned(),
        scopes: client
            .scopes
            .iter()
            .map(|s| AppScopeInfo {
                id: s.id.to_owned(),
                description: s.description.to_owned(),
            })
            .collect(),
    }))
}

/// The consent page's answer.
#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsentBody {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub state: Option<String>,
    /// `true` for Allow, `false` for Deny.
    pub allow: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConsentAnswer {
    /// Where to send the browser: the app's loopback address with `code` and `state`, or with
    /// `error=access_denied`.
    pub location: String,
}

/// Allow or deny an app (the consent page).
///
/// Site cookie only, from `https://cyex.tv` (`Origin`); an app token can't consent. Every field of
/// the authorize request is checked again.
#[utoipa::path(
    post,
    path = "/v1/auth/app/consent",
    tag = "auth",
    request_body = ConsentBody,
    responses(
        (status = 200, description = "Where to send the browser.", body = ConsentAnswer),
        (status = 400, description = "`unknown_client` or `bad_redirect_uri`.", body = ErrorBody),
        (status = 401, description = "`signed_out`: no site session.", body = ErrorBody),
        (status = 403, description = "`bad_origin`.", body = ErrorBody)
    )
)]
async fn consent(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    Json(body): Json<ConsentBody>,
) -> ApiResult<Response> {
    let req = AuthorizeRequest {
        response_type: Some("code"),
        client_id: body.client_id.as_deref(),
        redirect_uri: body.redirect_uri.as_deref(),
        code_challenge: body.code_challenge.as_deref(),
        code_challenge_method: body.code_challenge_method.as_deref(),
        state: body.state.as_deref(),
    };
    let location = state
        .auth
        .app_consent(&state.db, caller.as_ref(), &req, body.allow)
        .await
        .map_err(api_error)?;
    let mut response = Json(ConsentAnswer { location }).into_response();
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

/// The token request (RFC 6749 4.1.3 and 6), `application/x-www-form-urlencoded`.
#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct TokenForm {
    /// `authorization_code` or `refresh_token`.
    pub grant_type: Option<String>,
    /// `cyex-deck`.
    pub client_id: Option<String>,
    /// With `authorization_code`.
    pub code: Option<String>,
    /// With `authorization_code`: the PKCE verifier.
    pub code_verifier: Option<String>,
    /// With `authorization_code`: the same `redirect_uri` as at authorize.
    pub redirect_uri: Option<String>,
    /// With `refresh_token`.
    pub refresh_token: Option<String>,
}

/// A token answer (RFC 6749 5.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TokenAnswer {
    /// Send as `Authorization: Bearer <access_token>`.
    pub access_token: String,
    /// `Bearer`.
    pub token_type: String,
    /// Seconds: 3600.
    pub expires_in: i64,
    /// Single use: each refresh answers a new one. Presenting a used one signs the session out.
    pub refresh_token: String,
    /// Space-separated: `sets.emotes cosmetics.select`.
    pub scope: String,
}

/// An OAuth error answer (RFC 6749 5.2); `message` repeats the description, as elsewhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct OAuthErrorBody {
    /// `invalid_request`, `invalid_client`, `invalid_grant`, `unsupported_grant_type`,
    /// `server_error` (or `rate_limited`).
    pub error: String,
    pub error_description: String,
    pub message: String,
}

fn no_store(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

fn oauth_error(err: &AppAuthError) -> Response {
    let status = match err {
        AppAuthError::UnknownClient => StatusCode::UNAUTHORIZED,
        AppAuthError::Auth(cyex_tv_auth::AuthError::Db(inner)) => {
            tracing::error!(error = %inner, "app token database error");
            StatusCode::INTERNAL_SERVER_ERROR
        }
        _ => StatusCode::BAD_REQUEST,
    };
    let message = err.message();
    no_store(
        (
            status,
            Json(OAuthErrorBody {
                error: err.oauth_code().to_owned(),
                error_description: message.clone(),
                message,
            }),
        )
            .into_response(),
    )
}

/// Trade a code for tokens, or refresh.
///
/// `grant_type=authorization_code` with `code`, `code_verifier`, `redirect_uri` and `client_id`;
/// or `grant_type=refresh_token` with `refresh_token` and `client_id`. No client secret: the app
/// is a public client. A code works once, within 2 minutes; a wrong verifier uses it up. A refresh
/// token works once; presenting a used one signs the session out.
#[utoipa::path(
    post,
    path = "/v1/auth/app/token",
    tag = "auth",
    request_body(content = TokenForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Tokens.", body = TokenAnswer),
        (status = 400, description = "`invalid_request`, `invalid_grant` or `unsupported_grant_type`.", body = OAuthErrorBody),
        (status = 401, description = "`invalid_client`: unknown `client_id`.", body = OAuthErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody)
    )
)]
async fn token(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: Result<Form<TokenForm>, FormRejection>,
) -> Response {
    let Ok(Form(form)) = form else {
        return oauth_error(&AppAuthError::InvalidRequest(
            "Send the token request as application/x-www-form-urlencoded.",
        ));
    };
    let req = TokenRequest {
        grant_type: form.grant_type.as_deref(),
        client_id: form.client_id.as_deref(),
        code: form.code.as_deref(),
        code_verifier: form.code_verifier.as_deref(),
        redirect_uri: form.redirect_uri.as_deref(),
        refresh_token: form.refresh_token.as_deref(),
        user_agent: headers.get(USER_AGENT).and_then(|v| v.to_str().ok()),
    };
    match state.auth.app_token(&state.db, &req).await {
        Ok(tokens) => no_store(
            Json(TokenAnswer {
                access_token: tokens.access_token,
                token_type: "Bearer".to_owned(),
                expires_in: tokens.expires_in,
                refresh_token: tokens.refresh_token,
                scope: tokens.scope,
            })
            .into_response(),
        ),
        Err(err) => oauth_error(&err),
    }
}

/// The revoke request (RFC 7009).
#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
pub struct RevokeForm {
    /// An access or refresh token of the session to sign out.
    pub token: Option<String>,
    pub token_type_hint: Option<String>,
    pub client_id: Option<String>,
}

/// Sign an app out by one of its tokens (RFC 7009).
///
/// Ends the app session the access or refresh token belongs to, refresh tokens included. Answers
/// 200 whether or not the token was known. `POST /v1/auth/logout` with the Bearer token does the
/// same while the access token is still good.
#[utoipa::path(
    post,
    path = "/v1/auth/app/revoke",
    tag = "auth",
    request_body(content = RevokeForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Signed out (or nothing to sign out)."),
        (status = 400, description = "`invalid_request`: no form.", body = OAuthErrorBody)
    )
)]
async fn revoke(
    State(state): State<AppState>,
    form: Result<Form<RevokeForm>, FormRejection>,
) -> Response {
    let Ok(Form(form)) = form else {
        return oauth_error(&AppAuthError::InvalidRequest(
            "Send the revoke request as application/x-www-form-urlencoded.",
        ));
    };
    match state
        .auth
        .app_revoke(&state.db, form.token.as_deref().unwrap_or_default())
        .await
    {
        Ok(()) => no_store(StatusCode::OK.into_response()),
        Err(err) => oauth_error(&err),
    }
}
