//! Site-wide settings and an account's own choices:
//!
//! - **The site announcement:** one line staff put under the header for everyone (`info`,
//!   `maintenance` or `campaign`), with an optional link, a small emote or badge, and a start
//!   and end. Anyone reads the live one (`activeAnnouncement`, shared per instance for as long as
//!   the read cache keeps entries); staff with `site.announce` list, create, change and end them,
//!   each audited (`announcement.create`, `announcement.update`, `announcement.end`).
//! - **Notification preferences:** per kind, whether it is created at all (off: the server never
//!   stores it; see `notifications::create`) and whether the website pops it up. Hand-over
//!   requests and takedowns can't be turned off.
//! - **The collection:** the badges and paints an account owns, with since when and how each
//!   came (staff, the shop, a code, an event). Private unless the account turns it on; never who
//!   gave it, why, or what it cost.

use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use chrono::{DateTime, Utc};
use cyex_tv_auth::Caller;
use cyex_tv_db::{accounts, announcements as db, ids, prefs};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    AccountCollection, Announcement, AnnouncementInput, AnnouncementKind, CollectionItem,
    GrantKind, GrantSource, NotificationPref, NotificationPrefInput,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::json;
use sqlx::{Connection, PgConnection};

use crate::catalog::require_any;
use crate::errors::{Audit, record_audit};
use crate::errors::{bad, db_error, limited_as, not_found, parse_id, sqlx_error};
use crate::limits::AccountLimit;
use crate::read_cache::ANNOUNCEMENT_KEY;
use crate::state::AppState;
use crate::views;

/// The longest announcement line, link and link label.
pub const TEXT_MAX: usize = 200;
pub const LINK_MAX: usize = 500;
pub const LABEL_MAX: usize = 40;
/// Announcements `announcements` lists at most.
const LIST_MAX: i64 = 50;
/// Badges and paints a collection lists at most.
const COLLECTION_MAX: i64 = 500;

// The site announcement

/// The live announcement as last read, and when. `clear` bumps a generation; a read that started
/// before it doesn't store what it loaded, so a slow read can't bring back an edited or ended
/// line after the change cleared it.
#[derive(Default)]
pub struct AnnouncementCache {
    slot: Mutex<Slot>,
}

/// The generation, and the announcement read (with when) if one is kept.
type Slot = (u64, Option<(Instant, Option<Announcement>)>);

impl AnnouncementCache {
    fn lock(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, ttl: std::time::Duration) -> Option<Option<Announcement>> {
        match &self.lock().1 {
            Some((at, a)) if at.elapsed() < ttl => Some(a.clone()),
            _ => None,
        }
    }

    /// The generation now: take it before reading, hand it to `put`.
    pub fn generation(&self) -> u64 {
        self.lock().0
    }

    /// Stores `a`, read since `generation`, unless a `clear` ran meanwhile.
    pub fn put(&self, generation: u64, a: Option<Announcement>) {
        let mut slot = self.lock();
        if slot.0 == generation {
            slot.1 = Some((Instant::now(), a));
        }
    }

    /// Forgets it: the next read builds it again, and reads already running don't store theirs.
    pub fn clear(&self) {
        let mut slot = self.lock();
        slot.0 = slot.0.wrapping_add(1);
        slot.1 = None;
    }
}

/// Every api instance drops its cached announcement once the change commits.
async fn forget_announcement(conn: &mut PgConnection) -> ApiResult<()> {
    cyex_tv_db::outbox::publish_cache_forget(conn, &[ANNOUNCEMENT_KEY.to_owned()])
        .await
        .map_err(db_error)?;
    Ok(())
}

/// Whether a row is shown at `now`.
fn is_live(row: &db::Announcement, now: DateTime<Utc>) -> bool {
    row.ended_at.is_none()
        && row.starts_at.is_none_or(|s| s <= now)
        && row.ends_at.is_none_or(|e| e > now)
}

async fn view(state: &AppState, row: &db::Announcement) -> ApiResult<Announcement> {
    let emote = match row.emote_id {
        Some(id) => crate::moderation::emote(state, None, &ids::to_text(id)).await?,
        None => None,
    };
    Ok(Announcement {
        id: ids::to_text(row.id),
        kind: AnnouncementKind::parse(&row.kind),
        text: row.body.clone(),
        link_url: row.link_url.clone(),
        link_label: row.link_label.clone(),
        emote,
        badge_id: row.badge_id.map(ids::to_text),
        starts_at: row.starts_at.map(views::time),
        ends_at: row.ends_at.map(views::time),
        ended_at: row.ended_at.map(views::time),
        created_at: views::time(row.created_at),
        updated_at: views::time(row.updated_at),
        live: is_live(row, Utc::now()),
    })
}

