//! Merging one emote into another (ADR 0040).
//!
//! - **Who:** staff with `emotes.merge` (the built-in Admin role has it). Anyone else gets
//!   `403 forbidden`.
//! - **What moves:** every set membership of the source (channel, personal, global, any set)
//!   names the target, keeping its name and zero-width flag; a set that holds both keeps the
//!   target's membership and drops the source's (listed in the answer as a collapse).
//!   Favourites and "Hide for me" move too. Name, tags and files are the target's.
//! - **The source** is retired: deleted, `merged_into` the target. Its ID answers the target
//!   (REST `301`, GraphQL `mergedInto`); it is gone from search and can't be added anywhere.
//! - **The target** must be approved and not deleted (`merge_target_unavailable`); a target that
//!   was merged itself is followed to the final one; the source can't be the target
//!   (`merge_into_self`) or merged already (`already_merged`).
//! - **One transaction**: both emotes locked in ID order (`FOR NO KEY UPDATE`), then the sets
//!   holding the source in ID order (`FOR UPDATE`, as the set-version triggers lock them, ADR
//!   0035). A deadlock (`40P01`) is retried, at most three times in all.
//! - Audited as `emote.merged` on both emotes; both owners get `emote.merged` (not the one who
//!   merged); every affected set gets its events (`set.emote_removed` for the source,
//!   `set.emote_added` for the target where it moved).

use std::collections::HashMap;

use cyex_tv_auth::Caller;
use cyex_tv_db::DbError;
use cyex_tv_db::emotes::{self, CollapsedMembership, Emote as EmoteRow, MergeOutcome};
use cyex_tv_db::{ids, reads, sets};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    EmoteMergePreview, EmoteMergeResult, EmoteState, EventType, MergeCollapse,
};
use cyex_tv_shared::permissions;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, holds, load_set, not_found, parse_id, publish_member_event};
use crate::errors::{Audit, bad, conflict, forbidden, record_audit, sqlx_error};
use crate::moderation::{build, row};
use crate::state::AppState;

/// How many times a merge runs when Postgres picks it as a deadlock victim.
const ATTEMPTS: usize = 3;

pub const NOT_ALLOWED: &str = "Only staff with emotes.merge can merge emotes.";

fn into_self() -> ApiError {
    bad("merge_into_self", "An emote can't be merged into itself.")
}

fn unavailable() -> ApiError {
    conflict(
        "merge_target_unavailable",
        "That emote can't take another's place: it is deleted, waiting for review, rejected or taken down.",
    )
}

fn already_merged() -> ApiError {
    conflict(
        "already_merged",
        "This emote was merged into another already.",
    )
}

fn is_deadlock(err: &DbError) -> bool {
    matches!(err, DbError::Sqlx(sqlx::Error::Database(e)) if e.code().as_deref() == Some("40P01"))
}

/// One try of a merge: done, or a deadlock to retry (failures are errors).
enum Try<T> {
    Done(T),
    Retry,
}

fn db<T>(r: Result<T, DbError>) -> Result<Result<T, ()>, ApiError> {
    match r {
        Ok(v) => Ok(Ok(v)),
        Err(e) if is_deadlock(&e) => Ok(Err(())),
        Err(e) => Err(db_error(e)),
    }
}

/// The two emotes, checked and locked: the source (live, not merged) and the final target
/// (approved, live). `asked` is the target the caller named; `target` is where it led.
struct Pair {
    source: EmoteRow,
    target: EmoteRow,
    followed: bool,
}

/// Locks and checks both emotes. `Ok(None)` is a deadlock to retry.
async fn pair(conn: &mut PgConnection, source: Uuid, asked: Uuid) -> ApiResult<Option<Pair>> {
    if source == asked {
        return Err(into_self());
    }
    let target = match db(emotes::final_target(conn, asked).await)? {
        Ok(found) => found.unwrap_or(asked),
        Err(()) => return Ok(None),
    };
    if target == source {
        return Err(into_self());
    }
    let rows = match db(emotes::lock_for_merge(conn, source, target).await)? {
        Ok(rows) => rows,
        Err(()) => return Ok(None),
    };
    let by_id: HashMap<Uuid, (EmoteRow, Option<Uuid>)> =
        rows.into_iter().map(|(e, m)| (e.id, (e, m))).collect();
    let Some((s, s_merged)) = by_id.get(&source).cloned() else {
        return Err(not_found());
    };
    if s_merged.is_some() {
        return Err(already_merged());
    }
    if s.deleted_at.is_some() {
        return Err(not_found());
    }
    let Some((t, t_merged)) = by_id.get(&target).cloned() else {
        return Err(not_found());
    };
    // Merged between the lookup and the lock: the next try follows it.
    if t_merged.is_some() {
        return Ok(None);
    }
    if t.deleted_at.is_some() || t.state != EmoteState::Approved {
        return Err(unavailable());
    }
    Ok(Some(Pair {
        source: s,
        target: t,
        followed: target != asked,
    }))
}

