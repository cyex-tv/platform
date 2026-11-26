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
use cyex_tv_db::{accounts, ids, jobs, outbox, reads};
use cyex_tv_media::encode::Profile;
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

use crate::catalog::require_any;
use crate::catalog::{LOOKUP_LIMIT, db_error, forbidden, not_found, parse_id};
use crate::editors::has_flag;
use crate::errors::{self, Audit, record_audit};
use crate::errors::{bad, conflict, limited_as, paging, sqlx_error};
use crate::limits::AccountLimit;
use crate::state::AppState;
use crate::upload::media_error;
use crate::views::time;

/// The job the worker makes a picture's files with.
pub const TRANSCODE_JOB: &str = "cosmetic.transcode";
/// Badge pictures must be square and at least this many px (4x of 18 px).
pub const BADGE_MIN_SIDE: u32 = 72;
/// Paint pictures must be at least this tall (1x).
pub const PAINT_MIN_HEIGHT: u32 = 64;
/// Paint pictures are at most this many times as wide as they are tall.
pub const PAINT_MAX_ASPECT: u32 = 8;
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

/// Cosmetics writes per account (ADR 0015).
fn rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::Cosmetics,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of cosmetics changes. Try again in a while.",
    )
}

// Checks on input (pure, unit-tested below)

/// Badge and paint names: 1 to 32 characters after trimming, nothing hidden or controlling.
pub fn clean_cosmetic_name(raw: &str) -> ApiResult<String> {
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
    let name = clean_cosmetic_name(&input.name)?;
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

/// The limits a kind of picture is checked under, the same the worker decodes it with: badges
/// and paints the same file rules (PNG, APNG, JPEG, WebP or GIF up to 8 MB; animated at most 120
/// frames, 10 seconds and 64 MP of frames); a paint may be wider (4096 px a side, 2048 for a
/// badge).
pub fn picture_limits(kind: CosmeticKind) -> Limits {
    match kind {
        CosmeticKind::Badge => Limits::badge(),
        CosmeticKind::Paint => Limits::paint(),
    }
}

/// The sizes a kind of picture is made at (the worker's `cosmetics::profile`).
pub fn picture_profile(kind: CosmeticKind) -> &'static Profile {
    match kind {
        CosmeticKind::Badge => Profile::badge(),
        CosmeticKind::Paint => Profile::paint(),
    }
}

/// Whether a made picture lacks a size its kind is made at now (a badge from before 128, 256
/// and 512 px): making it again fills them in.
pub fn lacks_sizes(kind: CosmeticKind, files: &[EmoteFile]) -> bool {
    !files.is_empty() && !picture_profile(kind).is_complete(files.iter().map(|f| f.name.as_str()))
}

