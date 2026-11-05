//! AWS ElastiCache discovery.
//!
//! Uses the Query API (`Action=DescribeReplicationGroups`), which returns XML.
//! Serverless caches get their own call because they report an endpoint under a
//! different shape.

use std::collections::BTreeMap;

use crate::cloud::aws::{self, sigv4};
use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::config::Endpoint;
use crate::error::AppResult;

const SERVICE: &str = "elasticache";
const API_VERSION: &str = "2015-02-02";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    aws::require_keys(credentials)?;
    let region = aws::region(credentials);
    let mut out = discover_clusters(credentials, &region, timeout).await?;
    // Serverless is a separate API and is not available in every region; a
    // failure there must not hide the cluster results we already have.
    match discover_serverless(credentials, &region, timeout).await {
        Ok(mut serverless) => out.append(&mut serverless),
        Err(err) if err.kind == crate::error::ErrorKind::InvalidInput => {}
        Err(_) => {}
    }
    Ok(out)
}

fn endpoint_url(region: &str) -> String {
    format!("https://elasticache.{region}.amazonaws.com/")
}

async fn signed_query(
    credentials: &CloudCredentials,
    region: &str,
    body: &str,
    timeout: std::time::Duration,
) -> AppResult<String> {
    let url = endpoint_url(region);
    let signed = sigv4::sign(sigv4::SignRequest {
        method: "POST",
        url: &url,
        region,
        service: SERVICE,
        access_key_id: &credentials.access_key_id,
        secret_access_key: &credentials.secret_access_key,
        session_token: credentials.session_token.as_deref(),
        body: body.as_bytes(),
        headers: vec![(
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        )],
        timestamp: chrono::Utc::now().timestamp(),
    })?;

    let mut request = crate::cloud::http::Request::post(&url, body.as_bytes().to_vec())
        .header("Content-Type", "application/x-www-form-urlencoded");
    for (key, value) in signed.headers {
        request = request.header(key, value);
    }

    let response = crate::cloud::http::execute(&request, timeout).await?;
    if !response.is_success() {
        return Err(aws::aws_error(
            &String::from_utf8_lossy(&response.body),
            response.status,
        ));
    }
    Ok(String::from_utf8_lossy(&response.body).to_string())
}

async fn discover_clusters(
    credentials: &CloudCredentials,
    region: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let body = format!("Action=DescribeReplicationGroups&Version={API_VERSION}");
    let xml = signed_query(credentials, region, &body, timeout).await?;
    Ok(parse_replication_groups(&xml, region))
}

async fn discover_serverless(
    credentials: &CloudCredentials,
    region: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let body = format!("Action=DescribeServerlessCaches&Version={API_VERSION}");
    let xml = signed_query(credentials, region, &body, timeout).await?;
    Ok(parse_serverless_caches(&xml, region))
}

/// Parse `DescribeReplicationGroups` output.
///
/// Structure, per group:
/// `<ReplicationGroup><ReplicationGroupId/><NodeGroups><NodeGroup>
///   <PrimaryEndpoint><Address/><Port/></PrimaryEndpoint>
/// </NodeGroup></NodeGroups></ReplicationGroup>`
fn parse_replication_groups(xml: &str, region: &str) -> Vec<DiscoveredResource> {
    let mut out = Vec::new();

    for (_, block) in children(xml, "ReplicationGroups") {
        let id = aws::extract_tag(&block, "ReplicationGroupId").unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let description = aws::extract_tag(&block, "Description").unwrap_or_else(|| id.clone());
        let engine = aws::extract_tag(&block, "Engine").unwrap_or_else(|| "redis".into());
        let transit = aws::extract_tag(&block, "TransitEncryptionEnabled")
            .map(|value| value == "true")
            .unwrap_or(false);
        let status = aws::extract_tag(&block, "Status").unwrap_or_default();

        let mut endpoints = Vec::new();
        // Primary endpoint of each node group; the first is the writable one.
        for (_, node_group) in children(&block, "NodeGroups") {
            if let Some(endpoint) = parse_endpoint_block(&node_group) {
                endpoints.push(endpoint);
            }
        }

        let mut meta = BTreeMap::new();
        meta.insert("engine".into(), engine.clone());
        if !status.is_empty() {
            meta.insert("status".into(), status);
        }
        meta.insert(
            "transit_encryption".into(),
            if transit { "on" } else { "off" }.into(),
        );

        out.push(aws::resource(
            format!("arn:aws:elasticache:{region}:replicationgroup:{id}"),
            description,
            "aws_elasticache",
            crate::config::BackendKind::Redis,
            region.to_string(),
            endpoints,
            // ElastiCache only enables in-transit encryption when configured;
            // requiring TLS when it is off would make every connection fail.
            transit,
            None,
            meta,
        ));
    }
    out
}

