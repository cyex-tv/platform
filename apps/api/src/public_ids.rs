//! Snowflake public IDs at the API's edges (ADR 0027). Accounts, emotes and sets are shown by
//! their Snowflake and taken by it or by their old ULID; everything inside works on uuids.
//!
//! GraphQL: [`PublicIds`] reads the Snowflake-shaped strings of a request (its text and
//! variables), looks them up once, and runs the request with them, so `ids::from_text` and
//! `errors::parse_id` take both forms in every resolver. The answer's ID fields then get the
//! Snowflakes in place of ULIDs. REST: [`publish`] translates a JSON answer, and
//! [`canonical`] an ID from a path or form.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery,
};
use async_graphql::parser::types::ExecutableDocument;
use async_graphql::{Name, ServerResult, Value as GqlValue, Variables};
use cyex_tv_db::public_ids;
use cyex_tv_shared::error::ApiError;
use serde::Serialize;
use uuid::Uuid;

use crate::errors::{conn, db_error};
use crate::state::AppState;

/// The schema extension that translates IDs both ways.
pub struct PublicIds;

impl ExtensionFactory for PublicIds {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(PublicIdsRequest::default())
    }
}

#[derive(Default)]
struct PublicIdsRequest {
    /// The request's Snowflake-shaped strings, read while parsing.
    found: Mutex<Vec<i64>>,
    /// The request's ULID-shaped strings (at most [`MAX_ULIDS`]): old-form IDs, or other ULIDs.
    ulids: Mutex<Vec<Uuid>>,
}

/// How many ULID-shaped strings of a request are checked for the sunset notice.
const MAX_ULIDS: usize = 100;

fn ulids_in(value: &GqlValue, out: &mut Vec<Uuid>) {
    match value {
        GqlValue::String(s) => out.extend(ulid_of(s)),
        GqlValue::List(items) => items.iter().for_each(|v| ulids_in(v, out)),
        GqlValue::Object(map) => map.values().for_each(|v| ulids_in(v, out)),
        _ => {}
    }
}

/// ULID-shaped string literals (`"01J…"`) in GraphQL text.
fn ulid_literals_in(text: &str, out: &mut Vec<Uuid>) {
    out.extend(text.split('"').skip(1).step_by(2).filter_map(ulid_of));
}

fn strings_in(value: &GqlValue, out: &mut Vec<i64>) {
    match value {
        GqlValue::String(s) => out.extend(public_ids::parse(s)),
        GqlValue::List(items) => items.iter().for_each(|v| strings_in(v, out)),
        GqlValue::Object(map) => map.values().for_each(|v| strings_in(v, out)),
        _ => {}
    }
}

#[async_trait::async_trait]
impl Extension for PublicIdsRequest {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let mut found = Vec::new();
        public_ids::literals_in(query, &mut found);
        for value in variables.values() {
            strings_in(value, &mut found);
        }
        found.sort_unstable();
        found.dedup();
        if let Ok(mut slot) = self.found.lock() {
            *slot = found;
        }
        let mut ulids = Vec::new();
        ulid_literals_in(query, &mut ulids);
        for value in variables.values() {
            ulids_in(value, &mut ulids);
        }
        ulids.sort_unstable();
        ulids.dedup();
        ulids.truncate(MAX_ULIDS);
        if let Ok(mut slot) = self.ulids.lock() {
            *slot = ulids;
        }
        next.run(ctx, query, variables).await
    }

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> async_graphql::Response {
        let Some(state) = ctx.data_opt::<AppState>() else {
            return next.run(ctx, operation_name).await;
        };
        let found = self
            .found
            .lock()
            .map(|mut slot| std::mem::take(&mut *slot))
            .unwrap_or_default();
        let resolved = match resolve(state, &found).await {
            Ok(resolved) => resolved,
            // Unresolved Snowflakes name nothing: the resolvers answer `not_found`.
            Err(err) => {
                tracing::warn!(code = err.code, "resolving public IDs failed");
                HashMap::new()
            }
        };
        let mut response = public_ids::resolving(resolved, next.run(ctx, operation_name)).await;
        let ulids = self
            .ulids
            .lock()
            .map(|mut slot| std::mem::take(&mut *slot))
            .unwrap_or_default();
        if old_form_used(state, &ulids).await {
            response.extensions.insert(
                "deprecations".to_owned(),
                GqlValue::List(vec![GqlValue::String(
                    crate::meta::ULID_ID_INPUT.to_owned(),
                )]),
            );
        }
        if let Err(err) = publish_value(state, &mut response.data).await {
            // The ULIDs stay: still valid IDs, only in the old form.
            tracing::warn!(code = err.code, "translating public IDs failed");
        }
        response
    }
}

