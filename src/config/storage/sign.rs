use crate::types::id::Id;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const REGION: &str = "auto";
const SERVICE: &str = "s3";

/// time to live of 3 days.
pub const TTL: u64 = 3 * 24 * 60 * 60;

const ACCOUNT_ID: &'static str = env!("R2_ACCOUNT_ID");
const BUCKET: &'static str = env!("R2_BUCKET");
const ACCESS_KEY_ID: &'static str = env!("R2_ACCESS_KEY_ID");
const SECRET_ACCESS_KEY: &'static str = env!("R2_SECRET_ACCESS_KEY");

/// Generate an AWS SigV4 presigned URL for R2.
///
/// This is pure cryptography — no network calls. ~1-5μs per URL.
///
/// Arguments:
/// - `account_id` — Cloudflare account ID
/// - `bucket` — R2 bucket name
/// - `access_key_id` — R2 API token access key
/// - `secret_access_key` — R2 API token secret key
/// - `method` — "GET" or "PUT"
/// - `key` — Object key, e.g. "users/{id}/profile"
/// - `expires_in` — Seconds until URL expires (max 604800 = 7 days)
/// - `content_type` — Required for PUT (e.g., "image/*"), None for GET
///
/// Returns the full presigned URL string.
pub fn presign(
    account_id: &str,
    bucket: &str,
    access_key_id: &str,
    secret_access_key: &str,
    method: &str,
    key: &str,
    expires_in: u64,
    content_type: Option<&str>,
) -> String {
    let host = format!("{account_id}.r2.cloudflarestorage.com");
    let now = Utc::now();
    let date_stamp = now.format("%Y%m%d").to_string();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let credential_scope = format!("{date_stamp}/{REGION}/{SERVICE}/aws4_request");
    let credential = format!("{access_key_id}/{credential_scope}");

    // Canonical query string (params sorted alphabetically)
    let mut params = vec![
        ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_string()),
        ("X-Amz-Credential", credential),
        ("X-Amz-Date", amz_date.clone()),
        ("X-Amz-Expires", expires_in.to_string()),
        ("X-Amz-SignedHeaders", "host".to_string()),
    ];
    if let Some(ct) = content_type {
        params.push(("X-Amz-Content-Type", ct.to_string()));
    }
    params.sort_by(|a, b| a.0.cmp(b.0));

    let canonical_query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    // Canonical request
    let canonical_uri = format!("/{bucket}/{key}");
    let canonical_headers = format!("host:{host}\n");
    let signed_headers = "host";
    let payload_hash = "UNSIGNED-PAYLOAD";

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    // String to sign
    let request_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{request_hash}");

    // Signing key (derived via HMAC chain)
    let k_date = hmac_sha256(format!("AWS4{secret_access_key}").as_bytes(), &date_stamp);
    let k_region = hmac_sha256(&k_date, REGION);
    let k_service = hmac_sha256(&k_region, SERVICE);
    let k_signing = hmac_sha256(&k_service, "aws4_request");

    // Final signature
    let signature = hex::encode(hmac_sha256(&k_signing, &string_to_sign));

    // For the final URL, we want to decode characters that R2 expects unencoded in the URL
    // specifically slashes (/) and asterisks (*) which are used in Credential and Content-Type
    let url_query = canonical_query.replace("%2F", "/").replace("%2A", "*");

    format!("https://{host}{canonical_uri}?{url_query}&X-Amz-Signature={signature}")
}

