//! The hot reads at volume (a larger deployment: up to 100k emotes, 50k accounts, 20k channels).
//!
//! `hot_reads_use_their_indexes` seeds a few thousand rows of each kind, runs the real query
//! functions and checks, from Postgres' plan for each statement (`EXPLAIN`, not the scan counters, which other
//! activity inflates), that none of them read a whole table.
//! `timings_at_full_scale` (ignored; CI's `load` job runs it with `--nocapture`) seeds the full
//! size and prints each read's time before (the old query, without the new indexes) and after.

mod common;

use std::time::{Duration, Instant};

use common::TestDb;
use cyex_tv_db::reads::{self, MemberAccess};
use cyex_tv_db::{audit, channel_counts, ids, outbox, search};
use cyex_tv_shared::model::{EmoteSort, TrendingWindow};
use sqlx::PgConnection;
use uuid::Uuid;

struct Scale {
    emotes: i64,
    accounts: i64,
    channels: i64,
    members_per_set: i64,
    /// Activity rows naming the most used emote.
    activity: i64,
}

const SMALL: Scale = Scale {
    emotes: 20_000,
    accounts: 3_000,
    channels: 600,
    members_per_set: 150,
    activity: 5_000,
};

const FULL: Scale = Scale {
    emotes: 100_000,
    accounts: 50_000,
    channels: 20_000,
    members_per_set: 200,
    activity: 50_000,
};