/// Whether any of these ULIDs names an account, emote or set (other ULIDs are not deprecated),
/// which is then noted for the request's `Deprecation` headers (ADR 0027).
async fn old_form_used(state: &AppState, ulids: &[Uuid]) -> bool {
    if ulids.is_empty() {
        return false;
    }
    let Ok(mut conn) = conn(state).await else {
        return false;
    };
    let used = public_ids::publics(&mut conn, ulids)
        .await
        .is_ok_and(|found| !found.is_empty());
    if used {
        crate::meta::note(crate::meta::ULID_ID_INPUT);
    }
    used
}

async fn resolve(state: &AppState, found: &[i64]) -> Result<HashMap<i64, Uuid>, ApiError> {
    if found.is_empty() {
        return Ok(HashMap::new());
    }
    let mut conn = conn(state).await?;
    public_ids::resolve(&mut conn, found)
        .await
        .map_err(db_error)
}

fn ulid_of(text: &str) -> Option<Uuid> {
    cyex_tv_db::ids::is_ulid(text)
        .then(|| cyex_tv_db::ids::from_text(text))
        .flatten()
}

fn embedded(text: &str) -> Option<serde_json::Value> {
    let trimmed = text.trim_start();
    (trimmed.starts_with('{') || trimmed.starts_with('['))
        .then(|| serde_json::from_str(text).ok())
        .flatten()
}

fn collect(value: &GqlValue, out: &mut HashSet<Uuid>) {
    match value {
        GqlValue::Object(map) => {
            for (key, v) in map {
                let key = key.as_str();
                match v {
                    GqlValue::String(s) if public_ids::is_id_key(key) => out.extend(ulid_of(s)),
                    GqlValue::List(items) if public_ids::is_id_key(key) => {
                        for item in items {
                            match item {
                                GqlValue::String(s) => out.extend(ulid_of(s)),
                                other => collect(other, out),
                            }
                        }
                    }
                    GqlValue::String(s) if public_ids::is_embedded_json_key(key) => {
                        if let Some(inner) = embedded(s) {
                            public_ids::collect_json(&inner, out);
                        }
                    }
                    other => collect(other, out),
                }
            }
        }
        GqlValue::List(items) => items.iter().for_each(|v| collect(v, out)),
        _ => {}
    }
}

fn swap(text: &mut String, map: &HashMap<Uuid, i64>) {
    if let Some(public) = ulid_of(text).and_then(|id| map.get(&id)) {
        *text = public.to_string();
    }
}

fn replace(value: &mut GqlValue, map: &HashMap<Uuid, i64>) {
    match value {
        GqlValue::Object(obj) => {
            for (key, v) in obj.iter_mut() {
                let key: &Name = key;
                let id_key = public_ids::is_id_key(key.as_str());
                let json_key = public_ids::is_embedded_json_key(key.as_str());
                match v {
                    GqlValue::String(s) if id_key => swap(s, map),
                    GqlValue::List(items) if id_key => {
                        for item in items {
                            match item {
                                GqlValue::String(s) => swap(s, map),
                                other => replace(other, map),
                            }
                        }
                    }
                    GqlValue::String(s) if json_key => {
                        if let Some(mut inner) = embedded(s) {
                            public_ids::replace_json(&mut inner, map);
                            if let Ok(text) = serde_json::to_string(&inner) {
                                *s = text;
                            }
                        }
                    }
                    other => replace(other, map),
                }
            }
        }
        GqlValue::List(items) => items.iter_mut().for_each(|v| replace(v, map)),
        _ => {}
    }
}

/// Translates a GraphQL answer in place.
pub async fn publish_value(state: &AppState, value: &mut GqlValue) -> Result<(), ApiError> {
    let mut found = HashSet::new();
    collect(value, &mut found);
    if found.is_empty() {
        return Ok(());
    }
    let found: Vec<Uuid> = found.into_iter().collect();
    let mut conn = conn(state).await?;
    let map = public_ids::publics(&mut conn, &found)
        .await
        .map_err(db_error)?;
    drop(conn);
    if !map.is_empty() {
        replace(value, &map);
    }
    Ok(())
}

