//! Deleting an account (`deleteAccount`, ADR 0019). The api hides it at once and schedules it
//! ([`schedule_erase`]); signing in again within 30 days can undo that ([`restore`]); after that
//! the worker erases it ([`erase_account`]): what goes, what stays, with its events and audit row,
//! in one transaction. The account row stays, marked deleted, so audit rows and other people's
//! records keep pointing at something; everything that says who it was goes.

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EmoteState, EventType, SetKind, SetVisibility};
use serde_json::{Value, json};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{DbError, DbResult};
use crate::sets::Set;
use crate::{accounts, audit, channels, cosmetics, emotes, files, ids, jobs, outbox, sets};

/// Locks the account row for the deletion. `None` when it is unknown or already deleted.
pub async fn lock_live(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM accounts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(conn)
    .await?)
}

/// The account's emotes that stay: approved and in a set someone else owns (or the global set).
/// They lose their owner and show as made by a deleted user.
pub async fn emotes_to_keep(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT e.id FROM emotes e
         WHERE e.owner_account_id = $1 AND e.deleted_at IS NULL AND e.state = 'approved'
           AND EXISTS (SELECT 1 FROM emote_set_members m
                       JOIN emote_sets s ON s.id = m.set_id
                       WHERE m.emote_id = e.id
                         AND (s.owner_account_id IS NULL OR s.owner_account_id <> $1))
         ORDER BY e.id",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// Every other emote of the account, not deleted yet: these are taken down with the account.
pub async fn emotes_to_remove(
    conn: &mut PgConnection,
    account_id: Uuid,
    keep: &[Uuid],
) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM emotes
         WHERE owner_account_id = $1 AND deleted_at IS NULL AND NOT (id = ANY($2))
         ORDER BY id",
    )
    .bind(account_id)
    .bind(keep)
    .fetch_all(conn)
    .await?)
}

/// Kept emotes lose their owner.
pub async fn disown_emotes(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE emotes SET owner_account_id = NULL, updated_at = now() WHERE id = ANY($1)",
    )
    .bind(ids)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// The account's deleted emotes (those deleted now and before) lose their owner.
