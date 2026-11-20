//! The review queue's helpers around the verdicts: possible duplicates of an upload, the ready
//! reject reasons staff keep, the uploader's "this file is already here" check, and matching a
//! pasted list of logins to accounts for grants to many at once.

use std::collections::{BTreeSet, HashMap};

use cyex_tv_auth::Caller;
use cyex_tv_db::{accounts, audit, ids, reads, review};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{AccountSummary, DuplicateCandidate, Emote, LoginMatch, Platform};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::json;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, holds, not_found, parse_id, permissions_of};
use crate::errors::{bad, sqlx_error};
use crate::state::AppState;
use crate::views;

/// Possible duplicates one read returns at most.
pub const DUPLICATES_MAX: i64 = 12;
/// Public emotes one file check returns at most.
pub const SAME_FILE_MAX: i64 = 3;
/// Ready reasons: how many, and how long each.
pub const REASONS_MAX: usize = 12;
pub const REASON_LEN_MAX: usize = 200;
/// Logins one lookup takes at most.
pub const LOGINS_MAX: usize = 100;

/// Emotes built in the order of `wanted` (rows that are gone are skipped).
async fn emotes_in_order(
    state: &AppState,
    conn: &mut PgConnection,
    wanted: &[Uuid],
) -> ApiResult<Vec<Emote>> {
    let rows = reads::emotes(conn, wanted).await.map_err(db_error)?;
    let mut by_id: HashMap<Uuid, reads::EmoteRow> = rows.into_iter().map(|r| (r.id, r)).collect();
    let ordered: Vec<reads::EmoteRow> = wanted.iter().filter_map(|id| by_id.remove(id)).collect();
    views::emotes(conn, &ordered, &state.cdn_url)
        .await
        .map_err(db_error)
}

/// Other emotes that may be the same as `emote_id`: the same file (sha256) first, then similar
/// names, closest first, at most [`DUPLICATES_MAX`]. Any state but deleted. Needs `emotes.review`.
pub async fn duplicates(
    state: &AppState,
    caller: &Caller,
    emote_id: &str,
) -> ApiResult<Vec<DuplicateCandidate>> {
    let id = parse_id(emote_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !holds(&mut conn, caller, permissions::EMOTES_REVIEW).await? {
        return Err(forbidden("This needs emotes.review."));
    }
    if reads::emotes(&mut conn, &[id])
        .await
        .map_err(db_error)?
        .is_empty()
    {
        return Err(not_found());
    }
    let hits = reads::duplicates(&mut conn, id, DUPLICATES_MAX)
        .await
        .map_err(db_error)?;
    let wanted: Vec<Uuid> = hits.iter().map(|h| h.id).collect();
    let built = emotes_in_order(state, &mut conn, &wanted).await?;
    let hit_of: HashMap<String, &reads::DuplicateHit> =
        hits.iter().map(|h| (ids::to_text(h.id), h)).collect();
    Ok(built
        .into_iter()
        .filter_map(|emote| {
            let hit = hit_of.get(&emote.id)?;
            Some(DuplicateCandidate {
                same_file: hit.same_file,
                similarity: f64::from((hit.similarity * 100.0).round() / 100.0),
                emote,
            })
        })
        .collect())
}

/// Public emotes made from the file whose sha256 is `sha256` (64 hex digits), oldest first, at
/// most [`SAME_FILE_MAX`]: what an uploader is told before sending a file that is already here.
/// Only what anyone can already see.
pub async fn same_file(state: &AppState, sha256: &str) -> ApiResult<Vec<Emote>> {
    let digest = parse_sha256(sha256)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let wanted = reads::public_by_sha256(&mut conn, &digest, SAME_FILE_MAX)
        .await
        .map_err(db_error)?;
    emotes_in_order(state, &mut conn, &wanted).await
}

/// 64 hex digits (either case) as 32 bytes.
pub fn parse_sha256(raw: &str) -> ApiResult<[u8; 32]> {
    let raw = raw.trim();
    let wrong = || bad("bad_sha256", "Give a sha256 as 64 hex digits.");
    if raw.len() != 64 || !raw.is_ascii() {
        return Err(wrong());
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&raw[i * 2..i * 2 + 2], 16).map_err(|_| wrong())?;
    }
    Ok(out)
}

