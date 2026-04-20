use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use axum::{
    http::{header, Method},
    Router,
};
use gateway_server::{config::ServerConfig, routes::build_router, state::SharedState};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = ServerConfig::from_env();
    let state = SharedState::new(config.clone());
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::COOKIE,
            header::ORIGIN,
            header::HeaderName::from_static("x-pool-strategy"),
            header::HeaderName::from_static("x-goog-api-key"),
            header::HeaderName::from_static("x-api-key"),
        ]);
    let app = build_router(state).layer(cors);

    let listener = bind_in_range(&config).await?;
    let addr = listener.local_addr()?;
    info!("gateway-server started at http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn bind_in_range(config: &ServerConfig) -> Result<TcpListener> {
    for port in config.port_start..=config.port_end {
        let addr = format!("{}:{port}", config.bind_host);
        match TcpListener::bind(&addr).await {
            Ok(listener) => return Ok(listener),
            Err(e) => {
                error!("port {port} unavailable: {e}");
                continue;
            }
        }
    }
    Err(anyhow!(
        "no available ports in {}-{}",
        config.port_start,
        config.port_end
    ))
}

#[allow(dead_code)]
fn _app_router(app: Router) -> Router {
    app
}

#[allow(dead_code)]
fn _addr(addr: SocketAddr) -> SocketAddr {
    addr
}
