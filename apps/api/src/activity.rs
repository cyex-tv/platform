//! The public account activity feed (ADR 0016), like 7TV's profile "Activity" tab. Built from
//! `audit_log` rows that the owner's own changes and their account editors' changes write with
//! the account as their subject; the SQL side is `audit::activity_rows`.
//!
//! What never shows: moderation actions, reasons, staff-only changes, pending, private, rejected
//! and taken-down emotes, deleted sets, and anything about a blocked, deleted or unknown account
//! (an empty list). A line about an emote that isn't public (yet) is left out; it appears once
//! the emote is approved. Only listed sets show (ADR 0017): an unlisted set's ID is its secret and
//! a private set is nobody else's business, so lines about them are left out, whatever their
//! visibility was when the line was written.

use std::collections::{HashMap, HashSet};

use cyex_tv_db::cosmetics as cosmetics_db;
use cyex_tv_db::reads::{self, EmoteRow};
use cyex_tv_db::sets::Set;
use cyex_tv_db::{accounts, audit, channels, ids, sets};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    AccountSummary, ActivityCosmeticChange, ActivityEmote, ActivityEntry, ActivityKind,
    ActivityPage, ActivitySet, EmoteState, SetVisibility,
};
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::catalog::{conn, db_error, parse_id};
use crate::state::AppState;
use crate::views;

/// The most entries one page returns.
pub const ACTIVITY_PAGE_MAX: u32 = 100;
pub const ACTIVITY_PAGE: u32 = 30;
/// Candidate rows read per round. Rows can be dropped after the SQL filter (what it can't tell:
/// deleted sets, emotes that aren't public), so a page may take a few rounds. A page that runs out
/// of rounds comes back short with a `next` cursor to carry on from.
const BATCH: i64 = 100;
const ROUNDS: usize = 4;

/// Everything the rows of one round refer to.
struct Lookup {
    sets: HashMap<Uuid, Set>,
    channels: HashMap<Uuid, channels::Channel>,
    emotes: HashMap<Uuid, ActivityEmote>,
    badges: HashMap<Uuid, String>,
    paints: HashMap<Uuid, String>,
    people: HashMap<Uuid, AccountSummary>,
}

fn text(v: &Option<Value>, key: &str) -> Option<String> {
    v.as_ref()?.get(key)?.as_str().map(str::to_owned)
}

fn id_of(v: &Option<Value>, key: &str) -> Option<Uuid> {
    text(v, key).and_then(|t| ids::from_text(&t))
}

/// Every ID the rows mention, by kind.
#[derive(Default)]
struct Refs {
    sets: HashSet<Uuid>,
    channels: HashSet<Uuid>,
    emotes: HashSet<Uuid>,
    badges: HashSet<Uuid>,
    paints: HashSet<Uuid>,
    people: HashSet<Uuid>,
}

fn refs(rows: &[audit::Entry]) -> Refs {
    let mut r = Refs::default();
    for e in rows {
        r.people.extend(e.actor_account_id);
        let target = ids::from_text(&e.target_id);
        match e.action.as_str() {
            "set.created" | "set.renamed" => r.sets.extend(target),
            "set.emote_added" | "set.emote_removed" | "set.emote_updated" => {
                r.sets.extend(target);
                for side in [&e.before_json, &e.after_json] {
                    r.emotes.extend(id_of(side, "emote_id"));
                }
            }
            "channel.settings_changed" => {
                r.channels.extend(target);
                for side in [&e.before_json, &e.after_json] {
                    r.sets.extend(id_of(side, "active_set_id"));
                }
            }
            "cosmetics.selected" => {
                for side in [&e.before_json, &e.after_json] {
                    r.badges.extend(id_of(side, "badge_id"));
                    r.paints.extend(id_of(side, "paint_id"));
                }
            }
            "emote.approved" => r.emotes.extend(target),
            _ => {}
        }
    }
    r
}

fn list(set: &HashSet<Uuid>) -> Vec<Uuid> {
    set.iter().copied().collect()
}