/// The ready reject reasons, in order. Needs `emotes.review` or `review.settings`.
pub async fn reasons(state: &AppState, caller: &Caller) -> ApiResult<Vec<String>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let held = permissions_of(&mut conn, caller).await?;
    if !held
        .iter()
        .any(|p| p == permissions::EMOTES_REVIEW || p == permissions::REVIEW_SETTINGS)
    {
        return Err(forbidden("This needs emotes.review or review.settings."));
    }
    review::reasons(&mut conn).await.map_err(db_error)
}

/// Each reason trimmed, 1 to 200 characters on one line, no repeats (ignoring case), at most 12.
pub fn clean_reasons(raw: &[String]) -> ApiResult<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for r in raw {
        let r = r.trim();
        let len = r.chars().count();
        if len == 0 || len > REASON_LEN_MAX || text::has_unsafe(r, false) {
            return Err(bad(
                "bad_reason",
                "Each reason is 1 to 200 characters on one line.",
            ));
        }
        if !out.iter().any(|o| o.to_lowercase() == r.to_lowercase()) {
            out.push(r.to_owned());
        }
    }
    if out.len() > REASONS_MAX {
        return Err(bad("too_many_reasons", "At most 12 ready reasons."));
    }
    Ok(out)
}

/// Replaces the ready reasons (an empty list is allowed: reviewers then type every reason).
/// Needs `review.settings`. Audited (`review.reasons_changed`, before and after).
pub async fn set_reasons(
    state: &AppState,
    caller: &Caller,
    raw: &[String],
) -> ApiResult<Vec<String>> {
    let next = clean_reasons(raw)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !holds(&mut tx, caller, permissions::REVIEW_SETTINGS).await? {
        return Err(forbidden("This needs review.settings."));
    }
    review::lock(&mut tx).await.map_err(db_error)?;
    let before = review::reasons(&mut tx).await.map_err(db_error)?;
    if before != next {
        review::replace_reasons(&mut tx, &next, caller.account_id)
            .await
            .map_err(db_error)?;
        audit::write(
            &mut tx,
            &audit::NewEntry {
                actor: Some(caller.account_id),
                action: "review.reasons_changed",
                target_kind: "review_reasons",
                target_id: "reject",
                before: Some(json!({ "reasons": before })),
                after: Some(json!({ "reasons": next })),
            },
        )
        .await
        .map_err(db_error)?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(next)
}

/// One pasted login: `name`, `@name`, `twitch:name` or `kick:name`. The platform when one was
/// named, and the login; `None` when it can't be a login.
pub fn parse_login(raw: &str) -> Option<(Option<Platform>, String)> {
    let raw = raw.trim();
    let (platform, rest) = match raw.split_once(':') {
        Some((p, rest)) if p.eq_ignore_ascii_case("twitch") => (Some(Platform::Twitch), rest),
        Some((p, rest)) if p.eq_ignore_ascii_case("kick") => (Some(Platform::Kick), rest),
        Some(_) => return None,
        None => (None, raw),
    };
    let login = rest.trim().trim_start_matches('@');
    let len = login.chars().count();
    let fits = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if len == 0 || len > 40 || !login.chars().all(fits) {
        return None;
    }
    Some((platform, login.to_owned()))
}

/// Matches up to [`LOGINS_MAX`] pasted logins to accounts, in the order given (repeats once).
/// A bare login looks on Twitch and Kick; when both name different accounts it is `ambiguous`
/// (say `twitch:` or `kick:`). For staff who give or assign to many at once: needs
/// `cosmetics.grant`, `roles.assign`, `roles.manage` or `users.view`.
pub async fn accounts_by_login(
    state: &AppState,
    caller: &Caller,
    logins: &[String],
) -> ApiResult<Vec<LoginMatch>> {
    let mut seen = BTreeSet::new();
    let wanted: Vec<String> = logins
        .iter()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty() && seen.insert(l.trim_start_matches('@').to_lowercase()))
        .collect();
    if wanted.is_empty() || wanted.len() > LOGINS_MAX {
        return Err(bad("bad_logins", "Give 1 to 100 logins at a time."));
    }
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let held = permissions_of(&mut conn, caller).await?;
    let may = [
        permissions::COSMETICS_GRANT,
        permissions::ROLES_ASSIGN,
        permissions::ROLES_MANAGE,
        permissions::USERS_VIEW,
    ];
    if !held.iter().any(|p| may.contains(&p.as_str())) {
        return Err(forbidden(
            "This needs cosmetics.grant, roles.assign, roles.manage or users.view.",
        ));
    }
    // A list of logins is a batch of account lookups: it takes from the account search limit.
    crate::errors::limited_as(
        state,
        crate::limits::AccountLimit::AccountSearch,
        &format!("a:{}", ids::to_text(caller.account_id)),
        caller.account_id,
        "That's a lot of login checks. Try again in a minute.",
    )?;
    let mut found: Vec<(String, Result<Uuid, &'static str>)> = Vec::with_capacity(wanted.len());
    for login in wanted {
        let Some((platform, name)) = parse_login(&login) else {
            found.push((login, Err("bad_login")));
            continue;
        };
        let platforms = match platform {
            Some(p) => vec![p],
            None => vec![Platform::Twitch, Platform::Kick],
        };
        let mut hits: Vec<Uuid> = Vec::new();
        for p in platforms {
            if let Some(identity) = accounts::find_by_login(&mut conn, p, &name)
                .await
                .map_err(db_error)?
                && !hits.contains(&identity.account_id)
            {
                hits.push(identity.account_id);
            }
        }
        let result = match hits.as_slice() {
            [] => Err("not_found"),
            [one] => Ok(*one),
            _ => Err("ambiguous"),
        };
        found.push((login, result));
    }
    let account_ids: Vec<Uuid> = found.iter().filter_map(|(_, r)| r.ok()).collect();
    let summaries = reads::summaries(&mut conn, &account_ids)
        .await
        .map_err(db_error)?;
    Ok(found
        .into_iter()
        .map(|(login, r)| {
            // An account being deleted (or with no identity left) can't be given anything.
            let summary = r.ok().and_then(|id| summaries.get(&id));
            let problem = match (&r, summary) {
                (Err(p), _) => Some((*p).to_owned()),
                (Ok(_), None) => Some("not_found".to_owned()),
                (Ok(_), Some(_)) => None,
            };
            LoginMatch {
                login,
                account: summary.map(AccountSummary::from),
                problem,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_parses_hex_only() {
        let hex = "00ff".repeat(16);
        let bytes = parse_sha256(&hex).unwrap();
        assert_eq!(bytes[0], 0);
        assert_eq!(bytes[1], 0xff);
        assert!(parse_sha256(&hex.to_uppercase()).is_ok());
        assert!(parse_sha256("abc").is_err());
        assert!(parse_sha256(&"zz".repeat(32)).is_err());
        assert!(parse_sha256(&"\u{e9}".repeat(32)).is_err());
    }

    #[test]
    fn reasons_are_trimmed_deduplicated_and_capped() {
        let out =
            clean_reasons(&[" Blurry ".into(), "blurry".into(), "Not an emote".into()]).unwrap();
        assert_eq!(out, ["Blurry", "Not an emote"]);
        assert!(clean_reasons(&[String::new()]).is_err());
        assert!(clean_reasons(&["x".repeat(201)]).is_err());
        assert!(clean_reasons(&["two\nlines".into()]).is_err());
        let many: Vec<String> = (0..13).map(|i| format!("Reason {i}")).collect();
        assert_eq!(clean_reasons(&many).unwrap_err().code, "too_many_reasons");
        assert!(clean_reasons(&[]).unwrap().is_empty());
    }

    #[test]
    fn logins_parse_with_or_without_a_platform() {
        assert_eq!(parse_login(" @Alice "), Some((None, "Alice".into())));
        assert_eq!(
            parse_login("twitch:bob_1"),
            Some((Some(Platform::Twitch), "bob_1".into()))
        );
        assert_eq!(
            parse_login("KICK:carol-x"),
            Some((Some(Platform::Kick), "carol-x".into()))
        );
        assert_eq!(parse_login("youtube:dave"), None);
        assert_eq!(parse_login("has space"), None);
        assert_eq!(parse_login(""), None);
        assert_eq!(parse_login(&"a".repeat(41)), None);
    }
}
