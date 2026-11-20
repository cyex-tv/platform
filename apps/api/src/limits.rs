//! Request rate limits (PLAN.md Phase 7), per IP for every route class and per account on
//! GraphQL. A request over a limit gets `429 rate_limited` with `Retry-After`.
//!
//! The upload's 20 an hour per account lives in `upload`, since it counts what was made, and so
//! does the cap on emotes waiting for review. Editor changes are checked per account in
//! `editors`, cosmetics writes in `cosmetics`, set clones in `catalog`.
//!
//! Every limit can be set when the api starts (`CYEX_TV_LIMIT_*`, `Rates::from_env`). The
//! per-account ones (`AccountLimit`) can also be raised or lowered per role (`setRoleLimits`): an
//! account counts under the most generous override among its roles, else the default. What an
//! account gets is kept in memory for `ACCOUNT_TTL` (`Limits::load`, run once per request by
//! `MaybeCaller`), so the checks themselves never wait on the database.

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cyex_tv_auth::Caller;
use cyex_tv_db::ids;
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::LimitGroup;
use cyex_tv_shared::ratelimit::{Limiter, Rate, client_ip};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::rate_limited;
use crate::state::AppState;

#[derive(Clone, Copy, Debug)]
pub struct Rates {
    /// `/v1/auth/*`, per IP.
    pub auth: Rate,
    /// `POST /v1/gql`, per IP.
    pub gql: Rate,
    /// `POST /v1/gql`, per signed-in account.
    pub gql_account: Rate,
    /// `POST /v1/emotes`, per IP.
    pub upload: Rate,
    /// The public REST reads, per IP. Chat clients poll these; the CDN takes most of it.
    pub reads: Rate,
    /// Opening `/v1/gql/ws`, per IP.
    pub socket: Rate,
    /// Adding, changing and removing account editors, per account (ADR 0014).
    pub editors: Rate,
    /// Cosmetics writes (staff changes, grants, selections), per account (ADR 0015); staff shop
    /// and code writes count here too (ADR 0016).
    pub cosmetics: Rate,
    /// Starting a checkout, per account (ADR 0016).
    pub checkout: Rate,
    /// Redeem tries, right or wrong, per account (ADR 0016).
    pub redeem: Rate,
    /// `cloneSet`, per account: each copies up to a whole set.
    pub set_clones: Rate,
    /// `exportMyData`, per account: each reads everything about the account.
    pub data_export: Rate,
    /// `searchAccounts`, per account, or per IP when signed out.
    pub account_search: Rate,
    /// `quickSearch`, per account, or per IP when signed out. Typed live, so generous.
    pub quick_search: Rate,
    /// The public reads that fan out into many lookups (`accountActivity`, `emoteActivity`,
    /// `accountSets`), per account, or per IP when signed out.
    pub heavy_reads: Rate,
    /// `deleteAccount`, per account, tries with a wrong confirmation included.
    pub delete_account: Rate,
    /// `transferEmote`, per account, refused tries included (ADR 0014).
    pub emote_transfers: Rate,
    /// Hiding, unhiding, favouriting and unfavouriting, per account.
    pub personal_lists: Rate,
    /// Your own settings (notification preferences, the public collection).
    pub prefs: Rate,
    /// Emotes one account may upload, and get, per hour (`upload`).
    pub uploads_per_hour: u32,
    /// Emotes of one account that may wait for review at once (`upload`).
    pub pending_emotes: u32,
}

impl Default for Rates {
    fn default() -> Self {
        Self {
            auth: Rate::per_minute(30),
            gql: Rate::per_minute(300),
            gql_account: Rate::per_minute(600),
            upload: Rate::per_hour(30),
            reads: Rate::per_minute(1200),
            socket: Rate::per_minute(60),
            editors: Rate::per_hour(60),
            cosmetics: Rate::per_hour(300),
            checkout: Rate::per_hour(30),
            redeem: Rate::per_hour(30),
            set_clones: Rate::per_hour(30),
            data_export: Rate::per_hour(5),
            account_search: Rate::per_minute(60),
            quick_search: Rate::per_minute(240),
            heavy_reads: Rate::per_minute(120),
            delete_account: Rate::per_hour(3),
            emote_transfers: Rate::per_hour(60),
            personal_lists: Rate::per_minute(120),
            prefs: Rate::per_hour(120),
            uploads_per_hour: 20,
            pending_emotes: 20,
        }
    }
}

