use anyhow::Result;
use std::sync::Arc;
use tracing::info;
use axum::Router;
use tower_http::trace::TraceLayer;

mod db;
mod entities;
mod routes;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "trilium_rust=debug,tower_http=debug".to_string())
        )
        .init();

    info!("Starting Trilium Rust server v{}", env!("CARGO_PKG_VERSION"));

    // Initialize database
    let db = Arc::new(db::initialize().await?);
    info!("Database initialized successfully");

    // Create HTTP router
    let app = routes::create_router(db)
        .layer(TraceLayer::new_for_http());

    // Start server on port 8081 (8080 is used by Node.js version)
    let addr = std::env::var("TRILIUM_RUST_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8081".to_string());

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🚀 Server listening on http://{}", addr);
    info!("📝 Health check: http://{}/health", addr);
    info!("📚 API base: http://{}/api", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