/// The announcement shown now, for everyone (no caller needed).
pub async fn active(state: &AppState) -> ApiResult<Option<Announcement>> {
    let ttl = state.reads.ttl();
    if !ttl.is_zero()
        && let Some(cached) = state.announcement.get(ttl)
    {
        // A cached one that ran out since is gone at once.
        return Ok(cached.filter(|a| {
            a.ends_at
                .as_deref()
                .and_then(|e| DateTime::parse_from_rfc3339(e).ok())
                .is_none_or(|e| e > Utc::now())
        }));
    }
    let generation = state.announcement.generation();
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let row = db::live_at(&mut conn, Utc::now()).await.map_err(db_error)?;
    drop(conn);
    let out = match row {
        Some(r) => Some(view(state, &r).await?),
        None => None,
    };
    if !ttl.is_zero() {
        state.announcement.put(generation, out.clone());
    }
    Ok(out)
}

/// Every announcement, newest first (the latest 50), for staff with `site.announce`.
pub async fn list(state: &AppState, caller: &Caller) -> ApiResult<Vec<Announcement>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::SITE_ANNOUNCE]).await?;
    let rows = db::list(&mut conn, LIST_MAX).await.map_err(db_error)?;
    drop(conn);
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(view(state, r).await?);
    }
    Ok(out)
}

fn time(raw: Option<&str>, field: &str) -> ApiResult<Option<DateTime<Utc>>> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|t| Some(t.with_timezone(&Utc)))
            .map_err(|_| bad("bad_time", format!("{field} isn't a date and time."))),
    }
}

/// A link: `https://` somewhere, or a path on cyex.tv (`/shop`, never `//elsewhere`).
pub fn link(raw: Option<&str>) -> ApiResult<Option<String>> {
    let Some(url) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let ok = url.chars().count() <= LINK_MAX
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && ((url.starts_with('/') && !url.starts_with("//") && !url.starts_with("/\\"))
            || (url.starts_with("https://") && url.len() > "https://".len()));
    if ok {
        Ok(Some(url.to_owned()))
    } else {
        Err(bad(
            "bad_link",
            "Links start with https:// or are a path on cyex.tv, like /shop.",
        ))
    }
}

/// Checks what staff wrote, before the database is asked anything.
pub fn fields(input: &AnnouncementInput, now: DateTime<Utc>) -> ApiResult<db::Fields> {
    let body = input.text.trim();
    let len = body.chars().count();
    if len == 0 || len > TEXT_MAX || text::has_unsafe(body, false) {
        return Err(bad(
            "bad_text",
            "The announcement is one line of 1 to 200 characters.",
        ));
    }
    let link_url = link(input.link_url.as_deref())?;
    let label = input
        .link_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(l) = label
        && (l.chars().count() > LABEL_MAX || text::has_unsafe(l, false))
    {
        return Err(bad("bad_label", "Link words are up to 40 characters."));
    }
    if label.is_some() && link_url.is_none() {
        return Err(bad("bad_label", "Link words need a link."));
    }
    let emote_id = match input
        .emote_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => {
            Some(parse_id(raw).map_err(|_| bad("bad_emote", "That emote doesn't exist."))?)
        }
        None => None,
    };
    let badge_id = match input
        .badge_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => {
            Some(parse_id(raw).map_err(|_| bad("bad_badge", "That badge doesn't exist."))?)
        }
        None => None,
    };
    let starts_at = time(input.starts_at.as_deref(), "The start")?;
    let ends_at = time(input.ends_at.as_deref(), "The end")?;
    if let Some(end) = ends_at {
        if end <= now {
            return Err(bad("bad_time", "The end has to be in the future."));
        }
        if starts_at.is_some_and(|s| end <= s) {
            return Err(bad("bad_time", "The end has to come after the start."));
        }
    }
    Ok(db::Fields {
        kind: input.kind.as_str().to_owned(),
        body: body.to_owned(),
        link_url,
        link_label: label.map(str::to_owned),
        emote_id,
        badge_id,
        starts_at,
        ends_at,
    })
}