/// The environment variable that sets a rate: `CYEX_TV_LIMIT_` and the field's name in capitals.
pub const ENV_PREFIX: &str = "CYEX_TV_LIMIT_";

impl Rates {
    /// Every `Rate` with its field name, in the order `.env.example` lists them.
    fn rates_mut(&mut self) -> [(&'static str, &mut Rate); 19] {
        [
            ("auth", &mut self.auth),
            ("gql", &mut self.gql),
            ("gql_account", &mut self.gql_account),
            ("upload", &mut self.upload),
            ("reads", &mut self.reads),
            ("socket", &mut self.socket),
            ("editors", &mut self.editors),
            ("cosmetics", &mut self.cosmetics),
            ("checkout", &mut self.checkout),
            ("redeem", &mut self.redeem),
            ("set_clones", &mut self.set_clones),
            ("data_export", &mut self.data_export),
            ("account_search", &mut self.account_search),
            ("quick_search", &mut self.quick_search),
            ("heavy_reads", &mut self.heavy_reads),
            ("delete_account", &mut self.delete_account),
            ("emote_transfers", &mut self.emote_transfers),
            ("personal_lists", &mut self.personal_lists),
            ("prefs", &mut self.prefs),
        ]
    }

    /// The defaults, with what the environment sets instead: `CYEX_TV_LIMIT_GQL_ACCOUNT=600/1m`
    /// for a rate (`Rate::parse`: a count per `s`, `m`, `h` or `d`), and a whole number for
    /// `CYEX_TV_LIMIT_UPLOADS_PER_HOUR` and `CYEX_TV_LIMIT_PENDING_EMOTES`. An unset or empty
    /// variable keeps the default; one that doesn't read is an error naming it.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut rates = Self::default();
        let var = |field: &str| {
            let name = format!("{ENV_PREFIX}{}", field.to_ascii_uppercase());
            let value = get(&name).filter(|v| !v.trim().is_empty());
            (name, value)
        };
        for (field, rate) in rates.rates_mut() {
            if let (name, Some(raw)) = var(field) {
                *rate = Rate::parse(&raw).map_err(|e| format!("{name}: {e}"))?;
            }
        }
        for (field, count) in [
            ("uploads_per_hour", &mut rates.uploads_per_hour),
            ("pending_emotes", &mut rates.pending_emotes),
        ] {
            if let (name, Some(raw)) = var(field) {
                *count = raw
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|n| (1..=MAX_LIMIT).contains(n))
                    .ok_or_else(|| {
                        format!("{name}: {raw:?} isn't a whole number from 1 to {MAX_LIMIT}")
                    })?;
            }
        }
        Ok(rates)
    }

    /// The count of `limit` and the window it is per (`None` for `pending_emotes`).
    pub fn default_of(&self, limit: AccountLimit) -> (u32, Option<Duration>) {
        let rate = |r: Rate| (r.burst, Some(r.per));
        match limit {
            AccountLimit::GqlAccount => rate(self.gql_account),
            AccountLimit::UploadsPerHour => {
                (self.uploads_per_hour, Some(Duration::from_secs(3600)))
            }
            AccountLimit::PendingEmotes => (self.pending_emotes, None),
            AccountLimit::Editors => rate(self.editors),
            AccountLimit::Cosmetics => rate(self.cosmetics),
            AccountLimit::SetClones => rate(self.set_clones),
            AccountLimit::AccountSearch => rate(self.account_search),
            AccountLimit::QuickSearch => rate(self.quick_search),
            AccountLimit::HeavyReads => rate(self.heavy_reads),
            AccountLimit::PersonalLists => rate(self.personal_lists),
            AccountLimit::EmoteTransfers => rate(self.emote_transfers),
            AccountLimit::Prefs => rate(self.prefs),
            AccountLimit::Checkout => rate(self.checkout),
            AccountLimit::Redeem => rate(self.redeem),
            AccountLimit::DataExport => rate(self.data_export),
        }
    }
}