/// Fills the database in a few statements. Emote use is skewed (a few emotes are in most
/// channels, as on any emote site); one in five accounts has a channel with an active set of
/// `members_per_set` emotes. Returns the most used emote.
async fn seed(c: &mut PgConnection, s: &Scale) -> Uuid {
    let statements = [
        "INSERT INTO accounts (id, created_at)
         SELECT gen_random_uuid(), now() - g * interval '1 minute' FROM generate_series(1, $1) g",
        "CREATE TABLE seed_accounts AS
         SELECT row_number() OVER (ORDER BY id) AS n, id FROM accounts",
        "INSERT INTO identities (platform, platform_user_id, account_id, login, display_name)
         SELECT 'twitch', (1000 + n)::text, id, 'user' || n, 'User' || n FROM seed_accounts",
        "INSERT INTO files (id, sha256, mime, width, height, storage_key, variants)
         SELECT gen_random_uuid(), decode(md5(g::text) || md5('f' || g), 'hex'), 'image/webp',
                CASE WHEN g % 20 = 0 THEN 128 ELSE 32 END, 32, 'seed/' || g, '[]'
         FROM generate_series(1, $2) g",
        "CREATE TABLE seed_files AS
         SELECT row_number() OVER (ORDER BY id) AS n, id FROM files",
        "INSERT INTO emotes (id, owner_account_id, file_id, name, state, private, unlisted, nsfw,
                             animated, zero_width, created_at)
         SELECT gen_random_uuid(), a.id, f.id,
                (ARRAY['Pepe','Kek','Pog','Cat','Dance','Sad','Wide','Hype'])[1 + f.n % 8]
                || (ARRAY['Laugh','Clap','Spin','Jam','Cry','Wave','Sit','Run'])[1 + (f.n / 8) % 8]
                || f.n,
                CASE WHEN f.n % 20 = 0 THEN 'pending' ELSE 'approved' END,
                f.n % 50 = 1, f.n % 33 = 2 AND f.n % 50 <> 1, f.n % 50 = 3, f.n % 4 = 0, f.n % 40 = 5,
                now() - f.n * interval '1 minute'
         FROM seed_files f JOIN seed_accounts a ON a.n = 1 + f.n % $1",
        "CREATE TABLE seed_emotes AS
         SELECT row_number() OVER (ORDER BY created_at DESC) AS n, id FROM emotes",
        "INSERT INTO emote_tags (emote_id, tag)
         SELECT id, 'tag' || (n % 50) FROM seed_emotes WHERE n % 3 = 0
         UNION ALL
         SELECT id, 'kind' || (n % 7) FROM seed_emotes WHERE n % 5 = 0",
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity)
         SELECT gen_random_uuid(), id, 'set ' || n, 'channel', 1000
         FROM seed_accounts WHERE n % 2 = 0 AND n / 2 <= $3",
        "INSERT INTO channels (id, account_id, platform, platform_channel_id, login, active_set_id)
         SELECT gen_random_uuid(), st.owner_account_id, 'twitch', (1000 + a.n)::text,
                'user' || a.n, st.id
         FROM emote_sets st JOIN seed_accounts a ON a.id = st.owner_account_id",
        // Skewed: random()^3 picks the first (most used) emotes far more often.
        "INSERT INTO emote_set_members (set_id, emote_id, name, added_at)
         SELECT p.set_id, e.id, 'm' || e.n, p.added_at
         FROM (SELECT st.id AS set_id, 1 + floor(power(random(), 3) * $2)::bigint AS pick,
                      now() - k * interval '1 minute' AS added_at
               FROM emote_sets st CROSS JOIN generate_series(1, $4) k
               OFFSET 0) p
         JOIN seed_emotes e ON e.n = p.pick
         ON CONFLICT DO NOTHING",
        "ANALYZE",
    ];
    for sql in statements {
        // The sizes are this file's own numbers, written into the text.
        let sql = sql
            .replace("$1", &s.accounts.to_string())
            .replace("$2", &s.emotes.to_string())
            .replace("$3", &s.channels.to_string())
            .replace("$4", &s.members_per_set.to_string());
        sqlx::query(&sql)
            .execute(&mut *c)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {e}"));
    }
    let members: i64 = sqlx::query_scalar("SELECT count(*) FROM emote_set_members")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert!(
        members > s.channels * s.members_per_set / 2,
        "seeded {members} set members"
    );
    search::refresh_popularity(c).await.unwrap();
    // The stored channel counts, as the worker's minutely round leaves them (the triggers queued
    // every seeded member).
    channel_counts::refresh_queued(c).await.unwrap();
    let top: Uuid = sqlx::query_scalar("SELECT id FROM seed_emotes WHERE n = 1")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    // Activity naming the most used emote: channel owners adding it to their listed sets.
    let sets: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT id, owner_account_id FROM emote_sets")
        .fetch_all(&mut *c)
        .await
        .unwrap();
    let set_texts: Vec<String> = sets.iter().map(|(id, _)| ids::to_text(*id)).collect();
    let owners: Vec<Uuid> = sets.iter().map(|(_, owner)| *owner).collect();
    sqlx::query(
        "INSERT INTO audit_log (id, actor_account_id, action, target_kind, target_id, after_json,
                                subject_account_id, created_at)
         SELECT gen_random_uuid(), st.owner, 'set.emote_added', 'emote_set', st.target,
                jsonb_build_object('emote_id', $1::text), st.owner,
                now() - g * interval '1 second'
         FROM generate_series(1, $2) g
         JOIN unnest($3::text[], $4::uuid[]) WITH ORDINALITY AS st(target, owner, n)
           ON st.n = 1 + g % cardinality($3::text[])",
    )
    .bind(ids::to_text(top))
    .bind(s.activity)
    .bind(&set_texts)
    .bind(&owners)
    .execute(&mut *c)
    .await
    .unwrap();
    // Fresh statistics and a current visibility map, so the planner judges the seeded sizes and
    // the reads below don't depend on whether autovacuum got there first.
    sqlx::query("VACUUM (ANALYZE)")
        .execute(&mut *c)
        .await
        .unwrap();
    top
}