/// Checks a picture for its kind: badges square and at least 72×72, paints at least 64 tall
/// and at most 8:1 (wide).
pub fn check_dimensions(kind: CosmeticKind, width: u32, height: u32) -> ApiResult<()> {
    let (ok, message) = match kind {
        CosmeticKind::Badge => (
            width == height && width >= BADGE_MIN_SIDE,
            "Badge pictures are square and at least 72×72 px.",
        ),
        CosmeticKind::Paint if height < PAINT_MIN_HEIGHT => {
            (false, "Paint pictures are at least 64 px tall.")
        }
        CosmeticKind::Paint => (
            u64::from(width) <= u64::from(height) * u64::from(PAINT_MAX_ASPECT),
            "Paint pictures are at most 8 times as wide as they are tall (8:1).",
        ),
    };
    if ok {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "bad_dimensions",
        message,
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
    let host = format!(
        "{cdn_url}/{}",
        cosmetic_prefix(kind.as_str(), &sigv4::hex(&image.sha256))
    );
    CosmeticImage {
        id: ids::to_text(image.id),
        kind,
        files: EmoteFile::with_urls(files, &host),
        host,
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
    // The public topic only ever tells about released cosmetics (held or for sale); an
    // unreleased one is announced when it is released (`announce_release`).
    if event_type != EventType::CosmeticDeleted
        && !db::is_released(conn, kind, id, chrono::Utc::now())
            .await
            .map_err(db_error)?
    {
        return Ok(());
    }
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

/// Tells the public topic about a cosmetic that has just been released: it is held by exactly one
/// account (the first grant) or in a shop item on sale. Chat clients add it to their catalog.
/// Others are left alone, so a popular item does not make an event per sale.
pub(crate) async fn announce_release(
    conn: &mut PgConnection,
    cdn: &str,
    kind: CosmeticKind,
    id: Uuid,
    only_first_holder: bool,
) -> ApiResult<()> {
    if only_first_holder && db::holders(conn, kind, id).await.map_err(db_error)? != 1 {
        return Ok(());
    }
    let body = match kind {
        CosmeticKind::Badge => one_badge(conn, cdn, id).await.map(|b| to_json(&b)),
        CosmeticKind::Paint => one_paint(conn, cdn, id).await.map(|p| to_json(&p)),
    };
    let Ok(body) = body else {
        return Ok(());
    };
    publish_cosmetic(conn, EventType::CosmeticUpdated, kind, id, Some(body)).await
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

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

// Public reads

/// The catalog everyone sees: the cosmetics held by at least one account or for sale in the shop.
/// Chat clients need the held ones to draw chatters; the rest stay private until released.
pub async fn catalog(state: &AppState) -> ApiResult<CosmeticCatalog> {
    let mut conn = errors::conn(state).await?;
    let now = chrono::Utc::now();
    let badge_ids = db::released_ids(&mut conn, CosmeticKind::Badge, now)
        .await
        .map_err(db_error)?;
    let paint_ids = db::released_ids(&mut conn, CosmeticKind::Paint, now)
        .await
        .map_err(db_error)?;
    let (badges, paints) = build(
        &mut conn,
        &state.cdn_url,
        Some(&badge_ids),
        Some(&paint_ids),
    )
    .await?;
    Ok(CosmeticCatalog { badges, paints })
}

/// The whole catalog, unreleased cosmetics too: for staff with a `cosmetics.*` permission.
pub async fn catalog_all(state: &AppState) -> ApiResult<CosmeticCatalog> {
    let mut conn = errors::conn(state).await?;
    let (badges, paints) = build(&mut conn, &state.cdn_url, None, None).await?;
    Ok(CosmeticCatalog { badges, paints })
}

/// The catalog for this caller: all of it for cosmetics staff, the public one for everyone else.
pub async fn catalog_for(state: &AppState, caller: Option<&Caller>) -> ApiResult<CosmeticCatalog> {
    if let Some(caller) = caller {
        let mut conn = errors::conn(state).await?;
        if sees_unreleased(&mut conn, caller).await? {
            drop(conn);
            return catalog_all(state).await;
        }
    }
    catalog(state).await
}

/// Whether the caller holds a `cosmetics.*` permission (manage or grant).
pub async fn sees_unreleased(conn: &mut PgConnection, caller: &Caller) -> ApiResult<bool> {
    Ok(crate::catalog::permissions_of(conn, caller)
        .await?
        .iter()
        .any(|p| p.starts_with("cosmetics.")))
}

/// What up to 100 accounts show, with the badges and paints themselves. Only live accounts
/// showing something are listed; unknown IDs are skipped.
pub async fn account_cosmetics(
    state: &AppState,
    raw_ids: &[String],
) -> ApiResult<Vec<cyex_tv_shared::model::AccountCosmetics>> {
    if raw_ids.len() > 100 {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "too_many",
            "Look up at most 100 accounts at once.",
        ));
    }
    let mut wanted: Vec<Uuid> = raw_ids
        .iter()
        .filter_map(|r| ids::from_text(r.trim()))
        .collect();
    wanted.sort();
    wanted.dedup();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut conn = errors::conn(state).await?;
    let picks = db::selections_for_accounts(&mut conn, &wanted)
        .await
        .map_err(db_error)?;
    let badge_ids: Vec<Uuid> = picks.iter().filter_map(|p| p.1).collect();
    let paint_ids: Vec<Uuid> = picks.iter().filter_map(|p| p.2).collect();
    let (badges, paints) = build(
        &mut conn,
        &state.cdn_url,
        Some(&badge_ids),
        Some(&paint_ids),
    )
    .await?;
    Ok(picks
        .into_iter()
        .map(|(account, badge, paint)| {
            let badge = badge.and_then(|b| {
                let id = ids::to_text(b);
                badges.iter().find(|x| x.id == id).cloned()
            });
            let paint = paint.and_then(|p| {
                let id = ids::to_text(p);
                paints.iter().find(|x| x.id == id).cloned()
            });
            cyex_tv_shared::model::AccountCosmetics {
                account_id: ids::to_text(account),
                badge,
                paint,
            }
        })
        .filter(|c| c.badge.is_some() || c.paint.is_some())
        .collect())
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
    let mut conn = errors::conn(state).await?;
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
    let mut conn = errors::conn(state).await?;
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
    let mut conn = errors::conn(state).await?;
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
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("cosmetics.selected", "account", &ids::to_text(account))
                .change(Some(show(before)), Some(after_json))
                .subject(Some(account)),
        )
        .await?;
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    drop(conn);
    rate_limited(state, caller)?;
    let Some(store) = state.store.clone() else {
        return Err(ApiError::unavailable("Uploads aren't available right now."));
    };
    let bytes = file.clone();
    let limits = picture_limits(kind);
    let probe: Probe = tokio::task::spawn_blocking(move || cyex_tv_media::probe(&bytes, &limits))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(media_error)?;
    check_dimensions(kind, probe.width, probe.height)?;
    if probe.animated && picture_profile(kind).max_animated_scale_bytes.is_some() {
        // An animated badge must animate in chat: its 72 px file at most 512 KB. Said now, not
        // after the worker tried.
        let bytes = file.clone();
        tokio::task::spawn_blocking(move || {
            cyex_tv_media::encode::check_animated_scales(&bytes, &limits, picture_profile(kind))
        })
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(media_error)?;
    }
    let sha256: [u8; 32] = Sha256::digest(&file).into();
    let sha_hex = sigv4::hex(&sha256);
    let key = cosmetic_original_key(&sha_hex);
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);

    let mut conn = errors::conn(state).await?;
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
    let mut view = image_view(&image, &state.cdn_url);
    // Bytes we know whose original is gone from storage (it moved, or lost it): the record still
    // says made, so nothing would happen. Store the original again, forget the files and make
    // them again.
    if !created {
        let gone = |err| {
            tracing::error!(error = %err, "checking a cosmetic picture in storage failed");
            ApiError::unavailable("Uploads aren't available right now.")
        };
        let lost_original = !store.exists(&key).await.map_err(gone)?;
        if lost_original {
            store
                .put(
                    &key,
                    file.clone(),
                    Put {
                        content_type: probe.format.mime(),
                        cache_control: None,
                    },
                )
                .await
                .map_err(|err| {
                    tracing::error!(error = %err, "storing a cosmetic original again failed");
                    ApiError::unavailable("Uploads aren't available right now.")
                })?;
        }
        if lost_original {
            db::set_image_variants(&mut tx, image.id, &json!([]))
                .await
                .map_err(db_error)?;
            view.files.clear();
            view.ready = false;
        }
    }
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
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new(
                "cosmetic_image.uploaded",
                "cosmetic_image",
                &ids::to_text(image.id),
            )
            .change(
                None,
                Some(json!({ "kind": kind.as_str(), "sha256": sha_hex })),
            ),
        )
        .await?;
    }
    if created || !view.ready || lacks_sizes(kind, &view.files) {
        // Again for a picture whose files never got made, or were made before a size existed:
        // the job is safe to repeat.
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    Ok(db::get_image(&mut conn, id)
        .await
        .map_err(db_error)?
        .map(|i| image_view(&i, &state.cdn_url)))
}

