//! Snowflake public IDs of accounts, emotes and emote sets (ADR 0026).
//!
//! Inside, rows keep their uuid (ULID) primary keys and every join uses them. The API shows the
//! Snowflake, as a decimal string, and takes either form while clients move over. The edges
//! translate, and only they do:
//! - answers: [`publish_json`] (and the GraphQL layer, with [`is_id_key`]) turn the ULID of an
//!   account, emote or set into its Snowflake in fields named `id`, `ids`, `…Id`, `…Ids`,
//!   `…By` (and snake_case `…_id`, `…_ids`, `…_by`), and in the JSON text of `before`, `after` and `payload`;
//! - requests: [`resolve`] looks the request's Snowflakes up once, and [`resolving`] runs the
//!   request with them, so [`crate::ids::from_text`] reads both forms everywhere.
//!
//! IDs are made by Postgres (`snowflake_id()`, migration `20261212000001`): 41 bits of
//! milliseconds since [`EPOCH_MS`], 10 bits of instance, 12 bits of sequence.

use std::collections::{HashMap, HashSet};
use std::future::Future;

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::{self, RESOLVED};

/// The cyex epoch, 2026-01-01T00:00:00Z, in Unix milliseconds.
pub const EPOCH_MS: i64 = 1_767_225_600_000;
const INSTANCE_BITS: u32 = 10;
const SEQUENCE_BITS: u32 = 12;

/// What a Snowflake is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parts {
    pub time: DateTime<Utc>,
    pub instance: u16,
    pub sequence: u16,
}

/// Reads a Snowflake back into its time, instance and sequence.
pub fn decode(id: i64) -> Parts {
    let ms = (id >> (INSTANCE_BITS + SEQUENCE_BITS)) + EPOCH_MS;
    Parts {
        time: DateTime::from_timestamp_millis(ms).unwrap_or_default(),
        instance: u16::try_from((id >> SEQUENCE_BITS) & 0x3ff).unwrap_or(0),
        sequence: u16::try_from(id & 0xfff).unwrap_or(0),
    }
}

/// The Snowflake for a time (Unix milliseconds), instance and sequence.
pub fn encode(unix_ms: i64, instance: u16, sequence: u16) -> i64 {
    ((unix_ms - EPOCH_MS) << (INSTANCE_BITS + SEQUENCE_BITS))
        | (i64::from(instance & 0x3ff) << SEQUENCE_BITS)
        | i64::from(sequence & 0xfff)
}

/// The Snowflake form: 1 to 19 ASCII digits without leading zeros, within `i64`. Anything else,
/// a ULID included, is `None`.
pub fn parse(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 19
        || !bytes.iter().all(u8::is_ascii_digit)
        || (bytes.len() > 1 && bytes[0] == b'0')
    {
        return None;
    }
    text.parse().ok()
}

/// A new Snowflake from the database's generator. Rows get theirs by default; this is for tests
/// and tools.
pub async fn generate(conn: &mut PgConnection) -> DbResult<i64> {
    Ok(sqlx::query_scalar("SELECT snowflake_id()")
        .fetch_one(conn)
        .await?)
}

