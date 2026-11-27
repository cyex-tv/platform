//! The S3 client against a real S3 server (Versity Gateway in CI, standing in for R2). Runs when
//! `CYEX_TV_TEST_S3_ENDPOINT`, `CYEX_TV_TEST_S3_ACCESS_KEY_ID` and
//! `CYEX_TV_TEST_S3_SECRET_ACCESS_KEY` are set, and skips otherwise.

use bytes::Bytes;
use cyex_tv_storage::sigv4::Credentials;
use cyex_tv_storage::{IMMUTABLE, Put, S3, S3Config, Store, emote_key};
use url::Url;

fn config(bucket: &str, secret: Option<&str>) -> Option<S3Config> {
    let endpoint = std::env::var("CYEX_TV_TEST_S3_ENDPOINT").ok()?;
    let id = std::env::var("CYEX_TV_TEST_S3_ACCESS_KEY_ID").ok()?;
    let real = std::env::var("CYEX_TV_TEST_S3_SECRET_ACCESS_KEY").ok()?;
    Some(S3Config {
        bucket: bucket.to_owned(),
        endpoint: Url::parse(&endpoint).unwrap(),
        credentials: Credentials {
            access_key_id: id,
            secret_access_key: secret.map_or(real, str::to_owned),
            region: "us-east-1".to_owned(),
        },
        // The same server: CI's S3 is on this machine.
        upload_endpoint: Some(Url::parse(&endpoint).unwrap()),
    })
}

#[tokio::test]
async fn put_get_head_delete_against_s3() {
    let Some(cfg) = config("cyex-tv-test", None) else {
        eprintln!("CYEX_TV_TEST_S3_* not set; skipping");
        return;
    };
    S3::new(cfg.clone()).unwrap().ensure_bucket().await.unwrap();
    // A second time is fine.
    S3::new(cfg.clone()).unwrap().ensure_bucket().await.unwrap();
    let store = Store::s3(cfg).unwrap();

    let key = emote_key("0123abcd", "1x.webp");
    assert_eq!(store.get(&key).await.unwrap(), None);
    assert!(!store.exists(&key).await.unwrap());
    let body = Bytes::from_static(b"RIFF\x00\x00\x00\x00WEBPfake");
    store
        .put(
            &key,
            body.clone(),
            Put {
                content_type: "image/webp",
                cache_control: Some(IMMUTABLE),
            },
        )
        .await
        .unwrap();
    assert!(store.exists(&key).await.unwrap());
    let got = store.get(&key).await.unwrap().unwrap();
    assert_eq!(got.bytes, body);
    assert_eq!(got.content_type.as_deref(), Some("image/webp"));
    assert_eq!(got.cache_control.as_deref(), Some(IMMUTABLE));

    store.delete(&key).await.unwrap();
    store.delete(&key).await.unwrap();
    assert!(!store.exists(&key).await.unwrap());
}

#[tokio::test]
async fn a_wrong_secret_is_refused() {
    let Some(cfg) = config("cyex-tv-test", Some("not-the-secret")) else {
        return;
    };
    let store = Store::s3(cfg).unwrap();
    let err = store.get("emotes/x/1x.webp").await.unwrap_err();
    assert!(err.to_string().contains("403"), "{err}");
}

/// A direct upload (ADR 0036): the presigned PUT stores exactly the declared bytes and type, and
/// the server refuses a body of another length or another type under the same signature.
#[tokio::test]
async fn a_presigned_put_takes_only_what_was_signed() {
    let Some(cfg) = config("cyex-tv-test", None) else {
        eprintln!("CYEX_TV_TEST_S3_* not set; skipping");
        return;
    };
    S3::new(cfg.clone()).unwrap().ensure_bucket().await.unwrap();
    let store = Store::s3(cfg).unwrap();
    let key = cyex_tv_storage::upload_key("01presigntest");
    let body = b"\x89PNG\r\n\x1a\nnot really a png".to_vec();
    let put = store
        .presign_put(
            &key,
            "image/png",
            body.len() as u64,
            std::time::Duration::from_secs(300),
        )
        .expect("an upload endpoint is set");
    let http = reqwest::Client::new();
    let send = |body: Vec<u8>, content_type: &str| {
        http.put(&put.url)
            .header("content-type", content_type)
            .body(body)
            .send()
    };
    // Another length, then another type: refused, nothing stored.
    let longer = send([body.clone(), b"x".to_vec()].concat(), "image/png")
        .await
        .unwrap();
    assert!(!longer.status().is_success(), "{}", longer.status());
    let other_type = send(body.clone(), "text/html").await.unwrap();
    assert!(!other_type.status().is_success(), "{}", other_type.status());
    assert!(!store.exists(&key).await.unwrap());

    let ok = send(body.clone(), "image/png").await.unwrap();
    assert!(ok.status().is_success(), "{}", ok.status());
    let got = store.get(&key).await.unwrap().unwrap();
    assert_eq!(got.bytes, Bytes::from(body));
    assert_eq!(got.content_type.as_deref(), Some("image/png"));
    store.delete(&key).await.unwrap();
}
