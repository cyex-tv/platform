//! The query modules against a real Postgres. Each test gets its own migrated database.

mod common;

use chrono::{Duration, Utc};
use common::TestDb;
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::app_auth;
use cyex_tv_db::emotes::{self, EmotePatch, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::sets::{self, NewMember};
use cyex_tv_db::{DbError, audit, channels, files, ids, outbox, roles, search};
use cyex_tv_shared::model::{EmoteState, EventType, Platform, SetKind};
use cyex_tv_shared::permissions;
use serde_json::json;
use sqlx::postgres::PgListener;
use sqlx::{Executor, PgConnection};
use uuid::Uuid;

macro_rules! db {
    () => {
        match TestDb::new().await {
            Some(db) => db,
            None => return,
        }
    };
}

fn twitch<'a>(id: &'a str, login: &'a str) -> Profile<'a> {
    Profile {
        platform: Platform::Twitch,
        platform_user_id: id,
        login,
        display_name: login,
        avatar: None,
    }
}

fn kick<'a>(id: &'a str, login: &'a str) -> Profile<'a> {
    Profile {
        platform: Platform::Kick,
        ..twitch(id, login)
    }
}

async fn account(conn: &mut PgConnection, twitch_id: &str) -> Uuid {
    accounts::sign_in(conn, twitch(twitch_id, &format!("user{twitch_id}")))
        .await
        .unwrap()
        .account
        .id
}

async fn emote(conn: &mut PgConnection, owner: Uuid, name: &str, state: EmoteState) -> Uuid {
    let emote = emotes::create(
        conn,
        &NewEmote {
            owner_account_id: owner,
            name,
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    if state != EmoteState::Pending {
        emotes::set_state(conn, emote.id, state).await.unwrap();
    }
    emote.id
}

fn member(emote_id: Uuid) -> NewMember<'static> {
    NewMember {
        emote_id,
        name: None,
        zero_width: None,
        added_by: None,
    }
}

// Accounts and identities

#[tokio::test]
async fn sign_in_creates_then_reuses_the_account() {
    let db = db!();
    let mut c = db.conn().await;

    let first = accounts::sign_in(&mut c, twitch("100", "alice"))
        .await
        .unwrap();
    assert!(first.created);
    assert_eq!(first.identity.login, "alice");

    let mut renamed = twitch("100", "alice_2");
    renamed.avatar = Some("https://example.com/a.png");
    let second = accounts::sign_in(&mut c, renamed).await.unwrap();
    assert!(!second.created);
    assert_eq!(second.account.id, first.account.id);
    assert_eq!(second.identity.login, "alice_2");
    assert_eq!(
        second.identity.avatar.as_deref(),
        Some("https://example.com/a.png")
    );
    assert!(second.identity.refreshed_at >= first.identity.refreshed_at);

    // The same numeric ID on another platform is a different person.
    let other = accounts::sign_in(&mut c, kick("100", "alice"))
        .await
        .unwrap();
    assert!(other.created);
    assert_ne!(other.account.id, first.account.id);

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn concurrent_first_sign_ins_share_one_account() {
    let db = db!();
    let mut tasks = Vec::new();
    for _ in 0..6 {
        let pool = db.pool.clone();
        tasks.push(tokio::spawn(async move {
            let mut c = pool.acquire().await.unwrap();
            accounts::sign_in(&mut c, twitch("777", "racer"))
                .await
                .unwrap()
        }));
    }
    let mut ids = Vec::new();
    for task in tasks {
        ids.push(task.await.unwrap().account.id);
    }
    ids.dedup();
    assert_eq!(ids.len(), 1);
    db.close().await;
}

#[tokio::test]
async fn linking_and_unlinking_identities() {
    let db = db!();
    let mut c = db.conn().await;
    let alice = account(&mut c, "1").await;
    let bob = account(&mut c, "2").await;

    let linked = accounts::link(&mut c, alice, kick("10", "alice_k"))
        .await
        .unwrap();
    assert_eq!(linked.account_id, alice);
    assert_eq!(accounts::identities(&mut c, alice).await.unwrap().len(), 2);

    // Linking again is a refresh.
    let again = accounts::link(&mut c, alice, kick("10", "alice_k2"))
        .await
        .unwrap();
    assert_eq!(again.login, "alice_k2");

    // Already on another account.
    assert!(matches!(
        accounts::link(&mut c, bob, kick("10", "alice_k")).await,
        Err(DbError::IdentityTaken)
    ));
    // A second Twitch identity on the same account.
    assert!(matches!(
        accounts::link(&mut c, alice, twitch("3", "alt")).await,
        Err(DbError::PlatformAlreadyLinked)
    ));

    let removed = accounts::unlink(&mut c, alice, Platform::Twitch)
        .await
        .unwrap();
    assert_eq!(removed.platform_user_id, "1");
    assert!(matches!(
        accounts::unlink(&mut c, alice, Platform::Twitch).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        accounts::unlink(&mut c, alice, Platform::Kick).await,
        Err(DbError::LastIdentity)
    ));

    // The unlinked identity can sign in again, as a new account.
    let back = accounts::sign_in(&mut c, twitch("1", "alice"))
        .await
        .unwrap();
    assert!(back.created);

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn restricting_an_account() {
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "1").await;

    let restricted = accounts::set_restricted(&mut c, id, true).await.unwrap();
    let at = restricted.restricted_at.unwrap();
    // Restricting twice keeps the first time.
    let again = accounts::set_restricted(&mut c, id, true).await.unwrap();
    assert_eq!(again.restricted_at, Some(at));
    let lifted = accounts::set_restricted(&mut c, id, false).await.unwrap();
    assert_eq!(lifted.restricted_at, None);
    assert!(matches!(
        accounts::set_restricted(&mut c, Uuid::nil(), true).await,
        Err(DbError::NotFound)
    ));

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn timed_holds_end_and_follow_the_person() {
    use accounts::Hold;
    use chrono::{Duration, Utc};
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "71").await;
    accounts::link(&mut c, id, kick("72", "kick72"))
        .await
        .unwrap();

    // Tickets for a week, uploads for good.
    let week = Utc::now() + Duration::days(7);
    let a = accounts::set_hold(&mut c, id, Hold::Tickets, true, Some(week))
        .await
        .unwrap();
    assert!(a.tickets_blocked_at.is_some());
    assert!(a.tickets_blocked_until.is_some());
    let a = accounts::set_hold(&mut c, id, Hold::Uploads, true, None)
        .await
        .unwrap();
    assert!(a.restricted_at.is_some() && a.restricted_until.is_none());
    assert!(a.held());

    // Unlinking leaves a hold with the end; the next account with that identity gets both.
    accounts::unlink(&mut c, id, Platform::Kick).await.unwrap();
    let fresh = accounts::sign_in(&mut c, kick("72", "kick72"))
        .await
        .unwrap();
    assert!(fresh.created);
    assert!(fresh.account.restricted_at.is_some());
    assert_eq!(fresh.account.restricted_until, None);
    let until = fresh.account.tickets_blocked_until.unwrap();
    assert!((until - week).num_seconds().abs() <= 1, "{until} vs {week}");
    assert_eq!(fresh.account.blocked_at, None);

    // An end in the past reads as no hold at all, without anything sweeping it away.
    sqlx::query(
        "UPDATE accounts SET tickets_blocked_until = now() - interval '1 second' WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *c)
    .await
    .unwrap();
    let a = accounts::get(&mut c, id).await.unwrap().unwrap();
    assert_eq!(a.tickets_blocked_at, None);
    assert_eq!(a.tickets_blocked_until, None);
    assert!(a.restricted_at.is_some());
    // Lifted: gone, end and all.
    let a = accounts::set_hold(&mut c, id, Hold::Uploads, false, None)
        .await
        .unwrap();
    assert!(!a.held());

    // The history keeps every hold; a newer one of the same kind ends the running one.
    let first = accounts::record_hold(&mut c, id, Hold::Account, Some("spam"), None, None)
        .await
        .unwrap();
    let second = accounts::record_hold(&mut c, id, Hold::Account, Some("worse"), Some(week), None)
        .await
        .unwrap();
    let rows = accounts::restrictions(&mut c, id, 10).await.unwrap();
    assert_eq!(rows.len(), 2);
    let old = rows.iter().find(|r| r.id == first.id).unwrap();
    assert!(old.lifted_at.is_some());
    let new = rows.iter().find(|r| r.id == second.id).unwrap();
    assert!(new.lifted_at.is_none());
    assert_eq!(
        accounts::end_records(&mut c, id, Hold::Account, None)
            .await
            .unwrap(),
        1
    );

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn the_schema_rejects_bad_rows() {
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "1").await;

    // Logins are not IDs.
    let bad_id = accounts::link(&mut c, id, kick("not-a-number", "x")).await;
    assert!(matches!(bad_id, Err(DbError::Sqlx(_))));
    // Whitespace in emote names.
    let bad_name = emotes::create(
        &mut c,
        &NewEmote {
            owner_account_id: id,
            name: "two words",
            ..NewEmote::default()
        },
    )
    .await;
    assert!(bad_name.is_err());
    // A global set always has no owner.
    let owned_global = sqlx::query(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity)
         VALUES ($1, $2, 'x', 'global', 10)",
    )
    .bind(cyex_tv_db::ids::new_id())
    .bind(id)
    .execute(&mut *c)
    .await;
    assert!(owned_global.is_err());

    drop(c);
    db.close().await;
}

// Sessions

#[tokio::test]
async fn sessions_expire_slide_and_sign_out() {
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "1").await;
    let a = [1u8; 32];
    let b = [2u8; 32];
    let c_hash = [3u8; 32];

    let session = sessions::create(&mut c, &a, id, SessionKind::Site, Duration::days(30))
        .await
        .unwrap();
    assert_eq!(session.kind, SessionKind::Site);
    assert!(session.expires_at > Utc::now() + Duration::days(29));
    // An app session names its client (ADR 0021).
    app_auth::create_session(
        &mut c,
        &b,
        id,
        "cyex-deck",
        Duration::days(1),
        Duration::days(90),
        None,
    )
    .await
    .unwrap();
    sessions::create(&mut c, &c_hash, id, SessionKind::Site, Duration::days(30))
        .await
        .unwrap();

    assert!(sessions::find_active(&mut c, &a).await.unwrap().is_some());
    assert!(
        sessions::find_active(&mut c, &[9u8; 32])
            .await
            .unwrap()
            .is_none()
    );

    // Touch never shortens: the app session keeps its 90 days.
    let touched = sessions::touch(&mut c, &b, Duration::days(30))
        .await
        .unwrap()
        .unwrap();
    assert!(touched.expires_at > Utc::now() + Duration::days(89));

    // Expired sessions are invisible and get cleaned up.
    sqlx::query("UPDATE sessions SET created_at = now() - interval '2 days', expires_at = now() - interval '1 day' WHERE id_hash = $1")
        .bind(&a[..])
        .execute(&mut *c)
        .await
        .unwrap();
    assert!(sessions::find_active(&mut c, &a).await.unwrap().is_none());
    assert!(
        sessions::touch(&mut c, &a, Duration::days(30))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(sessions::delete_expired(&mut c).await.unwrap(), 1);

    // Sign out everywhere but here.
    assert_eq!(
        sessions::delete_for_account(&mut c, id, Some(&b))
            .await
            .unwrap(),
        1
    );
    assert!(sessions::find_active(&mut c, &b).await.unwrap().is_some());
    assert!(sessions::delete(&mut c, &b).await.unwrap());
    assert!(!sessions::delete(&mut c, &b).await.unwrap());

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn unscoped_app_sessions_are_retired() {
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "1").await;

    // No new unscoped app session: the CHECK refuses it.
    assert!(
        sessions::create(&mut c, &[1u8; 32], id, SessionKind::App, Duration::days(1))
            .await
            .is_err()
    );

    // The migration itself: put the old shape back, then run its SQL.
    sqlx::query("ALTER TABLE sessions DROP CONSTRAINT sessions_app_has_client")
        .execute(&mut *c)
        .await
        .unwrap();
    let (unscoped, site, scoped) = ([2u8; 32], [3u8; 32], [4u8; 32]);
    sessions::create(&mut c, &unscoped, id, SessionKind::App, Duration::days(1))
        .await
        .unwrap();
    sessions::create(&mut c, &site, id, SessionKind::Site, Duration::days(1))
        .await
        .unwrap();
    app_auth::create_session(
        &mut c,
        &scoped,
        id,
        "cyex-deck",
        Duration::days(1),
        Duration::days(30),
        None,
    )
    .await
    .unwrap();
    let migration = cyex_tv_db::MIGRATOR
        .iter()
        .find(|m| m.version == 20_261_210_005_001)
        .expect("the retire migration");
    c.execute(&*migration.sql).await.unwrap();

    assert!(
        sessions::find_active(&mut c, &unscoped)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        sessions::find_active(&mut c, &site)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        sessions::find_active(&mut c, &scoped)
            .await
            .unwrap()
            .is_some()
    );
    // The constraint is back.
    assert!(
        sessions::create(&mut c, &[5u8; 32], id, SessionKind::App, Duration::days(1))
            .await
            .is_err()
    );
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn sessions_of_deleted_accounts_are_inactive() {
    let db = db!();
    let mut c = db.conn().await;
    let id = account(&mut c, "1").await;
    let hash = [7u8; 32];
    sessions::create(&mut c, &hash, id, SessionKind::Site, Duration::days(30))
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *c)
        .await
        .unwrap();
    assert!(
        sessions::find_active(&mut c, &hash)
            .await
            .unwrap()
            .is_none()
    );
    drop(c);
    db.close().await;
}

// Channels

#[tokio::test]
async fn channels_upsert_and_look_up() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;

    let ch = channels::upsert(&mut c, owner, Platform::Twitch, "1", "Streamer")
        .await
        .unwrap();
    assert!(ch.allow_personal_emotes);
    assert_eq!(ch.active_set_id, None);

    let again = channels::upsert(&mut c, owner, Platform::Twitch, "1", "streamer_new")
        .await
        .unwrap();
    assert_eq!(again.id, ch.id);
    assert_eq!(again.login, "streamer_new");

    let by_login = channels::find_by_login(&mut c, Platform::Twitch, "STREAMER_NEW")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_login.id, ch.id);
    assert!(
        channels::find_by_login(&mut c, Platform::Kick, "streamer_new")
            .await
            .unwrap()
            .is_none()
    );

    let set = sets::create(&mut c, owner, "main", SetKind::Channel)
        .await
        .unwrap();
    let active = channels::set_active_set(&mut c, ch.id, Some(set.id))
        .await
        .unwrap();
    assert_eq!(active.active_set_id, Some(set.id));
    let off = channels::set_allow_personal_emotes(&mut c, ch.id, false)
        .await
        .unwrap();
    assert!(!off.allow_personal_emotes);

    // Deleting the active set clears it on the channel.
    assert!(sets::delete(&mut c, set.id).await.unwrap());
    let cleared = channels::find(&mut c, Platform::Twitch, "1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cleared.active_set_id, None);
    assert_eq!(
        channels::list_for_account(&mut c, owner)
            .await
            .unwrap()
            .len(),
        1
    );

    drop(c);
    db.close().await;
}

// Emotes and files

#[tokio::test]
async fn emotes_states_patches_and_tags() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let id = emote(&mut c, owner, "wave", EmoteState::Pending).await;

    let row = emotes::get(&mut c, id).await.unwrap().unwrap();
    assert_eq!(row.state, EmoteState::Pending);

    let patched = emotes::update(
        &mut c,
        id,
        &EmotePatch {
            name: Some("waving"),
            zero_width: Some(true),
            ..EmotePatch::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(patched.name, "waving");
    assert!(patched.zero_width);
    assert!(!patched.nsfw);

    emotes::set_tags(&mut c, id, &["hello".into(), "hand".into(), "hello".into()])
        .await
        .unwrap();
    assert_eq!(emotes::tags(&mut c, id).await.unwrap(), ["hand", "hello"]);
    emotes::set_tags(&mut c, id, &[]).await.unwrap();
    assert!(emotes::tags(&mut c, id).await.unwrap().is_empty());

    let other = emote(&mut c, owner, "nod", EmoteState::Pending).await;
    emotes::set_state(&mut c, other, EmoteState::Approved)
        .await
        .unwrap();
    let (pending, total) = emotes::list_by_state(&mut c, EmoteState::Pending, 10, 0)
        .await
        .unwrap();
    assert_eq!(total, 1);
    assert_eq!(pending[0].id, id);
    assert_eq!(
        emotes::list_by_owner(&mut c, owner, 10, 0)
            .await
            .unwrap()
            .len(),
        2
    );

    assert!(emotes::delete(&mut c, id).await.unwrap());
    assert!(!emotes::delete(&mut c, id).await.unwrap());
    assert!(emotes::get(&mut c, id).await.unwrap().is_none());
    assert!(matches!(
        emotes::set_state(&mut c, id, EmoteState::Approved).await,
        Err(DbError::NotFound)
    ));

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn linking_to_a_deleted_account_is_refused() {
    let db = db!();
    let mut c = db.conn().await;
    let alice = account(&mut c, "71").await;
    sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
        .bind(alice)
        .execute(&mut *c)
        .await
        .unwrap();
    assert!(matches!(
        accounts::link(&mut c, alice, kick("72", "late")).await,
        Err(DbError::NotFound)
    ));
    assert_eq!(
        accounts::find_identity(&mut c, Platform::Kick, "72")
            .await
            .unwrap(),
        None
    );
    drop(c);
    db.close().await;
}

async fn file_row(conn: &mut PgConnection, n: u8) -> Uuid {
    let sha = [n; 32];
    let key = format!("originals/{n}");
    let new = files::NewFile {
        sha256: &sha,
        dhash: None,
        mime: "image/png",
        width: 32,
        height: 32,
        frame_count: 1,
        storage_key: &key,
    };
    files::insert_or_get(conn, &new).await.unwrap().0.id
}

#[tokio::test]
async fn files_only_deleted_emotes_use_are_purged_and_revived() {
    let db = db!();
    let mut c = db.conn().await;
    let gone = account(&mut c, "81").await;
    let other = account(&mut c, "82").await;
    let shared = file_row(&mut c, 61).await;
    let only_theirs = file_row(&mut c, 62).await;
    let mut make = async |owner: Uuid, file_id: Uuid, name: &str| {
        emotes::create(
            &mut c,
            &NewEmote {
                owner_account_id: owner,
                file_id: Some(file_id),
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap()
        .id
    };
    let a = make(gone, shared, "a").await;
    let b = make(gone, only_theirs, "b").await;
    let _live = make(other, shared, "c").await;
    for id in [a, b] {
        assert!(emotes::delete(&mut c, id).await.unwrap());
    }
    // The shared file stays: another account's live emote uses the same bytes.
    assert_eq!(
        files::mark_orphans_purged(&mut c, gone).await.unwrap(),
        [only_theirs]
    );
    // Once only.
    assert!(
        files::mark_orphans_purged(&mut c, gone)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(files::in_use(&mut c, shared).await.unwrap());
    assert!(!files::in_use(&mut c, only_theirs).await.unwrap());
    let purged = files::get(&mut c, only_theirs).await.unwrap().unwrap();
    assert!(purged.purged_at.is_some());
    // An upload of the same bytes brings it back, once.
    assert!(files::revive(&mut c, only_theirs).await.unwrap());
    assert!(!files::revive(&mut c, only_theirs).await.unwrap());
    let back = files::get(&mut c, only_theirs).await.unwrap().unwrap();
    assert_eq!(back.purged_at, None);
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn files_are_shared_by_hash() {
    let db = db!();
    let mut c = db.conn().await;
    let sha = [5u8; 32];
    let new = files::NewFile {
        sha256: &sha,
        dhash: Some(-42),
        mime: "image/png",
        width: 112,
        height: 112,
        frame_count: 1,
        storage_key: "emotes/0505",
    };
    let (file, created) = files::insert_or_get(&mut c, &new).await.unwrap();
    assert!(created);
    let (same, created) = files::insert_or_get(&mut c, &new).await.unwrap();
    assert!(!created);
    assert_eq!(same.id, file.id);
    assert_eq!(same.dhash, Some(-42));

    let variants = json!([{ "name": "1x.webp" }]);
    assert!(
        files::set_variants(&mut c, file.id, &variants)
            .await
            .unwrap()
    );
    let found = files::find_by_sha256(&mut c, &sha).await.unwrap().unwrap();
    assert_eq!(found.variants, variants);

    drop(c);
    db.close().await;
}

// Sets

#[tokio::test]
async fn set_kinds_and_the_global_set() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;

    let channel = sets::create(&mut c, owner, "main", SetKind::Channel)
        .await
        .unwrap();
    assert_eq!(channel.capacity, sets::CHANNEL_CAPACITY);
    let personal = sets::create(&mut c, owner, "mine", SetKind::Personal)
        .await
        .unwrap();
    assert_eq!(personal.capacity, sets::PERSONAL_CAPACITY);
    assert!(matches!(
        sets::create(&mut c, owner, "again", SetKind::Personal).await,
        Err(DbError::PersonalSetExists)
    ));
    assert!(matches!(
        sets::create(&mut c, owner, "g", SetKind::Global).await,
        Err(DbError::InvalidSetKind)
    ));

    let global = sets::ensure_global(&mut c).await.unwrap();
    assert_eq!(global.kind, SetKind::Global);
    assert_eq!(global.owner_account_id, None);
    assert_eq!(sets::ensure_global(&mut c).await.unwrap().id, global.id);
    assert!(!sets::delete(&mut c, global.id).await.unwrap());

    assert_eq!(sets::list_for_owner(&mut c, owner).await.unwrap().len(), 2);
    assert_eq!(
        sets::personal_for(&mut c, owner).await.unwrap().unwrap().id,
        personal.id
    );
    let renamed = sets::rename(&mut c, channel.id, "main 2").await.unwrap();
    assert_eq!(renamed.name, "main 2");

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn set_membership_rules() {
    let db = db!();
    let mut c = db.conn().await;
    let alice = account(&mut c, "1").await;
    let bob = account(&mut c, "2").await;
    let alice_set = sets::create(&mut c, alice, "a", SetKind::Channel)
        .await
        .unwrap();
    let bob_set = sets::create(&mut c, bob, "b", SetKind::Channel)
        .await
        .unwrap();
    let alice_personal = sets::create(&mut c, alice, "p", SetKind::Personal)
        .await
        .unwrap();

    let approved = emote(&mut c, bob, "approved", EmoteState::Approved).await;
    let pending = emote(&mut c, alice, "pending", EmoteState::Pending).await;
    let rejected = emote(&mut c, alice, "rejected", EmoteState::Rejected).await;
    let taken = emote(&mut c, bob, "taken", EmoteState::TakenDown).await;

    // Approved emotes go anywhere.
    let m = sets::add_member(&mut c, alice_set.id, member(approved))
        .await
        .unwrap();
    assert_eq!(m.name, "approved");
    sets::add_member(&mut c, alice_personal.id, member(approved))
        .await
        .unwrap();

    // Pending: only in the owner's own channel sets (D10, D21).
    sets::add_member(&mut c, alice_set.id, member(pending))
        .await
        .unwrap();
    assert!(matches!(
        sets::add_member(&mut c, bob_set.id, member(pending)).await,
        Err(DbError::NotApproved)
    ));
    assert!(matches!(
        sets::add_member(&mut c, alice_personal.id, member(pending)).await,
        Err(DbError::NotApproved)
    ));
    let global = sets::ensure_global(&mut c).await.unwrap();
    assert!(matches!(
        sets::add_member(&mut c, global.id, member(pending)).await,
        Err(DbError::NotApproved)
    ));

    for gone in [rejected, taken, Uuid::nil()] {
        assert!(matches!(
            sets::add_member(&mut c, alice_set.id, member(gone)).await,
            Err(DbError::EmoteUnavailable)
        ));
    }
    assert!(matches!(
        sets::add_member(&mut c, Uuid::nil(), member(approved)).await,
        Err(DbError::NotFound)
    ));

    // Duplicates and name clashes.
    assert!(matches!(
        sets::add_member(&mut c, alice_set.id, member(approved)).await,
        Err(DbError::AlreadyInSet)
    ));
    let other = emote(&mut c, bob, "other", EmoteState::Approved).await;
    let clash = NewMember {
        name: Some("approved"),
        ..member(other)
    };
    assert!(matches!(
        sets::add_member(&mut c, alice_set.id, clash).await,
        Err(DbError::NameTaken)
    ));
    let renamed_in = NewMember {
        name: Some("other2"),
        zero_width: Some(true),
        added_by: Some(alice),
        ..member(other)
    };
    let added = sets::add_member(&mut c, alice_set.id, renamed_in)
        .await
        .unwrap();
    assert_eq!(added.name, "other2");
    assert!(added.zero_width);
    assert_eq!(added.added_by, Some(alice));

    // Rename inside the set.
    assert!(matches!(
        sets::update_member(&mut c, alice_set.id, other, Some("pending"), None).await,
        Err(DbError::NameTaken)
    ));
    let moved = sets::update_member(&mut c, alice_set.id, other, Some("other3"), Some(false))
        .await
        .unwrap();
    assert_eq!(moved.name, "other3");
    assert!(!moved.zero_width);

    let names: Vec<_> = sets::members(&mut c, alice_set.id)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(names, ["approved", "pending", "other3"]);

    sets::remove_member(&mut c, alice_set.id, other)
        .await
        .unwrap();
    assert!(matches!(
        sets::remove_member(&mut c, alice_set.id, other).await,
        Err(DbError::NotFound)
    ));

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn set_capacity_is_enforced() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let personal = sets::create(&mut c, owner, "p", SetKind::Personal)
        .await
        .unwrap();

    for i in 0..sets::PERSONAL_CAPACITY {
        let id = emote(&mut c, owner, &format!("e{i}"), EmoteState::Approved).await;
        sets::add_member(&mut c, personal.id, member(id))
            .await
            .unwrap();
    }
    let extra = emote(&mut c, owner, "extra", EmoteState::Approved).await;
    assert!(matches!(
        sets::add_member(&mut c, personal.id, member(extra)).await,
        Err(DbError::SetFull)
    ));
    assert!(matches!(
        sets::set_capacity(&mut c, personal.id, 4).await,
        Err(DbError::CapacityTooLow)
    ));
    sets::set_capacity(&mut c, personal.id, 6).await.unwrap();
    sets::add_member(&mut c, personal.id, member(extra))
        .await
        .unwrap();
    assert_eq!(sets::count(&mut c, personal.id).await.unwrap(), 6);

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn concurrent_adds_respect_capacity() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let set = sets::create(&mut c, owner, "p", SetKind::Personal)
        .await
        .unwrap();
    let mut ids = Vec::new();
    for i in 0..12 {
        ids.push(emote(&mut c, owner, &format!("e{i}"), EmoteState::Approved).await);
    }
    drop(c);

    let mut tasks = Vec::new();
    for id in ids {
        let pool = db.pool.clone();
        tasks.push(tokio::spawn(async move {
            let mut c = pool.acquire().await.unwrap();
            sets::add_member(&mut c, set.id, member(id)).await.is_ok()
        }));
    }
    let mut ok = 0;
    for task in tasks {
        ok += i32::from(task.await.unwrap());
    }
    assert_eq!(ok, sets::PERSONAL_CAPACITY);
    let mut c = db.conn().await;
    assert_eq!(
        sets::count(&mut c, set.id).await.unwrap(),
        i64::from(sets::PERSONAL_CAPACITY)
    );
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn set_editors_and_can_edit() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let editor = account(&mut c, "2").await;
    let stranger = account(&mut c, "3").await;
    let set = sets::create(&mut c, owner, "main", SetKind::Channel)
        .await
        .unwrap();

    assert!(sets::can_edit(&mut c, owner, set.id).await.unwrap());
    assert!(!sets::can_edit(&mut c, editor, set.id).await.unwrap());

    assert!(
        sets::add_editor(&mut c, set.id, editor, Some(owner))
            .await
            .unwrap()
    );
    assert!(
        !sets::add_editor(&mut c, set.id, editor, Some(owner))
            .await
            .unwrap()
    );
    assert!(sets::can_edit(&mut c, editor, set.id).await.unwrap());
    assert!(!sets::can_edit(&mut c, stranger, set.id).await.unwrap());

    let list = sets::editors(&mut c, set.id).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].granted_by, Some(owner));
    assert_eq!(
        sets::list_editable(&mut c, editor).await.unwrap()[0].id,
        set.id
    );

    assert!(sets::remove_editor(&mut c, set.id, editor).await.unwrap());
    assert!(!sets::can_edit(&mut c, editor, set.id).await.unwrap());

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn personal_sets_for_chatters() {
    let db = db!();
    let mut c = db.conn().await;
    let with = account(&mut c, "11").await;
    let empty = account(&mut c, "12").await;
    let _none = account(&mut c, "13").await;

    let set = sets::create(&mut c, with, "p", SetKind::Personal)
        .await
        .unwrap();
    sets::create(&mut c, empty, "p", SetKind::Personal)
        .await
        .unwrap();
    let id = emote(&mut c, with, "hi", EmoteState::Approved).await;
    sets::add_member(&mut c, set.id, member(id)).await.unwrap();

    let users = ["11", "12", "13", "99"].map(String::from);
    let found = sets::personal_for_users(&mut c, Platform::Twitch, &users)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].platform_user_id, "11");
    assert_eq!(found[0].set.id, set.id);
    // Same IDs on Kick are other people.
    assert!(
        sets::personal_for_users(&mut c, Platform::Kick, &users)
            .await
            .unwrap()
            .is_empty()
    );

    drop(c);
    db.close().await;
}

// Roles

#[tokio::test]
async fn roles_and_their_permissions() {
    let db = db!();
    let mut c = db.conn().await;
    let admin = account(&mut c, "1").await;
    let modr = account(&mut c, "2").await;

    let moderator = roles::create(
        &mut c,
        &roles::NewRole {
            name: "Moderator",
            description: Some("reviews uploads"),
            position: 10,
            permissions: &[permissions::EMOTES_REVIEW, permissions::EMOTES_TAKEDOWN],
            created_by: Some(admin),
        },
    )
    .await
    .unwrap();
    let staff = roles::create(
        &mut c,
        &roles::NewRole {
            name: "Staff",
            description: None,
            position: 50,
            permissions: &[permissions::ROLES_MANAGE, permissions::EMOTES_REVIEW],
            created_by: Some(admin),
        },
    )
    .await
    .unwrap();

    let dup = roles::NewRole {
        name: "MODERATOR",
        description: None,
        position: 1,
        permissions: &[],
        created_by: None,
    };
    assert!(matches!(
        roles::create(&mut c, &dup).await,
        Err(DbError::RoleNameTaken)
    ));
    let unknown = roles::NewRole {
        name: "x",
        permissions: &["emotes.everything"],
        ..dup
    };
    assert!(matches!(
        roles::create(&mut c, &unknown).await,
        Err(DbError::UnknownPermission(p)) if p == "emotes.everything"
    ));

    assert!(
        roles::assign(&mut c, modr, moderator.id, Some(admin))
            .await
            .unwrap()
    );
    assert!(
        !roles::assign(&mut c, modr, moderator.id, Some(admin))
            .await
            .unwrap()
    );
    roles::assign(&mut c, admin, staff.id, None).await.unwrap();
    roles::assign(&mut c, admin, moderator.id, None)
        .await
        .unwrap();

    assert_eq!(
        roles::permissions_of(&mut c, admin).await.unwrap(),
        ["emotes.review", "emotes.takedown", "roles.manage"]
    );
    assert_eq!(
        roles::highest_position(&mut c, admin).await.unwrap(),
        Some(50)
    );
    assert_eq!(
        roles::highest_position(&mut c, modr).await.unwrap(),
        Some(10)
    );
    let nobody = account(&mut c, "3").await;
    assert_eq!(roles::highest_position(&mut c, nobody).await.unwrap(), None);
    assert!(
        roles::permissions_of(&mut c, nobody)
            .await
            .unwrap()
            .is_empty()
    );

    let mut reviewers = vec![admin, modr];
    reviewers.sort();
    assert_eq!(
        roles::holders_of(&mut c, permissions::EMOTES_REVIEW)
            .await
            .unwrap(),
        reviewers
    );
    assert_eq!(
        roles::holders_of(&mut c, permissions::ROLES_MANAGE)
            .await
            .unwrap(),
        [admin]
    );

    let names: Vec<_> = roles::list(&mut c)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.name)
        .collect();
    // With the built-in roles (migration default_roles) around them.
    assert_eq!(names, ["Owner", "Admin", "Staff", "Moderator", "User"]);
    assert_eq!(
        roles::roles_of(&mut c, admin).await.unwrap()[0].id,
        staff.id
    );

    // Edit, then replace the permissions.
    let edited = roles::update(
        &mut c,
        moderator.id,
        &roles::RolePatch {
            description: Some(None),
            position: Some(20),
            ..roles::RolePatch::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(edited.description, None);
    assert_eq!(edited.position, 20);
    assert_eq!(edited.name, "Moderator");
    assert!(matches!(
        roles::update(
            &mut c,
            moderator.id,
            &roles::RolePatch {
                name: Some("staff"),
                ..roles::RolePatch::default()
            }
        )
        .await,
        Err(DbError::RoleNameTaken)
    ));

    roles::set_permissions(&mut c, moderator.id, &[permissions::AUDIT_VIEW])
        .await
        .unwrap();
    assert_eq!(
        roles::permissions(&mut c, moderator.id).await.unwrap(),
        ["audit.view"]
    );
    assert!(matches!(
        roles::set_permissions(&mut c, Uuid::nil(), &[]).await,
        Err(DbError::NotFound)
    ));
    assert!(matches!(
        roles::set_permissions(&mut c, moderator.id, &["nope.nope"]).await,
        Err(DbError::UnknownPermission(_))
    ));

    assert_eq!(roles::members(&mut c, moderator.id).await.unwrap().len(), 2);
    assert!(roles::unassign(&mut c, modr, moderator.id).await.unwrap());
    assert!(!roles::unassign(&mut c, modr, moderator.id).await.unwrap());
    assert!(roles::delete(&mut c, staff.id).await.unwrap());
    assert!(
        roles::holders_of(&mut c, permissions::ROLES_MANAGE)
            .await
            .unwrap()
            .is_empty()
    );

    drop(c);
    db.close().await;
}

// Audit

#[tokio::test]
async fn audit_entries_page_newest_first() {
    let db = db!();
    let mut c = db.conn().await;
    let actor = account(&mut c, "1").await;

    let mut ids = Vec::new();
    for i in 0..5 {
        let target = format!("r{i}");
        ids.push(
            audit::write(
                &mut c,
                &audit::NewEntry {
                    actor: Some(actor),
                    action: "roles.create",
                    target_kind: "role",
                    target_id: &target,
                    before: None,
                    after: Some(json!({ "n": i })),
                },
            )
            .await
            .unwrap(),
        );
    }
    audit::write(
        &mut c,
        &audit::NewEntry {
            actor: None,
            action: "emotes.approve",
            target_kind: "emote",
            target_id: "e1",
            before: Some(json!({ "state": "pending" })),
            after: Some(json!({ "state": "approved" })),
        },
    )
    .await
    .unwrap();

    let roles_only = audit::Filter {
        target_kind: Some("role"),
        ..audit::Filter::default()
    };
    let page = audit::list(&mut c, &roles_only, 2).await.unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].id, ids[4]);
    let next = audit::Filter {
        before_id: Some(page[1].id),
        ..roles_only.clone()
    };
    let page2 = audit::list(&mut c, &next, 10).await.unwrap();
    assert_eq!(
        page2.iter().map(|e| e.id).collect::<Vec<_>>(),
        [ids[2], ids[1], ids[0]]
    );

    let by_actor = audit::Filter {
        actor: Some(actor),
        ..audit::Filter::default()
    };
    assert_eq!(audit::list(&mut c, &by_actor, 100).await.unwrap().len(), 5);
    let by_action = audit::Filter {
        action: Some("emotes.approve"),
        ..audit::Filter::default()
    };
    assert_eq!(audit::list(&mut c, &by_action, 100).await.unwrap().len(), 1);
    let future = audit::Filter {
        since: Some(chrono::Utc::now() + chrono::Duration::days(1)),
        ..audit::Filter::default()
    };
    assert!(audit::list(&mut c, &future, 100).await.unwrap().is_empty());
    let past = audit::Filter {
        until: Some(chrono::Utc::now() - chrono::Duration::days(1)),
        ..audit::Filter::default()
    };
    assert!(audit::list(&mut c, &past, 100).await.unwrap().is_empty());
    let window = audit::Filter {
        since: Some(chrono::Utc::now() - chrono::Duration::days(1)),
        until: Some(chrono::Utc::now() + chrono::Duration::days(1)),
        ..audit::Filter::default()
    };
    assert_eq!(audit::list(&mut c, &window, 100).await.unwrap().len(), 6);
    let one = audit::Filter {
        target_kind: Some("emote"),
        target_id: Some("e1"),
        ..audit::Filter::default()
    };
    let entry = &audit::list(&mut c, &one, 10).await.unwrap()[0];
    assert_eq!(entry.actor_account_id, None);
    assert_eq!(entry.after_json, Some(json!({ "state": "approved" })));

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn audit_rolls_back_with_the_change() {
    let db = db!();
    let mut c = db.conn().await;
    {
        use sqlx::Connection;
        let mut tx = c.begin().await.unwrap();
        audit::write(
            &mut tx,
            &audit::NewEntry {
                actor: None,
                action: "roles.delete",
                target_kind: "role",
                target_id: "x",
                before: None,
                after: None,
            },
        )
        .await
        .unwrap();
        tx.rollback().await.unwrap();
    }
    assert!(
        audit::list(&mut c, &audit::Filter::default(), 10)
            .await
            .unwrap()
            .is_empty()
    );
    drop(c);
    db.close().await;
}

// Outbox

#[tokio::test]
async fn outbox_ids_topics_and_notify() {
    let db = db!();
    let mut listener = PgListener::connect_with(&db.pool).await.unwrap();
    listener.listen(outbox::NOTIFY_CHANNEL).await.unwrap();

    let mut c = db.conn().await;
    let set_id = cyex_tv_db::ids::new_id();
    let twitch_topic = outbox::channel_topic(Platform::Twitch, "1");
    let first = outbox::publish(
        &mut c,
        &outbox::NewEvent {
            event_type: EventType::SetEmoteAdded,
            topics: vec![twitch_topic.clone(), outbox::set_topic(set_id)],
            set_id: Some(set_id),
            payload: json!({ "name": "wave" }),
        },
    )
    .await
    .unwrap();
    let second = outbox::publish(
        &mut c,
        &outbox::NewEvent {
            event_type: EventType::PersonalUpdated,
            topics: vec![outbox::user_topic(Platform::Kick, "5")],
            set_id: None,
            payload: json!({}),
        },
    )
    .await
    .unwrap();
    assert!(second > first);

    let note = tokio::time::timeout(std::time::Duration::from_secs(5), listener.recv())
        .await
        .expect("a notification")
        .unwrap();
    assert_eq!(note.payload(), first.to_string());

    let replay = outbox::since(&mut c, 0, std::slice::from_ref(&twitch_topic), 10)
        .await
        .unwrap();
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].event_type, "set.emote_added");
    assert_eq!(replay[0].set_id, Some(set_id));
    assert!(
        outbox::since(&mut c, first, &[twitch_topic], 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(outbox::oldest_id(&mut c).await.unwrap(), Some(first));
    assert!(outbox::get(&mut c, second).await.unwrap().is_some());

    // Empty topic lists are refused by the schema.
    let none = outbox::publish(
        &mut c,
        &outbox::NewEvent {
            event_type: EventType::Resync,
            topics: vec![],
            set_id: None,
            payload: json!({}),
        },
    )
    .await;
    assert!(none.is_err());

    assert_eq!(
        outbox::delete_older_than(&mut c, Utc::now() + Duration::seconds(1))
            .await
            .unwrap(),
        2
    );
    assert_eq!(outbox::oldest_id(&mut c).await.unwrap(), None);

    drop(c);
    drop(listener);
    db.close().await;
}

// Trending rollup

async fn event(c: &mut PgConnection, kind: &str, topics: &[&str], emote: &str, hours_ago: i32) {
    let topics: Vec<String> = topics.iter().map(|t| (*t).to_owned()).collect();
    sqlx::query(
        "INSERT INTO events_outbox (type, topics, payload, created_at)
         VALUES ($1, $2, $3, date_trunc('hour', now()) - make_interval(hours => $4)
                             + interval '10 minutes')",
    )
    .bind(kind)
    .bind(&topics)
    .bind(json!({ "emote_id": emote }))
    .bind(hours_ago)
    .execute(c)
    .await
    .unwrap();
}

async fn buckets(c: &mut PgConnection, emote: Uuid) -> Vec<(i32, i32)> {
    sqlx::query_as("SELECT adds, removes FROM emote_channel_adds WHERE emote_id = $1 ORDER BY hour")
        .bind(emote)
        .fetch_all(c)
        .await
        .unwrap()
}

#[tokio::test]
async fn rollup_counts_distinct_channels_per_hour_and_is_idempotent() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let x = emote(&mut c, owner, "xx", EmoteState::Approved).await;
    let x_text = ids::to_text(x);
    let gone = ids::to_text(ids::new_id());

    // Three hours ago: two channels add it, one of them twice (counted once), one removes it.
    // The set topic is no channel.
    event(
        &mut c,
        "set.emote_added",
        &["channel:twitch:1", "channel:kick:2", "set:abc"],
        &x_text,
        3,
    )
    .await;
    event(&mut c, "set.emote_added", &["channel:twitch:1"], &x_text, 3).await;
    event(
        &mut c,
        "set.emote_removed",
        &["channel:twitch:1", "set:abc"],
        &x_text,
        3,
    )
    .await;
    // Two hours ago: one channel.
    event(&mut c, "set.emote_added", &["channel:twitch:3"], &x_text, 2).await;
    // Ignored: personal sets, events without a channel, unparsable or unknown emotes.
    event(
        &mut c,
        "personal.updated",
        &["channel:twitch:9"],
        &x_text,
        2,
    )
    .await;
    event(&mut c, "set.emote_added", &["set:abc"], &x_text, 2).await;
    event(&mut c, "set.emote_added", &["channel:twitch:1"], "nope", 2).await;
    event(&mut c, "set.emote_added", &["channel:twitch:1"], &gone, 2).await;

    assert_eq!(search::rollup_channel_adds(&mut c).await.unwrap(), 2);
    assert_eq!(buckets(&mut c, x).await, [(2, 1), (1, 0)]);

    // Again: the same table. Only the newest hour is redone, so one bucket is written.
    assert_eq!(search::rollup_channel_adds(&mut c).await.unwrap(), 1);
    assert_eq!(buckets(&mut c, x).await, [(2, 1), (1, 0)]);
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM emote_channel_adds")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert_eq!(total, 2);

    // A later event in the newest hour updates that bucket and leaves the older one.
    event(&mut c, "set.emote_added", &["channel:twitch:4"], &x_text, 2).await;
    assert_eq!(search::rollup_channel_adds(&mut c).await.unwrap(), 1);
    assert_eq!(buckets(&mut c, x).await, [(2, 1), (2, 0)]);

    drop(c);
    db.close().await;
}

#[tokio::test]
async fn name_words_and_initials_are_generated() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    for (name, words, initials) in [
        ("PepePoint", "pepe point", "pp"),
        ("KEKWait", "kek wait", "kw"),
        ("Emote2Fast", "emote 2 fast", "e2f"),
        ("snake_case-name.x", "snake case name x", "scnx"),
        ("pog", "pog", "p"),
    ] {
        let id = emote(&mut c, owner, name, EmoteState::Approved).await;
        let got: (String, String) =
            sqlx::query_as("SELECT name_words, name_initials FROM emotes WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *c)
                .await
                .unwrap();
        assert_eq!(got, (words.to_owned(), initials.to_owned()), "{name}");
    }
}

#[tokio::test]
async fn quick_search_ranks_by_tier_then_use() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let mut by_name = std::collections::HashMap::new();
    for name in [
        "xpogx",
        "KEKPog",
        "PogChamp",
        "POG",
        "pog",
        "PepePoint",
        "Hidden",
    ] {
        let state = if name == "Hidden" {
            EmoteState::Pending
        } else {
            EmoteState::Approved
        };
        by_name.insert(name, emote(&mut c, owner, name, state).await);
    }
    let names = |ids: Vec<Uuid>| -> Vec<&'static str> {
        ids.iter()
            .map(|id| *by_name.iter().find(|(_, v)| *v == id).unwrap().0)
            .collect()
    };
    let got = cyex_tv_db::search::quick(&mut c, "pog", 10).await.unwrap();
    assert_eq!(names(got), ["pog", "POG", "PogChamp", "KEKPog", "xpogx"]);
    let got = cyex_tv_db::search::quick(&mut c, "pp", 10).await.unwrap();
    assert_eq!(names(got), ["PepePoint"]);
    let got = cyex_tv_db::search::quick(&mut c, "PepePiont", 10)
        .await
        .unwrap();
    assert_eq!(names(got), ["PepePoint"]);
    let got = cyex_tv_db::search::quick(&mut c, "pog", 2).await.unwrap();
    assert_eq!(got.len(), 2);
    assert!(
        cyex_tv_db::search::quick(&mut c, "  ", 5)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        cyex_tv_db::search::quick(&mut c, "hidden", 5)
            .await
            .unwrap()
            .is_empty()
    );
}

/// One listed-emote rule for every search: `search` (relevance and the trending sort), `quick`
/// and `trending_rank` all find the public emote and none of the unlisted, private, NSFW (unless
/// asked), pending-deletion owner's or blocked owner's ones; an ended block lists them again.
#[tokio::test]
async fn every_search_uses_the_listed_emote_rule() {
    use cyex_tv_shared::model::{EmoteSort, TrendingWindow};
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let leaving = account(&mut c, "2").await;
    let blocked = account(&mut c, "3").await;
    let expired = account(&mut c, "4").await;
    let make = |name: &'static str, patch: EmotePatch<'static>, by: Uuid| (name, patch, by);
    let cases = [
        make("vispublic", EmotePatch::default(), owner),
        make(
            "visunlisted",
            EmotePatch {
                unlisted: Some(true),
                ..EmotePatch::default()
            },
            owner,
        ),
        make(
            "visprivate",
            EmotePatch {
                private: Some(true),
                ..EmotePatch::default()
            },
            owner,
        ),
        make(
            "visnsfw",
            EmotePatch {
                nsfw: Some(true),
                ..EmotePatch::default()
            },
            owner,
        ),
        make("visleaving", EmotePatch::default(), leaving),
        make("visblocked", EmotePatch::default(), blocked),
        make("visunblocked", EmotePatch::default(), expired),
    ];
    let mut by_name = Vec::new();
    for (name, patch, by) in &cases {
        let id = emote(&mut c, *by, name, EmoteState::Approved).await;
        emotes::update(&mut c, id, patch).await.unwrap();
        // Every one is trending, so only the rule can keep it out.
        sqlx::query(
            "INSERT INTO emote_channel_adds (emote_id, hour, adds, removes)
             VALUES ($1, date_trunc('hour', now()) - interval '1 hour', 5, 0)",
        )
        .bind(id)
        .execute(&mut *c)
        .await
        .unwrap();
        by_name.push((*name, id));
    }
    cyex_tv_db::privacy::schedule_erase(&mut c, leaving, Utc::now() + Duration::days(30))
        .await
        .unwrap();
    // A running block (for good) hides the owner's emotes; one that has ended shows them again.
    accounts::set_blocked(&mut c, blocked, true).await.unwrap();
    sqlx::query(
        "UPDATE accounts SET blocked_at = now() - interval '2 days',
                             blocked_until = now() - interval '1 day' WHERE id = $1",
    )
    .bind(expired)
    .execute(&mut *c)
    .await
    .unwrap();
    let names = |ids: &[Uuid]| -> Vec<&'static str> {
        let mut out: Vec<&'static str> = ids
            .iter()
            .filter_map(|id| by_name.iter().find(|(_, v)| v == id).map(|(n, _)| *n))
            .collect();
        out.sort_unstable();
        out
    };

    for sort in [EmoteSort::Relevance, EmoteSort::Trending] {
        for (nsfw, want) in [
            (false, vec!["vispublic", "visunblocked"]),
            (true, vec!["visnsfw", "vispublic", "visunblocked"]),
        ] {
            let hits = search::search(
                &mut c,
                &search::Query {
                    text: "vis",
                    nsfw,
                    sort,
                    window: TrendingWindow::Week,
                    limit: 50,
                    ..search::Query::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(names(&hits.ids), want, "search {sort:?} nsfw={nsfw}");
        }
    }
    let quick = search::quick(&mut c, "vis", 50).await.unwrap();
    assert_eq!(names(&quick), ["vispublic", "visunblocked"], "quick");
    for (name, id) in &by_name {
        let rank = search::trending_rank(&mut c, *id, 7, 100).await.unwrap();
        let listed = matches!(*name, "vispublic" | "visnsfw" | "visunblocked");
        assert_eq!(rank.is_some(), listed, "trending_rank {name}");
    }
    drop(c);
    db.close().await;
}

/// Erasing an account drops staff notes on it, blanks its restriction reasons and takes the free
/// text out of the audit rows about it; other accounts' rows keep theirs. The identity hold of a
/// restricted account stays (ban evasion), with no reason in it.
#[tokio::test]
async fn erasure_scrubs_notes_reasons_and_audit_text() {
    use sqlx::Connection;
    let db = db!();
    let mut c = db.conn().await;
    let gone = account(&mut c, "1").await;
    let other = account(&mut c, "2").await;
    let staff = account(&mut c, "3").await;
    for who in [gone, other] {
        cyex_tv_db::staff::add_note(&mut c, who, staff, "private remark")
            .await
            .unwrap();
        accounts::record_hold(
            &mut c,
            who,
            accounts::Hold::Uploads,
            Some("spam uploads"),
            None,
            Some(staff),
        )
        .await
        .unwrap();
        accounts::set_restricted(&mut c, who, true).await.unwrap();
        let tid = ids::to_text(who);
        audit::write(
            &mut c,
            &audit::NewEntry {
                actor: Some(staff),
                action: "account.note_added",
                target_kind: "account",
                target_id: &tid,
                before: None,
                after: Some(json!({ "note_id": "x", "body": "private remark" })),
            },
        )
        .await
        .unwrap();
        audit::write_for(
            &mut c,
            &audit::NewEntry {
                actor: Some(staff),
                action: "emote.taken_down",
                target_kind: "emote",
                target_id: "e1",
                before: Some(json!({ "reason": "old reason", "name": "x" })),
                after: Some(json!({ "reason": "spam", "name": "x" })),
            },
            Some(who),
        )
        .await
        .unwrap();
    }

    let mut tx = c.begin().await.unwrap();
    cyex_tv_db::privacy::erase_account(&mut tx, gone)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let notes = |who: Uuid| {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM staff_notes WHERE account_id = $1")
            .bind(who)
    };
    assert_eq!(notes(gone).fetch_one(&mut *c).await.unwrap(), 0);
    assert_eq!(notes(other).fetch_one(&mut *c).await.unwrap(), 1);
    let reasons = |who: Uuid| {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT reason FROM account_restrictions WHERE account_id = $1",
        )
        .bind(who)
    };
    assert_eq!(reasons(gone).fetch_all(&mut *c).await.unwrap(), [None]);
    assert_eq!(
        reasons(other).fetch_all(&mut *c).await.unwrap(),
        [Some("spam uploads".to_owned())]
    );
    let texts = |who: Uuid| {
        sqlx::query_scalar::<_, String>(
            "SELECT concat(before_json::text, after_json::text) FROM audit_log
              WHERE action IN ('account.note_added', 'emote.taken_down')
                AND (subject_account_id = $1 OR target_id = $2)",
        )
        .bind(who)
        .bind(ids::to_text(who))
    };
    let mine = texts(gone).fetch_all(&mut *c).await.unwrap();
    assert_eq!(mine.len(), 2, "{mine:?}");
    for row in &mine {
        for word in ["private remark", "spam", "old reason"] {
            assert!(!row.contains(word), "{row}");
        }
    }
    // What happened stays: the note's ID, the emote's name.
    assert!(mine.iter().any(|r| r.contains("note_id")), "{mine:?}");
    let theirs = texts(other).fetch_all(&mut *c).await.unwrap().join(" ");
    for word in ["private remark", "spam", "old reason"] {
        assert!(theirs.contains(word), "{theirs}");
    }
    // The hold on the identity stays, so a fresh sign-in is still restricted.
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_holds WHERE platform = 'twitch' AND platform_user_id = '1'",
    )
    .fetch_one(&mut *c)
    .await
    .unwrap();
    assert_eq!(held, 1);
    drop(c);
    db.close().await;
}

/// Erasing keeps the account row, so no foreign key's ON DELETE ever fires: erase has to deal
/// with every column that points at an account itself. Each one in the schema is either handled
/// by erase (with what it does) or kept on purpose, with the reason. A new column that
/// references accounts fails this test until it is put in one of the two lists.
#[tokio::test]
async fn every_reference_to_an_account_is_handled_by_erase() {
    let db = db!();
    let mut c = db.conn().await;
    let found: Vec<(String, String)> = sqlx::query_as(
        "SELECT kcu.table_name::text, kcu.column_name::text
           FROM information_schema.referential_constraints rc
           JOIN information_schema.key_column_usage kcu
             ON kcu.constraint_name = rc.constraint_name
            AND kcu.constraint_schema = rc.constraint_schema
           JOIN information_schema.constraint_column_usage ccu
             ON ccu.constraint_name = rc.unique_constraint_name
            AND ccu.constraint_schema = rc.unique_constraint_schema
          WHERE ccu.table_name = 'accounts' AND ccu.column_name = 'id'
          ORDER BY 1, 2",
    )
    .fetch_all(&mut *c)
    .await
    .unwrap();
    // Erased by `privacy::erase_account`: deleted, set to NULL, or the row closed.
    let handled: &[(&str, &str, &str)] = &[
        ("identities", "account_id", "deleted"),
        ("sessions", "account_id", "deleted"),
        ("emotes", "owner_account_id", "deleted or disowned (NULL)"),
        ("emotes", "uploaded_by", "set to NULL"),
        ("emote_sets", "owner_account_id", "the sets are deleted"),
        ("emote_set_members", "added_by", "set to NULL"),
        ("set_editors", "account_id", "deleted"),
        ("channels", "account_id", "deleted"),
        ("account_roles", "account_id", "deleted"),
        ("notifications", "account_id", "deleted"),
        ("hidden_emotes", "account_id", "deleted"),
        ("hidden_creators", "account_id", "deleted"),
        (
            "hidden_creators",
            "creator_id",
            "deleted (others' hides of the erased account)",
        ),
        ("favourite_emotes", "account_id", "deleted"),
        ("account_prefs", "account_id", "deleted"),
        ("account_editors", "account_id", "deleted"),
        ("account_editors", "editor_id", "deleted"),
        ("badge_entitlements", "account_id", "deleted"),
        ("paint_entitlements", "account_id", "deleted"),
        ("cosmetic_selections", "account_id", "deleted"),
        ("staff_notes", "account_id", "deleted"),
        ("reports", "reporter_id", "set to NULL"),
        ("oauth_states", "link_account_id", "deleted"),
        ("app_auth_codes", "account_id", "deleted"),
        ("idempotency_keys", "account_id", "deleted"),
        (
            "emote_transfer_requests",
            "from_account_id",
            "open ones closed",
        ),
        (
            "emote_transfer_requests",
            "to_account_id",
            "open ones closed",
        ),
        (
            "emote_transfer_requests",
            "requested_by",
            "open ones closed",
        ),
        ("audit_log", "subject_account_id", "free text removed"),
        ("account_restrictions", "account_id", "reasons removed"),
    ];
    // Kept on purpose: they record what someone did (the erased account shows as a deleted
    // user, with no name), or are records that must outlive the account.
    let kept: &[(&str, &str, &str)] = &[
        (
            "audit_log",
            "actor_account_id",
            "who did it; the trail stays",
        ),
        (
            "account_editors",
            "granted_by",
            "who granted an editor of another account",
        ),
        (
            "account_roles",
            "granted_by",
            "who gave another account a role",
        ),
        (
            "set_editors",
            "granted_by",
            "who made someone an editor of another set",
        ),
        ("roles", "created_by", "who made a role"),
        ("emotes", "reviewed_by", "who reviewed an upload"),
        ("blocked_images", "created_by", "who blocked an image"),
        ("reports", "resolved_by", "who resolved a report"),
        (
            "cosmetic_images",
            "created_by",
            "who uploaded a cosmetic image",
        ),
        ("badges", "created_by", "who made a badge"),
        ("paints", "created_by", "who made a paint"),
        (
            "badge_entitlements",
            "granted_by",
            "who gave another account a badge",
        ),
        (
            "paint_entitlements",
            "granted_by",
            "who gave another account a paint",
        ),
        ("shop_items", "created_by", "who made a shop item"),
        (
            "shop_orders",
            "account_id",
            "orders are kept for the accounts (payments)",
        ),
        (
            "shop_orders",
            "recipient_id",
            "orders are kept for the accounts (payments)",
        ),
        (
            "shop_order_entitlements",
            "account_id",
            "what an order delivered, kept with the order",
        ),
        ("code_batches", "created_by", "who made a code batch"),
        (
            "announcements",
            "created_by",
            "who put up a site announcement",
        ),
        ("code_batches", "disabled_by", "who disabled a code batch"),
        (
            "code_redemptions",
            "account_id",
            "a code is redeemed once; the record stays",
        ),
        (
            "account_restrictions",
            "created_by",
            "who placed a hold on another account",
        ),
        (
            "account_restrictions",
            "lifted_by",
            "who lifted a hold on another account",
        ),
        (
            "staff_notes",
            "author_id",
            "who wrote a note on another account",
        ),
        ("staff_notes", "deleted_by", "who deleted a note"),
        ("emote_notes", "author_id", "who wrote a review note"),
        ("emote_notes", "deleted_by", "who deleted a review note"),
        (
            "review_reasons",
            "updated_by",
            "who last changed the ready reject reasons",
        ),
        (
            "emote_transfer_requests",
            "resolved_by",
            "who answered a request",
        ),
    ];
    let known = |list: &[(&str, &str, &str)], table: &str, column: &str| {
        list.iter().any(|(t, c, _)| *t == table && *c == column)
    };
    let missing: Vec<String> = found
        .iter()
        .filter(|(t, c)| !known(handled, t, c) && !known(kept, t, c))
        .map(|(t, c)| format!("{t}.{c}"))
        .collect();
    assert!(
        missing.is_empty(),
        "references to accounts that erase neither handles nor keeps on purpose: {missing:?}"
    );
    // And no stale entries: every listed column exists.
    for (t, col, _) in handled.iter().chain(kept) {
        assert!(
            found.iter().any(|(ft, fc)| ft == t && fc == col),
            "{t}.{col} is listed but references no account"
        );
    }
    drop(c);
    db.close().await;
}

/// The columns erase now clears: emotes it uploaded for others lose `uploaded_by`, and open
/// transfer requests it is a party to are closed (audited), while others' stay open.
#[tokio::test]
async fn erasure_clears_uploader_and_closes_its_transfer_requests() {
    use sqlx::Connection;
    let db = db!();
    let mut c = db.conn().await;
    let gone = account(&mut c, "1").await;
    let owner = account(&mut c, "2").await;
    let other = account(&mut c, "3").await;
    let uploaded = emote(&mut c, owner, "byeditor", EmoteState::Approved).await;
    emotes::set_uploader(&mut c, uploaded, gone).await.unwrap();
    let asked = emote(&mut c, owner, "asked", EmoteState::Approved).await;
    let untouched = emote(&mut c, owner, "untouched", EmoteState::Approved).await;
    let later = Utc::now() + Duration::days(7);
    let mine = cyex_tv_db::transfers::create(&mut c, asked, owner, gone, gone, later)
        .await
        .unwrap()
        .unwrap();
    let theirs = cyex_tv_db::transfers::create(&mut c, untouched, owner, other, other, later)
        .await
        .unwrap()
        .unwrap();

    let mut tx = c.begin().await.unwrap();
    cyex_tv_db::privacy::erase_account(&mut tx, gone)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let uploader: Option<Uuid> = sqlx::query_scalar("SELECT uploaded_by FROM emotes WHERE id = $1")
        .bind(uploaded)
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert_eq!(uploader, None);
    let state = |id: Uuid| {
        sqlx::query_scalar::<_, String>("SELECT state FROM emote_transfer_requests WHERE id = $1")
            .bind(id)
    };
    assert_eq!(
        state(mine.id).fetch_one(&mut *c).await.unwrap(),
        "cancelled"
    );
    assert_eq!(
        state(theirs.id).fetch_one(&mut *c).await.unwrap(),
        "pending"
    );
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'emote.transfer_cancelled'
            AND after_json->>'reason' = 'account_deleted' AND target_id = $1",
    )
    .bind(ids::to_text(asked))
    .fetch_one(&mut *c)
    .await
    .unwrap();
    assert_eq!(audited, 1);
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn interrupted_concurrent_index_is_rebuilt() {
    let db = db!();
    let mut conn = db.conn().await;
    let run = |sql: &'static str| sqlx::query(sql);
    // A unique build over duplicate rows fails and leaves an invalid index behind, as an
    // interrupted build does. One carries the name of an index our migrations create
    // (`emotes_file_id`, dropped first), the other is someone else's.
    run("DROP INDEX emotes_file_id")
        .execute(&mut *conn)
        .await
        .unwrap();
    run("CREATE TABLE ix_probe (a int)")
        .execute(&mut *conn)
        .await
        .unwrap();
    run("INSERT INTO ix_probe VALUES (1), (1)")
        .execute(&mut *conn)
        .await
        .unwrap();
    for create in [
        "CREATE UNIQUE INDEX CONCURRENTLY emotes_file_id ON ix_probe (a)",
        "CREATE UNIQUE INDEX CONCURRENTLY ix_probe_foreign ON ix_probe (a)",
    ] {
        assert!(
            sqlx::query(create).execute(&mut *conn).await.is_err(),
            "{create}"
        );
    }
    let valid = |name: &'static str| {
        sqlx::query_scalar::<_, bool>(
            "SELECT indisvalid FROM pg_index WHERE indexrelid = to_regclass($1)",
        )
        .bind(name)
    };
    assert!(!valid("emotes_file_id").fetch_one(&mut *conn).await.unwrap());
    assert!(
        !valid("ix_probe_foreign")
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    );

    // Ours is still broken: the repair fails loudly instead of leaving it.
    assert!(cyex_tv_db::repair_invalid_indexes(&mut conn).await.is_err());

    // Once the rows allow it, the repair rebuilds ours and it is valid. The foreign one is
    // reported, never dropped or rebuilt: it is still there and still invalid.
    run("DELETE FROM ix_probe WHERE ctid = (SELECT max(ctid) FROM ix_probe)")
        .execute(&mut *conn)
        .await
        .unwrap();
    let repair = cyex_tv_db::repair_invalid_indexes(&mut conn).await.unwrap();
    assert_eq!(repair.rebuilt, ["emotes_file_id"]);
    assert_eq!(repair.left_alone, ["ix_probe_foreign"]);
    assert!(valid("emotes_file_id").fetch_one(&mut *conn).await.unwrap());
    assert!(
        !valid("ix_probe_foreign")
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    );

    // Nothing of ours left to do; the foreign one stays as it is, run after run.
    let again = cyex_tv_db::repair_invalid_indexes(&mut conn).await.unwrap();
    assert!(again.rebuilt.is_empty());
    assert_eq!(again.left_alone, ["ix_probe_foreign"]);
    assert!(
        !valid("ix_probe_foreign")
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    );
    drop(conn);
    db.close().await;
}

#[tokio::test]
async fn notification_prefs_changes_take_turns() {
    use cyex_tv_db::prefs;
    use sqlx::Connection;
    let db = db!();
    let mut conn = db.conn().await;
    let me = account(&mut conn, "9001").await;
    drop(conn);

    // First change: holds the row (created on the spot, there was none).
    let mut a = db.pool.acquire().await.unwrap();
    let mut tx_a = a.begin().await.unwrap();
    let seen = prefs::get_for_update(&mut tx_a, me).await.unwrap();
    assert_eq!(seen, prefs::Prefs::default());

    // Second change waits for the first instead of reading the old value.
    let pool = db.pool.clone();
    let second = tokio::spawn(async move {
        let mut b = pool.acquire().await.unwrap();
        let mut tx_b = b.begin().await.unwrap();
        let seen = prefs::get_for_update(&mut tx_b, me).await.unwrap();
        tx_b.commit().await.unwrap();
        seen
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "the second change must wait");

    prefs::set_notifications(&mut tx_a, me, &["review".to_owned()], &[])
        .await
        .unwrap();
    tx_a.commit().await.unwrap();
    let seen = second.await.unwrap();
    assert_eq!(seen.notify_off, ["review"]);
    drop(a);
    db.close().await;
}
