//! The API's coverage suite (one test binary, sharing `world`): the website's operations against
//! the real schema (`contract`), who may call each root field (`authz`), both live-update sockets
//! from one mutation (`live`), and input rules no other test sends a bad value to (`validation`).
//! Through the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set
//! (CI sets it) and skips otherwise.

#[path = "../common/mod.rs"]
mod common;

mod authz;
mod contract;
mod live;
mod validation;
mod world;