pub async fn disown_deleted(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE emotes SET owner_account_id = NULL, updated_at = now()
          WHERE owner_account_id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(account_id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Free text staff or the account wrote about the account, as audit rows carry it (a note's
/// `body`, a hold's `reason`, a report's `note`): erasure takes these keys out of every row about
/// the account, and keeps the rest of the row (who did what, when).
const AUDIT_TEXT_KEYS: &[&str] = &["body", "reason", "note"];

/// Removes everything else that belongs to or names the account, then marks it deleted. The
/// caller has taken the account's emotes out of sets and deleted or disowned them, and deleted
/// its sets and channels, publishing their events first.
///
/// Staff notes on the account go, its restrictions keep their dates but lose their reasons, and
/// the audit rows about it (its subject, or the account as target) lose their free text
/// ([`AUDIT_TEXT_KEYS`]).
///
/// What stays on purpose: `identity_holds` keeps the platform user ID of a restricted or blocked
/// account's identities (written by a trigger when the identities go, below), with the hold's
/// dates and no name or reason, so the same Twitch or Kick user can't sign in fresh to escape a
/// restriction (ban evasion; ADR 0019, amendment 2026/11/20). It is used up when that identity
/// signs in again.
pub async fn erase(conn: &mut PgConnection, account_id: Uuid) -> DbResult<()> {
    sqlx::query(
        "UPDATE audit_log
            SET before_json = CASE WHEN jsonb_typeof(before_json) = 'object'
                                   THEN before_json - $3::text[] ELSE before_json END,
                after_json = CASE WHEN jsonb_typeof(after_json) = 'object'
                                  THEN after_json - $3::text[] ELSE after_json END
          WHERE subject_account_id = $1 OR (target_kind = 'account' AND target_id = $2)",
    )
    .bind(account_id)
    .bind(ids::to_text(account_id))
    .bind(AUDIT_TEXT_KEYS)
    .execute(&mut *conn)
    .await?;
    // Open transfer requests the account is a party to can't go ahead: each is closed as
    // cancelled, audited with no actor (nobody did it), like an expiry.
    let closed: Vec<(Uuid, Uuid, Uuid, Uuid, Uuid)> = sqlx::query_as(
        "UPDATE emote_transfer_requests
            SET state = 'cancelled', resolved_at = now(), resolved_by = NULL
          WHERE state = 'pending'
            AND (from_account_id = $1 OR to_account_id = $1 OR requested_by = $1)
          RETURNING id, emote_id, from_account_id, to_account_id, requested_by",
    )
    .bind(account_id)
    .fetch_all(&mut *conn)
    .await?;
    for (id, emote, from, to, by) in closed {
        audit::write_for(
            conn,
            &audit::NewEntry {
                actor: None,
                action: "emote.transfer_cancelled",
                target_kind: "emote",
                target_id: &ids::to_text(emote),
                before: Some(json!({
                    "request_id": ids::to_text(id),
                    "from_id": ids::to_text(from),
                    "to_id": ids::to_text(to),
                    "requested_by": ids::to_text(by),
                })),
                after: Some(json!({ "reason": "account_deleted" })),
            },
            // The owner's feed, unless the owner is the one going.
            (from != account_id).then_some(from),
        )
        .await?;
    }
    for sql in [
        "DELETE FROM staff_notes WHERE account_id = $1",
        "UPDATE emotes SET uploaded_by = NULL WHERE uploaded_by = $1",
        "DELETE FROM oauth_states WHERE link_account_id = $1",
        "DELETE FROM app_auth_codes WHERE account_id = $1",
        "DELETE FROM idempotency_keys WHERE account_id = $1",
        "UPDATE account_restrictions SET reason = NULL WHERE account_id = $1",
        "DELETE FROM set_editors WHERE account_id = $1",
        "DELETE FROM account_editors WHERE account_id = $1 OR editor_id = $1",
        "DELETE FROM cosmetic_selections WHERE account_id = $1",
        "DELETE FROM badge_entitlements WHERE account_id = $1",
        "DELETE FROM paint_entitlements WHERE account_id = $1",
        "DELETE FROM notifications WHERE account_id = $1",
        "DELETE FROM hidden_emotes WHERE account_id = $1",
        "DELETE FROM hidden_creators WHERE account_id = $1 OR creator_id = $1",
        "DELETE FROM favourite_emotes WHERE account_id = $1",
        "DELETE FROM account_prefs WHERE account_id = $1",
        "DELETE FROM account_roles WHERE account_id = $1",
        "UPDATE reports SET reporter_id = NULL WHERE reporter_id = $1",
        "UPDATE emote_set_members SET added_by = NULL WHERE added_by = $1",
        "DELETE FROM sessions WHERE account_id = $1",
        "DELETE FROM api_keys WHERE account_id = $1",
        "DELETE FROM webhooks WHERE channel_id IN (SELECT id FROM channels WHERE account_id = $1)",
        "UPDATE webhooks SET created_by = NULL WHERE created_by = $1",
        "DELETE FROM identities WHERE account_id = $1",
        "UPDATE accounts SET deleted_at = coalesce(deleted_at, now()), erase_after = NULL
          WHERE id = $1",
    ] {
        sqlx::query(sql)
            .bind(account_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

// The 30-day undo window (ADR 0019)

/// How long a deleted account can be restored before the worker erases it.
pub const UNDO_DAYS: i64 = 30;

/// Hides the account at once and schedules it to be erased at `erase_after`: it reads as deleted
/// everywhere (profile, sets, emotes, channels, search), but keeps everything until then. Its
/// sessions go, and chat clients hear that its channels, personal set and cosmetics went quiet.
/// False when it was already deleted.
pub async fn schedule_erase(
    conn: &mut PgConnection,
    account_id: Uuid,
    erase_after: DateTime<Utc>,
) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE accounts SET deleted_at = now(), erase_after = $2
          WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(account_id)
    .bind(erase_after)
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query("DELETE FROM sessions WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;
    // Developer keys end with the account (ADR 0033); its channels' webhooks stop with it.
    crate::api_keys::delete_for_account(conn, account_id).await?;
    crate::webhooks::delete_for_account(conn, account_id).await?;
    announce(conn, account_id, false).await?;
    Ok(true)
}

/// Undoes [`schedule_erase`] while the window is open: the account, its sets, emotes and channels
/// show again, and chat clients hear them back. Blocked accounts stay deleted. False when there
/// is nothing to restore (never deleted, erased already, past the window, or blocked).
pub async fn restore(conn: &mut PgConnection, account_id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE accounts SET deleted_at = NULL, erase_after = NULL
          WHERE id = $1 AND erase_after > now()
            AND (blocked_at IS NULL OR blocked_until <= now())",
    )
    .bind(account_id)
    .execute(&mut *conn)
    .await?;
    if done.rows_affected() == 0 {
        return Ok(false);
    }
    announce(conn, account_id, true).await?;
    Ok(true)
}

/// Accounts whose window has passed, the longest overdue first, at most `limit`.
pub async fn due_for_erase(conn: &mut PgConnection, limit: i64) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM accounts WHERE erase_after <= now() ORDER BY erase_after, id LIMIT $1",
    )
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Events for chat clients when an account is hidden (`shown: false`) or restored: each channel's
/// active set, the personal set, and the cosmetics shown. Every topic is public, and nothing
/// here names the account.
async fn announce(conn: &mut PgConnection, account_id: Uuid, shown: bool) -> DbResult<()> {
    let user_topics: Vec<String> = accounts::identities(conn, account_id)
        .await?
        .iter()
        .map(|i| outbox::user_topic(i.platform, &i.platform_user_id))
        .collect();
    for ch in channels::list_for_account(conn, account_id).await? {
        let Some(active) = ch.active_set_id else {
            continue;
        };
        let set_id = shown.then_some(active);
        outbox::publish(
            conn,
            &outbox::NewEvent {
                event_type: EventType::ChannelActiveSetChanged,
                topics: vec![outbox::channel_topic(ch.platform, &ch.platform_channel_id)],
                set_id,
                payload: json!({ "set_id": set_id.map(ids::to_text) }),
            },
        )
        .await?;
    }
    if user_topics.is_empty() {
        return Ok(());
    }
    if let Some(set) = sets::personal_for(conn, account_id).await? {
        let mut topics = user_topics.clone();
        topics.push(outbox::set_topic(set.id));
        outbox::publish(
            conn,
            &outbox::NewEvent {
                event_type: EventType::PersonalUpdated,
                topics,
                set_id: Some(set.id),
                payload: json!({ "set_id": ids::to_text(set.id) }),
            },
        )
        .await?;
    }
    let (badge, paint) = cosmetics::selection(conn, account_id).await?;
    if badge.is_some() || paint.is_some() {
        let (badge, paint) = if shown { (badge, paint) } else { (None, None) };
        outbox::publish(
            conn,
            &outbox::NewEvent {
                event_type: EventType::UserCosmeticsUpdated,
                topics: user_topics,
                set_id: None,
                payload: json!({
                    "badge_id": badge.map(ids::to_text),
                    "paint_id": paint.map(ids::to_text),
                }),
            },
        )
        .await?;
    }
    Ok(())
}

/// What erasing an account did, as counts (the audit row holds the same, and no names).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Erased {
    pub emotes_kept: u64,
    pub emotes_deleted: u64,
    /// Files queued for deletion from storage and the CDN.
    pub files_deleted: u64,
    pub sets_deleted: u64,
    pub channels_deleted: u64,
}

impl Erased {
    pub fn to_json(self) -> Value {
        json!({
            "emotes_kept": self.emotes_kept,
            "emotes_deleted": self.emotes_deleted,
            "files_deleted": self.files_deleted,
            "sets_deleted": self.sets_deleted,
            "channels_deleted": self.channels_deleted,
        })
    }
}

/// The worker job that deletes a file's objects and purges the CDN.
pub const FILES_DELETE_JOB: &str = "files.delete";

/// The topics a change to `set` goes out on: the set (unless private, ADR 0017), the channels
/// showing it, and for a personal set its owner's identities.
async fn set_topics(conn: &mut PgConnection, set: &Set) -> DbResult<Vec<String>> {
    let mut topics = Vec::new();
    if set.visibility != SetVisibility::Private {
        topics.push(outbox::set_topic(set.id));
    }
    for ch in channels::using_set(conn, set.id).await? {
        topics.push(outbox::channel_topic(ch.platform, &ch.platform_channel_id));
    }
    if set.kind == SetKind::Personal
        && let Some(owner) = set.owner_account_id
    {
        for identity in accounts::identities(conn, owner).await? {
            topics.push(outbox::user_topic(
                identity.platform,
                &identity.platform_user_id,
            ));
        }
    }
    Ok(topics)
}

/// Erases an account for good, as `deleteAccount` did at once before the undo window: its emotes
/// that are approved and in other people's sets stay, owned by nobody; its other emotes leave
/// every set (with events) and are deleted, and their files are queued for deletion unless
/// another live emote uses the same bytes; its channels and sets go (with events), then
/// everything else ([`erase`]); an audit row `account.deleted` with the counts is written. Run it
/// in a transaction. `NotFound` when the account is unknown or erased already.
pub async fn erase_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Erased> {
    let locked: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM accounts
          WHERE id = $1 AND (deleted_at IS NULL OR erase_after IS NOT NULL)
          FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut *conn)
    .await?;
    if locked.is_none() {
        return Err(DbError::NotFound);
    }
    let mut done = Erased::default();
    let identities = accounts::identities(conn, account_id).await?;

    // Emotes: kept ones lose their owner; the rest leave every set and are deleted.
    let keep = emotes_to_keep(conn, account_id).await?;
    let remove = emotes_to_remove(conn, account_id, &keep).await?;
    for id in &remove {
        // Every topic is public: an event naming an emote nobody approved would leak its name.
        let approved = emotes::get(conn, *id)
            .await?
            .is_some_and(|e| e.state == EmoteState::Approved);
        for (set_id, name) in emotes::remove_from_all_sets(conn, *id).await? {
            let Some(set) = sets::get(conn, set_id).await? else {
                continue;
            };
            if set.owner_account_id == Some(account_id) || !approved {
                continue;
            }
            let topics = set_topics(conn, &set).await?;
            if topics.is_empty() {
                continue;
            }
            let event_type = if set.kind == SetKind::Personal {
                EventType::PersonalUpdated
            } else {
                EventType::SetEmoteRemoved
            };
            outbox::publish(
                conn,
                &outbox::NewEvent {
                    event_type,
                    topics,
                    set_id: Some(set.id),
                    payload: json!({
                        "set_id": ids::to_text(set.id),
                        "emote_id": ids::to_text(*id),
                        "name": name,
                    }),
                },
            )
            .await?;
        }
        if emotes::delete(conn, *id).await? {
            done.emotes_deleted += 1;
        }
    }
    done.emotes_kept = disown_emotes(conn, &keep).await?;
    // Their files leave storage and the CDN, unless another live emote uses the same bytes.
    for file_id in files::mark_orphans_purged(conn, account_id).await? {
        jobs::enqueue(
            conn,
            FILES_DELETE_JOB,
            &json!({ "file_id": ids::to_text(file_id), "reason": "account_deleted" }),
            10,
        )
        .await?;
        done.files_deleted += 1;
    }
    disown_deleted(conn, account_id).await?;

    // Channels stop showing a set; chat clients hear it before the channel goes.
    for ch in channels::list_for_account(conn, account_id).await? {
        if ch.active_set_id.is_some() {
            outbox::publish(
                conn,
                &outbox::NewEvent {
                    event_type: EventType::ChannelActiveSetChanged,
                    topics: vec![outbox::channel_topic(ch.platform, &ch.platform_channel_id)],
                    set_id: None,
                    payload: json!({ "set_id": null }),
                },
            )
            .await?;
        }
        done.channels_deleted += 1;
    }
    for identity in &identities {
        channels::delete_for(conn, account_id, identity.platform).await?;
    }

    // Sets; for the personal set, chat clients drop its emotes, and cosmetics stop showing.
    let user_topics: Vec<String> = identities
        .iter()
        .map(|i| outbox::user_topic(i.platform, &i.platform_user_id))
        .collect();
    for set in sets::list_for_owner(conn, account_id).await? {
        if set.kind == SetKind::Personal && !user_topics.is_empty() {
            let mut topics = user_topics.clone();
            topics.push(outbox::set_topic(set.id));
            outbox::publish(
                conn,
                &outbox::NewEvent {
                    event_type: EventType::PersonalUpdated,
                    topics,
                    set_id: Some(set.id),
                    payload: json!({ "set_id": ids::to_text(set.id) }),
                },
            )
            .await?;
        }
        if sets::delete(conn, set.id).await? {
            done.sets_deleted += 1;
        }
    }
    let shown = cosmetics::selection(conn, account_id).await?;
    if (shown.0.is_some() || shown.1.is_some()) && !user_topics.is_empty() {
        outbox::publish(
            conn,
            &outbox::NewEvent {
                event_type: EventType::UserCosmeticsUpdated,
                topics: user_topics,
                set_id: None,
                payload: json!({ "badge_id": null, "paint_id": null }),
            },
        )
        .await?;
    }

    erase(conn, account_id).await?;
    audit::write(
        conn,
        &audit::NewEntry {
            actor: Some(account_id),
            action: "account.deleted",
            target_kind: "account",
            target_id: &ids::to_text(account_id),
            before: None,
            after: Some(done.to_json()),
        },
    )
    .await?;
    Ok(done)
}