/// Postgres' plan (`EXPLAIN (COSTS OFF)`) for `$sql` with the same values bound as the real read
/// binds, as text. The plan is a property of the statement, the data and the planner settings
/// alone, so unlike the scan counters in `pg_stat_user_tables` (which count every scan start,
/// including each rescan of a nested loop's inner side, and are updated asynchronously) it gives
/// the same answer every run.
macro_rules! plan {
    ($c:expr, $sql:expr $(, $bind:expr)*) => {{
        let sql = format!("EXPLAIN (COSTS OFF) {}", $sql);
        sqlx::query_scalar::<_, String>(&sql)
            $(.bind($bind))*
            .fetch_all(&mut *$c)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {e}"))
            .join("
")
    }};
}

/// Asserts the plan reads none of `tables` whole.
fn assert_no_seq_scan(what: &str, plan: &str, tables: &[&str]) {
    for table in tables {
        let whole = format!("Seq Scan on {table}");
        assert!(
            !plan.contains(&whole),
            "{what}: the plan reads the whole {table} table:
{plan}"
        );
    }
}

/// Asserts the plan reads `table` through some index. Which index (the browse index for the sort,
/// or a filter's) is the planner's choice and may change with the data; that it isn't a
/// whole-table read is what the page's speed rests on. `expected` names the index meant.
fn assert_index_scan(what: &str, plan: &str, table: &str, expected: &[&str]) {
    assert_no_seq_scan(what, plan, &[table]);
    assert!(
        plan.contains("Index Scan") || plan.contains("Index Only Scan") || plan.contains("Bitmap"),
        "{what}: {table} is read through no index (expected something like {expected:?}):
{plan}"
    );
}

fn browse(sort: EmoteSort, text: &str) -> search::Query<'_> {
    search::Query {
        text,
        sort,
        window: TrendingWindow::Week,
        limit: 48,
        offset: 0,
        ..Default::default()
    }
}

async fn set_ids(c: &mut PgConnection, n: i64) -> Vec<Uuid> {
    sqlx::query_scalar("SELECT id FROM emote_sets ORDER BY id LIMIT $1")
        .bind(n)
        .fetch_all(&mut *c)
        .await
        .unwrap()
}

#[tokio::test]
async fn hot_reads_use_their_indexes() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let top = seed(c, &SMALL).await;
    // Plan as production does: its disks are SSDs, where a random read costs about a sequential
    // one (Postgres' default of 4 suits spinning disks and tips small, freshly seeded tables
    // toward a table read), and parallel workers would only add noise to the plans.
    sqlx::query("SET random_page_cost = 1.1")
        .execute(&mut *c)
        .await
        .unwrap();
    sqlx::query("SET max_parallel_workers_per_gather = 0")
        .execute(&mut *c)
        .await
        .unwrap();
    // The page comes from the browse indexes in order. (The total is a count apart; with most
    // emotes matching, Postgres reads the table for it, and that is the cheap way.)
    for (sort, index) in [
        (EmoteSort::Top, &["emotes_browse_top"][..]),
        (EmoteSort::New, &["emotes_browse_new", "emotes_state"][..]),
        (EmoteSort::Relevance, &["emotes_searchable_popularity"][..]),
    ] {
        let q = browse(sort, "");
        let plan = search::explain_page(c, &q).await.unwrap();
        assert_index_scan(&format!("browse {sort:?}"), &plan, "emotes", index);
        let hits = search::search(c, &q).await.unwrap();
        assert_eq!(hits.ids.len(), 48);
        assert!(hits.total > 15_000, "{sort:?} total {}", hits.total);
    }
    // Text search: at this size a table read can be the cheaper plan; the full-scale run prints
    // the plan.
    let hits = search::search(c, &browse(EmoteSort::Relevance, "PepeJam"))
        .await
        .unwrap();
    assert!(!hits.ids.is_empty());

    let sets = set_ids(c, 50).await;
    let access = MemberAccess::default();
    let plan = plan!(
        c,
        reads::newest_members_sql(),
        &sets,
        access.reviewer,
        access.edit_sets,
        access.account,
        12i64
    );
    assert_no_seq_scan("set previews", &plan, &["emote_set_members"]);
    let rows = reads::newest_members(c, &sets, &access, 12).await.unwrap();
    assert_eq!(rows.len(), 50 * 12);

    let popular: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM seed_emotes ORDER BY n LIMIT 48")
        .fetch_all(&mut *c)
        .await
        .unwrap();
    let plan = plan!(c, reads::CHANNEL_COUNTS_SQL, &popular);
    assert_index_scan("stored channel counts", &plan, "emotes", &["emotes_pkey"]);
    assert_no_seq_scan("stored channel counts", &plan, &["emote_set_members"]);
    let counts = reads::channel_counts(c, &popular).await.unwrap();
    assert_eq!(counts, channel_counts::live(c, &popular).await.unwrap());

    // The minutely round over a few queued emotes reads only their members.
    let plan = plan!(c, channel_counts::recount_sql(), &popular);
    assert_no_seq_scan(
        "channel count refresh",
        &plan,
        &["emote_set_members", "emotes"],
    );
    sqlx::query("INSERT INTO emote_channel_count_queue (emote_id) SELECT unnest($1::uuid[])")
        .bind(&popular)
        .execute(&mut *c)
        .await
        .unwrap();
    channel_counts::refresh_queued(c).await.unwrap();

    let plan = plan!(c, reads::EMOTE_CHANNELS_SQL, top, 24i64, 0i64);
    assert_no_seq_scan("emote channels", &plan, &["emote_set_members"]);
    reads::emote_channels(c, top, 24, 0).await.unwrap();

    let top_text = ids::to_text(top);
    let plan = plan!(
        c,
        audit::emote_activity_sql(),
        &top_text,
        None::<Uuid>,
        30i64
    );
    assert_no_seq_scan("emote activity", &plan, &["audit_log"]);
    let page = audit::emote_activity_rows(c, &top_text, None, 30)
        .await
        .unwrap();
    assert_eq!(page.len(), 30);
    db.close().await;
}

