//! Puts the git commit into the build as `CYEX_TV_BUILD_SHA` (shown by `GET /v1/meta`). A build
//! without git, or the variable already set (a Docker build argument), keeps what it has.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CYEX_TV_BUILD_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if std::env::var("CYEX_TV_BUILD_SHA").is_ok_and(|v| !v.trim().is_empty()) {
        return;
    }
    let sha = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if let Some(sha) = sha {
        println!("cargo:rustc-env=CYEX_TV_BUILD_SHA={sha}");
    }
}
