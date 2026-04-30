use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use reqwest::Client;

use craterun_bundle::manifest::compiled_manifest::CompiledManifest;
use craterun_runtime::types::RuntimePlan;

pub fn allocate_proxy_port() -> Result<u16, std::io::Error> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

struct ProxyRoute {
    path_prefix: String,
    target_host: String,
    target_port: u16,
}

pub struct ReverseProxy {
    routes: Vec<ProxyRoute>,
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
                    target_host: "127.0.0.1".to_string(),
                    target_port: planned.host_port,
                })
            })
            .collect();

        Self { routes }
    }

    pub async fn run(self, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let routes = std::sync::Arc::new(self.routes);
        let client = Client::new();

        let routes_clone = routes.clone();
        let client_clone = client.clone();

        let app = Router::new().fallback(any(move |req: Request| {
            let routes = routes_clone.clone();
            let client = client_clone.clone();
            async move { proxy_handler(req, &routes, &client).await }
        }));

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("Reverse proxy listening on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn proxy_handler(
    req: Request,
    routes: &[ProxyRoute],
    client: &Client,
) -> Response {
    let path = req.uri().path();

    let matched = routes
        .iter()
        .filter(|r| path.starts_with(&r.path_prefix))
        .max_by_key(|r| r.path_prefix.len());

    let route = match matched {
        Some(r) => r,
        None => {
            return (
                axum::http::StatusCode::BAD_GATEWAY,
                "no matching route",
            )
                .into_response();
        }
    };

    let stripped = if route.path_prefix == "/" {
        path.to_string()
    } else {
        path.strip_prefix(&route.path_prefix)
            .unwrap_or(path)
            .to_string()
    };

    let target_path = if stripped.is_empty() || !stripped.starts_with('/') {
        format!("/{}", stripped)
    } else {
        stripped
    };

    let target_url = format!(
        "http://{}:{}{}",
        route.target_host, route.target_port, target_path
    );

    let method = req.method().clone();
    let headers = req.headers().clone();

    let mut proxy_req = client.request(method, &target_url);
    for (key, val) in headers.iter() {
        if key != "host" {
            proxy_req = proxy_req.header(key, val);
        }
    }

    match proxy_req.send().await {
        Ok(resp) => {
            let status = axum::http::StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(axum::http::StatusCode::BAD_GATEWAY);
            let mut builder = Response::builder().status(status);
            for (key, val) in resp.headers().iter() {
                builder = builder.header(key, val);
            }
            let body = resp.bytes().await.unwrap_or_default();
            builder.body(Body::from(body)).unwrap_or_else(|_| {
                (axum::http::StatusCode::BAD_GATEWAY, "proxy error").into_response()
            })
        }
        Err(e) => {
            tracing::error!("proxy request failed: {}", e);
            (
                axum::http::StatusCode::BAD_GATEWAY,
                format!("upstream error: {}", e),
            )
                .into_response()
        }
    }
}
