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
    /// A badge or paint with this name exists (names are unique per kind, ignoring case).
    #[error("a cosmetic with this name exists")]
    CosmeticNameTaken,
    /// A selection named a cosmetic the account doesn't own.
    #[error("the account doesn't own that cosmetic")]
    NotOwned,
    /// A new redeem code's hash exists already (practically never: codes carry 80 random bits).
    #[error("a redeem code with this hash exists")]
    CodeTaken,
    /// The buyer has a pending order for this item and recipient already.
    #[error("a pending order for this exists")]
    PendingOrder,
    /// Personal sets can't be private, and the global set is always listed.
    #[error("sets of this kind can't have that visibility")]
    InvalidVisibility,
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

/// Whether `err` is a deadlock (`40P01`) or a serialization failure (`40001`): Postgres gave up
/// on this statement so another could go on, and running it again normally succeeds.
pub fn is_conflict(err: &DbError) -> bool {
    let DbError::Sqlx(err) = err else {
        return false;
    };
    err.as_database_error()
        .and_then(|db| db.code())
        .is_some_and(|code| code == "40P01" || code == "40001")
}

/// How often [`retrying!`] runs its work in all.
pub(crate) const ATTEMPTS: u32 = 3;

/// Pins an async block's output to `DbResult`, so `?` inside it knows what to convert to.
pub(crate) fn work<T, F: Future<Output = DbResult<T>>>(f: F) -> F {
    f
}

/// `retrying!(conn, |tx| { ... })`: runs the block in a transaction of its own (a savepoint when
/// `conn` is in one already) and runs it again, up to [`ATTEMPTS`] times in all, when Postgres
/// aborts it as a deadlock or a serialization failure ([`is_conflict`]). Rolling back to the
/// savepoint releases the locks the block took, so the other side of the deadlock can finish. A
/// conflict on locks the caller took before (outside the savepoint) can come back; after the
/// last try it is returned. `?` inside the block ends that try, not the function.
///
/// A macro rather than a function taking a closure: an async closure over a borrowed connection
/// makes a future the compiler can't prove `Send` for every lifetime.
macro_rules! retrying {
    ($conn:expr, |$tx:ident| $body:block) => {{
        let mut attempt = 1;
        loop {
            let mut savepoint = sqlx::Connection::begin(&mut *$conn).await?;
            let result = $crate::error::work(async {
                let $tx: &mut sqlx::PgConnection = &mut savepoint;
                $body
            })
            .await;
            match result {
                Ok(value) => {
                    savepoint.commit().await?;
                    break Ok(value);
                }
                Err(err) if attempt < $crate::error::ATTEMPTS && $crate::error::is_conflict(&err) => {
                    savepoint.rollback().await?;
                    tracing::info!(attempt, error = %err, "set write conflicted; trying again");
                    attempt += 1;
                }
                // Rolled back now, not when the connection is next used (what dropping the
                // transaction does): until then its locks would stay held, and a caller that
                // keeps the connection would keep every other writer of the set waiting.
                Err(err) => {
                    let _ = savepoint.rollback().await;
                    break Err(err);
                }
            }
        }
    }};
}
pub(crate) use retrying;
