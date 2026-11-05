//! AWS Signature Version 4.
//!
//! Implemented from the published algorithm rather than the SDK, for the same
//! reason as the HTTP client: one signing routine versus a large dependency
//! tree. Reviewed against the AWS test-suite vectors in the tests below.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

/// Everything needed to sign a request.
pub struct SignRequest<'a> {
    pub method: &'a str,
    pub url: &'a str,
    pub region: &'a str,
    pub service: &'a str,
    pub access_key_id: &'a str,
    pub secret_access_key: &'a str,
    pub session_token: Option<&'a str>,
    /// Request body, hashed as part of the canonical request.
    pub body: &'a [u8],
    /// Extra headers to sign (must also be sent).
    pub headers: Vec<(String, String)>,
    /// Seconds since the epoch; injectable for deterministic tests.
    pub timestamp: i64,
}

/// Signed headers to add to the outgoing request.
pub struct Signed {
    pub headers: Vec<(String, String)>,
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

/// `20240115T123456Z` and `20240115`.
fn amz_dates(timestamp: i64) -> (String, String) {
    let dt = chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_default();
    (
        dt.format("%Y%m%dT%H%M%SZ").to_string(),
        dt.format("%Y%m%d").to_string(),
    )
}

/// RFC 3986 encoding, as SigV4 requires (stricter than the URL default).
fn uri_encode(value: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Split a URL into (host, canonical path, canonical query).
fn split_url(url: &str) -> AppResult<(String, String, String)> {
    let rest = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .ok_or_else(|| AppError::internal(format!("URL missing scheme: {url}")))?;
    let (authority, path_and_query) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    // Strip a port: the Host header carries it, but the canonical URI does not.
    let host = authority.split(':').next().unwrap_or(authority).to_string();
    let (path, query) = match path_and_query.split_once('?') {
        Some((path, query)) => (path, query),
        None => (path_and_query, ""),
    };

    // Query parameters must be sorted by encoded name, then encoded value.
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| match pair.split_once('=') {
            Some((key, value)) => (uri_encode(key, true), uri_encode(value, true)),
            None => (uri_encode(pair, true), String::new()),
        })
        .collect();
    pairs.sort();
    let canonical_query = pairs
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    let path = if path.is_empty() { "/" } else { path };
    Ok((
        host,
        uri_encode(path, false),
        canonical_query,
    ))
}

/// Sign a request, returning the headers the caller must send.
pub fn sign(request: SignRequest<'_>) -> AppResult<Signed> {
    let (host, canonical_path, canonical_query) = split_url(request.url)?;
    let (amz_date, date_stamp) = amz_dates(request.timestamp);

    // Canonical headers: lowercase name, trimmed value, sorted by name.
    let mut headers: Vec<(String, String)> = request
        .headers
        .iter()
        .map(|(key, value)| (key.to_lowercase(), value.trim().to_string()))
        .collect();
    headers.push(("host".into(), host));
    headers.push(("x-amz-date".into(), amz_date.clone()));
    headers.push((
        "x-amz-content-sha256".into(),
        sha256_hex(request.body),
    ));
    if let Some(token) = request.session_token.filter(|token| !token.is_empty()) {
        headers.push(("x-amz-security-token".into(), token.to_string()));
    }
    headers.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = headers
        .iter()
        .map(|(key, value)| format!("{key}:{value}\n"))
        .collect();
    let signed_headers: String = headers
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        request.method,
        canonical_path,
        canonical_query,
        canonical_headers,
        signed_headers,
        sha256_hex(request.body)
    );

    let scope = format!(
        "{date_stamp}/{}/{}/aws4_request",
        request.region, request.service
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        scope,
        sha256_hex(canonical_request.as_bytes())
    );

    // Derive the signing key: kDate -> kRegion -> kService -> kSigning.
    let k_date = hmac(
        format!("AWS4{}", request.secret_access_key).as_bytes(),
        &date_stamp,
    );
    let k_region = hmac(&k_date, request.region);
    let k_service = hmac(&k_region, request.service);
    let k_signing = hmac(&k_service, "aws4_request");
    let signature = hex(&hmac(&k_signing, &string_to_sign));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        request.access_key_id, scope, signed_headers, signature
    );

    let mut out: Vec<(String, String)> = headers
        .into_iter()
        .filter(|(key, _)| key != "host")
        .collect();
    out.push(("Authorization".into(), authorization));
    Ok(Signed { headers: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical AWS documentation example: GET on S3 with a signed
    /// `Range` header. Expected signature is fixed by the docs.
    #[test]
    fn matches_the_aws_documentation_vector() {
        // 2013-05-24T00:00:00Z
        let timestamp = 1369353600;
        let signed = sign(SignRequest {
            method: "GET",
            url: "https://examplebucket.s3.amazonaws.com/test.txt",
            region: "us-east-1",
            service: "s3",
            access_key_id: "AKIAIOSFODNN7EXAMPLE",
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            session_token: None,
            body: b"",
            headers: vec![("Range".into(), "bytes=0-9".into())],
            timestamp,
        })
        .unwrap();

        let authorization = signed
            .headers
            .iter()
            .find(|(key, _)| key == "Authorization")
            .map(|(_, value)| value.clone())
            .unwrap();

        // The signature published in the AWS SigV4 documentation for this exact
        // request. Matching it proves the canonicalisation, the key derivation
        // and the signing chain are all correct.
        assert_eq!(
            authorization,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    #[test]
    fn produces_a_deterministic_signature() {
        let make = || {
            sign(SignRequest {
                method: "POST",
                url: "https://elasticache.us-east-1.amazonaws.com/",
                region: "us-east-1",
                service: "elasticache",
                access_key_id: "AKIA",
                secret_access_key: "secret",
                session_token: None,
                body: b"Action=DescribeReplicationGroups&Version=2015-02-02",
                headers: vec![(
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                )],
                timestamp: 1_700_000_000,
            })
            .unwrap()
        };
        let first = make();
        let second = make();
        assert_eq!(first.headers, second.headers);
    }

    #[test]
    fn query_parameters_are_sorted_and_encoded() {
        let (_, _, query) = split_url("https://x.amazonaws.com/?b=2&a=1&c=a b").unwrap();
        assert_eq!(query, "a=1&b=2&c=a%20b");
    }

    #[test]
    fn session_tokens_are_signed() {
        let signed = sign(SignRequest {
            method: "GET",
            url: "https://example.com/",
            region: "us-east-1",
            service: "sts",
            access_key_id: "AKIA",
            secret_access_key: "secret",
            session_token: Some("token-value"),
            body: b"",
            headers: vec![],
            timestamp: 1_700_000_000,
        })
        .unwrap();
        assert!(signed
            .headers
            .iter()
            .any(|(key, _)| key == "x-amz-security-token"));
        let authorization = signed
            .headers
            .iter()
            .find(|(key, _)| key == "Authorization")
            .unwrap()
            .1
            .clone();
        assert!(authorization.contains("x-amz-security-token"));
    }

    #[test]
    fn slash_encoding_depends_on_context() {
        assert_eq!(uri_encode("a/b", false), "a/b");
        assert_eq!(uri_encode("a/b", true), "a%2Fb");
    }
}