/// The emote (approved, public) and badge an announcement names exist.
async fn check_refs(state: &AppState, conn: &mut PgConnection, f: &db::Fields) -> ApiResult<()> {
    if let Some(e) = f.emote_id
        && crate::moderation::emote(state, None, &ids::to_text(e))
            .await?
            .is_none()
    {
        return Err(bad(
            "bad_emote",
            "That emote doesn't exist or isn't approved.",
        ));
    }
    if let Some(b) = f.badge_id
        && !db::badge_exists(conn, b).await.map_err(db_error)?
    {
        return Err(bad("bad_badge", "That badge doesn't exist."));
    }
    Ok(())
}

fn audit_json(r: &db::Announcement) -> serde_json::Value {
    json!({
        "kind": r.kind,
        "text": r.body,
        "link_url": r.link_url,
        "link_label": r.link_label,
        "emote_id": r.emote_id.map(ids::to_text),
        "badge_id": r.badge_id.map(ids::to_text),
        "starts_at": r.starts_at.map(views::time),
        "ends_at": r.ends_at.map(views::time),
        "ended_at": r.ended_at.map(views::time),
    })
}

pub async fn create(
    state: &AppState,
    caller: &Caller,
    input: &AnnouncementInput,
) -> ApiResult<Announcement> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::SITE_ANNOUNCE]).await?;
    let f = fields(input, Utc::now())?;
    check_refs(state, &mut conn, &f).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let row = db::create(&mut tx, caller.account_id, &f)
        .await
        .map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("announcement.create", "announcement", &ids::to_text(row.id))
            .change(None, Some(audit_json(&row))),
    )
    .await?;
    forget_announcement(&mut tx).await?;
    tx.commit().await.map_err(sqlx_error)?;
    state.forget_cached(&[ANNOUNCEMENT_KEY.to_owned()]);
    view(state, &row).await
}

pub async fn update(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &AnnouncementInput,
) -> ApiResult<Announcement> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::SITE_ANNOUNCE]).await?;
    let id = parse_id(id)?;
    let f = fields(input, Utc::now())?;
    check_refs(state, &mut conn, &f).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = db::lock(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if before.ended_at.is_some() {
        return Err(bad("ended", "This announcement has ended. Make a new one."));
    }
    let row = db::update(&mut tx, id, &f).await.map_err(db_error)?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("announcement.update", "announcement", &ids::to_text(id))
            .change(Some(audit_json(&before)), Some(audit_json(&row))),
    )
    .await?;
    forget_announcement(&mut tx).await?;
    tx.commit().await.map_err(sqlx_error)?;
    state.forget_cached(&[ANNOUNCEMENT_KEY.to_owned()]);
    view(state, &row).await
}

/// Ends it now; ending an ended one changes nothing (and writes no audit row).
pub async fn end(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Announcement> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::SITE_ANNOUNCE]).await?;
    let id = parse_id(id)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = db::lock(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let row = match db::end(&mut tx, id).await.map_err(db_error)? {
        Some(row) => {
            record_audit(
                &mut tx,
                caller.account_id,
                Audit::new("announcement.end", "announcement", &ids::to_text(id))
                    .change(Some(audit_json(&before)), Some(audit_json(&row))),
            )
            .await?;
            row
        }
        None => before,
    };
    forget_announcement(&mut tx).await?;
    tx.commit().await.map_err(sqlx_error)?;
    state.forget_cached(&[ANNOUNCEMENT_KEY.to_owned()]);
    view(state, &row).await
}

// Notification preferences

/// Every notification kind, in the order Settings lists them, and whether it must stay on.
pub const NOTIFICATION_KINDS: &[(&str, bool)] = &[
    ("emote.approved", false),
    ("emote.rejected", false),
    // A reviewer changed its public listing or personal-use approval (ADR 0039).
    ("emote.approvals_changed", false),
    ("emote.failed", false),
    ("emote.taken_down", true),
    ("emote.visibility_changed", true),
    ("emote.transfer_requested", true),
    ("emote.transferred", false),
    ("emote.transfer_declined", false),
    ("emote.transfer_cancelled", false),
    // Staff merged an emote of yours into another, or another into yours (ADR 0040).
    ("emote.merged", false),
    ("editor.added", false),
    ("editor.changed", false),
    ("editor.removed", false),
    ("cosmetic.granted", false),
    ("cosmetic.revoked", false),
    // A channel webhook was turned off after repeated failed deliveries (ADR 0034).
    ("webhook.disabled", true),
];