/// Makes a picture's files again from its stored original (a badge made before 128, 256 and 512
/// px gets them). The files it has stay in use until the new set is recorded. Needs
/// `cosmetics.manage`. Audited as `cosmetic_image.remade`.
pub async fn remake_image(state: &AppState, caller: &Caller, id: &str) -> ApiResult<CosmeticImage> {
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    rate_limited(state, caller)?;
    let Some(store) = state.store.clone() else {
        return Err(ApiError::unavailable("Pictures can't be made right now."));
    };
    let image = match ids::from_text(id.trim()) {
        Some(id) => db::get_image(&mut conn, id).await.map_err(db_error)?,
        None => None,
    }
    .ok_or_else(not_found)?;
    let original = store.exists(&image.storage_key).await.map_err(|err| {
        tracing::error!(error = %err, "checking a cosmetic original in storage failed");
        ApiError::unavailable("Pictures can't be made right now.")
    })?;
    if !original {
        return Err(conflict(
            "original_missing",
            "This picture's original is gone from storage. Upload the picture again to make it.",
        ));
    }
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    jobs::enqueue(
        &mut tx,
        TRANSCODE_JOB,
        &json!({ "image_id": ids::to_text(image.id) }),
        5,
    )
    .await
    .map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(
            "cosmetic_image.remade",
            "cosmetic_image",
            &ids::to_text(image.id),
        )
        .change(
            None,
            Some(json!({ "kind": image.kind, "sha256": sigv4::hex(&image.sha256) })),
        ),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(image_view(&image, &state.cdn_url))
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
    let name = clean_cosmetic_name(&input.name)?;
    let tip = tooltip(&input.tooltip)?;
    let image_id = parse_id(&input.image_id)?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    ready_image(&mut tx, CosmeticKind::Badge, image_id).await?;
    let row = db::create_badge(&mut tx, &name, &tip, image_id, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    let badge = one_badge(&mut tx, &state.cdn_url, row.id).await?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("badge.created", "badge", &ids::to_text(row.id))
            .change(None, Some(to_json(&badge))),
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
    let name = input.name.as_deref().map(clean_cosmetic_name).transpose()?;
    let tip = input.tooltip.as_deref().map(tooltip).transpose()?;
    let image_id = input.image_id.as_deref().map(parse_id).transpose()?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
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
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("badge.updated", "badge", &ids::to_text(id))
            .change(Some(to_json(&before)), Some(to_json(&badge))),
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Ok(before) = one_badge(&mut tx, &state.cdn_url, id).await else {
        return Ok(false);
    };
    db::delete_badge(&mut tx, id).await.map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("badge.deleted", "badge", &ids::to_text(id))
            .change(Some(to_json(&before)), None),
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
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
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(action, "paint", &ids::to_text(row.id))
            .change(before.as_ref().map(to_json), Some(to_json(&paint))),
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Ok(before) = one_paint(&mut tx, &state.cdn_url, id).await else {
        return Ok(false);
    };
    db::delete_paint(&mut tx, id).await.map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("paint.deleted", "paint", &ids::to_text(id))
            .change(Some(to_json(&before)), None),
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_GRANT]).await?;
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
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("cosmetic.granted", "account", &ids::to_text(account)).change(
            None,
            Some(json!({
                "kind": input.kind.as_str(),
                "cosmetic_id": ids::to_text(cosmetic),
                "reason": why,
            })),
        ),
    )
    .await?;
    let (rows, _) = db::entitlements(&mut tx, input.kind, Some(account), Some(cosmetic), 1, 0)
        .await
        .map_err(db_error)?;
    let out = entitlement_views(&mut tx, input.kind, rows)
        .await?
        .pop()
        .ok_or_else(ApiError::internal)?;
    announce_release(&mut tx, &state.cdn_url, input.kind, cosmetic, true).await?;
    let name = cosmetic_display_name(&mut tx, input.kind, cosmetic).await?;
    crate::account::notify(
        &mut tx,
        account,
        "cosmetic.granted",
        &json!({
            "kind": input.kind.as_str(),
            "cosmetic_id": ids::to_text(cosmetic),
            "name": name,
            "reason": why,
        }),
    )
    .await?;
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
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_GRANT]).await?;
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
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("cosmetic.revoked", "account", &ids::to_text(account)).change(
            Some(json!({
                "kind": kind.as_str(),
                "cosmetic_id": ids::to_text(cosmetic),
                "source": removed.source,
                "reason": removed.reason,
            })),
            Some(json!({ "reason": why })),
        ),
    )
    .await?;
    let name = cosmetic_display_name(&mut tx, kind, cosmetic).await?;
    crate::account::notify(
        &mut tx,
        account,
        "cosmetic.revoked",
        &json!({
            "kind": kind.as_str(),
            "cosmetic_id": ids::to_text(cosmetic),
            "name": name,
            "reason": why,
        }),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

/// A badge's or paint's name, for notifications. `None` when it is gone.
pub(crate) async fn cosmetic_display_name(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    id: Uuid,
) -> ApiResult<Option<String>> {
    Ok(match kind {
        CosmeticKind::Badge => db::get_badge(conn, id)
            .await
            .map_err(db_error)?
            .map(|b| b.name),
        CosmeticKind::Paint => db::get_paint(conn, id)
            .await
            .map_err(db_error)?
            .map(|p| p.name),
    })
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
    let paged = paging(page, per_page, ENTITLEMENTS_PAGE, ENTITLEMENTS_PAGE_MAX);
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_GRANT]).await?;
    let (rows, total) = db::entitlements(
        &mut conn,
        kind,
        account,
        cosmetic,
        paged.limit,
        paged.offset,
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
        assert_eq!(clean_cosmetic_name("  Dawn ").unwrap(), "Dawn");
        assert_eq!(clean_cosmetic_name(&"a".repeat(32)).unwrap().len(), 32);
        for bad in [
            "",
            "   ",
            &"a".repeat(33),
            "Da\u{202E}wn",
            "Da\u{200B}wn",
            "a\nb",
        ] {
            assert_eq!(
                clean_cosmetic_name(bad).unwrap_err().code,
                "bad_name",
                "{bad:?}"
            );
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

    fn file(name: &str) -> EmoteFile {
        EmoteFile {
            name: name.into(),
            format: ImageFormat::Webp,
            scale: 1,
            width: 18,
            height: 18,
            size: 1,
            is_static: false,
            url: String::new(),
            mime: String::new(),
        }
    }

    #[test]
    fn badges_take_the_badge_limits() {
        let badge = picture_limits(CosmeticKind::Badge);
        assert_eq!(badge.max_bytes, 8 * 1024 * 1024);
        assert_eq!(
            (
                badge.max_frames,
                badge.max_duration_ms,
                badge.max_total_pixels
            ),
            (120, 10_000, 64_000_000)
        );
        // Paints take the same file and animation rules, with a wider side.
        let paint = picture_limits(CosmeticKind::Paint);
        assert_eq!(paint, Limits::paint());
        assert_eq!(paint.max_bytes, 8 * 1024 * 1024);
        assert_eq!(
            (
                paint.max_frames,
                paint.max_duration_ms,
                paint.max_total_pixels
            ),
            (120, 10_000, 64_000_000)
        );
    }

    #[test]
    fn badges_from_before_the_ladder_lack_sizes() {
        let old: Vec<_> = ["1x.webp", "2x.webp", "4x.webp"].map(file).into();
        assert!(lacks_sizes(CosmeticKind::Badge, &old));
        let ladder: Vec<_> = [
            "1x.webp", "2x.webp", "4x.webp", "128.webp", "256.webp", "512.webp",
        ]
        .map(file)
        .into();
        // A ladder without the original (made before it existed) is made again.
        assert!(lacks_sizes(CosmeticKind::Badge, &ladder));
        let full: Vec<_> = [
            "1x.webp",
            "2x.webp",
            "4x.webp",
            "128.webp",
            "256.webp",
            "512.webp",
            "original.webp",
        ]
        .map(file)
        .into();
        assert!(!lacks_sizes(CosmeticKind::Badge, &full));
        // An animated badge whose large sizes were too heavy to animate keeps their stills.
        let stills: Vec<_> = [
            "1x.webp",
            "2x.webp",
            "4x.webp",
            "128_static.webp",
            "256_static.webp",
            "512_static.webp",
            "original_static.webp",
        ]
        .map(file)
        .into();
        assert!(!lacks_sizes(CosmeticKind::Badge, &stills));
        // Not made yet is not lacking (that's `ready`).
        assert!(!lacks_sizes(CosmeticKind::Badge, &[]));
        // Paints from before 512 px lack it; with it, or its still, they don't.
        assert!(lacks_sizes(CosmeticKind::Paint, &old));
        let paint: Vec<_> = ["1x.webp", "2x.webp", "4x.webp", "512.webp"]
            .map(file)
            .into();
        assert!(!lacks_sizes(CosmeticKind::Paint, &paint));
        let still: Vec<_> = ["1x.webp", "2x.webp", "4x.webp", "512_static.webp"]
            .map(file)
            .into();
        assert!(!lacks_sizes(CosmeticKind::Paint, &still));
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
        // Up to 8:1 wide, any taller.
        assert!(check_dimensions(CosmeticKind::Paint, 4096, 512).is_ok());
        assert!(check_dimensions(CosmeticKind::Paint, 512, 512).is_ok());
        assert!(check_dimensions(CosmeticKind::Paint, 100, 400).is_ok());
        assert_eq!(
            check_dimensions(CosmeticKind::Paint, 513, 64)
                .unwrap_err()
                .code,
            "bad_dimensions"
        );
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
