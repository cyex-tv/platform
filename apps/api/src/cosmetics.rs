//! Chat cosmetics (ADR 0015): the badge and paint catalog, who owns what, and what each account
//! shows. Shared by GraphQL and the REST routes.
//!
//! - `cosmetics.manage`: upload pictures, create, change and delete badges and paints.
//! - `cosmetics.grant`: grant and revoke, read entitlements.
//! - An account picks what it shows from what it owns; its account editors with `profile` may
//!   pick for it (ADR 0014). Editors never grant.
//!
//! Every staff write is audited in its transaction. Catalog changes publish `cosmetic.*` on the
//! `cosmetics` topic; a changed selection publishes `user.cosmetics_updated` on the account's
//! user topics (ADR 0005).

use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use bytes::Bytes;
use cyex_tv_auth::Caller;
use cyex_tv_db::cosmetics::{self as db, Image, NewImage, PaintFields};
use cyex_tv_db::{DbError, accounts, audit, ids, jobs, outbox, reads};
use cyex_tv_media::{Limits, Probe};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, Badge, CosmeticCatalog, CosmeticImage, CosmeticKind, CreateBadge, EmoteFile,
    Entitlement, EntitlementPage, EntitlementSource, EventType, GrantCosmetic, ImageFormat,
    OwnedCosmetics, Paint, PaintInput, PaintKind, PaintShadow, PaintShape, PaintStop, Platform,
    UpdateBadge, UserCosmetics,
};
use cyex_tv_shared::{permissions, text};
use cyex_tv_storage::{Put, cosmetic_original_key, cosmetic_prefix, sigv4};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{LOOKUP_LIMIT, db_error, forbidden, holds, not_found, parse_id};
use crate::editors::has_flag;
use crate::state::AppState;
use crate::upload::media_error;
use crate::views::time;

/// The job the worker makes a picture's files with.
pub const TRANSCODE_JOB: &str = "cosmetic.transcode";
/// Badge pictures must be square and at least this many px (4x of 18 px).
pub const BADGE_MIN_SIDE: u32 = 72;
/// Paint pictures must be at least this tall (1x).
pub const PAINT_MIN_HEIGHT: u32 = 64;
pub const NAME_MAX: usize = 32;
pub const TOOLTIP_MAX: usize = 64;
pub const REASON_MAX: usize = 500;
pub const STOPS_MIN: usize = 2;
pub const STOPS_MAX: usize = 10;
pub const SHADOWS_MAX: usize = 4;
/// Shadow offsets and radius, in px.
pub const SHADOW_OFFSET_MAX: f64 = 16.0;
pub const SHADOW_RADIUS_MAX: f64 = 16.0;
pub const ENTITLEMENTS_PAGE: u32 = 50;
pub const ENTITLEMENTS_PAGE_MAX: u32 = 100;

// Errors

fn bad(code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

async fn acquire(state: &AppState) -> ApiResult<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.db.acquire().await.map_err(sqlx_error)
}

async fn require(conn: &mut PgConnection, caller: &Caller, permission: &str) -> ApiResult<()> {
    if holds(conn, caller, permission).await? {
        Ok(())
    } else {
        Err(forbidden(&format!("This needs {permission}.")))
    }
}

/// Cosmetics writes per account (ADR 0015).
fn rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    state
        .limits
        .cosmetics
        .check(&ids::to_text(caller.account_id))
        .map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "That's a lot of cosmetics changes. Try again in a while.",
            )
        })
}

// Checks on input (pure, unit-tested below)

/// Badge and paint names: 1 to 32 characters after trimming, nothing hidden or controlling.
pub fn cosmetic_name(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > NAME_MAX || text::has_unsafe(name, false) {
        return Err(bad("bad_name", "Names are 1 to 32 characters."));
    }
    Ok(name.to_owned())
}

pub fn tooltip(raw: &str) -> ApiResult<String> {
    let tip = raw.trim();
    let len = tip.chars().count();
    if len == 0 || len > TOOLTIP_MAX || text::has_unsafe(tip, false) {
        return Err(bad("bad_tooltip", "Tooltips are 1 to 64 characters."));
    }
    Ok(tip.to_owned())
}

/// Grant reasons: 1 to 500 characters; new lines allowed.
pub fn reason(raw: &str) -> ApiResult<String> {
    let r = raw.trim();
    let len = r.chars().count();
    if len == 0 || len > REASON_MAX || text::has_unsafe(r, true) {
        return Err(bad("bad_reason", "Give a reason of 1 to 500 characters."));
    }
    Ok(r.to_owned())
}

/// `#RRGGBB` or `#RRGGBBAA`, stored upper case.
pub fn color(raw: &str) -> ApiResult<String> {
    let c = raw.trim();
    let hex = c.strip_prefix('#').unwrap_or("");
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad(
            "bad_color",
            format!("{c:?} is not a colour. Use #RRGGBB or #RRGGBBAA."),
        ));
    }
    Ok(format!("#{}", hex.to_ascii_uppercase()))
}

