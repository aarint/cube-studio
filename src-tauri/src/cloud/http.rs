//! Minimal HTTP/JSON client for cloud control planes.
//!
//! Providers expose REST APIs, and pulling in a full HTTP stack for a handful of
//! GET calls would add megabytes and a TLS backend to the binary. This module
//! speaks just enough HTTP/1.1 over the TLS connector we already depend on.
//!
//! Scope, deliberately limited:
//! * one request per connection (control-plane calls are rare)
//! * `Connection: close`
//! * no redirects, no chunked uploads, no cookies
//! * chunked *responses* are decoded, because several providers use them

use std::collections::BTreeMap;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{AppError, AppResult};

/// One HTTP response.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Parse the body as JSON, with the server's message on failure.
    pub fn json(&self) -> AppResult<serde_json::Value> {
        serde_json::from_slice(&self.body).map_err(|e| {
            AppError::server(format!(
                "the cloud API returned a response that is not JSON ({e}); first bytes: {}",
                String::from_utf8_lossy(&self.body[..self.body.len().min(200)])
            ))
        })
    }

    /// Turn a non-2xx response into an error that names the provider's message.
    pub fn into_result(self) -> AppResult<Response> {
        if self.is_success() {
            return Ok(self);
        }
        // Every provider wraps errors differently; surface the raw body, which
        // always contains the useful string.
        let body = String::from_utf8_lossy(&self.body).to_string();
        let detail = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| {
                for pointer in [
                    "/message",
                    "/error/message",
                    "/error/description",
                    "/Message",
                    "/description",
                    "/detail",
                ] {
                    if let Some(found) = value.pointer(pointer).and_then(|v| v.as_str()) {
                        return Some(found.to_string());
                    }
                }
                None
            })
            .unwrap_or(body);

        let kind = match self.status {
            401 | 403 => crate::error::ErrorKind::Unauthorized,
            404 => crate::error::ErrorKind::InvalidInput,
            429 => crate::error::ErrorKind::Unavailable,
            _ => crate::error::ErrorKind::ServerError,
        };
        Err(AppError::new(kind, detail)
            .with_detail(format!("HTTP {}", self.status)))
    }
}

/// A request description.
#[derive(Debug, Clone)]
pub struct Request {
    pub url: String,
    pub method: &'static str,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "GET",
            headers: BTreeMap::new(),
            body: None,
        }
    }

    pub fn post(url: impl Into<String>, body: Vec<u8>) -> Self {
        Self {
            url: url.into(),
            method: "POST",
            headers: BTreeMap::new(),
            body: Some(body),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {token}"))
    }

    /// HTTP basic authentication, used by the API-key providers.
    pub fn basic(self, user: &str, password: &str) -> Self {
        use base64::Engine as _;
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        self.header("Authorization", format!("Basic {encoded}"))
    }

    pub fn json_body<T: serde::Serialize>(mut self, value: &T) -> AppResult<Self> {
        self.body = Some(serde_json::to_vec(value)?);
        Ok(self.header("Content-Type", "application/json"))
    }
}

/// Split a URL into (tls, host, port, path-and-query).
fn parse_url(url: &str) -> AppResult<(bool, String, u16, String)> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| AppError::invalid(format!("URL is missing a scheme: {url}")))?;
    let tls = match scheme {
        "https" => true,
        "http" => false,
        other => {
            return Err(AppError::invalid(format!(
                "unsupported URL scheme {other}"
            )))
        }
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.contains(']') => (
            host.to_string(),
            port.parse::<u16>()
                .map_err(|_| AppError::invalid(format!("invalid port in {url}")))?,
        ),
        _ => (authority.to_string(), if tls { 443 } else { 80 }),
    };
    Ok((tls, host, port, path.to_string()))
}