/// The highest count a limit may be set to, from the environment or on a role.
pub const MAX_LIMIT: u32 = 1_000_000;

/// The per-account limits a role can raise or lower, by the key a role stores them under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AccountLimit {
    GqlAccount,
    UploadsPerHour,
    PendingEmotes,
    Editors,
    Cosmetics,
    SetClones,
    AccountSearch,
    QuickSearch,
    HeavyReads,
    PersonalLists,
    EmoteTransfers,
    Prefs,
    Checkout,
    Redeem,
    DataExport,
}

impl AccountLimit {
    pub const ALL: [Self; 15] = [
        Self::GqlAccount,
        Self::AccountSearch,
        Self::QuickSearch,
        Self::HeavyReads,
        Self::UploadsPerHour,
        Self::PendingEmotes,
        Self::Editors,
        Self::Cosmetics,
        Self::SetClones,
        Self::PersonalLists,
        Self::EmoteTransfers,
        Self::Prefs,
        Self::Checkout,
        Self::Redeem,
        Self::DataExport,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::GqlAccount => "gql_account",
            Self::UploadsPerHour => "uploads_per_hour",
            Self::PendingEmotes => "pending_emotes",
            Self::Editors => "editors",
            Self::Cosmetics => "cosmetics",
            Self::SetClones => "set_clones",
            Self::AccountSearch => "account_search",
            Self::QuickSearch => "quick_search",
            Self::HeavyReads => "heavy_reads",
            Self::PersonalLists => "personal_lists",
            Self::EmoteTransfers => "emote_transfers",
            Self::Prefs => "prefs",
            Self::Checkout => "checkout",
            Self::Redeem => "redeem",
            Self::DataExport => "data_export",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.key() == key)
    }

    pub const fn group(self) -> LimitGroup {
        match self {
            Self::GqlAccount | Self::AccountSearch | Self::QuickSearch | Self::HeavyReads => {
                LimitGroup::Requests
            }
            Self::UploadsPerHour | Self::PendingEmotes => LimitGroup::Uploads,
            _ => LimitGroup::Account,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// One role's overrides.
pub type RoleOverrides = BTreeMap<AccountLimit, u32>;

/// A role's stored overrides (`roles.limits`). Unknown keys and values out of range are skipped:
/// `setRoleLimits` never writes them, and a key a later version dropped shouldn't break a read.
pub fn overrides_from_json(value: &serde_json::Value) -> RoleOverrides {
    value
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| {
            let limit = AccountLimit::from_key(k)?;
            let n = u32::try_from(v.as_u64()?).ok()?;
            (1..=MAX_LIMIT).contains(&n).then_some((limit, n))
        })
        .collect()
}

pub fn overrides_to_json(overrides: &RoleOverrides) -> serde_json::Value {
    serde_json::Value::Object(
        overrides
            .iter()
            .map(|(l, n)| (l.key().to_owned(), serde_json::Value::from(*n)))
            .collect(),
    )
}

/// What one account may do: each `AccountLimit`'s count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountLimits([u32; AccountLimit::ALL.len()]);

impl AccountLimits {
    pub fn defaults(rates: &Rates) -> Self {
        let mut out = [0; AccountLimit::ALL.len()];
        for l in AccountLimit::ALL {
            out[l.index()] = rates.default_of(l).0;
        }
        Self(out)
    }

    /// Per limit, the most generous override among `roles`, else the default. A role that
    /// lowers a limit lowers it only when no other role the account holds sets it higher.
    pub fn resolve(rates: &Rates, roles: &[RoleOverrides]) -> Self {
        let mut out = Self::defaults(rates);
        for l in AccountLimit::ALL {
            if let Some(n) = roles.iter().filter_map(|r| r.get(&l).copied()).max() {
                out.0[l.index()] = n;
            }
        }
        out
    }

    pub fn get(&self, limit: AccountLimit) -> u32 {
        self.0[limit.index()]
    }
}

/// How long what an account may do is kept before it is read again.
pub const ACCOUNT_TTL: Duration = Duration::from_secs(30);
/// Accounts kept before stale entries are swept out.
const ACCOUNTS_SWEEP_AT: usize = 50_000;

/// What each account recently seen may do, per instance.
struct AccountCache {
    ttl: Duration,
    entries: Mutex<HashMap<Uuid, (Instant, Arc<AccountLimits>)>>,
}

impl AccountCache {
    fn get(&self, account: Uuid) -> Option<(Instant, Arc<AccountLimits>)> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.get(&account).cloned()
    }

    fn put(&self, account: Uuid, limits: Arc<AccountLimits>) {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() >= ACCOUNTS_SWEEP_AT {
            let ttl = self.ttl;
            entries.retain(|_, (at, _)| now.duration_since(*at) < ttl);
        }
        entries.insert(account, (now, limits));
    }
}

