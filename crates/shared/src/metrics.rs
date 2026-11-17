//! Prometheus metrics in the text format, on their own listener (PLAN.md Phase 9).
//!
//! Each service serves `GET /metrics` on `CYEX_TV_METRICS_BIND` when it is set, and nowhere when
//! it isn't. That address is meant for the private Docker network only, never a tunnel route.
//! Counters and histograms live in a [`Registry`]; gauges that cost a query are read at scrape
//! time through the `extra` callback given to [`serve`].
//!
//! Written here instead of pulling in a metrics crate: what's needed is small.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::config::ConfigError;

/// The env var naming the metrics listener, e.g. `0.0.0.0:9100`.
pub const BIND_VAR: &str = "CYEX_TV_METRICS_BIND";

/// Upper bounds of the request-duration buckets, in seconds.
pub const BUCKETS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Labels, in the order given. Values are escaped when written.
pub type Labels = Vec<(&'static str, String)>;

#[derive(Default)]
struct Histogram {
    buckets: [u64; BUCKETS.len()],
    count: u64,
    sum: f64,
}

#[derive(Default)]
struct Inner {
    help: BTreeMap<&'static str, (&'static str, &'static str)>,
    counters: BTreeMap<(&'static str, String), u64>,
    histograms: BTreeMap<(&'static str, String), Histogram>,
}

/// Counters and histograms for one process. Cheap to clone.
#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<Mutex<Inner>>,
}

fn lock(inner: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    // A panic while holding the lock leaves plain numbers behind; keep counting.
    inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `by` to a counter.
    pub fn add(&self, name: &'static str, help: &'static str, labels: &Labels, by: u64) {
        let mut inner = lock(&self.inner);
        inner.help.entry(name).or_insert((help, "counter"));
        *inner
            .counters
            .entry((name, label_text(labels)))
            .or_default() += by;
    }

    pub fn inc(&self, name: &'static str, help: &'static str, labels: &Labels) {
        self.add(name, help, labels, 1);
    }

    /// Records one observation, in seconds.
    pub fn observe(&self, name: &'static str, help: &'static str, labels: &Labels, secs: f64) {
        let mut inner = lock(&self.inner);
        inner.help.entry(name).or_insert((help, "histogram"));
        let h = inner
            .histograms
            .entry((name, label_text(labels)))
            .or_default();
        for (slot, bound) in h.buckets.iter_mut().zip(BUCKETS) {
            if secs <= bound {
                *slot += 1;
            }
        }
        h.count += 1;
        h.sum += secs;
    }

    /// The text exposition format, version 0.0.4.
    pub fn render(&self) -> String {
        let inner = lock(&self.inner);
        let mut out = String::new();
        for (&name, &(help, kind)) in &inner.help {
            let _ = writeln!(out, "# HELP {name} {help}");
            let _ = writeln!(out, "# TYPE {name} {kind}");
            for ((_, labels), value) in inner
                .counters
                .range((name, String::new())..)
                .take_while(|((n, _), _)| *n == name)
            {
                let _ = writeln!(out, "{name}{} {value}", braces(labels));
            }
            for ((n, labels), h) in &inner.histograms {
                if *n != name {
                    continue;
                }
                let sep = if labels.is_empty() { "" } else { "," };
                for (count, bound) in h.buckets.iter().zip(BUCKETS) {
                    let _ = writeln!(out, "{name}_bucket{{{labels}{sep}le=\"{bound}\"}} {count}");
                }
                let _ = writeln!(out, "{name}_bucket{{{labels}{sep}le=\"+Inf\"}} {}", h.count);
                let _ = writeln!(out, "{name}_sum{} {}", braces(labels), h.sum);
                let _ = writeln!(out, "{name}_count{} {}", braces(labels), h.count);
            }
        }
        out
    }
}

/// Writes a gauge (help, type and one sample per label set) for values read at scrape time.
pub fn gauge(out: &mut String, name: &str, help: &str, samples: &[(Labels, f64)]) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} gauge");
    for (labels, value) in samples {
        let _ = writeln!(out, "{name}{} {value}", braces(&label_text(labels)));
    }
}

fn braces(labels: &str) -> String {
    if labels.is_empty() {
        String::new()
    } else {
        format!("{{{labels}}}")
    }
}

fn label_text(labels: &Labels) -> String {
    let mut out = String::new();
    for (i, (key, value)) in labels.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "{key}=\"{}\"", escape(value));
    }
    out
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Middleware counting requests and their duration per route. The route is the matched pattern
/// (`/v1/emotes/{id}`), never the raw path, so IDs don't create new series.
pub async fn track(State(registry): State<Registry>, req: Request, next: Next) -> Response {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map_or_else(|| "unmatched".to_owned(), |p| p.as_str().to_owned());
    let method = req.method().as_str().to_owned();
    let started = Instant::now();
    let response = next.run(req).await;
    let labels: Labels = vec![
        ("method", method),
        ("route", route),
        ("status", response.status().as_u16().to_string()),
    ];
    registry.inc(
        "cyex_tv_http_requests_total",
        "HTTP requests answered, by method, route and status.",
        &labels,
    );
    registry.observe(
        "cyex_tv_http_request_duration_seconds",
        "Time to the response head, by method, route and status.",
        &labels,
        started.elapsed().as_secs_f64(),
    );
    response
}

