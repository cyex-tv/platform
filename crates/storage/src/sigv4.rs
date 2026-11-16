//! AWS Signature Version 4 for S3 requests (also what R2 and other S3 servers take).
//! Reference: "Authenticating Requests: Using the Authorization Header (AWS Signature Version 4)".

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use url::Url;

type HmacSha256 = Hmac<Sha256>;

/// `sha256("")`, the payload hash of a request without a body.
pub const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    // HMAC takes keys of any length; `new_from_slice` can't fail for it.
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// RFC 3986 encoding as SigV4 wants it: unreserved characters stay, everything else is `%XX`.
fn encode(raw: &str, keep_slash: bool) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b'/' if keep_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The S3 path for a bucket and key, encoded once (`/bucket/emotes/ab/1x.webp`).
pub fn object_path(bucket: &str, key: &str) -> String {
    format!("/{}/{}", encode(bucket, false), encode(key, true))
}

#[derive(Clone)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<redacted>")
            .field("region", &self.region)
            .finish()
    }
}

/// Signs a request. `headers` are the ones to sign, lowercase names; `host`, `x-amz-date` and
/// `x-amz-content-sha256` must be among them. `path` is already encoded (see `object_path`).
/// Returns the `Authorization` header.
pub fn authorization(
    creds: &Credentials,
    method: &str,
    url: &Url,
    path: &str,
    headers: &[(String, String)],
    payload_sha256: &str,
    now: DateTime<Utc>,
) -> String {
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date = now.format("%Y%m%d").to_string();

    let mut query: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (encode(&k, false), encode(&v, false)))
        .collect();
    query.sort();
    let canonical_query = query
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let mut sorted: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| {
            (
                k.to_lowercase(),
                v.split_whitespace().collect::<Vec<_>>().join(" "),
            )
        })
        .collect();
    sorted.sort();
    let canonical_headers: String = sorted.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let signed_headers = sorted
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_request = format!(
        "{method}\n{path}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_sha256}"
    );
    let scope = format!("{date}/{}/s3/aws4_request", creds.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let k_date = hmac(
        format!("AWS4{}", creds.secret_access_key).as_bytes(),
        date.as_bytes(),
    );
    let k_region = hmac(&k_date, creds.region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex(&hmac(&k_signing, string_to_sign.as_bytes()));

    format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        creds.access_key_id
    )
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    /// The GET example from AWS's SigV4 documentation for S3 ("Example: GET Object"), with its
    /// published example credentials.
    #[test]
    fn matches_the_aws_get_object_example() {
        let creds = Credentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_owned(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_owned(),
            region: "us-east-1".to_owned(),
        };
        let now = Utc.with_ymd_and_hms(2013, 5, 24, 0, 0, 0).unwrap();
        let url = Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap();
        let headers = vec![
            (
                "host".to_owned(),
                "examplebucket.s3.amazonaws.com".to_owned(),
            ),
            ("range".to_owned(), "bytes=0-9".to_owned()),
            ("x-amz-content-sha256".to_owned(), EMPTY_SHA256.to_owned()),
            ("x-amz-date".to_owned(), "20130524T000000Z".to_owned()),
        ];
        let auth = authorization(
            &creds,
            "GET",
            &url,
            "/test.txt",
            &headers,
            EMPTY_SHA256,
            now,
        );
        assert_eq!(
            auth,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    #[test]
    fn paths_are_encoded_once() {
        assert_eq!(
            object_path("cdn", "emotes/ab12/1x.webp"),
            "/cdn/emotes/ab12/1x.webp"
        );
        assert_eq!(object_path("cdn", "a b/ü"), "/cdn/a%20b/%C3%BC");
        assert_eq!(sha256_hex(b""), EMPTY_SHA256);
    }

    #[test]
    fn secrets_stay_out_of_debug() {
        let creds = Credentials {
            access_key_id: "id".to_owned(),
            secret_access_key: "very-secret".to_owned(),
            region: "auto".to_owned(),
        };
        assert!(!format!("{creds:?}").contains("very-secret"));
    }
}