pub fn stops(raw: &[PaintStop]) -> ApiResult<Vec<PaintStop>> {
    if raw.len() < STOPS_MIN || raw.len() > STOPS_MAX {
        return Err(bad("bad_stops", "Gradients take 2 to 10 stops."));
    }
    let mut last = 0.0_f64;
    let mut out = Vec::with_capacity(raw.len());
    for stop in raw {
        if !stop.at.is_finite() || !(0.0..=1.0).contains(&stop.at) || stop.at < last {
            return Err(bad(
                "bad_stops",
                "Stops go from 0 to 1, each at or after the one before.",
            ));
        }
        last = stop.at;
        out.push(PaintStop {
            at: stop.at,
            color: color(&stop.color)?,
        });
    }
    Ok(out)
}

pub fn shadows(raw: &[PaintShadow]) -> ApiResult<Vec<PaintShadow>> {
    if raw.len() > SHADOWS_MAX {
        return Err(bad("bad_shadows", "At most 4 shadows."));
    }
    raw.iter()
        .map(|s| {
            let offset_ok = |v: f64| v.is_finite() && v.abs() <= SHADOW_OFFSET_MAX;
            if !offset_ok(s.x)
                || !offset_ok(s.y)
                || !s.radius.is_finite()
                || !(0.0..=SHADOW_RADIUS_MAX).contains(&s.radius)
            {
                return Err(bad(
                    "bad_shadows",
                    "Shadow offsets are -16 to 16 px, the radius 0 to 16 px.",
                ));
            }
            Ok(PaintShadow {
                x: s.x,
                y: s.y,
                radius: s.radius,
                color: color(&s.color)?,
            })
        })
        .collect()
}

/// A paint's fields, checked: everything but the image, whose readiness needs the database.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedPaint {
    pub name: String,
    pub kind: PaintKind,
    pub angle: u16,
    pub repeat: bool,
    pub shape: PaintShape,
    pub color: Option<String>,
    pub stops: Vec<PaintStop>,
    pub shadows: Vec<PaintShadow>,
    pub image_id: Option<Uuid>,
}

pub fn paint_input(input: &PaintInput) -> ApiResult<CheckedPaint> {
    let name = cosmetic_name(&input.name)?;
    let angle = input.angle.unwrap_or(90);
    if angle > 360 {
        return Err(bad("bad_angle", "The angle is 0 to 360 degrees."));
    }
    let fallback = match input.color.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(c) => Some(color(c)?),
    };
    let image = input
        .image_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let (stops, image_id) = match input.kind {
        PaintKind::LinearGradient | PaintKind::RadialGradient => {
            if image.is_some() {
                return Err(bad("bad_image", "Only IMAGE paints take an image."));
            }
            (stops(&input.stops)?, None)
        }
        PaintKind::Image => {
            if !input.stops.is_empty() {
                return Err(bad("bad_stops", "IMAGE paints take no stops."));
            }
            let raw = image.ok_or_else(|| bad("bad_image", "IMAGE paints need an imageId."))?;
            (Vec::new(), Some(parse_id(raw)?))
        }
    };
    Ok(CheckedPaint {
        name,
        kind: input.kind,
        angle,
        repeat: input.repeat.unwrap_or(false),
        shape: input.shape.unwrap_or_default(),
        color: fallback,
        stops,
        shadows: shadows(&input.shadows)?,
        image_id,
    })
}

/// Checks a picture for its kind: badges square and at least 72×72, paints at least 64 tall.
pub fn check_dimensions(kind: CosmeticKind, width: u32, height: u32) -> ApiResult<()> {
    let ok = match kind {
        CosmeticKind::Badge => width == height && width >= BADGE_MIN_SIDE,
        CosmeticKind::Paint => height >= PAINT_MIN_HEIGHT,
    };
    if ok {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "bad_dimensions",
        match kind {
            CosmeticKind::Badge => "Badge pictures are square and at least 72×72 px.",
            CosmeticKind::Paint => "Paint pictures are at least 64 px tall.",
        },
    ))
}

/// Platform user IDs from a lookup: at most 100, digits only, each once.
pub fn user_ids(raw: &[String]) -> ApiResult<Vec<String>> {
    if raw.len() > LOOKUP_LIMIT {
        return Err(bad("too_many", "Look up at most 100 users at once."));
    }
    let mut seen = HashSet::new();
    Ok(raw
        .iter()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()))
        .filter(|id| seen.insert(id.clone()))
        .collect())
}

// Views