fn parse_serverless_caches(xml: &str, region: &str) -> Vec<DiscoveredResource> {
    let mut out = Vec::new();
    for (_, block) in children(xml, "ServerlessCaches") {
        let Some(name) = aws::extract_tag(&block, "ServerlessCacheName") else {
            continue;
        };
        let mut endpoints = Vec::new();
        if let Some(endpoint) = parse_endpoint_block(&block) {
            endpoints.push(endpoint);
        }
        let mut meta = BTreeMap::new();
        meta.insert("kind".into(), "serverless".into());
        if let Some(engine) = aws::extract_tag(&block, "Engine") {
            meta.insert("engine".into(), engine);
        }
        out.push(aws::resource(
            format!("arn:aws:elasticache:{region}:serverless:{name}"),
            name,
            "aws_elasticache",
            crate::config::BackendKind::Redis,
            region.to_string(),
            endpoints,
            true,
            None,
            meta,
        ));
    }
    out
}

/// Read `<Address>`/`<Port>` from a block that contains an endpoint.
fn parse_endpoint_block(block: &str) -> Option<Endpoint> {
    // The endpoint may be wrapped in `<PrimaryEndpoint>`/`<ReaderEndpoint>` or
    // appear directly (serverless). Prefer the primary when present.
    let scope = aws::extract_tag(block, "PrimaryEndpoint")
        .map(|inner| inner)
        .or_else(|| {
            aws::extract_tag(block, "Endpoint").map(|inner| inner)
        })
        .unwrap_or_else(|| block.to_string());

    let address = aws::extract_tag(&scope, "Address")?;
    let port = aws::extract_tag(&scope, "Port")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(6379);
    if address.trim().is_empty() {
        return None;
    }
    Some(Endpoint::new(address, port))
}