/// Scrape-time gauges: appends to the text after the registry's own lines.
pub type Extra = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

#[derive(Clone)]
struct Served {
    registry: Registry,
    extra: Option<Extra>,
}

/// The `/metrics` router, for tests and for [`serve`].
pub fn router(registry: Registry, extra: Option<Extra>) -> Router {
    Router::new()
        .route("/metrics", get(scrape))
        .with_state(Served { registry, extra })
}

async fn scrape(State(served): State<Served>) -> Response {
    let mut body = served.registry.render();
    if let Some(extra) = &served.extra {
        body.push_str(&extra().await);
    }
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
}

/// Reads `CYEX_TV_METRICS_BIND`. `None`: metrics are off.
pub fn bind_from(get: impl Fn(&str) -> Option<String>) -> Result<Option<SocketAddr>, ConfigError> {
    match get(BIND_VAR).filter(|v| !v.trim().is_empty()) {
        None => Ok(None),
        Some(v) => v
            .trim()
            .parse()
            .map(Some)
            .map_err(|e: std::net::AddrParseError| ConfigError::Invalid {
                name: BIND_VAR,
                reason: e.to_string(),
            }),
    }
}

/// Serves `/metrics` on `addr` until `shutdown` resolves.
pub async fn serve(
    addr: SocketAddr,
    registry: Registry,
    extra: Option<Extra>,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "metrics listening");
    axum::serve(listener, router(registry, extra))
        .with_graceful_shutdown(shutdown)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_and_histograms_render() {
        let r = Registry::new();
        let labels: Labels = vec![("route", "/a\"b".to_owned())];
        r.inc("x_total", "Xs.", &labels);
        r.add("x_total", "Xs.", &labels, 2);
        r.inc("x_total", "Xs.", &vec![("route", "/c".to_owned())]);
        r.observe("d_seconds", "Ds.", &vec![], 0.03);
        r.observe("d_seconds", "Ds.", &vec![], 20.0);
        let text = r.render();
        assert!(text.contains("# TYPE x_total counter\n"), "{text}");
        assert!(text.contains("x_total{route=\"/a\\\"b\"} 3\n"), "{text}");
        assert!(text.contains("x_total{route=\"/c\"} 1\n"), "{text}");
        assert!(text.contains("# TYPE d_seconds histogram\n"), "{text}");
        assert!(
            text.contains("d_seconds_bucket{le=\"0.025\"} 0\n"),
            "{text}"
        );
        assert!(text.contains("d_seconds_bucket{le=\"0.05\"} 1\n"), "{text}");
        assert!(text.contains("d_seconds_bucket{le=\"10\"} 1\n"), "{text}");
        assert!(text.contains("d_seconds_bucket{le=\"+Inf\"} 2\n"), "{text}");
        assert!(text.contains("d_seconds_count 2\n"), "{text}");
    }

    #[test]
    fn counters_of_one_name_stay_together() {
        let r = Registry::new();
        r.inc("a_total", "A.", &vec![("k", "1".to_owned())]);
        r.inc("b_total", "B.", &vec![]);
        r.inc("a_total", "A.", &vec![("k", "2".to_owned())]);
        let text = r.render();
        let a = text.find("# HELP a_total").unwrap();
        let b = text.find("# HELP b_total").unwrap();
        let k2 = text.find("a_total{k=\"2\"}").unwrap();
        assert!(a < k2 && k2 < b, "{text}");
        assert!(!text[b..].contains("a_total{"), "{text}");
    }

    #[test]
    fn gauges_render() {
        let mut out = String::new();
        gauge(
            &mut out,
            "g",
            "G.",
            &[(vec![("kind", "x".to_owned())], 4.0), (vec![], 1.5)],
        );
        assert_eq!(out, "# HELP g G.\n# TYPE g gauge\ng{kind=\"x\"} 4\ng 1.5\n");
    }

    #[test]
    fn the_bind_is_optional() {
        assert_eq!(bind_from(|_| None).unwrap(), None);
        assert_eq!(bind_from(|_| Some(" ".into())).unwrap(), None);
        assert_eq!(
            bind_from(|_| Some("0.0.0.0:9100".into())).unwrap(),
            Some(SocketAddr::from(([0, 0, 0, 0], 9100)))
        );
        assert!(bind_from(|_| Some("nope".into())).is_err());
    }
}
