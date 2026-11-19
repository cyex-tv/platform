//! Permission names (ADR 0010). Handlers name the one they need; roles hold sets of them.

pub const EMOTES_REVIEW: &str = "emotes.review";
pub const EMOTES_TAKEDOWN: &str = "emotes.takedown";
pub const EMOTES_EDIT_ANY: &str = "emotes.edit_any";
pub const SETS_EDIT_ANY: &str = "sets.edit_any";
pub const GLOBAL_SET_EDIT: &str = "global_set.edit";
pub const USERS_VIEW: &str = "users.view";
pub const USERS_RESTRICT: &str = "users.restrict";
pub const ROLES_MANAGE: &str = "roles.manage";
pub const AUDIT_VIEW: &str = "audit.view";
/// Create, change and delete chat badges, paints and their images (ADR 0015).
pub const COSMETICS_MANAGE: &str = "cosmetics.manage";
/// Grant and revoke cosmetics, and read who owns what (ADR 0015).
pub const COSMETICS_GRANT: &str = "cosmetics.grant";
/// Create, change and delete shop items (ADR 0016).
pub const SHOP_MANAGE: &str = "shop.manage";
/// See every order and refund them (ADR 0016).
pub const SHOP_ORDERS: &str = "shop.orders";
/// Make and disable redeem code batches (ADR 0016).
pub const CODES_MANAGE: &str = "codes.manage";

/// Every permission that exists. A role can only hold names from this list.
pub const ALL: &[&str] = &[
    EMOTES_REVIEW,
    EMOTES_TAKEDOWN,
    EMOTES_EDIT_ANY,
    SETS_EDIT_ANY,
    GLOBAL_SET_EDIT,
    USERS_VIEW,
    USERS_RESTRICT,
    ROLES_MANAGE,
    AUDIT_VIEW,
    COSMETICS_MANAGE,
    COSMETICS_GRANT,
    SHOP_MANAGE,
    SHOP_ORDERS,
    CODES_MANAGE,
];

pub fn is_known(name: &str) -> bool {
    ALL.contains(&name)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn names_are_unique_and_namespaced() {
        let unique: HashSet<_> = ALL.iter().collect();
        assert_eq!(unique.len(), ALL.len());
        for name in ALL {
            let (area, action) = name.split_once('.').unwrap();
            assert!(!area.is_empty() && !action.is_empty(), "{name}");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '.' || c == '_'),
                "{name}"
            );
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert!(is_known("emotes.review"));
        assert!(!is_known("emotes.everything"));
        assert!(!is_known(""));
    }
}
