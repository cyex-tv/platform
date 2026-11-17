//! Errors from the query modules. Rule violations get their own variant, so the api can answer
//! with a named error code instead of a generic failure.

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("not found")]
    NotFound,
    /// The Twitch or Kick account is already linked to another cyex.tv account.
    #[error("identity is linked to another account")]
    IdentityTaken,
    /// The account already has an identity on that platform.
    #[error("account already has an identity on this platform")]
    PlatformAlreadyLinked,
    #[error("the last identity can't be unlinked")]
    LastIdentity,
    #[error("the account already has a personal set")]
    PersonalSetExists,
    /// The global set is made once, by `sets::ensure_global`.
    #[error("sets of this kind can't be created here")]
    InvalidSetKind,
    #[error("the set is full")]
    SetFull,
    #[error("the capacity is below the number of emotes in the set")]
    CapacityTooLow,
    #[error("the name is already used in this set")]
    NameTaken,
    #[error("the emote is already in this set")]
    AlreadyInSet,
    /// Pending emotes only go in their owner's own sets, and personal sets take approved ones only.
    #[error("the emote isn't approved")]
    NotApproved,
    /// Rejected, taken down or deleted.
    #[error("the emote can't be used")]
    EmoteUnavailable,
    #[error("a role with this name exists")]
    RoleNameTaken,
    #[error("unknown permission: {0}")]
    UnknownPermission(String),
    /// The account has `editors::MAX_PER_ACCOUNT` editors already.
    #[error("the account has the most editors it can have")]
    TooManyEditors,
    #[error("the account is an editor already")]
    AlreadyEditor,
}

pub type DbResult<T> = Result<T, DbError>;

/// The constraint a unique violation hit, for mapping it to a named error.
pub(crate) fn unique_violation(err: &sqlx::Error) -> Option<&str> {
    let db = err.as_database_error()?;
    if db.is_unique_violation() {
        db.constraint()
    } else {
        None
    }
}