/// The rewritten reads answer exactly what the old ones did.
#[tokio::test]
async fn rewritten_reads_match_the_old_ones() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let top = seed(
        c,
        &Scale {
            emotes: 3_000,
            accounts: 500,
            channels: 60,
            members_per_set: 80,
            activity: 400,
        },
    )
    .await;
    for (sort, text) in [
        (EmoteSort::Top, ""),
        (EmoteSort::New, ""),
        (EmoteSort::Relevance, ""),
        (EmoteSort::Relevance, "pog"),
        (EmoteSort::Relevance, "kind3"),
        (EmoteSort::Top, "Laugh"),
        (EmoteSort::New, "ca"),
    ] {
        for offset in [0, 48, 2_900, 5_000] {
            let mut q = browse(sort, text);
            q.offset = offset;
            let new = search::search(c, &q).await.unwrap();
            let old = old_search(c, &q).await;
            assert_eq!(new, old, "{sort:?} {text:?} offset {offset}");
        }
    }
    let sets = set_ids(c, 20).await;
    for per_set in [1, 12, 500] {
        let new = reads::newest_members(c, &sets, &MemberAccess::default(), per_set)
            .await
            .unwrap();
        let old: Vec<reads::MemberRow> = sqlx::query_as(OLD_NEWEST)
            .bind(&sets)
            .bind(false)
            .bind(Vec::<Uuid>::new())
            .bind(None::<Uuid>)
            .bind(per_set)
            .fetch_all(&mut *c)
            .await
            .unwrap();
        assert_eq!(new, old, "per set {per_set}");
    }
    let text = ids::to_text(top);
    let mut before = None;
    loop {
        let new = audit::emote_activity_rows(c, &text, before, 50)
            .await
            .unwrap();
        let old: Vec<audit::SubjectEntry> = sqlx::query_as(&old_activity())
            .bind(&text)
            .bind(before)
            .bind(50i64)
            .fetch_all(&mut *c)
            .await
            .unwrap();
        assert_eq!(new, old);
        let Some(last) = new.last() else { break };
        before = Some(last.entry.id);
    }
    db.close().await;
}

/// Old events go in batches, all of them, and newer ones stay.
#[tokio::test]
async fn the_outbox_is_pruned_in_batches() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let old = outbox::DELETE_BATCH * 2 + 17;
    sqlx::query(
        "INSERT INTO events_outbox (type, topics, created_at)
         SELECT 'set.emote_added', ARRAY['set:x'],
                now() - interval '3 days' - g * interval '1 second'
         FROM generate_series(1, $1) g
         UNION ALL
         SELECT 'set.emote_added', ARRAY['set:x'], now() FROM generate_series(1, 5)",
    )
    .bind(old)
    .execute(&mut *c)
    .await
    .unwrap();
    let removed = outbox::delete_older_than(c, chrono::Utc::now() - chrono::Duration::hours(48))
        .await
        .unwrap();
    assert_eq!(removed, old as u64);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM events_outbox")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert_eq!(left, 5);
    db.close().await;
}

