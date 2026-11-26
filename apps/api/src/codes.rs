//! Redeem codes (ADR 0016). Staff with `codes.manage` make a batch of up to 1000 codes that each
//! give a shop item's cosmetics and/or chosen badges and paints; the plain codes are shown once,
//! in the answer that makes them, and only their sha256 is stored. Anyone signed in redeems a
//! code; entitlements get source `code`.
//!
//! Guessing is held back three ways: codes carry 80 random bits; tries are limited per account;
//! and wrong codes lock an account (5 in 15 minutes) and an IP (20 in 15 minutes) out for the
//! rest of that window. A request whose IP isn't known can't redeem (it would share one lockout
//! with every other such request). Codes are looked up by their sha256, so no comparison of the
//! code itself takes place.

use std::collections::HashSet;

use axum::http::StatusCode;
use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::codes::{self as db, BatchRow, NewBatch};
use cyex_tv_db::shop as shop_db;
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, CodeBatch, CodeBatchInput, CodeBatchPage, CosmeticKind, CreatedCodeBatch,
    RedeemResult,
};
use cyex_tv_shared::{permissions, text};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, not_found, parse_id, require_any};
use crate::errors::{self, Audit, bad, conflict, record_audit, sqlx_error};
use crate::shop::{Cosmetics, bundle, check_exist, limited, paging, staff_rate_limited, timestamp};
use crate::state::AppState;
use crate::views::time;

pub const BATCH_MAX: u32 = 1000;
pub const USES_MAX: u32 = 10_000;
pub const PREFIX_MAX: usize = 12;
/// Random characters after the prefix: 16 of Crockford's base32, 80 bits.
pub const BODY_LEN: usize = 16;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// Codes (pure, unit-tested below)

/// A new code: `PREFIX-XXXX-XXXX-XXXX-XXXX`.
pub fn generate(prefix: &str) -> String {
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).expect("the OS random source is available");
    // 80 bits as 16 five-bit groups.
    let mut acc: u128 = 0;
    for b in bytes {
        acc = (acc << 8) | u128::from(b);
    }
    let body: Vec<u8> = (0..BODY_LEN)
        .rev()
        .map(|i| ALPHABET[((acc >> (i * 5)) & 31) as usize])
        .collect();
    let groups: Vec<&str> = body
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap_or_default())
        .collect();
    format!("{prefix}-{}", groups.join("-"))
}

/// How a code is compared: upper case, no spaces or dashes, and the letters people mistake for
/// digits read as those (O as 0, I and L as 1). `None` when it can't be a code.
pub fn normalize(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c.to_ascii_uppercase() {
            ' ' | '-' | '\t' => {}
            'O' => out.push('0'),
            'I' | 'L' => out.push('1'),
            c if c.is_ascii_uppercase() || c.is_ascii_digit() => out.push(c),
            _ => return None,
        }
    }
    let len = out.len();
    (len > BODY_LEN && len <= BODY_LEN + PREFIX_MAX).then_some(out)
}

pub fn hash(normalized: &str) -> [u8; 32] {
    Sha256::digest(normalized.as_bytes()).into()
}

pub fn prefix(raw: &str) -> ApiResult<String> {
    let p = raw.trim().to_ascii_uppercase();
    if p.is_empty() || p.len() > PREFIX_MAX || !p.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(bad("bad_prefix", "Prefixes are 1 to 12 letters or digits."));
    }
    Ok(p)
}

pub fn batch_name(raw: &str) -> ApiResult<String> {
    let n = raw.trim();
    let len = n.chars().count();
    if len == 0 || len > 64 || text::has_unsafe(n, false) {
        return Err(bad("bad_name", "Names are 1 to 64 characters."));
    }
    Ok(n.to_owned())
}

// Views

