//! cyex.tv's own Paddle Billing account (ADR 0016): its settings, the two API calls the shop makes
//! (create a transaction for a checkout, refund one), and the webhook signature check.
//!
//! Separate from cyex.app's Paddle setup: other keys, other products, other webhook. No card data
//! ever reaches cyex.tv; the buyer pays in Paddle's checkout.

use std::fmt;
use std::time::Duration;

use cyex_tv_shared::config::ConfigError;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Sha256;

/// How far a webhook's `ts` may be from now, either way.
pub const WEBHOOK_TOLERANCE_SECS: i64 = 300;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaddleEnv {
    Sandbox,
    Production,
}

impl PaddleEnv {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::Production => "production",
        }
    }

    pub fn api_url(self) -> &'static str {
        match self {
            Self::Sandbox => "https://sandbox-api.paddle.com",
            Self::Production => "https://api.paddle.com",
        }
    }
}

/// `CYEX_TV_PADDLE_*`. Every value is optional: without the API key checkout answers
/// `checkout_unavailable`, without the webhook secret the webhook answers 503.
#[derive(Clone)]
pub struct PaddleConfig {
    pub env: PaddleEnv,
    pub api_key: Option<String>,
    pub webhook_secret: Option<String>,
    /// Paddle.js client-side token, handed to the website with each checkout.
    pub client_token: Option<String>,
    /// The API's origin; `env`'s unless a test points it at a fake server.
    pub api_url: String,
}

impl fmt::Debug for PaddleConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let set = |v: &Option<String>| if v.is_some() { "<set>" } else { "<unset>" };
        f.debug_struct("PaddleConfig")
            .field("env", &self.env)
            .field("api_key", &set(&self.api_key))
            .field("webhook_secret", &set(&self.webhook_secret))
            .field("client_token", &set(&self.client_token))
            .field("api_url", &self.api_url)
            .finish()
    }
}