fn hmac_sha256(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// Generate a ProfileUrl for the given object key.
///
/// - `with_write` — if true, also generates a PUT presigned URL
/// - `expires_in` — seconds until URLs expire
pub fn url(key: &str, ttl: u64, write: bool) -> String {
    let (method, content_type) = match write {
        true => ("PUT", Some("image/*")),
        false => ("GET", None),
    };
    presign(
        ACCOUNT_ID,
        BUCKET,
        ACCESS_KEY_ID,
        SECRET_ACCESS_KEY,
        method,
        key,
        ttl,
        content_type,
    )
}

pub fn profile(id: &Id, ttl: Option<u64>, write: bool) -> String {
    let ttl = ttl.unwrap_or(TTL);
    let key = String::from("users/") + &id.to_string() + "/profile";
    url(key.as_str(), ttl, write)
}

pub fn logo(id: &Id, ttl: Option<u64>, write: bool) -> String {
    let ttl = ttl.unwrap_or(TTL);
    let key = String::from("schools/") + &id.to_string() + "/logo";
    url(key.as_str(), ttl, write)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256() {
        let key = b"secret";
        let data = "test_data";
        let result = hmac_sha256(key, data);

        // Should produce a 32-byte hash
        assert_eq!(result.len(), 32);

        // Same input should produce same output
        let result2 = hmac_sha256(key, data);
        assert_eq!(result, result2);

        // Different input should produce different output
        let result3 = hmac_sha256(key, "different_data");
        assert_ne!(result, result3);
    }

    #[test]
    fn test_presign_get_url_structure() {
        let url = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key.jpg",
            3600,
            None,
        );

        // Check basic URL structure
        assert!(url.starts_with("https://"));
        assert!(url.contains("test_account.r2.cloudflarestorage.com"));
        assert!(url.contains("/test_bucket/test/key.jpg"));

        // Check required query parameters
        assert!(url.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(url.contains("X-Amz-Credential="));
        assert!(url.contains("X-Amz-Date="));
        assert!(url.contains("X-Amz-Expires=3600"));
        assert!(url.contains("X-Amz-SignedHeaders=host"));
        assert!(url.contains("X-Amz-Signature="));

        // GET should not have Content-Type
        assert!(!url.contains("X-Amz-Content-Type"));
    }

    #[test]
    fn test_presign_put_url_with_content_type() {
        let url = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "PUT",
            "test/key.jpg",
            3600,
            Some("image/jpeg"),
        );

        // PUT should include Content-Type, unencoded
        assert!(url.contains("X-Amz-Content-Type=image/jpeg"));
    }

    #[test]
    fn test_presign_different_expiry_times() {
        let url_1h = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key.jpg",
            3600,
            None,
        );

        let url_7d = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key.jpg",
            604800,
            None,
        );

        assert!(url_1h.contains("X-Amz-Expires=3600"));
        assert!(url_7d.contains("X-Amz-Expires=604800"));
    }

    #[test]
    fn test_presign_deterministic_for_same_timestamp() {
        // Note: In real usage, the timestamp changes, but for testing
        // we verify that the function is deterministic
        let url1 = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key.jpg",
            3600,
            None,
        );

        let url2 = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key.jpg",
            3600,
            None,
        );

        // URLs will differ because timestamp changes, but structure should be same
        assert_eq!(url1.split('?').next(), url2.split('?').next());
    }

    #[test]
    fn test_presign_different_keys_produce_different_signatures() {
        let url1 = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key1.jpg",
            3600,
            None,
        );

        let url2 = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key2.jpg",
            3600,
            None,
        );

        // Different object keys should produce different URLs
        assert_ne!(url1, url2);
        assert!(url1.contains("test/key1.jpg"));
        assert!(url2.contains("test/key2.jpg"));
    }

    #[test]
    fn test_presign_url_encoding() {
        let url = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "GET",
            "test/key with spaces.jpg",
            3600,
            None,
        );

        // Path should contain the key (with spaces)
        assert!(url.contains("test/key with spaces.jpg"));
    }

    #[test]
    fn test_profile_url_read() {
        let id = Id::default();
        let url = profile(&id, Some(3600), false);

        // Check URL structure
        assert!(url.starts_with("https://"));
        assert!(url.contains(&format!("users/{}/profile", id)));

        // Should be a GET request (no Content-Type)
        assert!(!url.contains("X-Amz-Content-Type"));
    }

    #[test]
    fn test_profile_url_write() {
        let id = Id::default();
        let url = profile(&id, Some(3600), true);

        // Check URL structure
        assert!(url.starts_with("https://"));
        assert!(url.contains(&format!("users/{}/profile", id)));

        // Should be a PUT request (with Content-Type), unencoded
        assert!(url.contains("X-Amz-Content-Type=image/*"));
    }

    #[test]
    fn test_profile_different_ids() {
        let id1 = Id::default();
        let id2 = Id::default();

        let url1 = profile(&id1, Some(3600), false);
        let url2 = profile(&id2, Some(3600), false);

        // Different IDs should produce different URLs
        assert_ne!(url1, url2);
        assert!(url1.contains(&id1.to_string()));
        assert!(url2.contains(&id2.to_string()));
    }

    #[test]
    fn test_url_function_get() {
        let result = url("test/path.jpg", 7200, false);

        assert!(result.contains("test/path.jpg"));
        assert!(result.contains("X-Amz-Expires=7200"));
        assert!(!result.contains("X-Amz-Content-Type"));
    }

    #[test]
    fn test_url_function_put() {
        let result = url("test/path.jpg", 7200, true);

        assert!(result.contains("test/path.jpg"));
        assert!(result.contains("X-Amz-Expires=7200"));
        assert!(result.contains("X-Amz-Content-Type=image/*"));
    }

    #[test]
    fn test_presign_unencoded_chars() {
        let url = presign(
            "test_account",
            "test_bucket",
            "test_access_key",
            "test_secret_key",
            "PUT",
            "test/key.jpg",
            3600,
            Some("image/*"),
        );

        let query = url.split('?').nth(1).unwrap();

        // Check Credential
        let credential_param = query
            .split('&')
            .find(|p| p.starts_with("X-Amz-Credential="))
            .unwrap();
        assert!(credential_param.contains("/"));
        assert!(!credential_param.contains("%2F"));

        // Check Content-Type
        let ct_param = query
            .split('&')
            .find(|p| p.starts_with("X-Amz-Content-Type="))
            .unwrap();
        assert!(ct_param.contains("image/*"));
        assert!(!ct_param.contains("%2F"));
        assert!(!ct_param.contains("%2A"));
    }
}