async fn batch_views(conn: &mut PgConnection, rows: &[BatchRow]) -> ApiResult<Vec<CodeBatch>> {
    let ids_: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let bundles = db::batch_bundles(conn, &ids_).await.map_err(db_error)?;
    let people: Vec<Uuid> = rows.iter().filter_map(|r| r.created_by).collect();
    let summaries = reads::summaries(conn, &people).await.map_err(db_error)?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let item_name = match r.item_id {
            Some(id) => shop_db::get_item(conn, id)
                .await
                .map_err(db_error)?
                .map(|i| i.name),
            None => None,
        };
        let bundle = bundles.get(&r.id).cloned().unwrap_or_default();
        let of = |k: CosmeticKind| {
            bundle
                .iter()
                .filter(|(kind, _)| *kind == k)
                .map(|(_, id)| ids::to_text(*id))
                .collect()
        };
        out.push(CodeBatch {
            id: ids::to_text(r.id),
            name: r.name.clone(),
            prefix: r.prefix.clone(),
            item_id: r.item_id.map(ids::to_text),
            item_name,
            badge_ids: of(CosmeticKind::Badge),
            paint_ids: of(CosmeticKind::Paint),
            code_count: u32::try_from(r.code_count).unwrap_or(0),
            uses_per_code: u32::try_from(r.uses_per_code).unwrap_or(0),
            redemptions: u64::try_from(r.redemptions).unwrap_or(0),
            expires_at: r.expires_at.map(time),
            disabled_at: r.disabled_at.map(time),
            created_by: r.created_by.map(|c| match summaries.get(&c) {
                Some(s) => AccountSummary {
                    id: ids::to_text(s.account_id),
                    display_name: s.display_name.clone(),
                    avatar: s.avatar.clone(),
                },
                None => AccountSummary {
                    id: ids::to_text(c),
                    display_name: String::new(),
                    avatar: None,
                },
            }),
            created_at: time(r.created_at),
        });
    }
    Ok(out)
}

async fn one_batch(conn: &mut PgConnection, row: &BatchRow) -> ApiResult<CodeBatch> {
    batch_views(conn, std::slice::from_ref(row))
        .await?
        .pop()
        .ok_or_else(ApiError::internal)
}

// Staff

/// Makes a batch and answers with its codes in plain text, the only time they are shown.
pub async fn create_batch(
    state: &AppState,
    caller: &Caller,
    input: &CodeBatchInput,
) -> ApiResult<CreatedCodeBatch> {
    let name = batch_name(&input.name)?;
    let prefix = prefix(&input.prefix)?;
    if input.count == 0 || input.count > BATCH_MAX {
        return Err(bad("bad_count", "A batch has 1 to 1000 codes."));
    }
    let uses = input.uses_per_code.unwrap_or(1);
    if uses == 0 || uses > USES_MAX {
        return Err(bad("bad_uses", "A code can be used 1 to 10000 times."));
    }
    let expires_at = timestamp(input.expires_at.as_deref())?;
    if expires_at.is_some_and(|t| t <= Utc::now()) {
        return Err(bad("bad_time", "The expiry has to be in the future."));
    }
    let item_id = input
        .item_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    let direct = bundle(&input.badge_ids, &input.paint_ids, item_id.is_some())?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::CODES_MANAGE]).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if let Some(item) = item_id
        && !shop_db::get_item(&mut tx, item)
            .await
            .map_err(db_error)?
            .is_some_and(|i| i.deleted_at.is_none())
    {
        return Err(bad("bad_item", "No shop item with that ID."));
    }
    check_exist(&mut tx, &direct).await?;
    let mut seen = HashSet::new();
    let mut codes = Vec::new();
    while codes.len() < input.count as usize {
        let code = generate(&prefix);
        if seen.insert(code.clone()) {
            codes.push(code);
        }
    }
    let hashes: Vec<[u8; 32]> = codes
        .iter()
        .map(|c| hash(&normalize(c).unwrap_or_default()))
        .collect();
    let row = db::create_batch(
        &mut tx,
        &NewBatch {
            name: &name,
            prefix: &prefix,
            item_id,
            uses_per_code: i32::try_from(uses).unwrap_or(1),
            expires_at,
            created_by: Some(caller.account_id),
        },
        &direct,
        &hashes,
    )
    .await
    .map_err(db_error)?;
    let batch = one_batch(&mut tx, &row).await?;
    // Never the codes themselves.
    record_audit(
        &mut tx,
        Some(caller.account_id),
        Audit::new("code_batch.created", "code_batch", &ids::to_text(row.id))
            .change(None, Some(serde_json::to_value(&batch).unwrap_or_default())),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(CreatedCodeBatch { batch, codes })
}

