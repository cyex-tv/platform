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
/// Change any emote's flags after review (ADR 0024): its visibility (off the listings, private)
/// and zero-width, without its name or tags. Audited; the owner is told of a visibility change.
pub const EMOTES_FLAGS: &str = "emotes.flags";
/// Approve or deny an emote's personal use (ADR 0039): the personal queue, and the personal-use
/// part of Approve both. Audited; the owner is told.
pub const EMOTES_REVIEW_PERSONAL: &str = "emotes.review_personal";
/// Give any emote to another account directly (ADR 0039). Nobody else can move an emote, its
/// owner included. Audited; the old and the new owner are told.
pub const EMOTES_TRANSFER: &str = "emotes.transfer";
/// Add, change and remove any account's editors on its behalf, as its owner can. Audited.
pub const EDITORS_MANAGE_ANY: &str = "editors.manage_any";
/// Change the review queue's ready reject reasons. Audited.
pub const REVIEW_SETTINGS: &str = "review.settings";
/// Set, change and end the site announcement under the header. Audited.
pub const SITE_ANNOUNCE: &str = "site.announce";

// Everyday permissions (ADR 0010, amendment 2026/11/22). The built-in User role allows them, so
// every account has them until a more senior role denies them (a "Muted" role, say).

/// Upload emotes (`POST /v1/emotes`), for yourself or as an account editor.
pub const EMOTES_UPLOAD: &str = "emotes.upload";
/// File reports on emotes.
pub const REPORTS_FILE: &str = "reports.file";

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
    REVIEW_SETTINGS,
    SITE_ANNOUNCE,
    EMOTES_FLAGS,
    EMOTES_UPLOAD,
    REPORTS_FILE,
    EMOTES_REVIEW_PERSONAL,
    EMOTES_TRANSFER,
];

/// What anyone with `roles.manage` may add to the built-in User role, which every account holds
/// (ADR 0010, amendment 2026/11/20): permissions that are fine for everyone. Every other
/// permission is a staff one, which only the owners add to that role.
pub const USER_ROLE_ALLOWED: &[&str] = &[EMOTES_UPLOAD, REPORTS_FILE];

pub fn is_known(name: &str) -> bool {
    ALL.contains(&name)
}

/// What a role says about one permission. A role that names neither leaves it to the roles below
/// (inherit), which is the same as having no row for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Effect {
    Allow,
    Deny,
}

impl Effect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            _ => None,
        }
    }
}

/// The role that decided a permission, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision<R> {
    pub effect: Effect,
    pub role: R,
    pub position: i32,
}

/// The one rule for what an account may do (ADR 0010, amendment 2026/11/22). `grants` are the
/// explicit Allow and Deny rows of every role the account holds, the built-in User role
/// (position 0) included, as `(role, position, permission, effect)`. For each permission the
/// most senior role that names it decides; at equal positions Deny wins. A permission no role
/// names is not granted. The owners are not resolved here: they hold everything.
pub fn resolve<R: Copy>(
    grants: impl IntoIterator<Item = (R, i32, String, Effect)>,
) -> std::collections::BTreeMap<String, Decision<R>> {
    let mut out: std::collections::BTreeMap<String, Decision<R>> = Default::default();
    for (role, position, permission, effect) in grants {
        let new = Decision {
            effect,
            role,
            position,
        };
        match out.get(&permission) {
            Some(old)
                if (old.position, old.effect == Effect::Deny)
                    >= (position, effect == Effect::Deny) => {}
            _ => {
                out.insert(permission, new);
            }
        }
    }
    out
}

/// The permissions `resolve` granted, sorted.
pub fn granted<R>(decisions: &std::collections::BTreeMap<String, Decision<R>>) -> Vec<String> {
    decisions
        .iter()
        .filter(|(_, d)| d.effect == Effect::Allow)
        .map(|(p, _)| p.clone())
        .collect()
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

    fn row(role: u8, position: i32, perm: &str, effect: Effect) -> (u8, i32, String, Effect) {
        (role, position, perm.to_owned(), effect)
    }

    #[test]
    fn a_deny_above_an_allow_wins() {
        // "Muted" (5) denies uploading, which the User role (0) allows.
        let d = resolve([
            row(0, 0, EMOTES_UPLOAD, Effect::Allow),
            row(5, 5, EMOTES_UPLOAD, Effect::Deny),
        ]);
        assert_eq!(d[EMOTES_UPLOAD].effect, Effect::Deny);
        assert_eq!(d[EMOTES_UPLOAD].role, 5);
        assert!(granted(&d).is_empty());
        // The order rows come in changes nothing.
        let d = resolve([
            row(5, 5, EMOTES_UPLOAD, Effect::Deny),
            row(0, 0, EMOTES_UPLOAD, Effect::Allow),
        ]);
        assert_eq!(d[EMOTES_UPLOAD].effect, Effect::Deny);
    }

    #[test]
    fn an_allow_above_a_deny_wins() {
        let d = resolve([
            row(5, 5, EMOTES_UPLOAD, Effect::Deny),
            row(9, 50, EMOTES_UPLOAD, Effect::Allow),
            row(0, 0, EMOTES_UPLOAD, Effect::Allow),
        ]);
        assert_eq!(d[EMOTES_UPLOAD].effect, Effect::Allow);
        assert_eq!(d[EMOTES_UPLOAD].role, 9);
        assert_eq!(granted(&d), [EMOTES_UPLOAD]);
    }

    #[test]
    fn inherit_falls_through_and_nothing_is_not_granted() {
        // Role 5 says nothing about uploading: the User role's Allow decides.
        let d = resolve([
            row(5, 5, EMOTES_REVIEW, Effect::Allow),
            row(0, 0, EMOTES_UPLOAD, Effect::Allow),
        ]);
        assert_eq!(d[EMOTES_UPLOAD].role, 0);
        assert_eq!(granted(&d), [EMOTES_REVIEW, EMOTES_UPLOAD]);
        assert!(!d.contains_key(AUDIT_VIEW));
        assert!(resolve(Vec::<(u8, i32, String, Effect)>::new()).is_empty());
    }

    #[test]
    fn at_equal_positions_deny_wins() {
        for rows in [
            [
                row(1, 7, AUDIT_VIEW, Effect::Allow),
                row(2, 7, AUDIT_VIEW, Effect::Deny),
            ],
            [
                row(2, 7, AUDIT_VIEW, Effect::Deny),
                row(1, 7, AUDIT_VIEW, Effect::Allow),
            ],
        ] {
            let d = resolve(rows);
            assert_eq!(d[AUDIT_VIEW].effect, Effect::Deny);
            assert_eq!(d[AUDIT_VIEW].role, 2);
        }
    }

    #[test]
    fn effects_round_trip_and_everyday_permissions_are_known() {
        for e in [Effect::Allow, Effect::Deny] {
            assert_eq!(Effect::parse(e.as_str()), Some(e));
        }
        assert_eq!(Effect::parse("inherit"), None);
        for p in USER_ROLE_ALLOWED {
            assert!(is_known(p));
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert!(is_known("emotes.review"));
        assert!(!is_known("emotes.everything"));
        assert!(!is_known(""));
    }
}
