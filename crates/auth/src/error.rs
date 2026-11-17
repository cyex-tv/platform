use axum::http::StatusCode;
use cyex_tv_db::DbError;
use cyex_tv_shared::error::ApiError;

/// Why a sign-in or a session check failed. Each has a stable code for clients.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("unknown platform")]
    UnknownPlatform,
    #[error("sign-in with this platform isn't set up")]
    NotConfigured,
    #[error("return_to must be a cyex.tv URL")]
    BadReturnTo,
    #[error("not signed in")]
    SignedOut,
    #[error("bad or expired state")]
    BadState,
    #[error("the user declined")]
    Declined,
    #[error("the platform answered badly: {0}")]
    Platform(String),
    #[error("identity linked to another account")]
    IdentityTaken,
    #[error("platform already linked")]
    PlatformAlreadyLinked,
    #[error("the last identity can't be unlinked")]
    LastIdentity,
    #[error("account deleted")]
    AccountDeleted,
    #[error("account blocked")]
    AccountBlocked,
    #[error("Origin not allowed")]
    BadOrigin,
    #[error(transparent)]
    Db(DbError),
}

impl From<DbError> for AuthError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::IdentityTaken => Self::IdentityTaken,
            DbError::PlatformAlreadyLinked => Self::PlatformAlreadyLinked,
            DbError::LastIdentity => Self::LastIdentity,
            other => Self::Db(other),
        }
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err.into())
    }
}

impl AuthError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::UnknownPlatform | Self::BadReturnTo | Self::BadState | Self::Declined => {
                StatusCode::BAD_REQUEST
            }
            Self::SignedOut => StatusCode::UNAUTHORIZED,
            Self::AccountDeleted | Self::AccountBlocked | Self::BadOrigin => StatusCode::FORBIDDEN,
            Self::IdentityTaken | Self::PlatformAlreadyLinked | Self::LastIdentity => {
                StatusCode::CONFLICT
            }
            Self::Platform(_) => StatusCode::BAD_GATEWAY,
            Self::NotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownPlatform => "unknown_platform",
            Self::NotConfigured => "unavailable",
            Self::BadReturnTo => "bad_return_to",
            Self::SignedOut => "signed_out",
            Self::BadState => "bad_state",
            Self::Declined => "declined",
            Self::Platform(_) => "platform_error",
            Self::IdentityTaken => "identity_taken",
            Self::PlatformAlreadyLinked => "platform_already_linked",
            Self::LastIdentity => "last_identity",
            Self::AccountDeleted => "account_deleted",
            Self::AccountBlocked => "account_blocked",
            Self::BadOrigin => "bad_origin",
            Self::Db(_) => "internal",
        }
    }

    /// Text for people. Never includes what the platform or the database said.
    pub fn message(&self) -> &'static str {
        match self {
            Self::UnknownPlatform => "The platform must be twitch or kick.",
            Self::NotConfigured => "Signing in with this platform isn't available right now.",
            Self::BadReturnTo => "return_to must be a cyex.tv URL.",
            Self::SignedOut => "Sign in to use this.",
            Self::BadState => "This sign-in expired or was already used. Start again.",
            Self::Declined => "Sign-in was cancelled on the platform.",
            Self::Platform(_) => "The platform didn't answer as expected. Try again.",
            Self::IdentityTaken => "That account is already linked to another cyex.tv account.",
            Self::PlatformAlreadyLinked => "This platform is already linked to your account.",
            Self::LastIdentity => "Your last linked account can't be removed.",
            Self::AccountDeleted => "This cyex.tv account was deleted.",
            Self::AccountBlocked => "This cyex.tv account is blocked.",
            Self::BadOrigin => "Requests with the session cookie must come from cyex.tv.",
            Self::Db(_) => "Something went wrong on our side.",
        }
    }
}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        match &err {
            AuthError::Db(inner) => tracing::error!(error = %inner, "auth database error"),
            AuthError::Platform(detail) => tracing::warn!(%detail, "platform error during sign-in"),
            _ => {}
        }
        ApiError::new(err.status(), err.code(), err.message())
    }
}
