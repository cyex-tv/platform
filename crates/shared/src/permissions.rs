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

// Finer permissions (ADR 0010, amendment 2026/11/19). Each one is a narrower way to do part of
// what an older, broader permission does; the broad one keeps working.

/// Give and take away roles below your own, without editing roles (`roles.manage` does both).
pub const ROLES_ASSIGN: &str = "roles.assign";
/// See and resolve reports (`emotes.review` and `emotes.takedown` still see them too).
pub const REPORTS_MANAGE: &str = "reports.manage";
/// Stop an account from uploading, for a time or for good (part of `users.restrict`).
pub const USERS_RESTRICT_UPLOADS: &str = "users.restrict_uploads";
/// Stop an account from filing reports and tickets (part of `users.restrict`).
pub const USERS_RESTRICT_TICKETS: &str = "users.restrict_tickets";
/// Block an account outright: no sign-in, sessions ended (part of `users.restrict`).
pub const USERS_BLOCK: &str = "users.block";
/// See where an account is signed in.
pub const USERS_SESSIONS: &str = "users.sessions";
/// Read and write staff notes on accounts.
pub const USERS_NOTES: &str = "users.notes";
/// See how an account's cosmetics and roles were given (the entitlement graph).
pub const ENTITLEMENTS_VIEW: &str = "entitlements.view";
/// See an account's orders and payments (`shop.orders` also refunds).
pub const BILLING_VIEW: &str = "billing.view";
/// See and change subscriptions.
pub const SUBSCRIPTIONS_MANAGE: &str = "subscriptions.manage";
/// Delete any account's emote, as its owner can (`emotes.edit_any` already renames, retags and
/// changes the flags and visibility of anyone's emotes). Audited.
pub const EMOTES_DELETE_ANY: &str = "emotes.delete_any";
/// Add, change and remove any account's editors on its behalf, as its owner can. Audited.
pub const EDITORS_MANAGE_ANY: &str = "editors.manage_any";

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
    ROLES_ASSIGN,
    REPORTS_MANAGE,
    USERS_RESTRICT_UPLOADS,
    USERS_RESTRICT_TICKETS,
    USERS_BLOCK,
    USERS_SESSIONS,
    USERS_NOTES,
    ENTITLEMENTS_VIEW,
    BILLING_VIEW,
    SUBSCRIPTIONS_MANAGE,
    EMOTES_DELETE_ANY,
    EDITORS_MANAGE_ANY,
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
