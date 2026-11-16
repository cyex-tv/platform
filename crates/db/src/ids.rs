//! IDs are ULIDs (ADR 0002): sortable by time, stored as `uuid`, written as 26-character text in
//! the API.

use ulid::Ulid;
use uuid::Uuid;

/// A new ID for any table except the outbox.
pub fn new_id() -> Uuid {
    Ulid::new().into()
}

/// The API's text form.
pub fn to_text(id: Uuid) -> String {
    Ulid::from(id).to_string()
}

/// Parses the API's text form. Case-insensitive, as ULIDs are.
pub fn from_text(text: &str) -> Option<Uuid> {
    Ulid::from_string(text).ok().map(Uuid::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips() {
        let id = new_id();
        let text = to_text(id);
        assert_eq!(text.len(), 26);
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
}