/// Direct child elements of `container`, as `(tag, inner_xml)` pairs.
///
/// The AWS Query API is inconsistent about list encoding: some services wrap
/// items in `<member>`, others in a named element (`<NodeGroup>`,
/// `<ServerlessCache>`). Scanning for direct children handles both without
/// guessing which convention a given service uses.
///
/// Nesting is tracked by depth, because a `<member>` can contain another
/// `<member>` (`NodeGroups > member > PrimaryEndpoint`), and a naive search for
/// the next closing tag would match the inner one.
fn children(xml: &str, container: &str) -> Vec<(String, String)> {
    let Some(inner) = aws::extract_tag(xml, container) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut cursor = 0usize;

    while cursor < inner.len() {
        let Some(open_rel) = inner[cursor..].find('<') else {
            break;
        };
        let open = cursor + open_rel;

        // Skip closing tags, comments, CDATA and processing instructions at
        // this level: only opening tags start a child.
        if inner[open..].starts_with("</")
            || inner[open..].starts_with("<!")
            || inner[open..].starts_with("<?")
        {
            cursor = open + 1;
            continue;
        }

        let Some(name_end_rel) = inner[open..].find(|c: char| c == '>' || c.is_whitespace())
        else {
            break;
        };
        let name_end = open + name_end_rel;
        let tag = inner[open + 1..name_end].to_string();

        // Self-closing element: it has no content.
        if inner[name_end..].starts_with("/>") {
            out.push((tag, String::new()));
            cursor = name_end + 2;
            continue;
        }

        let Some(gt_rel) = inner[name_end..].find('>') else {
            break;
        };
        let content_start = name_end + gt_rel + 1;

        // Walk forward counting same-name open/close tags so the match lands on
        // the *matching* close, not the first one.
        let open_tag = format!("<{tag}>");
        let close_tag = format!("</{tag}>");
        let mut depth = 1usize;
        let mut scan = content_start;
        let content_end = loop {
            let next_open = inner[scan..].find(&open_tag).map(|offset| (scan + offset, true));
            let next_close = inner[scan..].find(&close_tag).map(|offset| (scan + offset, false));
            let Some((position, is_open)) = (match (next_open, next_close) {
                (Some((open_at, _)), Some((close_at, _))) => Some(if open_at < close_at {
                    (open_at, true)
                } else {
                    (close_at, false)
                }),
                (Some(value), None) => Some(value),
                (None, Some(value)) => Some(value),
                (None, None) => None,
            }) else {
                break content_start;
            };

            if is_open {
                depth += 1;
                scan = position + open_tag.len();
            } else {
                depth -= 1;
                if depth == 0 {
                    break position;
                }
                scan = position + close_tag.len();
            }
        };

        out.push((tag.clone(), inner[content_start..content_end].to_string()));
        cursor = content_end + close_tag.len();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<DescribeReplicationGroupsResponse>
      <DescribeReplicationGroupsResult>
        <ReplicationGroups>
          <member>
            <ReplicationGroupId>orders-cache</ReplicationGroupId>
            <Description>orders primary cache</Description>
            <Status>available</Status>
            <Engine>redis</Engine>
            <TransitEncryptionEnabled>true</TransitEncryptionEnabled>
            <NodeGroups>
              <member>
                <PrimaryEndpoint>
                  <Address>orders-cache.abc.0001.euw1.cache.amazonaws.com</Address>
                  <Port>6379</Port>
                </PrimaryEndpoint>
              </member>
            </NodeGroups>
          </member>
          <member>
            <ReplicationGroupId>sessions</ReplicationGroupId>
            <Description>session store</Description>
            <Engine>redis</Engine>
            <TransitEncryptionEnabled>false</TransitEncryptionEnabled>
            <NodeGroups>
              <member>
                <PrimaryEndpoint>
                  <Address>sessions.def.0001.use1.cache.amazonaws.com</Address>
                  <Port>6380</Port>
                </PrimaryEndpoint>
              </member>
            </NodeGroups>
          </member>
        </ReplicationGroups>
      </DescribeReplicationGroupsResult>
    </DescribeReplicationGroupsResponse>"#;

    #[test]
    fn parses_replication_groups() {
        let found = parse_replication_groups(SAMPLE, "eu-west-1");
        assert_eq!(found.len(), 2);

        let orders = &found[0];
        assert_eq!(orders.name, "orders primary cache");
        assert_eq!(orders.endpoints.len(), 1);
        assert_eq!(orders.endpoints[0].port, 6379);
        assert!(orders.tls, "transit encryption should enable TLS");
        assert_eq!(orders.meta.get("engine").map(String::as_str), Some("redis"));

        let sessions = &found[1];
        assert_eq!(sessions.endpoints[0].port, 6380);
        assert!(!sessions.tls);
    }

    #[test]
    fn tolerates_an_empty_response() {
        let found = parse_replication_groups("<ReplicationGroups></ReplicationGroups>", "us-east-1");
        assert!(found.is_empty());
    }

    #[test]
    fn skips_groups_without_an_endpoint() {
        let xml = "<ReplicationGroups><member><ReplicationGroupId>creating</ReplicationGroupId></member></ReplicationGroups>";
        let found = parse_replication_groups(xml, "us-east-1");
        assert_eq!(found.len(), 1);
        assert!(found[0].endpoints.is_empty());
    }

    #[test]
    fn parses_serverless_endpoints() {
        let xml = r#"<ServerlessCaches>
          <member>
            <ServerlessCacheName>edge-cache</ServerlessCacheName>
            <Endpoint><Address>edge-cache.serverless.use1.cache.amazonaws.com</Address><Port>6379</Port></Endpoint>
            <Engine>redis</Engine>
          </member>
        </ServerlessCaches>"#;
        let found = parse_serverless_caches(xml, "us-east-1");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "edge-cache");
        assert_eq!(found[0].endpoints[0].port, 6379);
        assert!(found[0].tls);
    }
}