async fn require(conn: &mut PgConnection, caller: &Caller) -> ApiResult<()> {
    if holds(conn, caller, permissions::EMOTES_MERGE).await? {
        Ok(())
    } else {
        Err(forbidden(NOT_ALLOWED))
    }
}

async fn collapses(
    conn: &mut PgConnection,
    rows: &[CollapsedMembership],
) -> ApiResult<Vec<MergeCollapse>> {
    let set_ids: Vec<Uuid> = rows.iter().map(|c| c.set_id).collect();
    let names: HashMap<Uuid, String> = sets::get_many(conn, &set_ids)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|s| (s.id, s.name))
        .collect();
    Ok(rows
        .iter()
        .map(|c| MergeCollapse {
            set_id: ids::to_text(c.set_id),
            set_name: names.get(&c.set_id).cloned().unwrap_or_default(),
            kept_name: c.kept_name.clone(),
            dropped_name: c.dropped_name.clone(),
        })
        .collect())
}

async fn built(
    state: &AppState,
    conn: &mut PgConnection,
    id: Uuid,
) -> ApiResult<cyex_tv_shared::model::Emote> {
    let found = row(conn, id).await?.ok_or_else(not_found)?;
    build(state, conn, &found).await
}

/// `emoteMergePreview`: what merging `id` into `into_id` would do, with both emotes as they read
/// now. Needs `emotes.merge`; the same refusals as the merge.
pub async fn preview(
    state: &AppState,
    caller: &Caller,
    id: &str,
    into_id: &str,
) -> ApiResult<EmoteMergePreview> {
    let source = parse_id(id)?;
    let asked = parse_id(into_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller).await?;
    for _ in 0..ATTEMPTS {
        let mut tx = conn.begin().await.map_err(sqlx_error)?;
        let Some(p) = pair(&mut tx, source, asked).await? else {
            continue;
        };
        let plan = emotes::merge_plan(&mut tx, p.source.id, p.target.id)
            .await
            .map_err(db_error)?;
        let collapses = collapses(&mut tx, &plan.collapsed).await?;
        let source_view = built(state, &mut tx, p.source.id).await?;
        let target_view = built(state, &mut tx, p.target.id).await?;
        tx.rollback().await.map_err(sqlx_error)?;
        return Ok(EmoteMergePreview {
            source: source_view,
            target: target_view,
            followed: p.followed,
            moves: u32::try_from(plan.moved.len()).unwrap_or(u32::MAX),
            collapses,
        });
    }
    Err(ApiError::internal())
}

/// The notification payload, names as they read now.
async fn payload(
    conn: &mut PgConnection,
    role: &str,
    p: &Pair,
    actor: Uuid,
    outcome: &MergeOutcome,
) -> ApiResult<Value> {
    let mut who: Vec<Uuid> = vec![actor];
    who.extend(p.source.owner_account_id);
    who.extend(p.target.owner_account_id);
    let names = reads::summaries(conn, &who).await.map_err(db_error)?;
    Ok(json!({
        "role": role,
        "source_id": ids::to_text(p.source.id),
        "source_name": p.source.name,
        "emote_id": ids::to_text(p.target.id),
        "name": p.target.name,
        "target_id": ids::to_text(p.target.id),
        "target_name": p.target.name,
        "actor_id": ids::to_text(actor),
        "actor_name": names.get(&actor).map(|s| s.display_name.clone()),
        "moved": outcome.moved.len(),
        "collapsed": outcome.collapsed.len(),
    }))
}