async fn drop_new_indexes(c: &mut PgConnection) {
    for name in [
        "emote_set_members_emote_added",
        "emote_set_members_set_newest",
        "emotes_browse_top",
        "emotes_browse_new",
    ] {
        sqlx::query(&format!("DROP INDEX {name}"))
            .execute(&mut *c)
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "seeds the full scale; CI's load job runs it"]
async fn timings_at_full_scale() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let started = Instant::now();
    let top = seed(c, &FULL).await;
    let members: i64 = sqlx::query_scalar("SELECT count(*) FROM emote_set_members")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    println!(
        "seeded {} emotes, {} accounts, {} channels, {members} set members in {:?}",
        FULL.emotes,
        FULL.accounts,
        FULL.channels,
        started.elapsed()
    );
    let sets = set_ids(c, 50).await;
    let big: Vec<Uuid> = sets[..1].to_vec();
    let popular: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM seed_emotes ORDER BY n LIMIT 1000")
        .fetch_all(&mut *c)
        .await
        .unwrap();
    let top_text = ids::to_text(top);

    let mut lines = Vec::new();
    macro_rules! measure {
        ($label:expr, $run:expr) => {{
            // The first run warms the cache; the median of the next five counts.
            let _ = $run;
            let mut all = Vec::new();
            for _ in 0..5 {
                let start = Instant::now();
                let _ = $run;
                all.push(start.elapsed());
            }
            all.sort();
            let d: Duration = all[all.len() / 2];
            lines.push(format!(
                "{:<44} {:>9.1} ms",
                $label,
                d.as_secs_f64() * 1000.0
            ));
        }};
    }
    for phase in ["after", "before"] {
        let old = phase == "before";
        if old {
            drop_new_indexes(c).await;
            sqlx::query("ANALYZE").execute(&mut *c).await.unwrap();
        }
        for (label, sort, text) in [
            ("browse TOP", EmoteSort::Top, ""),
            ("browse NEW", EmoteSort::New, ""),
            ("browse default", EmoteSort::Relevance, ""),
            ("search \"PepeJam\"", EmoteSort::Relevance, "PepeJam"),
            ("browse TRENDING", EmoteSort::Trending, ""),
        ] {
            let q = browse(sort, text);
            if old {
                measure!(format!("{phase}: {label}"), old_search(c, &q).await);
            } else {
                measure!(
                    format!("{phase}: {label}"),
                    search::search(c, &q).await.unwrap()
                );
            }
        }
        measure!(
            format!("{phase}: quick search \"pep\""),
            search::quick(c, "pep", 8).await.unwrap()
        );
        if old {
            measure!(
                format!("{phase}: 50 set previews"),
                sqlx::query_as::<_, reads::MemberRow>(OLD_NEWEST)
                    .bind(&sets)
                    .bind(false)
                    .bind(Vec::<Uuid>::new())
                    .bind(None::<Uuid>)
                    .bind(12i64)
                    .fetch_all(&mut *c)
                    .await
                    .unwrap()
            );
            measure!(
                format!("{phase}: emote activity page"),
                sqlx::query_as::<_, audit::SubjectEntry>(&old_activity())
                    .bind(&top_text)
                    .bind(None::<Uuid>)
                    .bind(30i64)
                    .fetch_all(&mut *c)
                    .await
                    .unwrap()
            );
        } else {
            measure!(
                format!("{phase}: 50 set previews"),
                reads::newest_members(c, &sets, &MemberAccess::default(), 12)
                    .await
                    .unwrap()
            );
            measure!(
                format!("{phase}: emote activity page"),
                audit::emote_activity_rows(c, &top_text, None, 30)
                    .await
                    .unwrap()
            );
        }
        measure!(
            format!("{phase}: one whole set (members)"),
            reads::members(c, &big).await.unwrap()
        );
        // Before: counted live from the set members (as reads did). After: the stored count.
        if old {
            measure!(
                format!("{phase}: channel counts, 48 popular"),
                channel_counts::live(c, &popular[..48]).await.unwrap()
            );
            measure!(
                format!("{phase}: channel counts, 1000 popular"),
                channel_counts::live(c, &popular).await.unwrap()
            );
        } else {
            measure!(
                format!("{phase}: channel counts, 48 popular"),
                reads::channel_counts(c, &popular[..48]).await.unwrap()
            );
            measure!(
                format!("{phase}: channel counts, 1000 popular"),
                reads::channel_counts(c, &popular).await.unwrap()
            );
            // What the worker pays for them: a minutely round with the 1000 queued, and the
            // hourly recount of every emote (once each; they write).
            sqlx::query(
                "INSERT INTO emote_channel_count_queue (emote_id) SELECT unnest($1::uuid[])",
            )
            .bind(&popular)
            .execute(&mut *c)
            .await
            .unwrap();
            let start = Instant::now();
            channel_counts::refresh_queued(c).await.unwrap();
            lines.push(format!(
                "{:<44} {:>9.1} ms",
                format!("{phase}: worker round, 1000 popular queued"),
                start.elapsed().as_secs_f64() * 1000.0
            ));
            let start = Instant::now();
            let changed = channel_counts::reconcile_all(c).await.unwrap();
            assert_eq!(changed, 0, "the reconcile found stored counts off");
            lines.push(format!(
                "{:<44} {:>9.1} ms",
                format!("{phase}: worker reconcile, all emotes"),
                start.elapsed().as_secs_f64() * 1000.0
            ));
        }
        measure!(
            format!("{phase}: emote channels page"),
            reads::emote_channels(c, top, 24, 0).await.unwrap()
        );
        measure!(
            format!("{phase}: emote channels page at offset 2000"),
            reads::emote_channels(c, top, 24, 2000).await.unwrap()
        );
    }
    println!("{}", lines.join("\n"));
    db.close().await;
}