/// A REST answer as JSON, with Snowflakes in its ID fields.
pub async fn publish<T: Serialize>(
    state: &AppState,
    value: &T,
) -> Result<serde_json::Value, ApiError> {
    let mut json = serde_json::to_value(value).map_err(|_| ApiError::internal())?;
    let mut conn = conn(state).await?;
    public_ids::publish_json(&mut conn, &mut json)
        .await
        .map_err(db_error)?;
    Ok(json)
}

/// An ID from a REST path or form, as the ULID text the handlers parse: a known Snowflake
/// becomes its row's ULID; anything else is passed on as it came.
pub async fn canonical(state: &AppState, text: &str) -> Result<String, ApiError> {
    if public_ids::parse(text.trim()).is_none() {
        if let Some(id) = ulid_of(text.trim()) {
            old_form_used(state, &[id]).await;
        }
        return Ok(text.to_owned());
    }
    let mut conn = conn(state).await?;
    public_ids::canonical(&mut conn, text)
        .await
        .map_err(db_error)
}

/// [`canonical`] for many IDs in one lookup (the bulk reads, ADR 0032), in the order given.
pub async fn canonical_many(state: &AppState, texts: &[String]) -> Result<Vec<String>, ApiError> {
    let publics: Vec<i64> = texts
        .iter()
        .filter_map(|t| public_ids::parse(t.trim()))
        .collect();
    let ulids: Vec<Uuid> = texts.iter().filter_map(|t| ulid_of(t.trim())).collect();
    if !ulids.is_empty() {
        old_form_used(state, &ulids).await;
    }
    if publics.is_empty() {
        return Ok(texts.to_vec());
    }
    let map = resolve(state, &publics).await?;
    Ok(texts
        .iter()
        .map(|t| {
            public_ids::parse(t.trim())
                .and_then(|p| map.get(&p))
                .map_or_else(|| t.clone(), |id| cyex_tv_db::ids::to_text(*id))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use async_graphql::indexmap::IndexMap;

    use super::*;

    fn object(fields: Vec<(&str, GqlValue)>) -> GqlValue {
        GqlValue::Object(
            fields
                .into_iter()
                .map(|(k, v)| (Name::new(k), v))
                .collect::<IndexMap<_, _>>(),
        )
    }

    #[test]
    fn graphql_answers_are_translated_in_id_fields_only() {
        let set = cyex_tv_db::ids::new_id();
        let role = cyex_tv_db::ids::new_id();
        let s = cyex_tv_db::ids::to_text(set);
        let r = cyex_tv_db::ids::to_text(role);
        let mut value = object(vec![(
            "data",
            object(vec![
                ("id", GqlValue::String(s.clone())),
                ("name", GqlValue::String(s.clone())),
                ("roleId", GqlValue::String(r.clone())),
                (
                    "setIds",
                    GqlValue::List(vec![GqlValue::String(s.clone()), GqlValue::Null]),
                ),
                ("after", GqlValue::String(format!(r#"{{"set_id":"{s}"}}"#))),
            ]),
        )]);
        let mut found = HashSet::new();
        collect(&value, &mut found);
        assert_eq!(found, HashSet::from([set, role]));
        replace(&mut value, &HashMap::from([(set, 99)]));
        let GqlValue::Object(top) = &value else {
            unreachable!()
        };
        let GqlValue::Object(d) = &top["data"] else {
            unreachable!()
        };
        assert_eq!(d["id"], GqlValue::String("99".into()));
        assert_eq!(d["name"], GqlValue::String(s), "only ID fields change");
        assert_eq!(d["roleId"], GqlValue::String(r));
        assert_eq!(
            d["setIds"],
            GqlValue::List(vec![GqlValue::String("99".into()), GqlValue::Null])
        );
        assert_eq!(d["after"], GqlValue::String(r#"{"set_id":"99"}"#.into()));
    }

    #[test]
    fn variables_are_read_for_snowflakes() {
        let mut out = Vec::new();
        strings_in(
            &object(vec![
                ("setId", GqlValue::String("1314628736410763264".into())),
                (
                    "ids",
                    GqlValue::List(vec![GqlValue::String("12".into()), GqlValue::Null]),
                ),
                ("name", GqlValue::String("wave".into())),
            ]),
            &mut out,
        );
        assert_eq!(out, vec![1_314_628_736_410_763_264, 12]);
    }
}