async fn audit(
    conn: &mut PgConnection,
    caller: &Caller,
    p: &Pair,
    outcome: &MergeOutcome,
) -> ApiResult<()> {
    let source = ids::to_text(p.source.id);
    let target = ids::to_text(p.target.id);
    let collapsed: Vec<Value> = outcome
        .collapsed
        .iter()
        .map(|c| {
            json!({
                "set_id": ids::to_text(c.set_id),
                "kept_name": c.kept_name,
                "dropped_name": c.dropped_name,
            })
        })
        .collect();
    let after = json!({
        "source_id": source,
        "target_id": target,
        "source_name": p.source.name,
        "target_name": p.target.name,
        "moved": outcome.moved.len(),
        "collapsed": collapsed,
        "via": permissions::EMOTES_MERGE,
    });
    record_audit(
        conn,
        caller.account_id,
        Audit::new("emote.merged", "emote", &source)
            .change(
                Some(json!({
                    "name": p.source.name,
                    "owner_id": p.source.owner_account_id.map(ids::to_text),
                    "state": p.source.state,
                })),
                Some(after.clone()),
            )
            .subject(p.source.owner_account_id),
    )
    .await?;
    record_audit(
        conn,
        caller.account_id,
        Audit::new("emote.merged", "emote", &target)
            .change(None, Some(after))
            .subject(p.target.owner_account_id),
    )
    .await
}

async fn events(conn: &mut PgConnection, p: &Pair, outcome: &MergeOutcome) -> ApiResult<()> {
    // The source is retired by now, so its removals go out without a name (as an ID only).
    for m in &outcome.moved {
        let set = load_set(conn, m.set_id).await?;
        publish_member_event(conn, &set, EventType::SetEmoteRemoved, p.source.id, None).await?;
        publish_member_event(
            conn,
            &set,
            EventType::SetEmoteAdded,
            p.target.id,
            Some(&m.name),
        )
        .await?;
    }
    for c in &outcome.collapsed {
        let set = load_set(conn, c.set_id).await?;
        publish_member_event(conn, &set, EventType::SetEmoteRemoved, p.source.id, None).await?;
    }
    Ok(())
}

async fn notify(
    conn: &mut PgConnection,
    caller: &Caller,
    p: &Pair,
    outcome: &MergeOutcome,
) -> ApiResult<()> {
    let mut told: Vec<Uuid> = Vec::new();
    let both = p.source.owner_account_id == p.target.owner_account_id;
    for (owner, role) in [
        (
            p.source.owner_account_id,
            if both { "both" } else { "source" },
        ),
        (p.target.owner_account_id, "target"),
    ] {
        let Some(owner) = owner else { continue };
        if owner == caller.account_id || told.contains(&owner) {
            continue;
        }
        told.push(owner);
        let body = payload(conn, role, p, caller.account_id, outcome).await?;
        crate::account::notify(conn, owner, "emote.merged", &body).await?;
    }
    Ok(())
}

/// One try of the merge in its own transaction.
async fn attempt(
    state: &AppState,
    conn: &mut PgConnection,
    caller: &Caller,
    source: Uuid,
    asked: Uuid,
) -> ApiResult<Try<(Pair, MergeOutcome)>> {
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Some(p) = pair(&mut tx, source, asked).await? else {
        return Ok(Try::Retry);
    };
    let outcome = match db(emotes::merge(
        &mut tx,
        p.source.id,
        p.target.id,
        Some(caller.account_id),
    )
    .await)?
    {
        Ok(o) => o,
        Err(()) => return Ok(Try::Retry),
    };
    audit(&mut tx, caller, &p, &outcome).await?;
    events(&mut tx, &p, &outcome).await?;
    notify(&mut tx, caller, &p, &outcome).await?;
    match tx.commit().await {
        Ok(()) => {}
        Err(e) => {
            let e = DbError::Sqlx(e);
            if is_deadlock(&e) {
                return Ok(Try::Retry);
            }
            return Err(db_error(e));
        }
    }
    state.forget_cached(&[
        crate::read_cache::emote_key(p.source.id),
        crate::read_cache::emote_key(p.target.id),
    ]);
    Ok(Try::Done((p, outcome)))
}

/// `mergeEmote`: merges `id` into `into_id` (ADR 0040). Needs `emotes.merge`.
pub async fn merge(
    state: &AppState,
    caller: &Caller,
    id: &str,
    into_id: &str,
) -> ApiResult<EmoteMergeResult> {
    let source = parse_id(id)?;
    let asked = parse_id(into_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller).await?;
    for _ in 0..ATTEMPTS {
        let Try::Done((p, outcome)) = attempt(state, &mut conn, caller, source, asked).await?
        else {
            continue;
        };
        let collapsed = collapses(&mut conn, &outcome.collapsed).await?;
        let target = built(state, &mut conn, p.target.id).await?;
        return Ok(EmoteMergeResult {
            source_id: ids::to_text(p.source.id),
            target,
            moved: u32::try_from(outcome.moved.len()).unwrap_or(u32::MAX),
            collapsed,
        });
    }
    tracing::warn!("emote merge gave up after {ATTEMPTS} deadlocks");
    Err(ApiError::internal())
}