async fn lookup(
    conn: &mut PgConnection,
    rows: &[audit::Entry],
    owners: &[Uuid],
    cdn_url: &str,
) -> ApiResult<Lookup> {
    let mut r = refs(rows);
    // An approval shows the owner as its actor.
    r.people.extend(owners.iter().copied());
    let sets = sets::get_many(conn, &list(&r.sets))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();
    let channels = channels::get_many(conn, &list(&r.channels))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|c| (c.id, c))
        .collect();
    // Only emotes anyone may see: approved, not private, not deleted.
    let emote_rows: Vec<EmoteRow> = reads::emotes(conn, &list(&r.emotes))
        .await
        .map_err(db_error)?
        .into_iter()
        .filter(|e| e.state == EmoteState::Approved && !e.private)
        .collect();
    let built = views::emotes(conn, &emote_rows, cdn_url)
        .await
        .map_err(db_error)?;
    let emotes = emote_rows
        .iter()
        .zip(built)
        .map(|(row, emote)| {
            (
                row.id,
                ActivityEmote {
                    id: emote.id,
                    name: emote.name,
                    host: emote.host,
                    files: emote.files,
                },
            )
        })
        .collect();
    let badges = cosmetics_db::badges(conn, Some(&list(&r.badges)))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|b| (b.id, b.name))
        .collect();
    let paints = cosmetics_db::paints(conn, Some(&list(&r.paints)))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|p| (p.id, p.name))
        .collect();
    let people = reads::summaries(conn, &list(&r.people))
        .await
        .map_err(db_error)?
        .into_values()
        .map(|s| (s.account_id, AccountSummary::from(&s)))
        .collect();
    Ok(Lookup {
        sets,
        channels,
        emotes,
        badges,
        paints,
        people,
    })
}

impl Lookup {
    /// A listed set of this account, by ID.
    fn set_of(&self, id: Uuid, owner: Uuid) -> Option<&Set> {
        self.sets
            .get(&id)
            .filter(|s| s.owner_account_id == Some(owner) && s.visibility == SetVisibility::Listed)
    }

    fn named_set(&self, id: Uuid, owner: Uuid) -> Option<ActivitySet> {
        self.set_of(id, owner).map(|s| ActivitySet {
            id: ids::to_text(s.id),
            name: s.name.clone(),
        })
    }

    /// The emote as named at the time of the change; `None` when it isn't public.
    fn emote_named(&self, id: Uuid, name: Option<String>) -> Option<ActivityEmote> {
        let mut emote = self.emotes.get(&id)?.clone();
        if let Some(name) = name {
            emote.name = name;
        }
        Some(emote)
    }
}

fn change(
    from: Option<Uuid>,
    to: Option<Uuid>,
    names: &HashMap<Uuid, String>,
) -> Option<ActivityCosmeticChange> {
    if from == to {
        return None;
    }
    Some(ActivityCosmeticChange {
        from_id: from.map(ids::to_text),
        from_name: from.and_then(|id| names.get(&id).cloned()),
        to_id: to.map(ids::to_text),
        to_name: to.and_then(|id| names.get(&id).cloned()),
    })
}