// The queries as they were before the scale work, to compare answers and times.

async fn old_search(c: &mut PgConnection, q: &search::Query<'_>) -> search::Hits {
    let text: String = q.text.trim().chars().take(100).collect();
    let pattern = text
        .to_lowercase()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let mut join = String::new();
    let order = match q.sort {
        EmoteSort::Relevance => {
            "($1 <> '' AND lower(e.name) = lower($1)) DESC,
                  ($1 <> '' AND lower(e.name) LIKE $5 || '%') DESC,
                  CASE WHEN $1 = '' THEN 0 ELSE similarity(e.name, $1) END DESC,
                  e.popularity DESC,
                  e.id"
        }
        EmoteSort::Top => "e.popularity DESC, lower(e.name), e.id",
        EmoteSort::New => "e.created_at DESC, e.id DESC",
        EmoteSort::Trending => {
            join = format!(
                "JOIN (SELECT emote_id, sum(adds - removes)::bigint AS net
                       FROM emote_channel_adds
                       WHERE hour > now() - make_interval(days => {})
                       GROUP BY emote_id HAVING sum(adds - removes) > 0) tr ON tr.emote_id = e.id",
                q.window.days()
            );
            "tr.net DESC, e.popularity DESC, e.id"
        }
    };
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(&format!(
        "SELECT e.id, count(*) OVER () AS total
         FROM emotes e
         {join}
         WHERE e.state = 'approved' AND e.deleted_at IS NULL AND NOT e.private
           AND NOT e.unlisted
           AND NOT EXISTS (SELECT 1 FROM accounts oa
                           WHERE oa.id = e.owner_account_id AND oa.deleted_at IS NOT NULL)
           AND ($2::boolean IS NULL OR e.animated = $2)
           AND ($3::uuid IS NULL OR e.owner_account_id = $3)
           AND ($4 OR $10 OR NOT e.nsfw)
           AND (NOT $10 OR e.nsfw)
           AND (NOT $8 OR e.zero_width)
           AND (NOT $9 OR EXISTS (SELECT 1 FROM files f
                                  WHERE f.id = e.file_id AND f.width * 4 >= f.height * 7))
           AND (SELECT count(*) FROM emote_tags t
                WHERE t.emote_id = e.id AND t.tag = ANY($11::text[])) = cardinality($11::text[])
           AND ($1 = ''
                OR ($12 AND lower(e.name) = lower($1))
                OR (NOT $12
                    AND (lower(e.name) LIKE '%' || $5 || '%'
                         OR e.name % $1
                         OR EXISTS (SELECT 1 FROM emote_tags t
                                    WHERE t.emote_id = e.id AND t.tag LIKE $5 || '%'))))
         ORDER BY {order}
         LIMIT $6 OFFSET $7"
    ))
    .bind(&text)
    .bind(q.animated)
    .bind(q.owner)
    .bind(q.nsfw)
    .bind(&pattern)
    .bind(q.limit)
    .bind(q.offset)
    .bind(false)
    .bind(false)
    .bind(false)
    .bind(Vec::<String>::new())
    .bind(q.exact)
    .fetch_all(&mut *c)
    .await
    .unwrap();
    search::Hits {
        total: rows.first().map_or(0, |(_, t)| *t),
        ids: rows.into_iter().map(|(id, _)| id).collect(),
        capped: false,
    }
}