fn prefs_view(p: &prefs::Prefs) -> Vec<NotificationPref> {
    NOTIFICATION_KINDS
        .iter()
        .map(|(kind, required)| NotificationPref {
            kind: (*kind).to_owned(),
            notify: *required || !p.notify_off.iter().any(|k| k == kind),
            toast: !p.toast_off.iter().any(|k| k == kind),
            required: *required,
        })
        .collect()
}

fn rate(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::Prefs,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of changes to your settings. Try again in a while.",
    )
}

pub async fn my_notification_prefs(
    state: &AppState,
    caller: &Caller,
) -> ApiResult<Vec<NotificationPref>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let p = prefs::get(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    Ok(prefs_view(&p))
}

/// What to store from a change: kinds off and kinds that don't pop up, the others kept as they
/// were. Unknown kinds, a kind twice and turning off a required one are refused.
pub fn merge_prefs(
    current: &prefs::Prefs,
    changes: &[NotificationPrefInput],
) -> ApiResult<(Vec<String>, Vec<String>)> {
    let mut seen = std::collections::HashSet::new();
    let mut notify_off: Vec<String> = current.notify_off.clone();
    let mut toast_off: Vec<String> = current.toast_off.clone();
    for c in changes {
        let kind = c.kind.trim();
        let Some((_, required)) = NOTIFICATION_KINDS.iter().find(|(k, _)| *k == kind) else {
            return Err(bad("unknown_kind", "That isn't a kind of notification."));
        };
        if !seen.insert(kind) {
            return Err(bad("unknown_kind", "Each kind once."));
        }
        if *required && !c.notify {
            return Err(bad(
                "required_kind",
                "Hand-over requests and takedowns always notify you.",
            ));
        }
        notify_off.retain(|k| k != kind);
        toast_off.retain(|k| k != kind);
        if !c.notify {
            notify_off.push(kind.to_owned());
        }
        if !c.toast {
            toast_off.push(kind.to_owned());
        }
    }
    // Only kinds that exist stay stored, in the listed order.
    let order = |v: Vec<String>| -> Vec<String> {
        NOTIFICATION_KINDS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| v.iter().any(|x| x == k))
            .map(str::to_owned)
            .collect()
    };
    Ok((order(notify_off), order(toast_off)))
}