/// The feed line a row stands for, or `None` when it must not show.
fn entry(row: &audit::Entry, owner: Uuid, look: &Lookup) -> Option<ActivityEntry> {
    let target = ids::from_text(&row.target_id);
    let mut out = ActivityEntry {
        id: ids::to_text(row.id),
        kind: ActivityKind::SetCreated,
        actor: look.people.get(&row.actor_account_id?)?.clone(),
        set: None,
        emote: None,
        from_name: None,
        to_name: None,
        badge: None,
        paint: None,
        created_at: views::time(row.created_at),
    };
    match row.action.as_str() {
        "set.created" => {
            out.kind = ActivityKind::SetCreated;
            out.set = Some(look.named_set(target?, owner)?);
        }
        "set.renamed" => {
            out.kind = ActivityKind::SetRenamed;
            out.set = Some(look.named_set(target?, owner)?);
            out.from_name = text(&row.before_json, "name");
            out.to_name = text(&row.after_json, "name");
            if out.from_name == out.to_name {
                return None;
            }
        }
        "set.emote_added" | "set.emote_removed" | "set.emote_updated" => {
            out.set = Some(look.named_set(target?, owner)?);
            let side = if row.action == "set.emote_removed" {
                &row.before_json
            } else {
                &row.after_json
            };
            let emote_id = id_of(side, "emote_id")?;
            out.emote = Some(look.emote_named(emote_id, text(side, "name"))?);
            match row.action.as_str() {
                "set.emote_added" => out.kind = ActivityKind::SetEmoteAdded,
                "set.emote_removed" => out.kind = ActivityKind::SetEmoteRemoved,
                _ => {
                    out.kind = ActivityKind::SetEmoteRenamed;
                    out.from_name = text(&row.before_json, "name");
                    out.to_name = text(&row.after_json, "name");
                    // Only a new name is news; a row without the old name can't say.
                    if out.from_name.is_none() || out.from_name == out.to_name {
                        return None;
                    }
                }
            }
        }
        "channel.settings_changed" => {
            if look.channels.get(&target?)?.account_id != owner {
                return None;
            }
            let from = id_of(&row.before_json, "active_set_id");
            let to = id_of(&row.after_json, "active_set_id");
            if from == to {
                return None;
            }
            out.kind = ActivityKind::ActiveSetChanged;
            if let Some(to) = to {
                out.set = Some(look.named_set(to, owner)?);
            }
            out.to_name = out.set.as_ref().map(|s| s.name.clone());
            out.from_name = from
                .and_then(|id| look.set_of(id, owner))
                .map(|s| s.name.clone());
        }
        "cosmetics.selected" => {
            out.kind = ActivityKind::CosmeticsSelected;
            out.badge = change(
                id_of(&row.before_json, "badge_id"),
                id_of(&row.after_json, "badge_id"),
                &look.badges,
            );
            out.paint = change(
                id_of(&row.before_json, "paint_id"),
                id_of(&row.after_json, "paint_id"),
                &look.paints,
            );
            if out.badge.is_none() && out.paint.is_none() {
                return None;
            }
        }
        "emote.approved" => {
            out.kind = ActivityKind::EmoteUploaded;
            out.emote = Some(look.emote_named(target?, None)?);
            // The reviewer isn't shown: the owner uploaded it.
            out.actor = look.people.get(&owner)?.clone();
        }
        _ => return None,
    }
    Some(out)
}