pub struct Limits {
    pub auth: Limiter,
    pub gql: Limiter,
    pub gql_account: Limiter,
    pub upload: Limiter,
    pub reads: Limiter,
    pub socket: Limiter,
    pub editors: Limiter,
    pub cosmetics: Limiter,
    pub checkout: Limiter,
    pub redeem: Limiter,
    /// Wrong redeem codes lock an account and an IP out for a while (ADR 0016).
    pub redeem_lockout: RedeemLockout,
    pub set_clones: Limiter,
    pub data_export: Limiter,
    pub account_search: Limiter,
    pub quick_search: Limiter,
    pub heavy_reads: Limiter,
    pub delete_account: Limiter,
    pub emote_transfers: Limiter,
    pub personal_lists: Limiter,
    pub prefs: Limiter,
    /// Take the client IP from `CF-Connecting-IP` (`CYEX_TV_TRUST_CLOUDFLARE=true`).
    pub trust_cloudflare: bool,
    /// What the limits were started with.
    pub rates: Rates,
    defaults: Arc<AccountLimits>,
    accounts: AccountCache,
}

impl Limits {
    pub fn new(rates: Rates, trust_cloudflare: bool) -> Self {
        Self {
            auth: Limiter::new(rates.auth),
            gql: Limiter::new(rates.gql),
            gql_account: Limiter::new(rates.gql_account),
            upload: Limiter::new(rates.upload),
            reads: Limiter::new(rates.reads),
            socket: Limiter::new(rates.socket),
            editors: Limiter::new(rates.editors),
            cosmetics: Limiter::new(rates.cosmetics),
            checkout: Limiter::new(rates.checkout),
            redeem: Limiter::new(rates.redeem),
            redeem_lockout: RedeemLockout::default(),
            set_clones: Limiter::new(rates.set_clones),
            data_export: Limiter::new(rates.data_export),
            account_search: Limiter::new(rates.account_search),
            quick_search: Limiter::new(rates.quick_search),
            heavy_reads: Limiter::new(rates.heavy_reads),
            delete_account: Limiter::new(rates.delete_account),
            emote_transfers: Limiter::new(rates.emote_transfers),
            personal_lists: Limiter::new(rates.personal_lists),
            prefs: Limiter::new(rates.prefs),
            trust_cloudflare,
            rates,
            defaults: Arc::new(AccountLimits::defaults(&rates)),
            accounts: AccountCache {
                ttl: ACCOUNT_TTL,
                entries: Mutex::default(),
            },
        }
    }

    /// Keeps what an account may do for `ttl` instead of `ACCOUNT_TTL` (zero: read every time).
    pub fn with_account_ttl(mut self, ttl: Duration) -> Self {
        self.accounts.ttl = ttl;
        self
    }