const OLD_NEWEST: &str = "SELECT * FROM (
             SELECT m.set_id, m.emote_id, m.name, m.zero_width, m.added_by, m.added_at,
                e.owner_account_id AS emote_owner, e.name AS emote_name,
                e.zero_width AS emote_zero_width, e.private, e.unlisted, e.nsfw,
                e.animated, e.state, e.created_at AS emote_created_at,
                f.sha256, f.variants,
                    row_number() OVER (PARTITION BY m.set_id
                                       ORDER BY m.added_at DESC, m.emote_id DESC) AS rn
             FROM emote_set_members m
             JOIN emotes e ON e.id = m.emote_id AND e.deleted_at IS NULL
             LEFT JOIN files f ON f.id = e.file_id
             WHERE m.set_id = ANY($1)
               AND (e.state = 'approved'
                    OR (e.state = 'pending'
                        AND ($2 OR m.set_id = ANY($3) OR e.owner_account_id = $4)))
         ) t
         WHERE rn <= $5
         ORDER BY set_id, rn";

fn old_activity() -> String {
    "SELECT a.id, a.actor_account_id, a.action, a.target_kind, a.target_id, a.before_json,
                a.after_json, a.created_at, a.subject_account_id
         FROM audit_log a
         JOIN accounts s ON s.id = a.subject_account_id
                        AND s.deleted_at IS NULL AND (s.blocked_at IS NULL OR s.blocked_until <= now())
         LEFT JOIN accounts act ON act.id = a.actor_account_id
         WHERE a.subject_account_id IS NOT NULL
           AND ($2::uuid IS NULL OR a.id < $2)
           AND (
               (a.action = 'emote.approved' AND a.target_kind = 'emote' AND a.target_id = $1
                AND NOT EXISTS (SELECT 1 FROM audit_log p
                                WHERE p.action = 'emote.approved' AND p.target_kind = 'emote'
                                  AND p.target_id = a.target_id AND p.id < a.id))
               OR (a.action IN ('set.emote_added', 'set.emote_removed', 'set.emote_updated')
                   AND coalesce(a.after_json ->> 'emote_id', a.before_json ->> 'emote_id') = $1
                   AND act.id IS NOT NULL AND (act.blocked_at IS NULL OR act.blocked_until <= now()) AND act.deleted_at IS NULL
                   AND (a.actor_account_id = a.subject_account_id
                        OR a.after_json ->> 'via' = 'account_editor'))
           )
           AND (a.action NOT IN ('set.created', 'set.renamed', 'set.emote_added',
                                              'set.emote_removed', 'set.emote_updated')
               OR EXISTS (SELECT 1 FROM emote_sets st
                          WHERE st.id = ulid_to_uuid(a.target_id)
                            AND st.owner_account_id = a.subject_account_id
                            AND st.visibility = 'listed'))
         ORDER BY a.id DESC
         LIMIT $3"
        .to_owned()
}
