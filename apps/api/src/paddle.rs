//! cyex.tv's own Paddle Billing account (ADR 0016). The client lives in `cyex-tv-paddle`, so the
//! worker can cancel stale checkouts with it too; this module keeps the api's paths.

pub use cyex_tv_paddle::*;