    /// The limiter a per-account limit counts in; `None` for the upload counts, which count
    /// what was made (`upload`).
    fn limiter(&self, limit: AccountLimit) -> Option<&Limiter> {
        Some(match limit {
            AccountLimit::GqlAccount => &self.gql_account,
            AccountLimit::Editors => &self.editors,
            AccountLimit::Cosmetics => &self.cosmetics,
            AccountLimit::SetClones => &self.set_clones,
            AccountLimit::AccountSearch => &self.account_search,
            AccountLimit::QuickSearch => &self.quick_search,
            AccountLimit::HeavyReads => &self.heavy_reads,
            AccountLimit::PersonalLists => &self.personal_lists,
            AccountLimit::EmoteTransfers => &self.emote_transfers,
            AccountLimit::Prefs => &self.prefs,
            AccountLimit::Checkout => &self.checkout,
            AccountLimit::Redeem => &self.redeem,
            AccountLimit::DataExport => &self.data_export,
            AccountLimit::UploadsPerHour | AccountLimit::PendingEmotes => return None,
        })
    }

    /// What `account` may do as last read, however old; the defaults for one not read yet and
    /// for nobody signed in. Never waits: `load` keeps it current.
    pub fn of(&self, account: Option<Uuid>) -> Arc<AccountLimits> {
        account
            .and_then(|a| self.accounts.get(a))
            .map_or_else(|| self.defaults.clone(), |(_, l)| l)
    }

    /// What `account` may do, read from its roles when not read in the last `ACCOUNT_TTL`. When
    /// the database can't be reached it goes on with what it had (or the defaults).
    pub async fn load(&self, db: &PgPool, account: Uuid) -> Arc<AccountLimits> {
        if let Some((at, limits)) = self.accounts.get(account)
            && at.elapsed() < self.accounts.ttl
        {
            return limits;
        }
        let read = async {
            let mut conn = db.acquire().await.map_err(cyex_tv_db::DbError::from)?;
            cyex_tv_db::roles::limit_overrides_of(&mut conn, account).await
        };
        match read.await {
            Ok(rows) => {
                let roles: Vec<RoleOverrides> = rows.iter().map(overrides_from_json).collect();
                let limits = Arc::new(AccountLimits::resolve(&self.rates, &roles));
                self.accounts.put(account, limits.clone());
                limits
            }
            Err(err) => {
                tracing::warn!(error = %err, "reading an account's role limits failed");
                self.of(Some(account))
            }
        }
    }

    /// Reads `account` again on its next request (its roles changed).
    pub fn forget(&self, account: Uuid) {
        self.accounts
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&account);
    }

    /// Reads every account again on its next request (a role's limits changed, or a role went).
    pub fn forget_all(&self) {
        self.accounts
            .entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// The rate `account` counts under for `limit`: its count, over the default's window.
    pub fn rate(&self, limit: AccountLimit, account: Option<Uuid>) -> Rate {
        let (_, window) = self.rates.default_of(limit);
        Rate {
            burst: self.of(account).get(limit),
            per: window.unwrap_or(Duration::from_secs(3600)),
        }
    }

    /// Takes one token of `limit` for `key` (an account, or an IP when `account` is `None`), at
    /// that account's rate. The upload counts aren't token buckets and always pass here.
    pub fn take(
        &self,
        limit: AccountLimit,
        key: &str,
        account: Option<Uuid>,
    ) -> Result<(), Duration> {
        match self.limiter(limit) {
            Some(limiter) => limiter.check_with(key, self.rate(limit, account)),
            None => Ok(()),
        }
    }
}

/// Counts failures per key; `max` of them inside `window` (from the first) lock the key until the
/// window ends. In memory, like the other limits.
pub struct Lockout {
    max: u32,
    window: Duration,
    entries: Mutex<HashMap<String, (u32, Instant)>>,
}

impl Lockout {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            entries: Mutex::default(),
        }
    }

    /// How long `key` is still locked out, if it is.
    pub fn locked(&self, key: &str) -> Option<Duration> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let (count, since) = entries.get(key)?;
        let left = self.window.checked_sub(since.elapsed())?;
        (*count >= self.max).then_some(left)
    }

    pub fn fail(&self, key: &str) {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() >= 10_000 {
            let window = self.window;
            entries.retain(|_, (_, since)| now.duration_since(*since) < window);
        }
        let entry = entries.entry(key.to_owned()).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }
        entry.0 += 1;
    }
}

