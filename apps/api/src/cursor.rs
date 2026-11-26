//! Opaque list cursors (ADR 0030). A cursor is a position in one list (the key values of the
//! last row a page showed, `cyex_tv_db::keyset`) and the list it belongs to, as base64url text
//! with a check value. Clients pass it back as `after` and never read it; its layout may change.
//!
//! The check makes a cursor that was cut, edited or made up, or one from another list (other
//! filters or order), an error (`bad_cursor`) instead of a silent wrong page. It is not a secret:
//! a cursor only says where to start, and every read applies the caller's access rules again.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, SecondsFormat, Utc};
use cyex_tv_db::keyset::{Key, KeyType, Value, fits};
use cyex_tv_shared::error::{ApiError, ApiResult};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::bad;

/// Longest cursor taken, in characters. Real ones are far shorter.
pub const MAX_LEN: usize = 1024;
/// Layout version, so an old cursor after a change is `bad_cursor`, not misread.
const VERSION: u64 = 1;
/// Bytes of the check value.
const CHECK: usize = 8;

/// The error for a cursor this list didn't give out.
pub fn bad_cursor() -> ApiError {
    bad(
        "bad_cursor",
        "This cursor isn't one this list gave out (or the list's filters changed). Start again \
         from the first page.",
    )
}

fn digest(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"cyex.tv cursor\0");
    for p in parts {
        h.update((p.len() as u64).to_be_bytes());
        h.update(p);
    }
    h.finalize().into()
}

/// The list's identity: its name, order and filters, as a short hash.
fn scope_tag(scope: &str) -> String {
    digest(&[scope.as_bytes()])[..6]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn to_json(v: &Value) -> Json {
    match v {
        Value::Int(n) => json!({ "i": n }),
        Value::Real(r) => json!({ "r": f64::from(*r) }),
        Value::Text(t) => json!({ "t": t }),
        Value::Time(t) => json!({ "d": t.to_rfc3339_opts(SecondsFormat::AutoSi, true) }),
        Value::Uuid(u) => json!({ "u": u.to_string() }),
        Value::Bool(b) => json!({ "b": b }),
    }
}

fn from_json(j: &Json, ty: KeyType) -> Option<Value> {
    let obj = j.as_object()?;
    if obj.len() != 1 {
        return None;
    }
    let (tag, v) = obj.iter().next()?;
    Some(match (tag.as_str(), ty) {
        ("i", KeyType::Int) => Value::Int(v.as_i64()?),
        ("r", KeyType::Real) => {
            let f = v.as_f64()?;
            if !f.is_finite() {
                return None;
            }
            #[allow(clippy::cast_possible_truncation)]
            Value::Real(f as f32)
        }
        ("t", KeyType::Text) => Value::Text(v.as_str()?.to_owned()),
        ("d", KeyType::Time) => Value::Time(
            DateTime::parse_from_rfc3339(v.as_str()?)
                .ok()?
                .with_timezone(&Utc),
        ),
        ("u", KeyType::Uuid) => Value::Uuid(Uuid::parse_str(v.as_str()?).ok()?),
        ("b", KeyType::Bool) => Value::Bool(v.as_bool()?),
        _ => return None,
    })
}

/// The cursor for the position `values` in the list `scope`.
pub fn encode(scope: &str, values: &[Value]) -> String {
    let body = json!({
        "v": VERSION,
        "s": scope_tag(scope),
        "k": values.iter().map(to_json).collect::<Vec<_>>(),
    });
    let mut bytes = serde_json::to_vec(&body).unwrap_or_default();
    let check = digest(&[&bytes]);
    bytes.extend_from_slice(&check[..CHECK]);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `encode` for an optional position (the last page has none).
pub fn next(scope: &str, values: Option<&Vec<Value>>) -> Option<String> {
    values.map(|v| encode(scope, v))
}

/// The position in `raw`, a cursor of the list `scope` ordered by `keys`. Anything else is
/// `bad_cursor`. An empty cursor is none (the first page).
pub fn decode(raw: Option<&str>, scope: &str, keys: &[Key]) -> ApiResult<Option<Vec<Value>>> {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    parse(raw, scope, keys).map(Some).ok_or_else(bad_cursor)
}

fn parse(raw: &str, scope: &str, keys: &[Key]) -> Option<Vec<Value>> {
    if raw.len() > MAX_LEN {
        return None;
    }
    let bytes = URL_SAFE_NO_PAD.decode(raw).ok()?;
    let split = bytes.len().checked_sub(CHECK)?;
    let (body, check) = bytes.split_at(split);
    if digest(&[body])[..CHECK] != *check {
        return None;
    }
    let body: Json = serde_json::from_slice(body).ok()?;
    if body.get("v")?.as_u64()? != VERSION || body.get("s")?.as_str()? != scope_tag(scope) {
        return None;
    }
    let raw_keys = body.get("k")?.as_array()?;
    if raw_keys.len() != keys.len() {
        return None;
    }
    let values: Vec<Value> = raw_keys
        .iter()
        .zip(keys)
        .map(|(j, k)| from_json(j, k.ty))
        .collect::<Option<_>>()?;
    fits(keys, &values).then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> Vec<Key> {
        vec![
            Key::asc("a", KeyType::Int),
            Key::asc("b", KeyType::Real),
            Key::asc("c", KeyType::Text),
            Key::desc("d", KeyType::Time),
            Key::asc("e", KeyType::Uuid),
            Key::asc("f", KeyType::Bool),
        ]
    }

    fn values() -> Vec<Value> {
        vec![
            Value::Int(-42),
            Value::Real(0.333_333_34),
            Value::Text("Pepe \"Jam\" 猫".into()),
            Value::Time(
                DateTime::parse_from_rfc3339("2026-11-26T10:00:00.123456Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            Value::Uuid(Uuid::from_u128(7)),
            Value::Bool(true),
        ]
    }

    #[test]
    fn a_cursor_reads_back_as_its_position() {
        let c = encode("emotes:new", &values());
        assert!(c.len() < MAX_LEN);
        assert!(
            c.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        assert_eq!(
            decode(Some(&c), "emotes:new", &keys()).unwrap(),
            Some(values())
        );
        assert_eq!(decode(None, "emotes:new", &keys()).unwrap(), None);
        assert_eq!(decode(Some("  "), "emotes:new", &keys()).unwrap(), None);
    }

    #[test]
    fn other_lists_edits_and_junk_are_bad_cursors() {
        let c = encode("emotes:new", &values());
        let code = |raw: &str, scope: &str, keys: &[Key]| {
            decode(Some(raw), scope, keys).unwrap_err().code.to_string()
        };
        assert_eq!(code(&c, "emotes:top", &keys()), "bad_cursor");
        assert_eq!(code(&c, "emotes:new", &keys()[..5]), "bad_cursor");
        assert_eq!(code("nope", "emotes:new", &keys()), "bad_cursor");
        assert_eq!(
            code(&"a".repeat(MAX_LEN + 1), "emotes:new", &keys()),
            "bad_cursor"
        );
        // One character changed anywhere fails the check.
        let mut edited: Vec<char> = c.chars().collect();
        let mid = edited.len() / 2;
        edited[mid] = if edited[mid] == 'A' { 'B' } else { 'A' };
        let edited: String = edited.into_iter().collect();
        assert_eq!(code(&edited, "emotes:new", &keys()), "bad_cursor");
        // A well-formed cursor whose values don't fit the keys.
        let wrong = encode("emotes:new", &[Value::Int(1)]);
        assert_eq!(code(&wrong, "emotes:new", &keys()), "bad_cursor");
    }
}
