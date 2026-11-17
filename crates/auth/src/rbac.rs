//! The role rules (ADR 0010), as plain functions so every one is unit-tested:
//!
//! - **Owner:** the accounts named by `CYEX_TV_OWNER_IDS` hold every permission and can't be
//!   demoted through the API.
//! - **No escalation:** you only grant permissions you hold, and only create, edit, delete or
//!   assign roles below your own highest `position`.
//! - **No lock-out:** a change that would leave nobody but the owner holding `roles.manage` is
//!   refused, unless the owner makes it.

use std::collections::BTreeSet;

use cyex_tv_shared::config::ConfigError;
use cyex_tv_shared::model::Platform;
use cyex_tv_shared::permissions;
use uuid::Uuid;

/// Who the owners are: platform identities (`twitch:12345678`) and, optionally, account IDs.
/// Identities are what's known before the first sign-in, and they never change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Owners {
    /// `platform:platform_user_id`, for example `kick:7654321`.
    pub identities: Vec<String>,
    pub accounts: Vec<Uuid>,
}

impl Owners {
    /// Parses `CYEX_TV_OWNER_IDS`: a comma-separated list of `twitch:<id>`, `kick:<id>` or
    /// 26-character account IDs. Empty means no owner.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let mut out = Self::default();
        for entry in raw.split(',').map(str::trim).filter(|e| !e.is_empty()) {
            let invalid = || ConfigError::Invalid {
                name: "CYEX_TV_OWNER_IDS",
                reason: format!("{entry:?} is not twitch:<id>, kick:<id> or an account ID"),
            };
            if let Some((platform, id)) = entry.split_once(':') {
                let platform: Platform = platform.parse().map_err(|_| invalid())?;
                if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(invalid());
                }
                out.identities.push(format!("{platform}:{id}"));
            } else {
                out.accounts
                    .push(cyex_tv_db::ids::from_text(entry).ok_or_else(invalid)?);
            }
        }
        Ok(out)
    }

    pub fn is_empty(&self) -> bool {
        self.identities.is_empty() && self.accounts.is_empty()
    }
}

/// Why a role change was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RuleError {
    #[error("you can only grant permissions you hold")]
    Escalation,
    #[error("you can only manage roles below your own highest role")]
    NotBelowYou,
    #[error("the owner can't be demoted")]
    OwnerProtected,
    #[error("this would leave nobody but the owner able to manage roles")]
    LastManager,
}

impl RuleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Escalation => "escalation",
            Self::NotBelowYou => "not_below_you",
            Self::OwnerProtected => "owner_protected",
            Self::LastManager => "last_manager",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Escalation => "You can only grant permissions you hold.",
            Self::NotBelowYou => "You can only manage roles below your own highest role.",
            Self::OwnerProtected => "The owner's roles can't be changed by others.",
            Self::LastManager => {
                "This would leave nobody but the owner able to manage roles. Give roles.manage to someone else first."
            }
        }
    }
}

/// The one acting: what they hold, and how senior they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Actor {
    pub owner: bool,
    pub permissions: BTreeSet<String>,
    /// The position of their most senior role, if they have any.
    pub highest: Option<i32>,
}

impl Actor {
    pub fn holds(&self, permission: &str) -> bool {
        self.owner || self.permissions.contains(permission)
    }

    /// Granting `perms`, in a role or by assigning a role that holds them.
    pub fn may_grant<'a>(&self, perms: impl IntoIterator<Item = &'a str>) -> Result<(), RuleError> {
        if self.owner {
            return Ok(());
        }
        match perms.into_iter().all(|p| self.permissions.contains(p)) {
            true => Ok(()),
            false => Err(RuleError::Escalation),
        }
    }

    /// Creating, editing, deleting or assigning a role at `position`, or moving one there.
    pub fn may_manage(&self, position: i32) -> Result<(), RuleError> {
        if self.owner {
            return Ok(());
        }
        match self.highest {
            Some(highest) if position < highest => Ok(()),
            _ => Err(RuleError::NotBelowYou),
        }
    }

    /// Acting on another account (restricting it, taking a role from it): its most senior role
    /// (`target_highest`, `None` without roles) must be below yours. The owner may always.
    pub fn may_act_on(&self, target_highest: Option<i32>) -> Result<(), RuleError> {
        if self.owner {
            return Ok(());
        }
        match (target_highest, self.highest) {
            (None, _) => Ok(()),
            (Some(target), Some(mine)) if target < mine => Ok(()),
            _ => Err(RuleError::NotBelowYou),
        }
    }

    /// Taking a role away from `target_is_owner`'s account.
    pub fn may_demote(&self, target_is_owner: bool) -> Result<(), RuleError> {
        if target_is_owner && !self.owner {
            return Err(RuleError::OwnerProtected);
        }
        Ok(())
    }

    /// After a change: `before` and `after` are the non-owner accounts holding `roles.manage`.
    /// Going from some to none is refused, unless the owner does it.
    pub fn keeps_a_manager(&self, before: &[Uuid], after: &[Uuid]) -> Result<(), RuleError> {
        if !self.owner && !before.is_empty() && after.is_empty() {
            return Err(RuleError::LastManager);
        }
        Ok(())
    }
}

