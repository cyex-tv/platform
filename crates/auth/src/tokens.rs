//! Random tokens, their hashes, and PKCE (RFC 7636).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use cyex_tv_db::sessions::TokenHash;
use sha2::{Digest, Sha256};

/// Length of a token from [`random_token`]: 32 bytes in unpadded base64url.
pub const TOKEN_LEN: usize = 43;

/// 256 random bits from the OS, as 43 base64url characters. Used for session tokens, `state` and
/// the PKCE verifier (which RFC 7636 wants 43 to 128 characters long).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    // The OS random source failing is not something to carry on from.
    getrandom::fill(&mut bytes).expect("the OS random source is available");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// The sha256 that is stored instead of the token.
pub fn hash(token: &str) -> TokenHash {
    Sha256::digest(token.as_bytes()).into()
}

/// Whether `raw` looks like a token we issued, so junk is refused before any lookup.
pub fn is_well_formed(raw: &str) -> bool {
    raw.len() == TOKEN_LEN
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The S256 code challenge for a verifier.
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_well_formed() {
        let a = random_token();
        let b = random_token();
        assert_ne!(a, b);
        assert!(is_well_formed(&a));
        assert!(!is_well_formed(&a[1..]));
        assert!(!is_well_formed(&format!("{}=", &a[1..])));
        assert!(!is_well_formed(""));
    }

    #[test]
    fn hash_is_sha256() {
        // sha256("abc"), from FIPS 180-2.
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(hash("abc"), expected);
    }

    #[test]
    fn pkce_challenge_is_base64url_sha256() {
        // Computed separately: base64.urlsafe_b64encode(hashlib.sha256(v).digest()) without `=`.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mJ92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "ngF5GsXcbwljx6u133FFr3Xht9xooA_DuaX_3QwODtc"
        );
    }
}