/// Stops every code of a batch from working.
pub async fn disable_batch(state: &AppState, caller: &Caller, id: &str) -> ApiResult<CodeBatch> {
    let id = parse_id(id)?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::CODES_MANAGE]).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if db::get_batch(&mut tx, id)
        .await
        .map_err(db_error)?
        .is_none()
    {
        return Err(not_found());
    }
    if db::disable_batch(&mut tx, id, Some(caller.account_id))
        .await
        .map_err(db_error)?
    {
        record_audit(
            &mut tx,
            Some(caller.account_id),
            Audit::new("code_batch.disabled", "code_batch", &ids::to_text(id))
                .change(None, Some(json!({ "disabled": true }))),
        )
        .await?;
    }
    let row = db::get_batch(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let out = one_batch(&mut tx, &row).await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

pub async fn batches(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<CodeBatchPage> {
    let (limit, offset) = paging(page, per_page);
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::CODES_MANAGE]).await?;
    let (rows, total) = db::batches(&mut conn, limit, offset)
        .await
        .map_err(db_error)?;
    Ok(CodeBatchPage {
        items: batch_views(&mut conn, &rows).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

// Redeeming

fn wrong_code() -> ApiError {
    bad("bad_code", "That code doesn't work.")
}

fn locked_out(wait: std::time::Duration) -> ApiError {
    let minutes = wait.as_secs().div_ceil(60).max(1);
    ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "locked_out",
        format!("Too many wrong codes. Try again in {minutes} min."),
    )
    .retry_after(wait.as_secs().max(1))
}

fn unknown_client() -> ApiError {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "unknown_client",
        "This request can't redeem codes: where it came from isn't known.",
    )
}

/// The IP a redeem is locked out by: `None` when it isn't known (no address, or the shared
/// stand-in the limits use when there is none).
pub fn known_ip(ip: Option<&str>) -> Option<&str> {
    ip.map(str::trim)
        .filter(|i| !i.is_empty() && *i != "unknown")
}

/// Redeems a code for the caller, from `ip`.
pub async fn redeem(
    state: &AppState,
    caller: &Caller,
    ip: Option<&str>,
    raw: &str,
) -> ApiResult<RedeemResult> {
    let ip = known_ip(ip).ok_or_else(unknown_client)?;
    let account_key = ids::to_text(caller.account_id);
    let lock = &state.limits.redeem_lockout;
    if let Some(wait) = lock
        .account
        .locked(&account_key)
        .or_else(|| lock.ip.locked(ip))
    {
        return Err(locked_out(wait));
    }
    limited(state, crate::limits::AccountLimit::Redeem, caller)?;
    let failed = || {
        lock.account.fail(&account_key);
        lock.ip.fail(ip);
        wrong_code()
    };
    let Some(normalized) = normalize(raw) else {
        return Err(failed());
    };
    let digest = hash(&normalized);
    let mut conn = errors::conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Some(code) = db::lock_code(&mut tx, &digest).await.map_err(db_error)? else {
        return Err(failed());
    };
    let batch = db::get_batch(&mut tx, code.batch_id)
        .await
        .map_err(db_error)?
        .ok_or_else(failed)?;
    if batch.disabled_at.is_some() {
        return Err(conflict("code_disabled", "That code was turned off."));
    }
    if batch.expires_at.is_some_and(|t| t <= Utc::now()) {
        return Err(conflict("code_expired", "That code has expired."));
    }
    if db::has_redeemed(&mut tx, code.id, caller.account_id)
        .await
        .map_err(db_error)?
    {
        return Err(conflict("already_redeemed", "You used that code already."));
    }
    if code.uses >= batch.uses_per_code {
        return Err(conflict("code_used_up", "That code has been used up."));
    }
    let mut gives = db::batch_bundles(&mut tx, &[batch.id])
        .await
        .map_err(db_error)?
        .remove(&batch.id)
        .unwrap_or_default();
    if let Some(item) = batch.item_id {
        for c in shop_db::item_bundles(&mut tx, &[item])
            .await
            .map_err(db_error)?
            .remove(&item)
            .unwrap_or_default()
        {
            if !gives.contains(&c) {
                gives.push(c);
            }
        }
    }
    // Entitlements are always locked badges first, then paints, each by ID (as orders and
    // refunds do), so two grants can't wait on each other.
    gives.sort_by_key(|(kind, id)| (*kind == CosmeticKind::Paint, *id));
    let owned = shop_db::owned_of(&mut tx, caller.account_id, &gives)
        .await
        .map_err(db_error)?;
    if owned.len() == gives.len() {
        return Err(conflict(
            "already_owned",
            "You own everything this code gives.",
        ));
    }
    let reason = format!("Code from {}", batch.name);
    let mut granted = Vec::new();
    for (kind, cosmetic) in &gives {
        if shop_db::grant_from(
            &mut tx,
            *kind,
            caller.account_id,
            *cosmetic,
            "code",
            &reason,
            None,
            Some(code.id),
        )
        .await
        .map_err(db_error)?
        {
            granted.push((*kind, *cosmetic));
        }
    }
    for (kind, cosmetic) in &granted {
        crate::cosmetics::announce_release(&mut tx, &state.cdn_url, *kind, *cosmetic, true).await?;
    }
    db::record_redemption(&mut tx, code.id, caller.account_id)
        .await
        .map_err(db_error)?;
    record_audit(
        &mut tx,
        Some(caller.account_id),
        Audit::new("code.redeemed", "code_batch", &ids::to_text(batch.id)).change(
            None,
            Some(json!({
                "code_id": ids::to_text(code.id),
                "granted": granted
                    .iter()
                    .map(|(k, c)| json!({ "kind": k.as_str(), "cosmetic_id": ids::to_text(*c) }))
                    .collect::<Vec<_>>(),
            })),
        ),
    )
    .await?;
    crate::shop::notify_cosmetics(
        &mut tx,
        caller.account_id,
        "cosmetic.granted",
        &granted,
        "code",
        &format!("Redeemed a code ({})", batch.name),
        None,
    )
    .await?;
    let cosmetics = Cosmetics::load(state, &mut tx, granted.iter()).await?;
    let (badges, paints) = cosmetics.of(&granted);
    tx.commit().await.map_err(sqlx_error)?;
    Ok(RedeemResult {
        name: batch.name,
        badges,
        paints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_have_the_shape_and_normalize_back() {
        let code = generate("GIFT");
        assert_eq!(code.len(), 4 + 4 * 5);
        assert!(code.starts_with("GIFT-"));
        let n = normalize(&code).unwrap();
        assert_eq!(n.len(), 4 + BODY_LEN);
        // Case, spaces, dashes, and look-alike letters don't matter.
        let sloppy = code.to_lowercase().replace('-', " ");
        assert_eq!(normalize(&sloppy).unwrap(), n);
        assert_eq!(normalize("g0ld-AAAA").unwrap_or_default(), "");
        assert_eq!(
            normalize("GOLD-0000-1111-2222-3333"),
            normalize("G0LD-OOOO-IIII-2222-3333")
        );
        assert_eq!(normalize("GIFT-ABCD-EFGH-JKMN-PQR$"), None);
        assert_eq!(normalize(""), None);
        assert_eq!(normalize(&"A".repeat(BODY_LEN)), None);
        assert_eq!(normalize(&"A".repeat(BODY_LEN + PREFIX_MAX + 1)), None);
        assert_ne!(generate("X"), generate("X"));
    }

    #[test]
    fn hashes_differ_per_code() {
        let a = hash("GIFTABCDEFGHJKMNPQRS");
        assert_eq!(a, hash("GIFTABCDEFGHJKMNPQRS"));
        assert_ne!(a, hash("GIFTABCDEFGHJKMNPQRT"));
    }

    #[test]
    fn redeeming_needs_a_known_ip() {
        assert_eq!(known_ip(Some("203.0.113.9")), Some("203.0.113.9"));
        assert_eq!(known_ip(Some("unknown")), None);
        assert_eq!(known_ip(Some(" ")), None);
        assert_eq!(known_ip(None), None);
    }

    #[test]
    fn prefixes_and_names() {
        assert_eq!(prefix(" gift ").unwrap(), "GIFT");
        for bad in ["", "GIFT-1", "ABCDEFGHIJKLM", "ÄB"] {
            assert_eq!(prefix(bad).unwrap_err().code, "bad_prefix", "{bad}");
        }
        assert!(batch_name("Launch giveaway").is_ok());
        assert_eq!(batch_name("").unwrap_err().code, "bad_name");
    }
}