/// Every permission, for the owner.
pub fn all_permissions() -> Vec<String> {
    permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(perms: &[&str], highest: Option<i32>) -> Actor {
        Actor {
            owner: false,
            permissions: perms.iter().map(|p| (*p).to_owned()).collect(),
            highest,
        }
    }

    fn owner() -> Actor {
        Actor {
            owner: true,
            ..Actor::default()
        }
    }

    #[test]
    fn owners_parse() {
        let o = Owners::parse(" twitch:12345678, kick:7654321 ,").unwrap();
        assert_eq!(o.identities, ["twitch:12345678", "kick:7654321"]);
        assert!(o.accounts.is_empty());
        let id = cyex_tv_db::ids::new_id();
        let o = Owners::parse(&cyex_tv_db::ids::to_text(id)).unwrap();
        assert_eq!(o.accounts, [id]);
        assert!(Owners::parse("").unwrap().is_empty());
        for bad in [
            "youtube:1",
            "twitch:",
            "twitch:12a",
            "kick:1:2",
            "not-an-id",
        ] {
            assert!(Owners::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_owner_holds_everything_and_passes_every_rule() {
        let o = owner();
        for p in permissions::ALL {
            assert!(o.holds(p));
        }
        assert_eq!(o.may_grant(permissions::ALL.iter().copied()), Ok(()));
        assert_eq!(o.may_manage(i32::MAX), Ok(()));
        assert_eq!(o.may_demote(true), Ok(()));
        assert_eq!(o.keeps_a_manager(&[Uuid::nil()], &[]), Ok(()));
        assert_eq!(all_permissions().len(), permissions::ALL.len());
    }

    #[test]
    fn no_escalation_of_permissions() {
        let a = actor(&["roles.manage", "emotes.review"], Some(50));
        assert_eq!(a.may_grant(["emotes.review"]), Ok(()));
        assert_eq!(a.may_grant(["emotes.review", "roles.manage"]), Ok(()));
        assert_eq!(a.may_grant(["audit.view"]), Err(RuleError::Escalation));
        assert_eq!(
            a.may_grant(["emotes.review", "users.restrict"]),
            Err(RuleError::Escalation)
        );
        assert_eq!(a.may_grant([]), Ok(()));
        // roles.manage can't be granted by someone who lacks it.
        let reviewer = actor(&["emotes.review"], Some(50));
        assert_eq!(
            reviewer.may_grant(["roles.manage"]),
            Err(RuleError::Escalation)
        );
    }

    #[test]
    fn only_roles_below_your_highest() {
        let a = actor(&["roles.manage"], Some(50));
        assert_eq!(a.may_manage(49), Ok(()));
        assert_eq!(a.may_manage(-10), Ok(()));
        assert_eq!(a.may_manage(50), Err(RuleError::NotBelowYou));
        assert_eq!(a.may_manage(51), Err(RuleError::NotBelowYou));
        let roleless = actor(&[], None);
        assert_eq!(roleless.may_manage(i32::MIN), Err(RuleError::NotBelowYou));
    }

    #[test]
    fn only_accounts_below_your_highest() {
        let a = actor(&["users.restrict"], Some(50));
        assert_eq!(a.may_act_on(None), Ok(()));
        assert_eq!(a.may_act_on(Some(49)), Ok(()));
        assert_eq!(a.may_act_on(Some(50)), Err(RuleError::NotBelowYou));
        assert_eq!(a.may_act_on(Some(100)), Err(RuleError::NotBelowYou));
        let roleless = actor(&[], None);
        assert_eq!(roleless.may_act_on(None), Ok(()));
        assert_eq!(roleless.may_act_on(Some(0)), Err(RuleError::NotBelowYou));
        assert_eq!(owner().may_act_on(Some(i32::MAX)), Ok(()));
    }

    #[test]
    fn the_owner_cant_be_demoted_by_others() {
        let a = actor(&["roles.manage"], Some(100));
        assert_eq!(a.may_demote(true), Err(RuleError::OwnerProtected));
        assert_eq!(a.may_demote(false), Ok(()));
    }

    #[test]
    fn no_lock_out() {
        let a = actor(&["roles.manage"], Some(100));
        let me = cyex_tv_db::ids::new_id();
        let other = cyex_tv_db::ids::new_id();
        assert_eq!(a.keeps_a_manager(&[me], &[]), Err(RuleError::LastManager));
        assert_eq!(a.keeps_a_manager(&[me, other], &[other]), Ok(()));
        assert_eq!(a.keeps_a_manager(&[me], &[me]), Ok(()));
        // Nobody held it before: nothing to protect.
        assert_eq!(a.keeps_a_manager(&[], &[]), Ok(()));
    }
}