impl PaddleConfig {
    pub fn from_source(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let value = |name: &str| {
            get(name)
                .map(|v| v.trim().to_owned())
                .filter(|v| !v.is_empty())
        };
        let env = match value("CYEX_TV_PADDLE_ENV").as_deref() {
            None | Some("sandbox") => PaddleEnv::Sandbox,
            Some("production") => PaddleEnv::Production,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "CYEX_TV_PADDLE_ENV",
                    reason: format!("{other:?} is not sandbox or production"),
                });
            }
        };
        let api_key = value("CYEX_TV_PADDLE_API_KEY");
        let client_token = value("CYEX_TV_PADDLE_CLIENT_TOKEN");
        // Keys say which environment they are for; a mix-up would charge real money in a test,
        // or fail every checkout in production.
        let (key_wrong, token_wrong) = match env {
            PaddleEnv::Sandbox => ("pdl_live_", "live_"),
            PaddleEnv::Production => ("pdl_sdbx_", "test_"),
        };
        if api_key.as_deref().is_some_and(|k| k.starts_with(key_wrong)) {
            return Err(ConfigError::Invalid {
                name: "CYEX_TV_PADDLE_API_KEY",
                reason: format!("is not a {} key", env.as_str()),
            });
        }
        if client_token
            .as_deref()
            .is_some_and(|t| t.starts_with(token_wrong))
        {
            return Err(ConfigError::Invalid {
                name: "CYEX_TV_PADDLE_CLIENT_TOKEN",
                reason: format!("is not a {} token", env.as_str()),
            });
        }
        Ok(Self {
            env,
            api_key,
            webhook_secret: value("CYEX_TV_PADDLE_WEBHOOK_SECRET"),
            client_token,
            api_url: env.api_url().to_owned(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PaddleError {
    #[error("paddle isn't configured")]
    NotConfigured,
    #[error("paddle request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("paddle answered {status}: {code}")]
    Api { status: u16, code: String },
    #[error("paddle's answer couldn't be read")]
    Shape,
}

/// A transaction made for a checkout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub checkout_url: Option<String>,
    /// The currency Paddle gave it, checked again when it completes.
    pub currency_code: Option<String>,
}

pub struct Paddle {
    pub cfg: PaddleConfig,
    http: reqwest::Client,
}

impl Paddle {
    pub fn new(cfg: PaddleConfig) -> Result<Self, reqwest::Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("cyex.tv (+https://cyex.tv)")
            .build()?;
        Ok(Self { cfg, http })
    }

    pub fn checkout_ready(&self) -> bool {
        self.cfg.api_key.is_some()
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, PaddleError> {
        let key = self
            .cfg
            .api_key
            .as_deref()
            .ok_or(PaddleError::NotConfigured)?;
        let response = self
            .http
            .post(format!("{}{path}", self.cfg.api_url))
            .bearer_auth(key)
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let value: Value = response.json().await.map_err(|_| PaddleError::Shape)?;
        if !status.is_success() {
            let code = value["error"]["code"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned();
            return Err(PaddleError::Api {
                status: status.as_u16(),
                code,
            });
        }
        Ok(value)
    }

    /// `POST /transactions` for one of `price_id`, tagged with our order ID. Paddle's checkout
    /// (Paddle.js overlay or the hosted link) takes the payment.
    pub async fn create_transaction(
        &self,
        price_id: &str,
        order_id: &str,
    ) -> Result<Transaction, PaddleError> {
        let value = self
            .post(
                "/transactions",
                json!({
                    "items": [{ "price_id": price_id, "quantity": 1 }],
                    "custom_data": { "cyex_tv_order_id": order_id },
                }),
            )
            .await?;
        let data = &value["data"];
        let id = data["id"].as_str().ok_or(PaddleError::Shape)?.to_owned();
        Ok(Transaction {
            id,
            checkout_url: data["checkout"]["url"].as_str().map(str::to_owned),
            currency_code: data["currency_code"].as_str().map(str::to_owned),
        })
    }

    /// `POST /adjustments`: a full refund of a completed transaction. Paddle may need to approve
    /// it; its answer comes as `adjustment.*` webhooks. Returns the adjustment ID.
    pub async fn refund(&self, transaction_id: &str, reason: &str) -> Result<String, PaddleError> {
        let value = self
            .post(
                "/adjustments",
                json!({
                    "action": "refund",
                    "type": "full",
                    "transaction_id": transaction_id,
                    "reason": reason,
                }),
            )
            .await?;
        value["data"]["id"]
            .as_str()
            .map(str::to_owned)
            .ok_or(PaddleError::Shape)
    }
}

// Webhooks

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureError {
    /// No header, or not `ts=…;h1=…`.
    Malformed,
    /// `ts` too far from now.
    Stale,
    /// No `h1` matches.
    Mismatch,
}

/// Checks `Paddle-Signature: ts=<unix>;h1=<hex>[;h1=<hex>]` against the raw body: HMAC-SHA256 of
/// `"<ts>:<body>"` with the endpoint's secret, compared in constant time. Several `h1` appear
/// while a secret is rotated; one matching is enough.
pub fn verify_signature(
    secret: &str,
    header: Option<&str>,
    body: &[u8],
    now_unix: i64,
) -> Result<(), SignatureError> {
    let header = header.ok_or(SignatureError::Malformed)?;
    let mut ts: Option<&str> = None;
    let mut signatures = Vec::new();
    for part in header.split(';') {
        match part.trim().split_once('=') {
            Some(("ts", v)) => ts = Some(v),
            Some(("h1", v)) => signatures.push(v),
            _ => {}
        }
    }
    let ts = ts.ok_or(SignatureError::Malformed)?;
    let at: i64 = ts.parse().map_err(|_| SignatureError::Malformed)?;
    if signatures.is_empty() {
        return Err(SignatureError::Malformed);
    }
    if (now_unix - at).abs() > WEBHOOK_TOLERANCE_SECS {
        return Err(SignatureError::Stale);
    }
    for hex in signatures {
        let Some(expected) = from_hex(hex) else {
            continue;
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| SignatureError::Mismatch)?;
        mac.update(ts.as_bytes());
        mac.update(b":");
        mac.update(body);
        if mac.verify_slice(&expected).is_ok() {
            return Ok(());
        }
    }
    Err(SignatureError::Mismatch)
}

/// The header Paddle would send for `body` at `ts` (tests and the fake server use it).
pub fn sign(secret: &str, body: &[u8], ts: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(ts.to_string().as_bytes());
    mac.update(b":");
    mac.update(body);
    let hex: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("ts={ts};h1={hex}")
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || s.len() > 128 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The parts of a webhook the shop reads (Paddle's notification payload).
#[derive(Clone, Debug, Deserialize)]
pub struct Notification {
    pub event_id: String,
    pub event_type: String,
    #[serde(default)]
    pub data: Value,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn load(pairs: &[(&str, &str)]) -> Result<PaddleConfig, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        PaddleConfig::from_source(|n| map.get(n).cloned())
    }

    #[test]
    fn config_defaults_to_sandbox_with_nothing_set() {
        let cfg = load(&[]).unwrap();
        assert_eq!(cfg.env, PaddleEnv::Sandbox);
        assert_eq!(cfg.api_url, "https://sandbox-api.paddle.com");
        assert!(cfg.api_key.is_none() && cfg.webhook_secret.is_none());
        let cfg = load(&[("CYEX_TV_PADDLE_API_KEY", "  ")]).unwrap();
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn config_refuses_keys_of_the_other_environment() {
        assert!(load(&[("CYEX_TV_PADDLE_API_KEY", "pdl_live_apikey_x")]).is_err());
        assert!(load(&[("CYEX_TV_PADDLE_CLIENT_TOKEN", "live_abc")]).is_err());
        assert!(
            load(&[
                ("CYEX_TV_PADDLE_ENV", "production"),
                ("CYEX_TV_PADDLE_API_KEY", "pdl_sdbx_apikey_x")
            ])
            .is_err()
        );
        let cfg = load(&[
            ("CYEX_TV_PADDLE_ENV", "production"),
            ("CYEX_TV_PADDLE_API_KEY", "pdl_live_apikey_x"),
            ("CYEX_TV_PADDLE_CLIENT_TOKEN", "live_abc"),
        ])
        .unwrap();
        assert_eq!(cfg.api_url, "https://api.paddle.com");
        assert!(load(&[("CYEX_TV_PADDLE_ENV", "live")]).is_err());
        assert!(!format!("{cfg:?}").contains("pdl_live"));
    }

    #[test]
    fn signatures() {
        let body = br#"{"event_id":"evt_1"}"#;
        let now = 1_800_000_000;
        let header = sign("pdl_ntfset_secret", body, now);
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&header), body, now),
            Ok(())
        );
        // A few minutes of clock difference are fine, more is stale.
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&header), body, now + 299),
            Ok(())
        );
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&header), body, now + 301),
            Err(SignatureError::Stale)
        );
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&header), body, now - 301),
            Err(SignatureError::Stale)
        );
        assert_eq!(
            verify_signature("another", Some(&header), body, now),
            Err(SignatureError::Mismatch)
        );
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&header), b"{}", now),
            Err(SignatureError::Mismatch)
        );
        // Rotation: one of several matching is enough.
        let rotated = format!("{header};h1=00ff");
        assert_eq!(
            verify_signature("pdl_ntfset_secret", Some(&rotated), body, now),
            Ok(())
        );
        for bad in ["", "ts=1", "h1=00", "ts=x;h1=00", "nonsense"] {
            assert_eq!(
                verify_signature("s", Some(bad), body, now),
                Err(SignatureError::Malformed),
                "{bad}"
            );
        }
        assert_eq!(
            verify_signature("s", None, body, now),
            Err(SignatureError::Malformed)
        );
    }
}