pub async fn set_notification_prefs(
    state: &AppState,
    caller: &Caller,
    changes: &[NotificationPrefInput],
) -> ApiResult<Vec<NotificationPref>> {
    if changes.len() > NOTIFICATION_KINDS.len() {
        return Err(cyex_tv_shared::error::ApiError::limit(
            "too_many",
            "Too many kinds at once.",
            cyex_tv_shared::limit_errors::n(NOTIFICATION_KINDS.len()),
            cyex_tv_shared::limit_errors::n(changes.len()),
        ));
    }
    rate(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let current = prefs::get_for_update(&mut tx, caller.account_id)
        .await
        .map_err(db_error)?;
    let (notify_off, toast_off) = merge_prefs(&current, changes)?;
    let p = prefs::set_notifications(&mut tx, caller.account_id, &notify_off, &toast_off)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(prefs_view(&p))
}

// The collection

fn collection_source(raw: &str) -> GrantSource {
    match raw {
        "purchase" => GrantSource::Purchase,
        "code" => GrantSource::Code,
        "event" => GrantSource::Event,
        _ => GrantSource::Staff,
    }
}

/// An account's owned badges and paints: for the account itself always, for everyone else only
/// while it shows them (and never for a deleted or blocked account). `None` otherwise.
pub async fn collection(
    state: &AppState,
    caller: Option<&Caller>,
    account_id: &str,
) -> ApiResult<Option<AccountCollection>> {
    let Some(id) = ids::from_text(account_id.trim()) else {
        return Ok(None);
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let Some(account) = accounts::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    let own = caller.is_some_and(|c| c.account_id == id);
    if !own && (account.deleted_at.is_some() || account.blocked_at.is_some()) {
        return Ok(None);
    }
    let p = prefs::get(&mut conn, id).await.map_err(db_error)?;
    if !own && !p.show_collection {
        return Ok(None);
    }
    let items = prefs::owned(&mut conn, id, COLLECTION_MAX)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|o| CollectionItem {
            kind: if o.kind == "paint" {
                GrantKind::Paint
            } else {
                GrantKind::Badge
            },
            cosmetic_id: ids::to_text(o.item_id),
            name: o.item_name,
            owned_since: views::time(o.granted_at),
            source: collection_source(&o.source),
        })
        .collect();
    Ok(Some(AccountCollection {
        account_id: ids::to_text(id),
        public: p.show_collection,
        items,
    }))
}

/// Shows or hides your collection on your profile. Returns it as you see it.
pub async fn set_collection_public(
    state: &AppState,
    caller: &Caller,
    public: bool,
) -> ApiResult<AccountCollection> {
    rate(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    prefs::set_show_collection(&mut conn, caller.account_id, public)
        .await
        .map_err(db_error)?;
    drop(conn);
    collection(state, Some(caller), &ids::to_text(caller.account_id))
        .await?
        .ok_or_else(not_found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> Announcement {
        Announcement {
            id: "a".into(),
            kind: AnnouncementKind::Info,
            text: text.into(),
            link_url: None,
            link_label: None,
            emote: None,
            badge_id: None,
            starts_at: None,
            ends_at: None,
            ended_at: None,
            created_at: "2026-11-20T00:00:00Z".into(),
            updated_at: "2026-11-20T00:00:00Z".into(),
            live: true,
        }
    }

    /// A read that started before a change cleared the cache doesn't store the old line.
    #[test]
    fn a_read_from_before_a_clear_is_not_stored() {
        let ttl = std::time::Duration::from_secs(60);
        let cache = AnnouncementCache::default();
        let slow = cache.generation();
        cache.clear();
        cache.put(slow, Some(line("old")));
        assert_eq!(cache.get(ttl), None);

        let fresh = cache.generation();
        cache.put(fresh, Some(line("new")));
        assert_eq!(cache.get(ttl), Some(Some(line("new"))));
        cache.clear();
        assert_eq!(cache.get(ttl), None);
    }

    fn input() -> AnnouncementInput {
        AnnouncementInput {
            kind: AnnouncementKind::Info,
            text: "  Hello  ".into(),
            link_url: None,
            link_label: None,
            emote_id: None,
            badge_id: None,
            starts_at: None,
            ends_at: None,
        }
    }

    #[test]
    fn links_are_https_or_a_path_here() {
        assert_eq!(link(Some(" /shop ")).unwrap().as_deref(), Some("/shop"));
        assert!(link(Some("https://cyex.tv/x")).is_ok());
        assert_eq!(link(Some("  ")).unwrap(), None);
        for bad in [
            "//evil.example",
            "/\\evil",
            "http://cyex.tv",
            "javascript:alert(1)",
            "https://",
            "/a b",
        ] {
            assert!(link(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn fields_trim_and_check_the_window() {
        let now = Utc::now();
        let f = fields(&input(), now).unwrap();
        assert_eq!(f.body, "Hello");
        let mut i = input();
        i.text = "x".repeat(201);
        assert!(fields(&i, now).is_err());
        let mut i = input();
        i.text = "two\nlines".into();
        assert!(fields(&i, now).is_err());
        let mut i = input();
        i.ends_at = Some("2000-01-01T00:00:00Z".into());
        assert!(fields(&i, now).is_err());
        let mut i = input();
        i.starts_at = Some("2999-01-02T00:00:00Z".into());
        i.ends_at = Some("2999-01-01T00:00:00Z".into());
        assert!(fields(&i, now).is_err());
        let mut i = input();
        i.link_label = Some("Go".into());
        assert!(fields(&i, now).is_err(), "a label without a link");
        let mut i = input();
        i.starts_at = Some("soon".into());
        assert!(fields(&i, now).is_err());
    }

    #[test]
    fn prefs_merge_and_refuse_required_kinds() {
        let cur = prefs::Prefs::default();
        let (off, quiet) = merge_prefs(
            &cur,
            &[
                NotificationPrefInput {
                    kind: "cosmetic.granted".into(),
                    notify: false,
                    toast: false,
                },
                NotificationPrefInput {
                    kind: "emote.approved".into(),
                    notify: true,
                    toast: false,
                },
            ],
        )
        .unwrap();
        assert_eq!(off, vec!["cosmetic.granted"]);
        assert_eq!(quiet, vec!["emote.approved", "cosmetic.granted"]);
        let req = NotificationPrefInput {
            kind: "emote.transfer_requested".into(),
            notify: false,
            toast: true,
        };
        assert!(merge_prefs(&cur, &[req]).is_err());
        let unknown = NotificationPrefInput {
            kind: "emote.everything".into(),
            notify: true,
            toast: true,
        };
        assert!(merge_prefs(&cur, &[unknown]).is_err());
    }
}
