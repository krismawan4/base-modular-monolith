use std::net::SocketAddr;

use auth::{build_client, build_in_memory_client};
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

async fn root() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "modular-monolith-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() {
    // Inisialisasi logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .init();

    // Koneksi Database PostgreSQL via sqlx
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:password@127.0.0.1:5432/modular_monolith".to_string()
    });

    let auth_client = match sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
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

    // Middleware CORS — izinkan frontend Astro (localhost:4321) saat dev
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Rakit router utama dengan merge pattern sesuai gemini.md
    let app = Router::new()
        .route("/", get(root))
        .merge(auth::http::auth_routes(auth_client))
        .layer(cors)
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

    // TODO (Step 4): jalankan Tonic gRPC server serentak via tokio::select!
    // tokio::select! {
    //     _ = axum::serve(listener, app)    => {},
    //     _ = grpc::serve(auth_client_grpc) => {},
    // }
}