/// Wrong redeem codes: 5 per account or 20 per IP within 15 minutes lock it out until the 15
/// minutes are over.
pub struct RedeemLockout {
    pub account: Lockout,
    pub ip: Lockout,
}

impl Default for RedeemLockout {
    fn default() -> Self {
        Self {
            account: Lockout::new(5, Duration::from_secs(15 * 60)),
            ip: Lockout::new(20, Duration::from_secs(15 * 60)),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(Rates::default(), false)
    }
}

/// The client's IP as the limits saw it, for limits applied inside GraphQL resolvers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientIp(pub String);

/// The key a per-person limit counts under: the account when signed in, else the client's IP.
fn person_key(caller: Option<&Caller>, ip: &str) -> String {
    match caller {
        Some(c) => format!("a:{}", ids::to_text(c.account_id)),
        None => format!("ip:{ip}"),
    }
}

/// Takes one token of the `heavy_reads` limit for a public read that fans out into many lookups.
pub fn heavy_read(state: &AppState, caller: Option<&Caller>, ip: &str) -> ApiResult<()> {
    state
        .limits
        .take(
            AccountLimit::HeavyReads,
            &person_key(caller, ip),
            caller.map(|c| c.account_id),
        )
        .map_err(|_| rate_limited("That's a lot of reads. Try again in a minute."))
}

/// Takes one token of the `account_search` limit.
pub fn account_search(state: &AppState, caller: Option<&Caller>, ip: &str) -> ApiResult<()> {
    state
        .limits
        .take(
            AccountLimit::AccountSearch,
            &person_key(caller, ip),
            caller.map(|c| c.account_id),
        )
        .map_err(|_| rate_limited("That's a lot of searches. Try again in a minute."))
}

/// Takes one token of the `quick_search` limit.
pub fn quick_search(state: &AppState, caller: Option<&Caller>, ip: &str) -> ApiResult<()> {
    state
        .limits
        .take(
            AccountLimit::QuickSearch,
            &person_key(caller, ip),
            caller.map(|c| c.account_id),
        )
        .map_err(|_| rate_limited("That's a lot of searches. Try again in a minute."))
}

/// The 429 answer.
pub fn too_many(wait: std::time::Duration) -> Response {
    let mut response = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        "Too many requests. Slow down and try again shortly.",
    )
    .into_response();
    let secs = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    if let Ok(v) = HeaderValue::from_str(&secs.max(1).to_string()) {
        response.headers_mut().insert(RETRY_AFTER, v);
    }
    response
}

/// Which per-IP limit a request falls under, if any (health checks have none).
fn class<'a>(limits: &'a Limits, method: &Method, path: &str) -> Option<&'a Limiter> {
    if path.starts_with("/v1/auth/") {
        return Some(&limits.auth);
    }
    match (method, path) {
        (&Method::POST, "/v1/gql") => Some(&limits.gql),
        (_, "/v1/gql/ws") => Some(&limits.socket),
        (&Method::POST, "/v1/emotes" | "/v1/cosmetics/images") => Some(&limits.upload),
        (&Method::POST, "/v1/personal-sets/lookup") => Some(&limits.reads),
        (&Method::GET | &Method::HEAD, p) if p.starts_with("/v1/") => Some(&limits.reads),
        _ => None,
    }
}

pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0);
    let ip = client_ip(request.headers(), peer, state.limits.trust_cloudflare);
    let mut request = request;
    request.extensions_mut().insert(ClientIp(ip.clone()));
    if let Some(limiter) = class(&state.limits, request.method(), request.uri().path())
        && let Err(wait) = limiter.check(&ip)
    {
        tracing::info!(%ip, path = request.uri().path(), "rate limited");
        return too_many(wait);
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_fall_into_their_classes() {
        let l = Limits::default();
        let same = |a: Option<&Limiter>, b: &Limiter| a.is_some_and(|a| std::ptr::eq(a, b));
        assert!(same(
            class(&l, &Method::GET, "/v1/auth/twitch/login"),
            &l.auth
        ));
        assert!(same(class(&l, &Method::POST, "/v1/auth/logout"), &l.auth));
        assert!(same(class(&l, &Method::POST, "/v1/gql"), &l.gql));
        assert!(same(class(&l, &Method::GET, "/v1/gql/ws"), &l.socket));
        assert!(same(class(&l, &Method::POST, "/v1/emotes"), &l.upload));
        assert!(same(
            class(&l, &Method::POST, "/v1/cosmetics/images"),
            &l.upload
        ));
        assert!(same(
            class(&l, &Method::GET, "/v1/cosmetics/users"),
            &l.reads
        ));
        assert!(same(class(&l, &Method::GET, "/v1/sets/global"), &l.reads));
        assert!(same(
            class(&l, &Method::POST, "/v1/personal-sets/lookup"),
            &l.reads
        ));
        assert!(class(&l, &Method::GET, "/health").is_none());
    }

    #[test]
    fn lockouts_hold_for_the_window() {
        let l = Lockout::new(3, Duration::from_millis(200));
        for _ in 0..2 {
            l.fail("a");
        }
        assert!(l.locked("a").is_none());
        l.fail("a");
        assert!(l.locked("a").is_some());
        assert!(l.locked("b").is_none());
        std::thread::sleep(Duration::from_millis(220));
        assert!(l.locked("a").is_none());
        // A new window starts over.
        l.fail("a");
        assert!(l.locked("a").is_none());
    }

    #[test]
    fn rates_come_from_the_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| (*v).to_owned())
            }
        };
        // Nothing set: today's values.
        let none = Rates::from_env(|_| None).unwrap();
        let d = Rates::default();
        assert_eq!(none.gql_account, d.gql_account);
        assert_eq!(none.uploads_per_hour, 20);
        assert_eq!(none.pending_emotes, 20);

        let r = Rates::from_env(env(&[
            ("CYEX_TV_LIMIT_GQL_ACCOUNT", "900/1m"),
            ("CYEX_TV_LIMIT_DATA_EXPORT", "2/1h"),
            ("CYEX_TV_LIMIT_AUTH", " 10/30s "),
            ("CYEX_TV_LIMIT_UPLOADS_PER_HOUR", "40"),
            ("CYEX_TV_LIMIT_PENDING_EMOTES", "5"),
            ("CYEX_TV_LIMIT_READS", ""),
        ]))
        .unwrap();
        assert_eq!(r.gql_account, Rate::per_minute(900));
        assert_eq!(r.data_export, Rate::per_hour(2));
        assert_eq!(r.auth.burst, 10);
        assert_eq!(r.auth.per, Duration::from_secs(30));
        assert_eq!(r.uploads_per_hour, 40);
        assert_eq!(r.pending_emotes, 5);
        // Empty keeps the default; the others are untouched.
        assert_eq!(r.reads, d.reads);
        assert_eq!(r.prefs, d.prefs);

        // Every rate has its variable.
        let mut names = Vec::new();
        for (field, _) in Rates::default().rates_mut() {
            names.push(field);
        }
        assert_eq!(names.len(), 19);
        assert!(names.contains(&"delete_account"));

        for (name, value) in [
            ("CYEX_TV_LIMIT_GQL", "fast"),
            ("CYEX_TV_LIMIT_PREFS", "0/1h"),
            ("CYEX_TV_LIMIT_PENDING_EMOTES", "0"),
            ("CYEX_TV_LIMIT_UPLOADS_PER_HOUR", "20/1h"),
            ("CYEX_TV_LIMIT_PENDING_EMOTES", "-3"),
        ] {
            let err = Rates::from_env(|n| (n == name).then(|| value.to_owned())).unwrap_err();
            assert!(err.starts_with(name), "{err}");
        }
    }

    fn overrides(pairs: &[(AccountLimit, u32)]) -> RoleOverrides {
        pairs.iter().copied().collect()
    }

    #[test]
    fn an_account_gets_its_most_generous_role() {
        let rates = Rates::default();
        // No roles with overrides: the defaults.
        let plain = AccountLimits::resolve(&rates, &[]);
        assert_eq!(plain, AccountLimits::defaults(&rates));
        assert_eq!(plain.get(AccountLimit::PendingEmotes), 20);
        assert_eq!(plain.get(AccountLimit::GqlAccount), 600);

        let partner = overrides(&[
            (AccountLimit::PendingEmotes, 100),
            (AccountLimit::GqlAccount, 1200),
        ]);
        let trial = overrides(&[(AccountLimit::PendingEmotes, 5)]);
        let both = AccountLimits::resolve(&rates, &[trial.clone(), partner.clone()]);
        assert_eq!(both.get(AccountLimit::PendingEmotes), 100);
        assert_eq!(both.get(AccountLimit::GqlAccount), 1200);
        // What no role sets stays the default.
        assert_eq!(both.get(AccountLimit::Prefs), rates.prefs.burst);
        // A role alone can lower a limit.
        let lowered = AccountLimits::resolve(&rates, &[trial]);
        assert_eq!(lowered.get(AccountLimit::PendingEmotes), 5);
        // Defaults follow the environment.
        let custom = Rates {
            pending_emotes: 7,
            ..Rates::default()
        };
        assert_eq!(
            AccountLimits::resolve(&custom, &[]).get(AccountLimit::PendingEmotes),
            7
        );
    }

    #[test]
    fn stored_overrides_read_back_and_skip_what_isnt_a_limit() {
        let o = overrides(&[(AccountLimit::Redeem, 3), (AccountLimit::Editors, 500)]);
        let json = overrides_to_json(&o);
        assert_eq!(json, serde_json::json!({ "redeem": 3, "editors": 500 }));
        assert_eq!(overrides_from_json(&json), o);
        let messy = serde_json::json!({
            "redeem": 3, "nope": 5, "prefs": 0, "checkout": -1, "editors": "9", "gql_account": 2_000_000
        });
        assert_eq!(
            overrides_from_json(&messy),
            overrides(&[(AccountLimit::Redeem, 3)])
        );
        assert!(overrides_from_json(&serde_json::json!([])).is_empty());
        for l in AccountLimit::ALL {
            assert_eq!(AccountLimit::from_key(l.key()), Some(l));
        }
        assert_eq!(AccountLimit::ALL.len(), 15);
    }

    #[test]
    fn an_account_counts_under_its_own_rate() {
        let l = Limits::default();
        let raised = Uuid::from_u128(1);
        l.accounts.put(
            raised,
            Arc::new(AccountLimits::resolve(
                &l.rates,
                &[overrides(&[(AccountLimit::DataExport, 7)])],
            )),
        );
        assert_eq!(
            l.rate(AccountLimit::DataExport, Some(raised)),
            Rate::per_hour(7)
        );
        assert_eq!(
            l.rate(AccountLimit::DataExport, Some(Uuid::from_u128(2))),
            Rate::per_hour(5)
        );
        assert_eq!(l.rate(AccountLimit::DataExport, None), Rate::per_hour(5));
        for _ in 0..7 {
            assert!(l.take(AccountLimit::DataExport, "a", Some(raised)).is_ok());
        }
        assert!(l.take(AccountLimit::DataExport, "a", Some(raised)).is_err());
        for _ in 0..5 {
            assert!(l.take(AccountLimit::DataExport, "b", None).is_ok());
        }
        assert!(l.take(AccountLimit::DataExport, "b", None).is_err());
        // The upload counts aren't buckets.
        assert!(l.take(AccountLimit::PendingEmotes, "a", None).is_ok());
        // Forgotten: back to the defaults until read again.
        l.forget(raised);
        assert_eq!(
            l.rate(AccountLimit::DataExport, Some(raised)),
            Rate::per_hour(5)
        );
        l.accounts.put(raised, l.defaults.clone());
        l.forget_all();
        assert!(l.accounts.get(raised).is_none());
    }

    #[test]
    fn the_answer_says_when_to_retry() {
        let r = too_many(std::time::Duration::from_millis(1500));
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(r.headers()[RETRY_AFTER], "2");
    }
}