/// An account's public activity, newest first. `before` is an entry ID to page back from;
/// `limit` is 1 to 100, default 30. Empty for an unknown, deleted or blocked account.
pub async fn account_activity(
    state: &AppState,
    account_id: &str,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<Vec<ActivityEntry>> {
    Ok(account_activity_page(state, account_id, before, limit)
        .await?
        .entries)
}

/// `account_activity` with the cursor for the next page.
pub async fn account_activity_page(
    state: &AppState,
    account_id: &str,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<ActivityPage> {
    let before_id = before.map(parse_id).transpose()?;
    let limit = limit.unwrap_or(ACTIVITY_PAGE).clamp(1, ACTIVITY_PAGE_MAX) as usize;
    let Some(owner) = ids::from_text(account_id.trim()) else {
        return Ok(ActivityPage::default());
    };
    let mut conn = conn(state).await?;
    let Some(account) = accounts::get(&mut conn, owner).await.map_err(db_error)? else {
        return Ok(ActivityPage::default());
    };
    if account.deleted_at.is_some() || account.blocked_at.is_some() {
        return Ok(ActivityPage::default());
    }
    let mut page = Page::new(limit, before_id);
    while page.wants_more() {
        let rows = audit::activity_rows(&mut conn, owner, page.cursor, BATCH)
            .await
            .map_err(db_error)?;
        let look = lookup(&mut conn, &rows, &[owner], &state.cdn_url).await?;
        let lines = rows.iter().map(|row| (row.id, entry(row, owner, &look)));
        page.round(rows.len(), lines);
    }
    Ok(page.finish())
}

/// One emote's public activity, newest first (gap list G2): when it was uploaded (approved), and
/// when it was added to, removed from or renamed in listed sets, by their owners or account
/// editors. Each line is built as on the set owner's own feed, so the same things never show
/// (moderation, reviewers, reasons, unlisted, private and deleted sets). Empty unless the emote
/// is public: approved, not private, not deleted. `before` and `limit` as `accountActivity`.
pub async fn emote_activity(
    state: &AppState,
    emote_id: &str,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<Vec<ActivityEntry>> {
    Ok(emote_activity_page(state, emote_id, before, limit)
        .await?
        .entries)
}

/// `emote_activity` with the cursor for the next page.
pub async fn emote_activity_page(
    state: &AppState,
    emote_id: &str,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<ActivityPage> {
    let before_id = before.map(parse_id).transpose()?;
    let limit = limit.unwrap_or(ACTIVITY_PAGE).clamp(1, ACTIVITY_PAGE_MAX) as usize;
    let Some(emote) = ids::from_text(emote_id.trim()) else {
        return Ok(ActivityPage::default());
    };
    let mut conn = conn(state).await?;
    let public = reads::emotes(&mut conn, &[emote])
        .await
        .map_err(db_error)?
        .first()
        .is_some_and(|e| e.state == EmoteState::Approved && !e.private);
    if !public {
        return Ok(ActivityPage::default());
    }
    let key = ids::to_text(emote);
    let mut page = Page::new(limit, before_id);
    while page.wants_more() {
        let rows = audit::emote_activity_rows(&mut conn, &key, page.cursor, BATCH)
            .await
            .map_err(db_error)?;
        let entries: Vec<audit::Entry> = rows.iter().map(|r| r.entry.clone()).collect();
        let owners: Vec<Uuid> = rows
            .iter()
            .map(|r| r.subject_account_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let look = lookup(&mut conn, &entries, &owners, &state.cdn_url).await?;
        let lines = rows
            .iter()
            .map(|r| (r.entry.id, entry(&r.entry, r.subject_account_id, &look)));
        page.round(rows.len(), lines);
    }
    Ok(page.finish())
}

/// The paging loop's state: lines so far, where the next round reads from, and whether the
/// feed has more.
struct Page {
    limit: usize,
    rounds: usize,
    cursor: Option<Uuid>,
    out: Vec<ActivityEntry>,
    /// The last row looked at, when the page ended before the feed did.
    more_after: Option<Uuid>,
    done: bool,
}

impl Page {
    fn new(limit: usize, cursor: Option<Uuid>) -> Self {
        Self {
            limit,
            rounds: 0,
            cursor,
            out: Vec::new(),
            more_after: None,
            done: false,
        }
    }

    fn wants_more(&self) -> bool {
        !self.done && self.rounds < ROUNDS
    }

    /// Takes one round's rows (their IDs, newest first, with the line each makes if any).
    fn round(
        &mut self,
        fetched: usize,
        lines: impl Iterator<Item = (Uuid, Option<ActivityEntry>)>,
    ) {
        self.rounds += 1;
        let mut last = None;
        for (id, line) in lines {
            last = Some(id);
            if let Some(line) = line {
                self.out.push(line);
                if self.out.len() == self.limit {
                    // Resume right after the line that filled the page.
                    self.more_after = Some(id);
                    self.done = true;
                    return;
                }
            }
        }
        match last {
            Some(id) if fetched >= BATCH as usize => {
                self.cursor = Some(id);
                self.more_after = Some(id);
            }
            _ => {
                self.more_after = None;
                self.done = true;
            }
        }
    }

    fn finish(self) -> ActivityPage {
        ActivityPage {
            entries: self.out,
            next: self.more_after.map(ids::to_text),
        }
    }
}
