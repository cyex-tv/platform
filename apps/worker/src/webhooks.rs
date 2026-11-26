//! `webhook.deliver`: sends one webhook delivery (ADR 0034).
//!
//! - The body is JSON: the delivery's and webhook's IDs, the channel, and the event as it was
//!   when it happened. `X-Cyex-Signature: t=<unix seconds>,v1=<hex HMAC-SHA256 of "<t>.<body>">`
//!   signs it with the webhook's secret; `X-Cyex-Delivery` and `X-Cyex-Event` name it.
//! - SSRF guard: https only; the host is resolved here and every address it gives must be public
//!   (`cyex_tv_shared::webhooks::is_public_ip`), else nothing is sent. The request goes to those
//!   checked addresses only (no second lookup), through no proxy, and redirects are never
//!   followed (a 3xx is a failure). 5 s to connect, 10 s in all, and at most 64 KB of the answer
//!   is read.
//! - A 2xx is delivered. Anything else is tried again with the queue's backoff, 8 attempts in
//!   all; a delivery that fails them all counts against the webhook, and 5 in a row turn it off
//!   and tell the channel's owner (and whoever made it) with a `webhook.disabled` notification.
//!   A test delivery is sent once and never counts.

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use cyex_tv_db::jobs::Job;
use cyex_tv_db::webhooks::{self as db, Attempt};
use cyex_tv_db::{accounts, channels, ids, notifications};
use cyex_tv_shared::webhooks::{
    PORT, SIGNATURE_HEADER, TEST_EVENT, is_public_ip, redacted, signature_all,
};
use serde_json::json;

use crate::{Ctx, JobError};

pub const KIND: &str = db::DELIVER_KIND;
/// How long connecting may take.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How long one attempt may take in all.
pub const TIMEOUT: Duration = Duration::from_secs(10);
/// The most of an answer that is read.
pub const MAX_RESPONSE: usize = 64 * 1024;
/// How long resolving the host may take.
pub const DNS_TIMEOUT: Duration = Duration::from_secs(3);
/// Deliveries sent at once by one worker (`webhook_lanes`): their own lanes, so a slow receiver
/// never holds up the queue's other jobs.
pub const LANES: usize = 4;

/// Which addresses a delivery may reach.
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    allow: fn(IpAddr) -> bool,
    /// Sends an `https://` URL as `http://`. For tests against a local server only.
    plain_http: bool,
    /// Any port, not only 443. For tests against a local server only.
    any_port: bool,
}

impl Default for Policy {
    /// Public addresses only, over https.
    fn default() -> Self {
        Self {
            allow: is_public_ip,
            plain_http: false,
            any_port: false,
        }
    }
}

impl Policy {
    /// For tests: loopback is allowed and the request goes out as plain http, so a local test
    /// server can stand in for a receiver. Never used by the worker itself.
    /// For tests: this policy, but any port, so the address check is what a local test server
    /// on a random port meets. Never used by the worker itself.
    #[must_use]
    pub fn any_port_for_tests(mut self) -> Self {
        self.any_port = true;
        self
    }

    pub fn loopback_for_tests() -> Self {
        Self {
            allow: |ip| ip.is_loopback() || is_public_ip(ip),
            plain_http: true,
            any_port: true,
        }
    }
}

/// What sending once came to.
#[derive(Debug, PartialEq, Eq)]
pub struct Sent {
    pub status: Option<u16>,
    pub error: Option<String>,
}

impl Sent {
    fn failed(error: impl Into<String>) -> Self {
        Self {
            status: None,
            error: Some(error.into()),
        }
    }

    pub fn ok(&self) -> bool {
        self.error.is_none() && self.status.is_some_and(|s| (200..300).contains(&s))
    }
}

/// The addresses `host` may be reached at, all of them allowed by `policy`; an error when it
/// doesn't resolve or any address it gives is refused (a name with one private address among
/// public ones is refused whole, so it can't be raced to the private one).
pub async fn checked_addrs(
    host: &str,
    port: u16,
    policy: &Policy,
) -> Result<Vec<SocketAddr>, String> {
    let literal = host.trim_start_matches('[').trim_end_matches(']');
    let addrs: Vec<SocketAddr> = match literal.parse::<IpAddr>() {
        Ok(ip) => vec![SocketAddr::new(ip, port)],
        Err(_) => tokio::time::timeout(DNS_TIMEOUT, tokio::net::lookup_host((host, port)))
            .await
            .map_err(|_| "resolving the host name timed out".to_owned())?
            .map_err(|_| "the host name doesn't resolve".to_owned())?
            .collect(),
    };
    if addrs.is_empty() {
        return Err("the host name doesn't resolve".to_owned());
    }
    // An IPv4-mapped IPv6 address is judged as the IPv4 address it is.
    if let Some(bad) = addrs.iter().find(|a| !(policy.allow)(a.ip())) {
        return Err(format!(
            "the host resolves to an address that isn't allowed ({})",
            bad.ip()
        ));
    }
    Ok(addrs)
}

