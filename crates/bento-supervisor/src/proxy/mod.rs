//! Local reverse proxy that maps path prefixes to container host ports.
//! The user never sees individual service ports — only this single proxy URL.

use std::net::SocketAddr;
use std::sync::Arc;

use bento_bundle::manifest::compiled_manifest::CompiledManifest;
use bento_runtime::types::RuntimePlan;

pub fn allocate_proxy_port() -> Result<u16, std::io::Error> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

struct ProxyRoute {
    path_prefix: String,
    target_port: u16,
}

pub struct ReverseProxy {
    routes: Arc<Vec<ProxyRoute>>,
}

impl ReverseProxy {
    pub fn new(manifest: &CompiledManifest, plan: &RuntimePlan) -> Self {
        let routes: Vec<ProxyRoute> = manifest
            .routes
            .iter()
            .filter_map(|route| {
                let planned = plan.services.iter().find(|s| s.name == route.service)?;
                Some(ProxyRoute {
                    path_prefix: route.path.clone(),
                    target_port: planned.host_port,
                })
            })
            .collect();

        Self {
            routes: Arc::new(routes),
        }
    }

    pub async fn run(self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("Reverse proxy listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let routes = self.routes.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, &routes).await {
                    tracing::debug!("proxy connection error: {}", e);
                }
            });
        }
    }
}

/// Handle a single TCP connection by reading the HTTP request,
/// matching a route, forwarding to the upstream container, and
/// relaying the response byte-for-byte.
async fn handle_connection(
    mut client_stream: tokio::net::TcpStream,
    routes: &[ProxyRoute],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Read the request (enough to parse the first line for path matching)
    let mut buf = vec![0u8; 8192];
    let n = client_stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let request_bytes = &buf[..n];

    // Parse the request path from the first line (e.g. "GET /api/health HTTP/1.1")
    let first_line = request_bytes
        .split(|&b| b == b'\n')
        .next()
        .unwrap_or(request_bytes);
    let first_line_str = String::from_utf8_lossy(first_line);
    let path = first_line_str
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    // Match the longest path prefix
    let route = routes
        .iter()
        .filter(|r| path.starts_with(&r.path_prefix))
        .max_by_key(|r| r.path_prefix.len());

    let route = match route {
        Some(r) => r,
        None => {
            let resp = b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 16\r\n\r\nno matching route";
            client_stream.write_all(resp).await?;
            return Ok(());
        }
    };

    tracing::debug!(
        "proxy: {} -> 127.0.0.1:{} (matched prefix '{}')",
        path,
        route.target_port,
        route.path_prefix,
    );

    // Rewrite the path: strip the prefix for non-root routes
    let upstream_path = if route.path_prefix == "/" {
        path.to_string()
    } else {
        let stripped = path.strip_prefix(&route.path_prefix).unwrap_or(path);
        if stripped.is_empty() || !stripped.starts_with('/') {
            format!("/{}", stripped)
        } else {
            stripped.to_string()
        }
    };

    // Rewrite the request line to use the upstream path
    let mut rewritten = Vec::new();
    let parts: Vec<&str> = first_line_str.trim().splitn(3, ' ').collect();
    if parts.len() == 3 {
        rewritten.extend_from_slice(
            format!("{} {} {}\r\n", parts[0], upstream_path, parts[2]).as_bytes(),
        );
    } else {
        rewritten.extend_from_slice(first_line);
        rewritten.extend_from_slice(b"\r\n");
    }

    // Append remaining headers, replacing any Connection header with "close".
    // This prevents HTTP keep-alive from reusing a connection across different
    // route targets — each request must get its own connection and route match.
    let rest = &request_bytes[first_line.len()..];
    let rest = if rest.starts_with(b"\r\n") {
        &rest[2..]
    } else if rest.starts_with(b"\n") {
        &rest[1..]
    } else {
        rest
    };

    let rest_str = String::from_utf8_lossy(rest);
    let mut wrote_connection = false;
    for line in rest_str.split("\r\n") {
        if line.is_empty() {
            if !wrote_connection {
                rewritten.extend_from_slice(b"Connection: close\r\n");
            }
            rewritten.extend_from_slice(b"\r\n");
            break;
        }
        if line.to_lowercase().starts_with("connection:") {
            rewritten.extend_from_slice(b"Connection: close\r\n");
            wrote_connection = true;
        } else {
            rewritten.extend_from_slice(line.as_bytes());
            rewritten.extend_from_slice(b"\r\n");
        }
    }

    // Append any body after the headers
    if let Some(body_start) = rest_str.find("\r\n\r\n") {
        let body = &rest[body_start + 4..];
        if !body.is_empty() {
            rewritten.extend_from_slice(body);
        }
    }

    // Connect to the upstream container
    let upstream_addr = format!("127.0.0.1:{}", route.target_port);
    let mut upstream = match tokio::net::TcpStream::connect(&upstream_addr).await {
        Ok(s) => s,
        Err(e) => {
            let body = format!("upstream connect error: {}", e);
            let resp = format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            client_stream.write_all(resp.as_bytes()).await?;
            return Ok(());
        }
    };

    // Forward the request
    upstream.write_all(&rewritten).await?;

    // Relay the response back — simple bidirectional copy
    tokio::io::copy_bidirectional(&mut client_stream, &mut upstream).await?;

    Ok(())
}
