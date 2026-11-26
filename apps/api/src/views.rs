//! Database rows to API shapes, shared by the GraphQL resolvers and the REST reads.
//!
//! Who sees a pending emote in a set: its owner, the set's owner, editors and account editors, and
//! moderators (`emotes.review`). Everyone else, chat clients included, sees approved emotes only. Rejected,
//! taken-down and deleted emotes are never shown.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use cyex_tv_db::reads::{self, EmoteRow, MemberRow, Summary};
use cyex_tv_db::sets::Set;
use cyex_tv_db::{DbResult, channels, editors, ids, sets};
use cyex_tv_shared::model::{
    AccountSummary, Channel, Emote, EmoteFile, EmoteFlags, EmoteProcessing, EmoteSet, EmoteState,
    EmoteVisibility, SetEmote, SetSummary, SetVisibility,
};
use sqlx::PgConnection;
use uuid::Uuid;

/// Times as RFC 3339 in UTC with milliseconds, for example `2026-11-16T20:01:02.345Z`.
pub fn time(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Whose eyes a set is built for.
#[derive(Clone, Copy, Debug, Default)]
pub struct Viewer {
    pub account_id: Option<Uuid>,
    /// Holds `emotes.review`: sees every pending emote.
    pub reviewer: bool,
}

impl Viewer {
    /// Signed out, or a chat client.
    pub fn public() -> Self {
        Self::default()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn summary(summaries: &HashMap<Uuid, Summary>, id: Option<Uuid>) -> Option<AccountSummary> {
    summaries.get(&id?).map(AccountSummary::from)
}

/// The files of an emote, from `files.variants` (written by the transcode job in this
/// shape). Anything that doesn't parse is left out rather than failing the read.
fn files(variants: Option<&serde_json::Value>) -> Vec<EmoteFile> {
    match variants {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

/// The 1x file is at least 1.75 times as wide as it is tall.
fn is_wide(files: &[EmoteFile]) -> bool {
    files
        .iter()
        .find(|f| f.name == "1x.webp")
        .is_some_and(|f| u32::from(f.width) * 4 >= u32::from(f.height) * 7)
}

/// Made, failed for good, or still being made. A file with its 1x WebP is ready whatever else
/// happened (the AVIF step may have failed after it).
fn processing_of(row: &EmoteRow, files: &[EmoteFile]) -> EmoteProcessing {
    if files.iter().any(|f| f.name == "1x.webp") {
        EmoteProcessing::Ready
    } else if row.processing_failed {
        EmoteProcessing::Failed
    } else {
        EmoteProcessing::Processing
    }
}

fn emote(
    row: &EmoteRow,
    tags: &HashMap<Uuid, Vec<String>>,
    summaries: &HashMap<Uuid, Summary>,
    counts: &HashMap<Uuid, u32>,
    cdn_url: &str,
) -> Emote {
    let host = row
        .sha256
        .as_deref()
        .map(|sha| format!("{cdn_url}/emotes/{}", hex(sha)))
        .unwrap_or_default();
    let files = files(row.variants.as_ref());
    let processing = processing_of(row, &files);
    let wide = is_wide(&files);
    let files = EmoteFile::with_urls(files, &host);
    Emote {
        id: ids::to_text(row.id),
        name: row.name.clone(),
        owner: summary(summaries, row.owner),
        state: row.state,
        processing,
        flags: EmoteFlags {
            zero_width: row.zero_width,
            private: row.private,
            // Retired: NSFW emotes aren't allowed (ADR 0020); an old flag only keeps the emote
            // off the listings until staff save its flags.
            nsfw: false,
            wide,
        },
        visibility: EmoteVisibility::of(row.private, row.unlisted),
        animated: row.animated,
        tags: tags.get(&row.id).cloned().unwrap_or_default(),
        host,
        files,
        created_at: time(row.created_at),
        channel_count: counts.get(&row.id).copied().unwrap_or(0),
    }
}

/// Builds emotes in the order given, with their tags and owners.
pub async fn emotes(
    conn: &mut PgConnection,
    rows: &[EmoteRow],
    cdn_url: &str,
) -> DbResult<Vec<Emote>> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let tags = reads::tags(conn, &ids).await?;
    let owners: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.owner)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let summaries = reads::summaries(conn, &owners).await?;
    let counts = reads::channel_counts(conn, &ids).await?;
    Ok(rows
        .iter()
        .map(|row| emote(row, &tags, &summaries, &counts, cdn_url))
        .collect())
}

/// Whether `viewer` may see this member. `edits` holds the sets the viewer owns or edits.
fn visible(row: &MemberRow, viewer: &Viewer, edits: &HashSet<Uuid>) -> bool {
    match row.state {
        EmoteState::Approved => true,
        EmoteState::Pending => {
            viewer.reviewer
                || (viewer.account_id.is_some() && viewer.account_id == row.emote_owner)
                || edits.contains(&row.set_id)
        }
        EmoteState::Rejected | EmoteState::TakenDown => false,
    }
}

/// The sets `viewer` owns or edits, among `sets`.
async fn editable(
    conn: &mut PgConnection,
    sets: &[Set],
    viewer: &Viewer,
) -> DbResult<HashSet<Uuid>> {
    let mut out = HashSet::new();
    let Some(account) = viewer.account_id else {
        return Ok(out);
    };
    for set in sets {
        if set.owner_account_id == Some(account)
            || sets::can_edit(conn, account, set.id).await?
            || account_editor(conn, set, account).await?
        {
            out.insert(set.id);
        }
    }
    Ok(out)
}

/// An account editor of the set's owner who works on sets (`emotes` or `sets`, ADR 0014) and
/// whose grant covers this one.
async fn account_editor(conn: &mut PgConnection, set: &Set, account: Uuid) -> DbResult<bool> {
    let Some(owner) = set.owner_account_id else {
        return Ok(false);
    };
    let works_on_sets = editors::permissions_of(conn, owner, account)
        .await?
        .is_some_and(|flags| flags.iter().any(|f| f == "emotes" || f == "sets"));
    Ok(works_on_sets && editors::covers_set(conn, owner, account, set.id).await?)
}

fn set_emote(
    row: &MemberRow,
    tags: &HashMap<Uuid, Vec<String>>,
    summaries: &HashMap<Uuid, Summary>,
    counts: &HashMap<Uuid, u32>,
    cdn_url: &str,
) -> SetEmote {
    SetEmote {
        name: row.name.clone(),
        zero_width: row.zero_width,
        added_by: row.added_by.map(ids::to_text),
        added_at: time(row.added_at),
        emote: emote(&row.emote(), tags, summaries, counts, cdn_url),
    }
}

/// Builds several sets with their emotes, in the order given.
pub async fn sets(
    conn: &mut PgConnection,
    sets: &[Set],
    viewer: &Viewer,
    cdn_url: &str,
) -> DbResult<Vec<EmoteSet>> {
    build_sets(conn, sets, viewer, cdn_url, None).await
}

/// `sets`, or with `preview` only the newest that many emotes of each set, newest first and
/// read as such, however large the sets are.
async fn build_sets(
    conn: &mut PgConnection,
    sets: &[Set],
    viewer: &Viewer,
    cdn_url: &str,
    preview: Option<usize>,
) -> DbResult<Vec<EmoteSet>> {
    let set_ids: Vec<Uuid> = sets.iter().map(|s| s.id).collect();
    let edits = editable(conn, sets, viewer).await?;
    let rows: Vec<MemberRow> = match preview {
        None => reads::members(conn, &set_ids)
            .await?
            .into_iter()
            .filter(|row| visible(row, viewer, &edits))
            .collect(),
        Some(0) => Vec::new(),
        Some(n) => {
            let edit_sets: Vec<Uuid> = edits.iter().copied().collect();
            let access = reads::MemberAccess {
                account: viewer.account_id,
                reviewer: viewer.reviewer,
                edit_sets: &edit_sets,
            };
            reads::newest_members(
                conn,
                &set_ids,
                &access,
                i64::try_from(n).unwrap_or(i64::MAX),
            )
            .await?
        }
    };

    let emote_ids: Vec<Uuid> = rows.iter().map(|r| r.emote_id).collect();
    let tags = reads::tags(conn, &emote_ids).await?;
    let accounts: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.emote_owner)
        .chain(sets.iter().filter_map(|s| s.owner_account_id))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let summaries = reads::summaries(conn, &accounts).await?;
    let counts = reads::channel_counts(conn, &emote_ids).await?;
    // Editors count every member; everyone else, the members they can see.
    let edit_sets: Vec<Uuid> = edits.iter().copied().collect();
    let mut used = reads::visible_counts(
        conn,
        &set_ids,
        &reads::MemberAccess {
            account: viewer.account_id,
            reviewer: viewer.reviewer,
            edit_sets: &edit_sets,
        },
    )
    .await?;
    if !edit_sets.is_empty() {
        used.extend(sets::counts(conn, &edit_sets).await?);
    }

    let mut by_set: HashMap<Uuid, Vec<SetEmote>> = HashMap::new();
    for row in &rows {
        by_set
            .entry(row.set_id)
            .or_default()
            .push(set_emote(row, &tags, &summaries, &counts, cdn_url));
    }
    Ok(sets
        .iter()
        .map(|set| EmoteSet {
            id: ids::to_text(set.id),
            name: set.name.clone(),
            kind: set.kind,
            owner: summary(&summaries, set.owner_account_id),
            capacity: u32::try_from(set.capacity).unwrap_or(0),
            emotes: by_set.remove(&set.id).unwrap_or_default(),
            visibility: set.visibility,
            listed: set.visibility == SetVisibility::Listed,
            slots_used: used
                .get(&set.id)
                .map_or(0, |n| u32::try_from(*n).unwrap_or(u32::MAX)),
            created_at: time(set.created_at),
        })
        .collect())
}

/// Sets as profile cards: facts, whether a channel shows each, and the newest `preview` emotes
/// the viewer may see.
pub async fn set_summaries(
    conn: &mut PgConnection,
    rows: &[Set],
    viewer: &Viewer,
    cdn_url: &str,
    preview: usize,
) -> DbResult<Vec<SetSummary>> {
    let ids: Vec<Uuid> = rows.iter().map(|s| s.id).collect();
    let active = sets::active(conn, &ids).await?;
    let built = build_sets(conn, rows, viewer, cdn_url, Some(preview)).await?;
    Ok(built
        .into_iter()
        .zip(rows)
        .map(|(set, row)| {
            SetSummary {
                id: set.id,
                name: set.name,
                kind: set.kind,
                owner: set.owner,
                visibility: set.visibility,
                listed: set.listed,
                capacity: set.capacity,
                slots_used: set.slots_used,
                active: active.contains(&row.id),
                // Newest first, as read.
                preview: set.emotes,
                created_at: set.created_at,
            }
        })
        .collect())
}

pub async fn set(
    conn: &mut PgConnection,
    set: &Set,
    viewer: &Viewer,
    cdn_url: &str,
) -> DbResult<EmoteSet> {
    let mut built = sets(conn, std::slice::from_ref(set), viewer, cdn_url).await?;
    Ok(built.remove(0))
}

/// One page of a set's members the viewer may see, oldest first, narrowed by `query`, and how
/// many match in all.
#[allow(clippy::too_many_arguments)]
pub async fn set_emotes(
    conn: &mut PgConnection,
    set: &Set,
    viewer: &Viewer,
    cdn_url: &str,
    query: Option<&str>,
    limit: i64,
    offset: i64,
    after: Option<&[cyex_tv_db::keyset::Value]>,
) -> DbResult<(Vec<SetEmote>, i64, Option<Vec<cyex_tv_db::keyset::Value>>)> {
    let edits: Vec<Uuid> = editable(conn, std::slice::from_ref(set), viewer)
        .await?
        .into_iter()
        .collect();
    let access = reads::MemberAccess {
        account: viewer.account_id,
        reviewer: viewer.reviewer,
        edit_sets: &edits,
    };
    let (page, total) =
        reads::members_page(conn, set.id, &access, query, limit, offset, after).await?;
    let rows = page.rows;
    let emote_ids: Vec<Uuid> = rows.iter().map(|r| r.emote_id).collect();
    let tags = reads::tags(conn, &emote_ids).await?;
    let owners: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.emote_owner)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let summaries = reads::summaries(conn, &owners).await?;
    let counts = reads::channel_counts(conn, &emote_ids).await?;
    let items = rows
        .iter()
        .map(|row| set_emote(row, &tags, &summaries, &counts, cdn_url))
        .collect();
    Ok((items, total, page.next))
}

/// One member as `SetEmote`, for the answer to a set edit (the editor sees pending emotes).
pub async fn member(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_id: Uuid,
    cdn_url: &str,
) -> DbResult<Option<SetEmote>> {
    let Some(row) = reads::member(conn, set_id, emote_id).await? else {
        return Ok(None);
    };
    let tags = reads::tags(conn, &[emote_id]).await?;
    let owners: Vec<Uuid> = row.emote_owner.into_iter().collect();
    let summaries = reads::summaries(conn, &owners).await?;
    let counts = reads::channel_counts(conn, &[emote_id]).await?;
    Ok(Some(set_emote(&row, &tags, &summaries, &counts, cdn_url)))
}

fn channel_of(ch: &channels::Channel, summaries: &HashMap<Uuid, Summary>) -> Option<Channel> {
    Some(Channel {
        platform: ch.platform,
        platform_channel_id: ch.platform_channel_id.clone(),
        login: ch.login.clone(),
        account: summary(summaries, Some(ch.account_id))?,
        active_set_id: ch.active_set_id.map(ids::to_text),
        allow_personal_emotes: ch.allow_personal_emotes,
    })
}

/// A channel, or `None` when its account is gone.
pub async fn channel(conn: &mut PgConnection, ch: &channels::Channel) -> DbResult<Option<Channel>> {
    let summaries = reads::summaries(conn, &[ch.account_id]).await?;
    Ok(channel_of(ch, &summaries))
}

/// Several channels in the order given, with one query for their accounts. Channels whose
/// account is gone are left out.
pub async fn channels_of(
    conn: &mut PgConnection,
    chs: &[channels::Channel],
) -> DbResult<Vec<Channel>> {
    let accounts: Vec<Uuid> = chs.iter().map(|c| c.account_id).collect();
    let summaries = reads::summaries(conn, &accounts).await?;
    Ok(chs
        .iter()
        .filter_map(|c| channel_of(c, &summaries))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_rfc3339_millis_utc() {
        let t = DateTime::parse_from_rfc3339("2026-11-16T20:01:02.3456+01:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(time(t), "2026-11-16T19:01:02.345Z");
    }

    #[test]
    fn files_skip_what_doesnt_parse() {
        let variants = serde_json::json!([
            { "name": "1x.webp", "format": "webp", "scale": 1, "width": 32, "height": 32, "size": 900, "is_static": false },
            { "name": "broken" }
        ]);
        let parsed = files(Some(&variants));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "1x.webp");
        assert!(files(None).is_empty());
    }

    #[test]
    fn an_emote_view_gives_every_file_its_url_and_mime() {
        let variants = serde_json::json!([
            { "name": "1x.webp", "format": "webp", "scale": 1, "width": 32, "height": 32, "size": 900, "is_static": false },
            { "name": "1x.avif", "format": "avif", "scale": 1, "width": 32, "height": 32, "size": 600, "is_static": false }
        ]);
        let host = "https://cdn.example.test/emotes/ab".to_owned();
        let files = EmoteFile::with_urls(files(Some(&variants)), &host);
        assert_eq!(files[0].url, "https://cdn.example.test/emotes/ab/1x.webp");
        assert_eq!(files[0].mime, "image/webp");
        assert_eq!(files[1].url, "https://cdn.example.test/emotes/ab/1x.avif");
        assert_eq!(files[1].mime, "image/avif");
    }

    #[test]
    fn hex_is_lowercase() {
        assert_eq!(hex(&[0x0a, 0xff]), "0aff");
    }
}