/// Sends `body` to `url` once, signed with `secret`.
pub async fn send(
    url: &str,
    body: &[u8],
    secrets: &[&str],
    delivery_id: &str,
    event_type: &str,
    policy: &Policy,
) -> Sent {
    let Ok(mut url) = url::Url::parse(url) else {
        return Sent::failed("the URL doesn't parse");
    };
    if url.scheme() != "https" {
        return Sent::failed("only https URLs are delivered");
    }
    let Some(host) = url.host_str().map(str::to_owned) else {
        return Sent::failed("the URL has no host");
    };
    let port = url.port_or_known_default().unwrap_or(PORT);
    if port != PORT && !policy.any_port {
        return Sent::failed("only port 443 is delivered to");
    }
    let addrs = match checked_addrs(&host, port, policy).await {
        Ok(addrs) => addrs,
        Err(err) => return Sent::failed(err),
    };
    if policy.plain_http {
        let _ = url.set_scheme("http");
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TIMEOUT)
        .https_only(!policy.plain_http)
        .user_agent("cyex.tv-webhooks/1");
    // Pin the checked addresses: the client must not look the name up again (DNS rebinding).
    if host.parse::<IpAddr>().is_err() && !host.starts_with('[') {
        builder = builder.resolve_to_addrs(&host, &addrs);
    }
    let client = match builder.build() {
        Ok(c) => c,
        Err(err) => return Sent::failed(format!("couldn't set up the request: {err}")),
    };
    let timestamp = chrono::Utc::now().timestamp();
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .header(SIGNATURE_HEADER, signature_all(secrets, timestamp, body))
        .header("X-Cyex-Delivery", delivery_id)
        .header("X-Cyex-Event", event_type)
        .body(body.to_vec())
        .send()
        .await;
    let mut response = match response {
        Ok(r) => r,
        Err(err) if err.is_timeout() => return Sent::failed("timed out"),
        Err(err) if err.is_connect() => return Sent::failed("couldn't connect"),
        Err(_) => return Sent::failed("the request failed"),
    };
    let status = response.status().as_u16();
    // Read a little of the answer, so the connection closes cleanly, and no more.
    let mut read = 0usize;
    while read < MAX_RESPONSE {
        match response.chunk().await {
            Ok(Some(chunk)) => read += chunk.len(),
            _ => break,
        }
    }
    let error = match status {
        200..=299 => None,
        300..=399 => Some(format!("answered {status}; redirects are not followed")),
        _ => Some(format!("answered {status}")),
    };
    Sent {
        status: Some(status),
        error,
    }
}

/// Runs a `webhook.deliver` job.
pub async fn run(ctx: Ctx, job: Job) -> Result<(), JobError> {
    deliver_with(ctx, job, Policy::default()).await
}

fn db_failure(err: impl std::fmt::Display) -> JobError {
    JobError::Retry(err.to_string())
}