/// The rows of these Snowflakes, among accounts, emotes and sets. Unknown ones are left out.
pub async fn resolve(conn: &mut PgConnection, publics: &[i64]) -> DbResult<HashMap<i64, Uuid>> {
    if publics.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(i64, Uuid)> = sqlx::query_as(
        "SELECT public_id, id FROM accounts WHERE public_id = ANY($1)
         UNION ALL SELECT public_id, id FROM emotes WHERE public_id = ANY($1)
         UNION ALL SELECT public_id, id FROM emote_sets WHERE public_id = ANY($1)",
    )
    .bind(publics)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// The Snowflakes of these rows, among accounts, emotes and sets. Other IDs (badges, roles,
/// rows that are gone) are left out.
pub async fn publics(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<HashMap<Uuid, i64>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT id, public_id FROM accounts WHERE id = ANY($1)
         UNION ALL SELECT id, public_id FROM emotes WHERE id = ANY($1)
         UNION ALL SELECT id, public_id FROM emote_sets WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// The Snowflake of one account, emote or set, as the API writes it.
pub async fn public_text(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<String>> {
    Ok(publics(conn, &[id]).await?.get(&id).map(i64::to_string))
}

/// Runs `fut` with `resolved` (from [`resolve`]) readable by [`ids::from_text`], on top of what
/// an outer request already resolved.
pub async fn resolving<F: Future>(resolved: HashMap<i64, Uuid>, fut: F) -> F::Output {
    let mut all = RESOLVED.try_with(Clone::clone).unwrap_or_default();
    all.extend(resolved);
    RESOLVED.scope(all, fut).await
}

/// An ID from a client, in the ULID form the rest of the code reads: a known Snowflake becomes
/// its row's ULID, anything else is returned as it came (and fails, or not, where it is parsed).
pub async fn canonical(conn: &mut PgConnection, text: &str) -> DbResult<String> {
    let Some(public) = parse(text.trim()) else {
        return Ok(text.to_owned());
    };
    Ok(resolve(conn, &[public])
        .await?
        .get(&public)
        .map_or_else(|| text.to_owned(), |id| ids::to_text(*id)))
}

/// Snowflake-shaped string literals (`"1234…"`) in GraphQL text, for [`resolve`].
pub fn literals_in(text: &str, out: &mut Vec<i64>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end < bytes.len() && bytes[end] == b'"' && end > start {
                if let Some(public) = text.get(start..end).and_then(parse) {
                    out.push(public);
                }
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
}

/// Snowflake-shaped strings anywhere in a JSON value (GraphQL variables, request bodies).
pub fn strings_in(value: &Value, out: &mut Vec<i64>) {
    match value {
        Value::String(s) => out.extend(parse(s)),
        Value::Array(items) => items.iter().for_each(|v| strings_in(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings_in(v, out)),
        _ => {}
    }
}

/// Whether a field holds IDs that are translated in answers: `id`, `ids`, `…Id`, `…Ids`, `…By`
/// and their snake_case forms.
pub fn is_id_key(key: &str) -> bool {
    key == "id"
        || key == "ids"
        // A batch's changed items (`BatchOutcome.done`).
        || key == "done"
        || key.ends_with("Id")
        || key.ends_with("Ids")
        || key.ends_with("_id")
        || key.ends_with("_ids")
        // Who did something: `addedBy`, `createdBy`, `grantedBy`, `requested_by`.
        || key.ends_with("By")
        || key.ends_with("_by")
}

/// Fields whose text is JSON in its own right (audit `before` and `after`, event and
/// notification `payload`, the account export `exportMyData`).
pub fn is_embedded_json_key(key: &str) -> bool {
    matches!(key, "before" | "after" | "payload" | "exportMyData")
}

fn ulid_of(text: &str) -> Option<Uuid> {
    ids::is_ulid(text).then(|| ids::from_text(text)).flatten()
}

fn embedded(text: &str) -> Option<Value> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(text).ok()
    } else {
        None
    }
}

/// The ULIDs in the ID fields of `value` (see [`is_id_key`]), for [`publics`].
pub fn collect_json(value: &Value, out: &mut HashSet<Uuid>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if is_id_key(key) {
                    match v {
                        Value::String(s) => out.extend(ulid_of(s)),
                        Value::Array(items) => {
                            out.extend(items.iter().filter_map(Value::as_str).filter_map(ulid_of))
                        }
                        other => collect_json(other, out),
                    }
                } else if let (true, Value::String(s)) = (is_embedded_json_key(key), v) {
                    if let Some(inner) = embedded(s) {
                        collect_json(&inner, out);
                    }
                } else {
                    collect_json(v, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_json(v, out)),
        _ => {}
    }
}

fn replace_text(text: &mut String, map: &HashMap<Uuid, i64>) {
    if let Some(public) = ulid_of(text).and_then(|id| map.get(&id)) {
        *text = public.to_string();
    }
}

/// Puts the Snowflakes of `map` in place of the ULIDs [`collect_json`] found.
pub fn replace_json(value: &mut Value, map: &HashMap<Uuid, i64>) {
    match value {
        Value::Object(obj) => {
            for (key, v) in obj.iter_mut() {
                if is_id_key(key) {
                    match v {
                        Value::String(s) => replace_text(s, map),
                        Value::Array(items) => {
                            for item in items {
                                if let Value::String(s) = item {
                                    replace_text(s, map);
                                }
                            }
                        }
                        other => replace_json(other, map),
                    }
                } else if is_embedded_json_key(key) {
                    if let Value::String(s) = v {
                        if let Some(mut inner) = embedded(s) {
                            replace_json(&mut inner, map);
                            if let Ok(text) = serde_json::to_string(&inner) {
                                *s = text;
                            }
                        }
                    } else {
                        replace_json(v, map);
                    }
                } else {
                    replace_json(v, map);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| replace_json(v, map)),
        _ => {}
    }
}

/// Translates an answer in place: account, emote and set ULIDs in its ID fields become their
/// Snowflakes.
pub async fn publish_json(conn: &mut PgConnection, value: &mut Value) -> DbResult<()> {
    let mut found = HashSet::new();
    collect_json(value, &mut found);
    if found.is_empty() {
        return Ok(());
    }
    let found: Vec<Uuid> = found.into_iter().collect();
    let map = publics(conn, &found).await?;
    if !map.is_empty() {
        replace_json(value, &map);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn snowflakes_encode_and_decode() {
        // 2026-11-21T12:00:00.123Z, instance 5, sequence 42.
        let ms = 1_795_262_400_123;
        let id = encode(ms, 5, 42);
        let parts = decode(id);
        assert_eq!(parts.time.timestamp_millis(), ms);
        assert_eq!(parts.instance, 5);
        assert_eq!(parts.sequence, 42);
        assert!(id > 0);
        // The epoch itself is time zero.
        assert_eq!(encode(EPOCH_MS, 0, 0), 0);
        // 41 bits of milliseconds last until 2095.
        let last = encode(EPOCH_MS + (1 << 41) - 1, 1023, 4095);
        assert_eq!(last, i64::MAX);
    }

    #[test]
    fn only_digit_strings_parse() {
        assert_eq!(
            parse("1314628736410763264"),
            Some(1_314_628_736_410_763_264)
        );
        assert_eq!(parse("7"), Some(7));
        assert_eq!(parse("0"), Some(0));
        for bad in [
            "",
            "007",
            "-1",
            "+1",
            " 1",
            "1.5",
            "9223372036854775808",
            "12345678901234567890",
            "01JD8X00000000000000000005",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn literals_are_found_in_graphql_text() {
        let mut out = Vec::new();
        literals_in(
            r#"{ emote(id: "1314628736410763264") { id } a: set(id:"42") b: x(n: 42, s: "4a2", t: "") }"#,
            &mut out,
        );
        assert_eq!(out, vec![1_314_628_736_410_763_264, 42]);
        let mut out = Vec::new();
        strings_in(
            &json!({ "ids": ["5", "x", 6], "input": { "ownerId": "77" } }),
            &mut out,
        );
        out.sort_unstable();
        assert_eq!(out, vec![5, 77]);
    }

    #[test]
    fn id_keys() {
        for yes in [
            "id",
            "ids",
            "done",
            "ownerId",
            "setIds",
            "emote_id",
            "set_ids",
            "targetId",
            "addedBy",
            "requested_by",
        ] {
            assert!(is_id_key(yes), "{yes}");
        }
        for no in ["name", "identity", "idle", "valid", "paid", "nearby"] {
            assert!(!is_id_key(no), "{no}");
        }
    }

    #[test]
    fn answers_are_translated_only_in_id_fields() {
        let emote = ids::new_id();
        let account = ids::new_id();
        let badge = ids::new_id();
        let (e, a, b) = (
            ids::to_text(emote),
            ids::to_text(account),
            ids::to_text(badge),
        );
        let mut value = json!({
            "data": {
                "emote": { "id": e, "name": e, "owner": { "id": a }, "badgeId": b },
                "favouriteEmoteIds": [e, b],
                "audit": [{ "targetId": e, "before": format!(r#"{{"owner_id":"{a}","note":"{a}"}}"#), "after": "not json" }],
                "count": 3,
            }
        });
        let mut found = HashSet::new();
        collect_json(&value, &mut found);
        assert_eq!(found, HashSet::from([emote, account, badge]));
        // Badges aren't accounts, emotes or sets: the database has no Snowflake for them.
        let map = HashMap::from([(emote, 11), (account, 22)]);
        replace_json(&mut value, &map);
        let d = &value["data"];
        assert_eq!(d["emote"]["id"], "11");
        assert_eq!(d["emote"]["name"], json!(e), "names are never touched");
        assert_eq!(d["emote"]["owner"]["id"], "22");
        assert_eq!(d["emote"]["badgeId"], json!(b));
        assert_eq!(d["favouriteEmoteIds"], json!(["11", b]));
        assert_eq!(d["audit"][0]["targetId"], "11");
        let before: Value =
            serde_json::from_str(d["audit"][0]["before"].as_str().unwrap()).unwrap();
        assert_eq!(before["owner_id"], "22");
        assert_eq!(before["note"], json!(a));
        assert_eq!(d["audit"][0]["after"], "not json");
        assert_eq!(d["count"], 3);
    }
}
