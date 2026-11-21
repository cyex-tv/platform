//! IDs are ULIDs (ADR 0002): sortable by time, stored as `uuid`, written as 26-character text in
//! the API.
//!
//! Accounts, emotes and emote sets also have a Snowflake `public_id` (ADR 0027), which the API
//! shows instead of their ULID and takes as well as it. [`from_text`] reads both forms: a ULID
//! directly, a Snowflake through the IDs [`crate::public_ids::resolving`] looked up for the
//! request it runs in.

use std::collections::HashMap;
use std::sync::Mutex;

use ulid::{Generator, Ulid};
use uuid::Uuid;

/// IDs made in this process strictly increase, even within one millisecond, so "newest first"
/// by ID (the audit log and its paging) matches the order things happened in.
static GENERATOR: Mutex<Generator> = Mutex::new(Generator::new());

tokio::task_local! {
    /// The Snowflakes of the current request that name an account, emote or set, and their
    /// rows. Set by `public_ids::resolving`.
    pub(crate) static RESOLVED: HashMap<i64, Uuid>;
}

/// A new ID for any table except the outbox.
pub fn new_id() -> Uuid {
    let next = GENERATOR
        .lock()
        .ok()
        .and_then(|mut generator| generator.generate().ok());
    match next {
        Some(id) => id.into(),
        // `generate` fails only after 2^80 IDs in one millisecond; a fresh ULID is fine then.
        // (Not `unwrap_or_default`: `Ulid::default()` is not a fresh ID.)
        None => Ulid::new().into(),
    }
}

/// The ULID text form. For accounts, emotes and sets the API shows their Snowflake instead; the
/// edges translate (`public_ids`).
pub fn to_text(id: Uuid) -> String {
    Ulid::from(id).to_string()
}

/// Parses an ID from the API: a ULID (case-insensitive, as ULIDs are), or the Snowflake of an
/// account, emote or set that the current request resolved. A Snowflake outside such a request,
/// or one that names nothing, is `None`, as a malformed ID is.
pub fn from_text(text: &str) -> Option<Uuid> {
    if let Some(public) = crate::public_ids::parse(text) {
        return RESOLVED
            .try_with(|resolved| resolved.get(&public).copied())
            .ok()
            .flatten();
    }
    Ulid::from_string(text).ok().map(Uuid::from)
}

/// Whether `text` is in the ULID form (26 Crockford base32 characters).
pub fn is_ulid(text: &str) -> bool {
    text.len() == 26 && Ulid::from_string(text).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_increase_within_a_millisecond() {
        let ids: Vec<Uuid> = (0..1000).map(|_| new_id()).collect();
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn text_round_trips() {
        let id = new_id();
        let text = to_text(id);
        assert_eq!(text.len(), 26);
        assert!(is_ulid(&text));
        assert_eq!(from_text(&text), Some(id));
        assert_eq!(from_text(&text.to_lowercase()), Some(id));
        assert_eq!(from_text("not-an-id"), None);
        assert_eq!(from_text(""), None);
    }

    #[test]
    fn ids_sort_by_creation() {
        let a = new_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = new_id();
        assert!(a < b);
    }

    #[tokio::test]
    async fn snowflakes_read_only_what_the_request_resolved() {
        let id = new_id();
        // Outside a request: nothing.
        assert_eq!(from_text("1314628736410763264"), None);
        let resolved = HashMap::from([(1_314_628_736_410_763_264, id)]);
        RESOLVED
            .scope(resolved, async {
                assert_eq!(from_text("1314628736410763264"), Some(id));
                // Unknown, or not a Snowflake at all.
                assert_eq!(from_text("1314628736410763265"), None);
                assert_eq!(from_text("-1"), None);
                assert_eq!(from_text("99999999999999999999"), None);
                // ULIDs still parse as before.
                assert_eq!(from_text(&to_text(id)), Some(id));
            })
            .await;
    }
}
