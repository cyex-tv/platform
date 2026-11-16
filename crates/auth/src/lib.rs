//! Signing in to cyex.tv with Twitch or Kick, and the sessions that follow (ADR 0009).
//!
//! - [`flow`]: the OAuth authorization code flow with `state` bound to a pre-login cookie, and
//!   PKCE S256 where the platform takes it (Kick requires it; Twitch doesn't document it).
//! - [`session`]: who is calling, from the site cookie or an app Bearer token, with the Origin
//!   check for cookie callers on writes.
//! - [`providers`]: the Twitch and Kick endpoints and response shapes.
//!
//! Platform access tokens are used once, to read the user, then revoked and dropped.

pub mod config;
pub mod cookies;
mod error;
pub mod flow;
pub mod origin;
pub mod providers;
pub mod rbac;
pub mod return_to;
pub mod session;
pub mod tokens;

pub use config::AuthConfig;
pub use error::AuthError;
pub use flow::Auth;
pub use session::{Caller, Via};
