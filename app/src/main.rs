use std::net::SocketAddr;
use std::sync::Arc;

use auth::{build_client, build_in_memory_client};
use axum::{
    http::{header, HeaderValue, Method},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

async fn root() -> Json<Value> {
    Json(json!({
        "status": "success",
        "data": {
            "service": "modular-monolith-api",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:password@127.0.0.1:5432/modular_monolith".to_string()
    });

    let auth_client = match PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Terhubung ke database PostgreSQL di {database_url}");
            build_client(pool)
                .await
                .expect("Gagal menginisialisasi skema database Auth")
        }
        Err(err) => {
            tracing::warn!(
                "Tidak dapat terhubung ke PostgreSQL ({err}). Menjalankan dengan in-memory store sebagai fallback..."
            );
            tracing::info!("Tip: Jalankan `docker compose up -d postgres` untuk mengaktifkan database.");
            build_in_memory_client()
        }
    };

    let jwt_secret = Arc::new(std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "super_secret_jwt_key_that_is_at_least_32_characters_long!".to_string()
    }));

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_credentials(true);

    let app = Router::new()
        .route("/", get(root))
        .merge(auth::auth_routes(auth_client, jwt_secret))
        .layer(cors)
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http());

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Format HOST:PORT tidak valid");
    tracing::info!("HTTP server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