/// Runs a `webhook.deliver` job under `policy`.
pub async fn deliver_with(ctx: Ctx, job: Job, policy: Policy) -> Result<(), JobError> {
    let Some(id) = job.payload.get("delivery_id").and_then(|v| v.as_i64()) else {
        return Err(JobError::Permanent("no delivery_id".into()));
    };
    let mut conn = ctx.db.acquire().await.map_err(db_failure)?;
    // Gone with its webhook, or finished already: nothing to do.
    let Some(delivery) = db::delivery(&mut conn, id).await.map_err(db_failure)? else {
        return Ok(());
    };
    if delivery.state != "pending" {
        return Ok(());
    }
    let Some(hook) = db::get(&mut conn, delivery.webhook_id)
        .await
        .map_err(db_failure)?
    else {
        return Ok(());
    };
    let test = delivery.event_type == TEST_EVENT;
    if !hook.enabled && !test {
        db::record_attempt(
            &mut conn,
            id,
            &Attempt {
                attempts: delivery.attempts,
                response_status: None,
                error: Some("the webhook is off"),
                duration_ms: 0,
                state: "failed",
            },
        )
        .await
        .map_err(db_failure)?;
        return Ok(());
    }
    let Some(channel) = channels::get(&mut conn, hook.channel_id)
        .await
        .map_err(db_failure)?
    else {
        return Ok(());
    };
    // A blocked or deleted account's channels send nothing (the block turns the webhooks off
    // too; this covers deliveries already queued).
    let live = accounts::get(&mut conn, channel.account_id)
        .await
        .map_err(db_failure)?
        .is_some_and(|a| {
            a.deleted_at.is_none()
                && a.blocked_at
                    .is_none_or(|_| a.blocked_until.is_some_and(|u| u <= chrono::Utc::now()))
        });
    if !live {
        db::record_attempt(
            &mut conn,
            id,
            &Attempt {
                attempts: delivery.attempts,
                response_status: None,
                error: Some("the channel's account is blocked or deleted"),
                duration_ms: 0,
                state: "failed",
            },
        )
        .await
        .map_err(db_failure)?;
        return Ok(());
    }
    drop(conn);

    let delivery_id = delivery.id.to_string();
    let body = json!({
        "delivery_id": delivery_id,
        "webhook_id": ids::to_text(hook.id),
        "channel": {
            "platform": channel.platform,
            "id": channel.platform_channel_id,
            "login": channel.login,
        },
        "event": delivery.body,
    });
    let bytes = serde_json::to_vec(&body).map_err(|e| JobError::Permanent(e.to_string()))?;
    let started = Instant::now();
    let sent = send(
        &hook.url,
        &bytes,
        &hook.signing_secrets(chrono::Utc::now()),
        &delivery_id,
        &delivery.event_type,
        &policy,
    )
    .await;
    let ms = i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX);
    let last = test || job.attempts >= job.max_attempts;
    let state = if sent.ok() {
        "delivered"
    } else if last {
        "failed"
    } else {
        "pending"
    };

    let mut tx = ctx.db.begin().await.map_err(db_failure)?;
    db::record_attempt(
        &mut tx,
        id,
        &Attempt {
            attempts: job.attempts,
            response_status: sent.status.map(i32::from),
            error: sent.error.as_deref(),
            duration_ms: ms,
            state,
        },
    )
    .await
    .map_err(db_failure)?;
    let finished = sent.ok() || last;
    if finished
        && !test
        && db::record_outcome(&mut tx, hook.id, sent.ok())
            .await
            .map_err(db_failure)?
            .notify
    {
        // The URL may hold a token: the notification names its host only. Once a day at most.
        let payload = json!({
            "webhook_id": ids::to_text(hook.id),
            "platform": channel.platform,
            "channel_id": channel.platform_channel_id,
            "login": channel.login,
            "url": redacted(&hook.url),
        });
        notifications::create(&mut tx, channel.account_id, "webhook.disabled", &payload)
            .await
            .map_err(db_failure)?;
        if let Some(by) = hook.created_by.filter(|by| *by != channel.account_id) {
            notifications::create(&mut tx, by, "webhook.disabled", &payload)
                .await
                .map_err(db_failure)?;
        }
        tracing::info!(webhook = %ids::to_text(hook.id), "webhook turned off after failures");
    }
    if state != "pending" {
        db::prune(&mut tx, hook.id).await.map_err(db_failure)?;
    }
    tx.commit().await.map_err(db_failure)?;
    match (state, sent.error) {
        ("pending", Some(err)) => Err(JobError::Retry(err)),
        ("pending", None) => Err(JobError::Retry("not delivered".into())),
        // Delivered, or failed for good: the delivery log has it, the job is done.
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn private_hosts_are_refused_before_connecting() {
        let p = Policy::default();
        for (host, port) in [
            ("127.0.0.1", 443),
            ("[::1]", 443),
            ("::ffff:127.0.0.1", 443),
            ("169.254.169.254", 80),
            ("10.1.2.3", 443),
            ("[fd00:ec2::254]", 443),
            ("localhost", 443),
        ] {
            let err = checked_addrs(host, port, &p).await.unwrap_err();
            assert!(
                err.contains("isn't allowed") || err.contains("doesn't resolve"),
                "{host}: {err}"
            );
        }
        assert!(checked_addrs("1.1.1.1", 443, &p).await.is_ok());
        // The test policy lets loopback through, and nothing private.
        let t = Policy::loopback_for_tests();
        assert!(checked_addrs("127.0.0.1", 8080, &t).await.is_ok());
        assert!(checked_addrs("10.0.0.1", 8080, &t).await.is_err());
    }

    #[tokio::test]
    async fn only_https_goes_out() {
        let sent = send(
            "http://example.com/",
            b"{}",
            &["s"],
            "1",
            "webhook.test",
            &Policy::default(),
        )
        .await;
        assert_eq!(sent.error.as_deref(), Some("only https URLs are delivered"));
        let sent = send(
            "https://127.0.0.1/",
            b"{}",
            &["s"],
            "1",
            "webhook.test",
            &Policy::default(),
        )
        .await;
        assert!(sent.error.unwrap().contains("isn't allowed"));
        let sent = send(
            "https://1.1.1.1:8443/",
            b"{}",
            &["s"],
            "1",
            "webhook.test",
            &Policy::default(),
        )
        .await;
        assert_eq!(sent.error.as_deref(), Some("only port 443 is delivered to"));
        assert!(
            !Sent {
                status: Some(301),
                error: None
            }
            .ok()
        );
        assert!(
            Sent {
                status: Some(204),
                error: None
            }
            .ok()
        );
    }
}