/// Execute a request.
pub async fn execute(
    request: &Request,
    timeout: std::time::Duration,
) -> AppResult<Response> {
    let (tls, host, port, path) = parse_url(&request.url)?;

    let exchange = async {
        let stream = tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|e| AppError::network(format!("cannot reach {host}:{port}: {e}")))?;
        stream.set_nodelay(true).ok();

        // Build the request head once; it is the same for both transports.
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: cube-studio/{}\r\nAccept: application/json\r\nConnection: close\r\n",
            request.method,
            path,
            host,
            env!("CARGO_PKG_VERSION")
        );
        for (key, value) in &request.headers {
            head.push_str(&format!("{key}: {value}\r\n"));
        }
        if let Some(body) = &request.body {
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");

        let mut payload = head.into_bytes();
        if let Some(body) = &request.body {
            payload.extend_from_slice(body);
        }

        if tls {
            let connector = tls_connector()?;
            let server_name = rustls::pki_types::ServerName::try_from(host.clone())
                .map_err(|_| AppError::invalid(format!("invalid hostname {host}")))?;
            let mut tls_stream = connector
                .connect(server_name, stream)
                .await
                .map_err(|e| AppError::network(format!("TLS handshake failed: {e}")))?;
            tls_stream.write_all(&payload).await?;
            tls_stream.flush().await?;
            let mut raw = Vec::new();
            tls_stream.read_to_end(&mut raw).await?;
            Ok::<Vec<u8>, AppError>(raw)
        } else {
            let mut stream = stream;
            stream.write_all(&payload).await?;
            stream.flush().await?;
            let mut raw = Vec::new();
            stream.read_to_end(&mut raw).await?;
            Ok(raw)
        }
    };

    let raw = tokio::time::timeout(timeout, exchange)
        .await
        .map_err(|_| {
            AppError::new(
                crate::error::ErrorKind::Timeout,
                format!("{} timed out after {}s", request.url, timeout.as_secs()),
            )
        })??;

    parse_response(&raw)
}

fn tls_connector() -> AppResult<tokio_rustls::TlsConnector> {
    use std::sync::Arc;
    let mut roots = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(tokio_rustls::TlsConnector::from(Arc::new(config)))
}

/// Parse a raw HTTP/1.1 response.
fn parse_response(raw: &[u8]) -> AppResult<Response> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| AppError::server("malformed HTTP response: no header terminator"))?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut body = raw[split + 4..].to_vec();

    let mut lines = head.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| AppError::server("malformed HTTP response: empty"))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| AppError::server(format!("malformed status line: {status_line}")))?;

    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    // Decode chunked transfer encoding, which AWS and Confluent both use.
    if headers
        .get("transfer-encoding")
        .map(|value| value.contains("chunked"))
        .unwrap_or(false)
    {
        body = decode_chunked(&body)?;
    }

    Ok(Response {
        status,
        headers,
        body,
    })
}

fn decode_chunked(body: &[u8]) -> AppResult<Vec<u8>> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    loop {
        let line_end = body[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| AppError::server("truncated chunked response"))?;
        let size_text = String::from_utf8_lossy(&body[cursor..line_end]);
        // Chunk extensions appear after a semicolon.
        let size_hex = size_text.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|_| AppError::server(format!("invalid chunk size {size_hex}")))?;
        cursor = line_end + 2;
        if size == 0 {
            break;
        }
        let end = cursor + size;
        if end > body.len() {
            return Err(AppError::server("truncated chunk body"));
        }
        out.extend_from_slice(&body[cursor..end]);
        cursor = end + 2; // trailing CRLF
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_url() {
        let (tls, host, port, path) =
            parse_url("https://elasticache.us-east-1.amazonaws.com:443/x?y=1").unwrap();
        assert!(tls);
        assert_eq!(host, "elasticache.us-east-1.amazonaws.com");
        assert_eq!(port, 443);
        assert_eq!(path, "/x?y=1");
    }

    #[test]
    fn defaults_ports_by_scheme() {
        let (_, _, port, path) = parse_url("http://example.com").unwrap();
        assert_eq!(port, 80);
        assert_eq!(path, "/");
    }

    #[test]
    fn rejects_unsupported_schemes() {
        assert!(parse_url("ftp://example.com").is_err());
        assert!(parse_url("example.com").is_err());
    }

    #[test]
    fn parses_a_plain_response() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"a\":1}";
        let response = parse_response(raw).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("content-type").unwrap(), "application/json");
        assert_eq!(response.json().unwrap()["a"], 1);
    }

    #[test]
    fn decodes_chunked_bodies() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"a\":1}\r\n0\r\n\r\n";
        let response = parse_response(raw).unwrap();
        assert_eq!(response.json().unwrap()["a"], 1);
    }

    #[test]
    fn non_success_becomes_an_error_with_the_provider_message() {
        let raw = b"HTTP/1.1 403 Forbidden\r\n\r\n{\"message\":\"not authorized\"}";
        let err = parse_response(raw).unwrap().into_result().unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::Unauthorized);
        assert_eq!(err.message, "not authorized");
    }

    #[test]
    fn basic_auth_header_is_well_formed() {
        let request = Request::get("https://example.com").basic("AKIA", "secret");
        let header = request.headers.get("Authorization").unwrap();
        assert!(header.starts_with("Basic "));
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(header.trim_start_matches("Basic "))
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "AKIA:secret");
    }
}