fn files_of(variants: &Value) -> Vec<EmoteFile> {
    match variants {
        Value::Array(items) => items
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn kind_of(image: &Image) -> CosmeticKind {
    CosmeticKind::parse(&image.kind).unwrap_or(CosmeticKind::Badge)
}

pub fn image_view(image: &Image, cdn_url: &str) -> CosmeticImage {
    let files = files_of(&image.variants);
    let kind = kind_of(image);
    let ready = files.iter().any(|f| f.format == ImageFormat::Webp);
    CosmeticImage {
        id: ids::to_text(image.id),
        kind,
        host: format!(
            "{cdn_url}/{}",
            cosmetic_prefix(kind.as_str(), &sigv4::hex(&image.sha256))
        ),
        files,
        animated: image.animated,
        width: u32::try_from(image.width).unwrap_or(0),
        height: u32::try_from(image.height).unwrap_or(0),
        ready,
        created_at: time(image.created_at),
    }
}

fn badge_view(row: &db::Badge, images: &HashMap<Uuid, Image>, cdn: &str) -> Option<Badge> {
    Some(Badge {
        id: ids::to_text(row.id),
        name: row.name.clone(),
        tooltip: row.tooltip.clone(),
        image: image_view(images.get(&row.image_id)?, cdn),
        created_at: time(row.created_at),
        updated_at: time(row.updated_at),
    })
}

fn paint_view(row: &db::Paint, images: &HashMap<Uuid, Image>, cdn: &str) -> Paint {
    Paint {
        id: ids::to_text(row.id),
        name: row.name.clone(),
        kind: PaintKind::parse(&row.kind).unwrap_or(PaintKind::LinearGradient),
        angle: u16::try_from(row.angle).unwrap_or(90),
        repeat: row.repeat,
        shape: PaintShape::parse(&row.shape).unwrap_or_default(),
        color: row.color.clone(),
        stops: serde_json::from_value(row.stops.clone()).unwrap_or_default(),
        shadows: serde_json::from_value(row.shadows.clone()).unwrap_or_default(),
        image: row
            .image_id
            .and_then(|id| images.get(&id))
            .map(|i| image_view(i, cdn)),
        created_at: time(row.created_at),
        updated_at: time(row.updated_at),
    }
}

async fn images_for(conn: &mut PgConnection, ids: Vec<Uuid>) -> ApiResult<HashMap<Uuid, Image>> {
    Ok(db::images(conn, &ids)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| (i.id, i))
        .collect())
}

/// Badges and paints (all, or only the given IDs), as the API shows them.
pub(crate) async fn build(
    conn: &mut PgConnection,
    cdn: &str,
    badge_ids: Option<&[Uuid]>,
    paint_ids: Option<&[Uuid]>,
) -> ApiResult<(Vec<Badge>, Vec<Paint>)> {
    let badges = db::badges(conn, badge_ids).await.map_err(db_error)?;
    let paints = db::paints(conn, paint_ids).await.map_err(db_error)?;
    let image_ids: Vec<Uuid> = badges
        .iter()
        .map(|b| b.image_id)
        .chain(paints.iter().filter_map(|p| p.image_id))
        .collect();
    let images = images_for(conn, image_ids).await?;
    Ok((
        badges
            .iter()
            .filter_map(|b| badge_view(b, &images, cdn))
            .collect(),
        paints.iter().map(|p| paint_view(p, &images, cdn)).collect(),
    ))
}

async fn one_badge(conn: &mut PgConnection, cdn: &str, id: Uuid) -> ApiResult<Badge> {
    build(conn, cdn, Some(&[id]), Some(&[]))
        .await?
        .0
        .pop()
        .ok_or_else(not_found)
}

async fn one_paint(conn: &mut PgConnection, cdn: &str, id: Uuid) -> ApiResult<Paint> {
    build(conn, cdn, Some(&[]), Some(&[id]))
        .await?
        .1
        .pop()
        .ok_or_else(not_found)
}

fn summary(map: &HashMap<Uuid, reads::Summary>, id: Uuid) -> AccountSummary {
    match map.get(&id) {
        Some(s) => AccountSummary {
            id: ids::to_text(s.account_id),
            display_name: s.display_name.clone(),
            avatar: s.avatar.clone(),
        },
        None => AccountSummary {
            id: ids::to_text(id),
            display_name: String::new(),
            avatar: None,
        },
    }
}

// Events and audit

async fn publish_cosmetic(
    conn: &mut PgConnection,
    event_type: EventType,
    kind: CosmeticKind,
    id: Uuid,
    cosmetic: Option<Value>,
) -> ApiResult<()> {
    let mut payload = json!({ "cosmetic_kind": kind.as_str(), "cosmetic_id": ids::to_text(id) });
    if let Some(c) = cosmetic {
        payload["cosmetic"] = c;
    }
    outbox::publish(
        conn,
        &outbox::NewEvent {
            event_type,
            topics: vec![outbox::COSMETICS_TOPIC.to_owned()],
            set_id: None,
            payload,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

/// `user.cosmetics_updated` on every identity of the account.
pub(crate) async fn publish_selection(
    conn: &mut PgConnection,
    account: Uuid,
    (badge, paint): (Option<Uuid>, Option<Uuid>),
) -> ApiResult<()> {
    let topics: Vec<String> = accounts::identities(conn, account)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| outbox::user_topic(i.platform, &i.platform_user_id))
        .collect();
    if topics.is_empty() {
        return Ok(());
    }
    outbox::publish(
        conn,
        &outbox::NewEvent {
            event_type: EventType::UserCosmeticsUpdated,
            topics,
            set_id: None,
            payload: json!({
                "badge_id": badge.map(ids::to_text),
                "paint_id": paint.map(ids::to_text),
            }),
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn write_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    target_kind: &str,
    target_id: Uuid,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    audit::write(
        conn,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind,
            target_id: &ids::to_text(target_id),
            before,
            after,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

// Public reads

pub async fn catalog(state: &AppState) -> ApiResult<CosmeticCatalog> {
    let mut conn = acquire(state).await?;
    let (badges, paints) = build(&mut conn, &state.cdn_url, None, None).await?;
    Ok(CosmeticCatalog { badges, paints })
}

/// What up to 100 chatters show. Only those showing something are listed.
pub async fn user_cosmetics(
    state: &AppState,
    platform: Platform,
    raw_ids: &[String],
) -> ApiResult<Vec<UserCosmetics>> {
    let wanted = user_ids(raw_ids)?;
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut conn = acquire(state).await?;
    Ok(db::selections_for_users(&mut conn, platform, &wanted)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|s| UserCosmetics {
            platform_user_id: s.platform_user_id,
            badge_id: s.badge_id.map(ids::to_text),
            paint_id: s.paint_id.map(ids::to_text),
        })
        .collect())
}

// The account's own

/// Whose cosmetics the caller acts on: their own, or an account they edit with `profile`.
async fn subject(conn: &mut PgConnection, caller: &Caller, raw: Option<&str>) -> ApiResult<Uuid> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(caller.account_id);
    };
    let owner = parse_id(raw)?;
    if owner == caller.account_id
        || has_flag(
            conn,
            owner,
            caller.account_id,
            cyex_tv_shared::model::EditorPermission::Profile,
        )
        .await?
    {
        return Ok(owner);
    }
    Err(forbidden(
        "Only the account and its editors with the profile permission can do this.",
    ))
}

async fn owned_of(
    state: &AppState,
    conn: &mut PgConnection,
    account: Uuid,
) -> ApiResult<OwnedCosmetics> {
    let badge_ids = db::owned(conn, CosmeticKind::Badge, account)
        .await
        .map_err(db_error)?;
    let paint_ids = db::owned(conn, CosmeticKind::Paint, account)
        .await
        .map_err(db_error)?;
    let (badges, paints) = build(conn, &state.cdn_url, Some(&badge_ids), Some(&paint_ids)).await?;
    let (badge, paint) = db::selection(conn, account).await.map_err(db_error)?;
    let summaries = reads::summaries(conn, &[account]).await.map_err(db_error)?;
    Ok(OwnedCosmetics {
        account: summary(&summaries, account),
        badges,
        paints,
        badge_id: badge.map(ids::to_text),
        paint_id: paint.map(ids::to_text),
    })
}

/// The caller's cosmetics, or those of an account they edit with `profile`.
pub async fn my_cosmetics(
    state: &AppState,
    caller: &Caller,
    account_id: Option<&str>,
) -> ApiResult<OwnedCosmetics> {
    let mut conn = acquire(state).await?;
    let account = subject(&mut conn, caller, account_id).await?;
    owned_of(state, &mut conn, account).await
}

/// What a selection field does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Keep,
    Clear,
    Set(String),
}

fn picked(current: Option<Uuid>, pick: &Pick) -> ApiResult<Option<Uuid>> {
    match pick {
        Pick::Keep => Ok(current),
        Pick::Clear => Ok(None),
        Pick::Set(raw) if raw.trim().is_empty() => Ok(None),
        Pick::Set(raw) => parse_id(raw).map(Some),
    }
}

/// Picks the badge and paint an account shows, from what it owns.
pub async fn select(
    state: &AppState,
    caller: &Caller,
    owner_id: Option<&str>,
    badge: &Pick,
    paint: &Pick,
) -> ApiResult<OwnedCosmetics> {
    rate_limited(state, caller)?;
    let mut conn = acquire(state).await?;
    let account = subject(&mut conn, caller, owner_id).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = db::selection(&mut tx, account).await.map_err(db_error)?;
    let after = (picked(before.0, badge)?, picked(before.1, paint)?);
    for (kind, id) in [
        (CosmeticKind::Badge, after.0),
        (CosmeticKind::Paint, after.1),
    ] {
        if let Some(id) = id
            && !db::owns(&mut tx, kind, account, id)
                .await
                .map_err(db_error)?
        {
            return Err(conflict(
                "not_owned",
                "You can only show cosmetics the account owns.",
            ));
        }
    }
    if after != before {
        db::select(&mut tx, account, after.0, after.1)
            .await
            .map_err(db_error)?;
        publish_selection(&mut tx, account, after).await?;
        // The owner's own picks are audited too: they are on their public activity feed.
        let show = |(b, p): (Option<Uuid>, Option<Uuid>)| json!({ "badge_id": b.map(ids::to_text), "paint_id": p.map(ids::to_text) });
        let mut after_json = show(after);
        if account != caller.account_id {
            after_json["via"] = json!("account_editor");
        }
        audit::write_for(
            &mut tx,
            &audit::NewEntry {
                actor: Some(caller.account_id),
                action: "cosmetics.selected",
                target_kind: "account",
                target_id: &ids::to_text(account),
                before: Some(show(before)),
                after: Some(after_json),
            },
            Some(account),
        )
        .await
        .map_err(db_error)?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    owned_of(state, &mut conn, account).await
}

// Staff: pictures

/// Stores a staff-uploaded picture and queues its files. Needs `cosmetics.manage`.
pub async fn upload_image(
    state: &AppState,
    caller: &Caller,
    kind: CosmeticKind,
    file: Bytes,
) -> ApiResult<CosmeticImage> {
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    drop(conn);
    rate_limited(state, caller)?;
    let Some(store) = state.store.clone() else {
        return Err(ApiError::unavailable("Uploads aren't available right now."));
    };
    let bytes = file.clone();
    let probe: Probe =
        tokio::task::spawn_blocking(move || cyex_tv_media::probe(&bytes, &Limits::default()))
            .await
            .map_err(|_| ApiError::internal())?
            .map_err(media_error)?;
    check_dimensions(kind, probe.width, probe.height)?;
    let sha256: [u8; 32] = Sha256::digest(&file).into();
    let sha_hex = sigv4::hex(&sha256);
    let key = cosmetic_original_key(&sha_hex);
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);

    let mut conn = acquire(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let (image, created) = db::insert_image(
        &mut tx,
        &NewImage {
            kind,
            sha256: &sha256,
            mime: probe.format.mime(),
            width: side(probe.width),
            height: side(probe.height),
            frame_count: side(probe.frames),
            animated: probe.animated,
            storage_key: &key,
            created_by: Some(caller.account_id),
        },
    )
    .await
    .map_err(db_error)?;
    let view = image_view(&image, &state.cdn_url);
    if created {
        store
            .put(
                &key,
                file,
                Put {
                    content_type: probe.format.mime(),
                    cache_control: None,
                },
            )
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "storing a cosmetic original failed");
                ApiError::unavailable("Uploads aren't available right now.")
            })?;
        write_audit(
            &mut tx,
            caller,
            "cosmetic_image.uploaded",
            "cosmetic_image",
            image.id,
            None,
            Some(json!({ "kind": kind.as_str(), "sha256": sha_hex })),
        )
        .await?;
    }
    if created || !view.ready {
        // Again for a picture whose files never got made: the job is safe to repeat.
        jobs::enqueue(
            &mut tx,
            TRANSCODE_JOB,
            &json!({ "image_id": ids::to_text(image.id) }),
            5,
        )
        .await
        .map_err(db_error)?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(view)
}

/// A picture, to see whether its files are made. Needs `cosmetics.manage`.
pub async fn image(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<Option<CosmeticImage>> {
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    Ok(db::get_image(&mut conn, id)
        .await
        .map_err(db_error)?
        .map(|i| image_view(&i, &state.cdn_url)))
}

/// A picture of `kind` whose files are made.
async fn ready_image(conn: &mut PgConnection, kind: CosmeticKind, id: Uuid) -> ApiResult<()> {
    let image = db::get_image(conn, id)
        .await
        .map_err(db_error)?
        .filter(|i| kind_of(i) == kind)
        .ok_or_else(|| {
            bad(
                "bad_image",
                format!("No {} picture with that ID.", kind.as_str()),
            )
        })?;
    if !files_of(&image.variants)
        .iter()
        .any(|f| f.format == ImageFormat::Webp)
    {
        return Err(conflict(
            "image_not_ready",
            "That picture's files are still being made. Try again in a moment.",
        ));
    }
    Ok(())
}

// Staff: badges

pub async fn create_badge(
    state: &AppState,
    caller: &Caller,
    input: &CreateBadge,
) -> ApiResult<Badge> {
    let name = cosmetic_name(&input.name)?;
    let tip = tooltip(&input.tooltip)?;
    let image_id = parse_id(&input.image_id)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    ready_image(&mut tx, CosmeticKind::Badge, image_id).await?;
    let row = db::create_badge(&mut tx, &name, &tip, image_id, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    let badge = one_badge(&mut tx, &state.cdn_url, row.id).await?;
    write_audit(
        &mut tx,
        caller,
        "badge.created",
        "badge",
        row.id,
        None,
        Some(to_json(&badge)),
    )
    .await?;
    publish_cosmetic(
        &mut tx,
        EventType::CosmeticCreated,
        CosmeticKind::Badge,
        row.id,
        Some(to_json(&badge)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(badge)
}

pub async fn update_badge(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &UpdateBadge,
) -> ApiResult<Badge> {
    let id = parse_id(id)?;
    let name = input.name.as_deref().map(cosmetic_name).transpose()?;
    let tip = input.tooltip.as_deref().map(tooltip).transpose()?;
    let image_id = input.image_id.as_deref().map(parse_id).transpose()?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = one_badge(&mut tx, &state.cdn_url, id).await?;
    if let Some(image_id) = image_id {
        ready_image(&mut tx, CosmeticKind::Badge, image_id).await?;
    }
    db::update_badge(&mut tx, id, name.as_deref(), tip.as_deref(), image_id)
        .await
        .map_err(db_error)?;
    let badge = one_badge(&mut tx, &state.cdn_url, id).await?;
    write_audit(
        &mut tx,
        caller,
        "badge.updated",
        "badge",
        id,
        Some(to_json(&before)),
        Some(to_json(&badge)),
    )
    .await?;
    publish_cosmetic(
        &mut tx,
        EventType::CosmeticUpdated,
        CosmeticKind::Badge,
        id,
        Some(to_json(&badge)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(badge)
}

/// Deletes a badge: every account loses it and stops showing it.
pub async fn delete_badge(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Ok(before) = one_badge(&mut tx, &state.cdn_url, id).await else {
        return Ok(false);
    };
    db::delete_badge(&mut tx, id).await.map_err(db_error)?;
    write_audit(
        &mut tx,
        caller,
        "badge.deleted",
        "badge",
        id,
        Some(to_json(&before)),
        None,
    )
    .await?;
    publish_cosmetic(
        &mut tx,
        EventType::CosmeticDeleted,
        CosmeticKind::Badge,
        id,
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

// Staff: paints

async fn paint_write(
    state: &AppState,
    caller: &Caller,
    id: Option<Uuid>,
    input: &PaintInput,
) -> ApiResult<Paint> {
    let checked = paint_input(input)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = match id {
        Some(id) => Some(one_paint(&mut tx, &state.cdn_url, id).await?),
        None => None,
    };
    if let Some(image_id) = checked.image_id {
        ready_image(&mut tx, CosmeticKind::Paint, image_id).await?;
    }
    let stops = to_json(&checked.stops);
    let shadows = to_json(&checked.shadows);
    let fields = PaintFields {
        name: &checked.name,
        kind: checked.kind.as_str(),
        angle: i32::from(checked.angle),
        repeat: checked.repeat,
        shape: checked.shape.as_str(),
        color: checked.color.as_deref(),
        stops: &stops,
        shadows: &shadows,
        image_id: checked.image_id,
    };
    let row = match id {
        Some(id) => db::replace_paint(&mut tx, id, &fields).await,
        None => db::create_paint(&mut tx, &fields, Some(caller.account_id)).await,
    }
    .map_err(db_error)?;
    let paint = one_paint(&mut tx, &state.cdn_url, row.id).await?;
    let (action, event) = match before {
        Some(_) => ("paint.updated", EventType::CosmeticUpdated),
        None => ("paint.created", EventType::CosmeticCreated),
    };
    write_audit(
        &mut tx,
        caller,
        action,
        "paint",
        row.id,
        before.as_ref().map(to_json),
        Some(to_json(&paint)),
    )
    .await?;
    publish_cosmetic(
        &mut tx,
        event,
        CosmeticKind::Paint,
        row.id,
        Some(to_json(&paint)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(paint)
}

pub async fn create_paint(
    state: &AppState,
    caller: &Caller,
    input: &PaintInput,
) -> ApiResult<Paint> {
    paint_write(state, caller, None, input).await
}

/// Replaces every field of a paint.
pub async fn update_paint(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &PaintInput,
) -> ApiResult<Paint> {
    let id = parse_id(id)?;
    paint_write(state, caller, Some(id), input).await
}

pub async fn delete_paint(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_MANAGE).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Ok(before) = one_paint(&mut tx, &state.cdn_url, id).await else {
        return Ok(false);
    };
    db::delete_paint(&mut tx, id).await.map_err(db_error)?;
    write_audit(
        &mut tx,
        caller,
        "paint.deleted",
        "paint",
        id,
        Some(to_json(&before)),
        None,
    )
    .await?;
    publish_cosmetic(
        &mut tx,
        EventType::CosmeticDeleted,
        CosmeticKind::Paint,
        id,
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

// Staff: entitlements

async fn exists(conn: &mut PgConnection, kind: CosmeticKind, id: Uuid) -> ApiResult<bool> {
    Ok(match kind {
        CosmeticKind::Badge => db::get_badge(conn, id).await.map_err(db_error)?.is_some(),
        CosmeticKind::Paint => db::get_paint(conn, id).await.map_err(db_error)?.is_some(),
    })
}

async fn entitlement_views(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    rows: Vec<db::EntitlementRow>,
) -> ApiResult<Vec<Entitlement>> {
    let people: Vec<Uuid> = rows
        .iter()
        .flat_map(|r| std::iter::once(r.account_id).chain(r.granted_by))
        .collect();
    let summaries = reads::summaries(conn, &people).await.map_err(db_error)?;
    Ok(rows
        .into_iter()
        .map(|r| Entitlement {
            account: summary(&summaries, r.account_id),
            kind,
            cosmetic_id: ids::to_text(r.cosmetic_id),
            source: EntitlementSource::parse(&r.source).unwrap_or(EntitlementSource::Grant),
            reason: r.reason,
            granted_by: r.granted_by.map(|g| summary(&summaries, g)),
            granted_at: time(r.granted_at),
        })
        .collect())
}

/// Gives an account a badge or paint, with a reason. Needs `cosmetics.grant`.
pub async fn grant(
    state: &AppState,
    caller: &Caller,
    input: &GrantCosmetic,
) -> ApiResult<Entitlement> {
    let why = reason(&input.reason)?;
    let account = parse_id(&input.account_id)?;
    let cosmetic = parse_id(&input.cosmetic_id)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_GRANT).await?;
    rate_limited(state, caller)?;
    let live = accounts::get(&mut conn, account)
        .await
        .map_err(db_error)?
        .is_some_and(|a| a.deleted_at.is_none() && a.blocked_at.is_none());
    if !live || !exists(&mut conn, input.kind, cosmetic).await? {
        return Err(not_found());
    }
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let new = db::grant(
        &mut tx,
        input.kind,
        account,
        cosmetic,
        "grant",
        Some(&why),
        Some(caller.account_id),
    )
    .await
    .map_err(db_error)?;
    if !new {
        return Err(conflict("already_owned", "They own that already."));
    }
    write_audit(
        &mut tx,
        caller,
        "cosmetic.granted",
        "account",
        account,
        None,
        Some(json!({
            "kind": input.kind.as_str(),
            "cosmetic_id": ids::to_text(cosmetic),
            "reason": why,
        })),
    )
    .await?;
    let (rows, _) = db::entitlements(&mut tx, input.kind, Some(account), Some(cosmetic), 1, 0)
        .await
        .map_err(db_error)?;
    let out = entitlement_views(&mut tx, input.kind, rows)
        .await?
        .pop()
        .ok_or_else(ApiError::internal)?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

/// Takes a badge or paint away; if the account showed it, it stops. False when they didn't own
/// it. Needs `cosmetics.grant`.
pub async fn revoke(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    kind: CosmeticKind,
    cosmetic_id: &str,
    why: Option<&str>,
) -> ApiResult<bool> {
    let account = parse_id(account_id)?;
    let cosmetic = parse_id(cosmetic_id)?;
    let why = why
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(reason)
        .transpose()?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_GRANT).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = db::selection(&mut tx, account).await.map_err(db_error)?;
    let Some(removed) = db::revoke(&mut tx, kind, account, cosmetic)
        .await
        .map_err(db_error)?
    else {
        return Ok(false);
    };
    let after = db::selection(&mut tx, account).await.map_err(db_error)?;
    if after != before {
        publish_selection(&mut tx, account, after).await?;
    }
    write_audit(
        &mut tx,
        caller,
        "cosmetic.revoked",
        "account",
        account,
        Some(json!({
            "kind": kind.as_str(),
            "cosmetic_id": ids::to_text(cosmetic),
            "source": removed.source,
            "reason": removed.reason,
        })),
        Some(json!({ "reason": why })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

/// Who owns what, newest first: for one account, one cosmetic, or both. Needs `cosmetics.grant`.
pub async fn entitlements(
    state: &AppState,
    caller: &Caller,
    kind: CosmeticKind,
    account_id: Option<&str>,
    cosmetic_id: Option<&str>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<EntitlementPage> {
    let filter = |raw: Option<&str>| {
        raw.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(parse_id)
            .transpose()
    };
    let account = filter(account_id)?;
    let cosmetic = filter(cosmetic_id)?;
    let per_page = per_page
        .unwrap_or(ENTITLEMENTS_PAGE)
        .clamp(1, ENTITLEMENTS_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    let offset = i64::from(page - 1) * i64::from(per_page);
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::COSMETICS_GRANT).await?;
    let (rows, total) = db::entitlements(
        &mut conn,
        kind,
        account,
        cosmetic,
        i64::from(per_page),
        offset,
    )
    .await
    .map_err(db_error)?;
    Ok(EntitlementPage {
        items: entitlement_views(&mut conn, kind, rows).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stop(at: f64, c: &str) -> PaintStop {
        PaintStop {
            at,
            color: c.to_owned(),
        }
    }

    fn shadow(x: f64, y: f64, radius: f64) -> PaintShadow {
        PaintShadow {
            x,
            y,
            radius,
            color: "#000000AA".to_owned(),
        }
    }

    fn gradient(stops: Vec<PaintStop>) -> PaintInput {
        PaintInput {
            name: "Dawn".to_owned(),
            kind: PaintKind::LinearGradient,
            angle: None,
            repeat: None,
            shape: None,
            color: None,
            stops,
            shadows: Vec::new(),
            image_id: None,
        }
    }

    #[test]
    fn names_and_tooltips() {
        assert_eq!(cosmetic_name("  Dawn ").unwrap(), "Dawn");
        assert_eq!(cosmetic_name(&"a".repeat(32)).unwrap().len(), 32);
        for bad in [
            "",
            "   ",
            &"a".repeat(33),
            "Da\u{202E}wn",
            "Da\u{200B}wn",
            "a\nb",
        ] {
            assert_eq!(cosmetic_name(bad).unwrap_err().code, "bad_name", "{bad:?}");
        }
        assert!(tooltip(&"t".repeat(64)).is_ok());
        assert_eq!(tooltip(&"t".repeat(65)).unwrap_err().code, "bad_tooltip");
        assert_eq!(reason("line one\nline two").unwrap(), "line one\nline two");
        assert_eq!(reason("").unwrap_err().code, "bad_reason");
        assert_eq!(reason("a\u{2066}b").unwrap_err().code, "bad_reason");
    }

    #[test]
    fn colours() {
        assert_eq!(color("#ff00aa").unwrap(), "#FF00AA");
        assert_eq!(color("#ff00aa80").unwrap(), "#FF00AA80");
        for bad in [
            "ff00aa",
            "#ff00a",
            "#ff00aa8",
            "#gg00aa",
            "red",
            "#ff00aa80ff",
            "",
        ] {
            assert_eq!(color(bad).unwrap_err().code, "bad_color", "{bad:?}");
        }
    }

    #[test]
    fn stops_are_two_to_ten_ascending_in_range() {
        assert!(stops(&[stop(0.0, "#000000"), stop(1.0, "#FFFFFF")]).is_ok());
        // Hard stops (two at the same place) are allowed.
        assert!(
            stops(&[
                stop(0.0, "#000000"),
                stop(0.5, "#111111"),
                stop(0.5, "#222222")
            ])
            .is_ok()
        );
        assert_eq!(
            stops(&[stop(0.0, "#000000")]).unwrap_err().code,
            "bad_stops"
        );
        let eleven: Vec<_> = (0..11)
            .map(|i| stop(f64::from(i) / 10.0, "#000000"))
            .collect();
        assert_eq!(stops(&eleven).unwrap_err().code, "bad_stops");
        assert_eq!(stops(&eleven[..10]).unwrap().len(), 10);
        for bad in [
            vec![stop(0.6, "#000000"), stop(0.4, "#000000")],
            vec![stop(-0.1, "#000000"), stop(1.0, "#000000")],
            vec![stop(0.0, "#000000"), stop(1.1, "#000000")],
            vec![stop(f64::NAN, "#000000"), stop(1.0, "#000000")],
        ] {
            assert_eq!(stops(&bad).unwrap_err().code, "bad_stops");
        }
        assert_eq!(
            stops(&[stop(0.0, "black"), stop(1.0, "#FFFFFF")])
                .unwrap_err()
                .code,
            "bad_color"
        );
        assert_eq!(
            stops(&[stop(0.0, "#abcdef"), stop(1.0, "#FFFFFF")]).unwrap()[0].color,
            "#ABCDEF"
        );
    }

    #[test]
    fn shadows_are_bounded() {
        assert_eq!(
            shadows(&[shadow(1.0, -2.0, 3.0)]).unwrap()[0].color,
            "#000000AA"
        );
        assert!(shadows(&vec![shadow(0.0, 0.0, 0.0); 4]).is_ok());
        assert_eq!(
            shadows(&vec![shadow(0.0, 0.0, 0.0); 5]).unwrap_err().code,
            "bad_shadows"
        );
        for bad in [
            shadow(17.0, 0.0, 1.0),
            shadow(0.0, -17.0, 1.0),
            shadow(0.0, 0.0, -1.0),
            shadow(0.0, 0.0, 17.0),
            shadow(f64::INFINITY, 0.0, 1.0),
        ] {
            assert_eq!(shadows(&[bad]).unwrap_err().code, "bad_shadows");
        }
    }

    #[test]
    fn paints_by_kind() {
        let ok = paint_input(&gradient(vec![stop(0.0, "#000000"), stop(1.0, "#ffffff")])).unwrap();
        assert_eq!(
            (ok.angle, ok.repeat, ok.shape),
            (90, false, PaintShape::Circle)
        );
        let mut wide = gradient(vec![stop(0.0, "#000000"), stop(1.0, "#ffffff")]);
        wide.angle = Some(361);
        assert_eq!(paint_input(&wide).unwrap_err().code, "bad_angle");
        wide.angle = Some(360);
        wide.color = Some("#12345".into());
        assert_eq!(paint_input(&wide).unwrap_err().code, "bad_color");
        wide.color = Some(String::new());
        assert_eq!(paint_input(&wide).unwrap().color, None);

        let mut with_image = gradient(vec![stop(0.0, "#000000"), stop(1.0, "#ffffff")]);
        with_image.image_id = Some(ids::to_text(ids::new_id()));
        assert_eq!(paint_input(&with_image).unwrap_err().code, "bad_image");

        let mut image = gradient(Vec::new());
        image.kind = PaintKind::Image;
        assert_eq!(paint_input(&image).unwrap_err().code, "bad_image");
        let id = ids::new_id();
        image.image_id = Some(ids::to_text(id));
        assert_eq!(paint_input(&image).unwrap().image_id, Some(id));
        image.stops = vec![stop(0.0, "#000000"), stop(1.0, "#ffffff")];
        assert_eq!(paint_input(&image).unwrap_err().code, "bad_stops");

        let mut short = gradient(vec![stop(0.0, "#000000")]);
        short.kind = PaintKind::RadialGradient;
        assert_eq!(paint_input(&short).unwrap_err().code, "bad_stops");
    }

    #[test]
    fn picture_sizes() {
        assert!(check_dimensions(CosmeticKind::Badge, 72, 72).is_ok());
        assert!(check_dimensions(CosmeticKind::Badge, 128, 128).is_ok());
        assert_eq!(
            check_dimensions(CosmeticKind::Badge, 36, 36)
                .unwrap_err()
                .code,
            "bad_dimensions"
        );
        assert_eq!(
            check_dimensions(CosmeticKind::Badge, 100, 72)
                .unwrap_err()
                .code,
            "bad_dimensions"
        );
        assert!(check_dimensions(CosmeticKind::Paint, 512, 64).is_ok());
        assert!(check_dimensions(CosmeticKind::Paint, 64, 63).is_err());
    }

    #[test]
    fn lookups_take_digits_once() {
        let got = user_ids(&[
            "1".into(),
            " 2 ".into(),
            "1".into(),
            "x".into(),
            String::new(),
        ])
        .unwrap();
        assert_eq!(got, ["1", "2"]);
        let many: Vec<String> = (0..101).map(|i| i.to_string()).collect();
        assert_eq!(user_ids(&many).unwrap_err().code, "too_many");
    }

    #[test]
    fn picks() {
        let id = ids::new_id();
        assert_eq!(picked(Some(id), &Pick::Keep).unwrap(), Some(id));
        assert_eq!(picked(Some(id), &Pick::Clear).unwrap(), None);
        assert_eq!(picked(Some(id), &Pick::Set(String::new())).unwrap(), None);
        assert_eq!(
            picked(None, &Pick::Set(ids::to_text(id))).unwrap(),
            Some(id)
        );
        assert_eq!(
            picked(None, &Pick::Set("nope".into())).unwrap_err().code,
            "not_found"
        );
    }
}
