//! The stable error codes (`cyex_tv_shared::error_codes`) cover every code the source uses.
//!
//! The scan reads the non-test source of the api, the auth and shared crates and the events
//! service, and collects each string literal in a code position: the first argument of `bad(`,
//! `conflict(`, `coded(`, `ApiError::limit(` and `limit_error(`, the code of
//! `ApiError::new(status, "code", ..)`, the value of `set("code", "..")` and of a `"code": ".."`
//! JSON literal, and the `=> "code"` arms of `AuthError::code`. A code that is built at run
//! time cannot be seen here; there are none today, and a new way of making one should be added to
//! the scan with it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use cyex_tv_shared::error_codes::{ERROR_CODES, is_known};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under `dir` that is not a test file.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "tests" && name != "target" {
                sources(&path, out);
            }
        } else if name.ends_with(".rs") && name != "tests.rs" {
            out.push(path);
        }
    }
}

/// The text of `src` before its test module.
fn code_part(src: &str) -> &str {
    src.find("#[cfg(test)]").map_or(src, |at| &src[..at])
}

/// The string literal at the start of `rest` (after whitespace), if there is one.
fn literal(rest: &str) -> Option<&str> {
    let rest = rest.trim_start().strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn is_code(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The codes used in `src`, by the ways the api makes them.
fn codes_in(src: &str, found: &mut Vec<String>) {
    let src = code_part(src);
    // Calls whose first argument is the code. The name must start a word: `is_bad(` is not `bad(`.
    for call in ["bad(", "conflict(", "coded("] {
        for (at, _) in src.match_indices(call) {
            let before = src[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(code) = literal(&src[at + call.len()..]) {
                found.push(code.to_owned());
            }
        }
    }
    // `ApiError::new(status, "code", message)` and `Self::new(..)` inside `ApiError`'s impl.
    for call in ["ApiError::new(", "Self::new("] {
        for (at, _) in src.match_indices(call) {
            let rest = &src[at + call.len()..];
            let Some((status, after)) = rest.split_once(',') else {
                continue;
            };
            if status.contains('(') {
                continue;
            }
            if let Some(code) = literal(after) {
                found.push(code.to_owned());
            }
        }
    }
    // GraphQL: `e.set("code", "invalid_topic")`; JSON bodies built by hand:
    // `json!({ "code": "bad_request" })`.
    for marker in ["set(\"code\",", "\"code\":"] {
        for (at, _) in src.match_indices(marker) {
            if let Some(code) = literal(&src[at + marker.len()..]) {
                found.push(code.to_owned());
            }
        }
    }
    // Limit errors: `ApiError::limit("code", ..)` and GraphQL's `limit_error("code", ..)`.
    for call in ["ApiError::limit(", "limit_error("] {
        for (at, _) in src.match_indices(call) {
            let before = src[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(code) = literal(&src[at + call.len()..]) {
                found.push(code.to_owned());
            }
        }
    }
}

/// The arms of `AuthError::code`: `Self::SignedOut => "signed_out",`.
fn auth_codes(src: &str, found: &mut Vec<String>) {
    let start = src.find("pub fn code(&self)").expect("AuthError::code");
    let body = &src[start..];
    let body = &body[..body.find("\n    }\n").expect("end of AuthError::code")];
    for line in body.lines() {
        if let Some((_, code)) = line.split_once("=>")
            && let Some(code) = literal(code)
        {
            found.push(code.to_owned());
        }
    }
}

#[test]
fn every_code_the_source_uses_is_listed() {
    let root = root();
    let mut files = Vec::new();
    // The auth crate has its own helpers named `bad(`; its codes come from `AuthError::code`.
    for dir in [
        "apps/api/src",
        "crates/shared/src",
        "crates/auth/src/error.rs",
    ] {
        let path = root.join(dir);
        if path.is_file() {
            files.push(path);
        } else {
            sources(&path, &mut files);
        }
    }
    let mut used: BTreeMap<String, String> = BTreeMap::new();
    for file in files {
        let src = fs::read_to_string(&file).unwrap();
        let mut found = Vec::new();
        // The events socket has its own frame codes (`End::Error`), not API error codes; and the
        // list itself holds codes only as data.
        let name = file.to_string_lossy().replace('\\', "/");
        if !name.contains("apps/events/") && !name.ends_with("shared/src/error_codes.rs") {
            codes_in(&src, &mut found);
        }
        if name.ends_with("crates/auth/src/error.rs") {
            auth_codes(&src, &mut found);
        }
        for code in found {
            assert!(
                is_code(&code),
                "{name}: {code:?} is in a code position but is not snake_case"
            );
            used.entry(code).or_insert_with(|| name.clone());
        }
    }
    assert!(
        used.len() > 100,
        "the scan found only {} codes: did the source change shape?",
        used.len()
    );
    let missing: Vec<String> = used
        .iter()
        .filter(|(code, _)| !is_known(code))
        .map(|(code, file)| format!("{code} (used in {file})"))
        .collect();
    assert!(
        missing.is_empty(),
        "add these to crates/shared/src/error_codes.rs: {missing:#?}"
    );
}

#[test]
fn every_listed_code_is_used_somewhere() {
    // The reverse: a code that nothing uses any more is dead weight in the contract. Codes the
    // scan cannot see (made at run time) would go in this list, with the reason.
    const BY_OTHER_MEANS: [&str; 0] = [];
    let root = root();
    let mut files = Vec::new();
    for dir in ["apps/api/src", "crates/shared/src", "crates/auth/src"] {
        sources(&root.join(dir), &mut files);
    }
    let all: String = files
        .iter()
        .filter(|f| !f.ends_with("error_codes.rs"))
        .map(|f| code_part(&fs::read_to_string(f).unwrap()).to_owned())
        .collect();
    let unused: Vec<&str> = ERROR_CODES
        .iter()
        .map(|e| e.code)
        .filter(|code| !BY_OTHER_MEANS.contains(code) && !all.contains(&format!("\"{code}\"")))
        .collect();
    assert!(unused.is_empty(), "listed but never used: {unused:?}");
}

#[test]
fn openapi_lists_the_codes() {
    let spec = serde_json::to_value(cyex_tv_api::openapi()).unwrap();
    let schema = &spec["components"]["schemas"]["ErrorCode"];
    let listed: Vec<&str> = schema["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let want: Vec<&str> = ERROR_CODES.iter().map(|e| e.code).collect();
    assert_eq!(listed, want);
    assert_eq!(
        schema["x-codes"].as_array().unwrap().len(),
        ERROR_CODES.len()
    );
}
