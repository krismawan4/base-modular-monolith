use std::net::SocketAddr;

use auth::build_client;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    // Inisialisasi logging
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .init();

    // Bootstrap DynAuthClient — satu instance, di-share via Arc
    let auth_client = build_client();

    // Middleware CORS — izinkan frontend Astro (localhost:4321) saat dev
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Rakit router utama dengan merge pattern sesuai gemini.md
    let app = Router::new()
        .merge(auth::http::auth_routes(auth_client))
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::info!("HTTP server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // TODO (Step 4): jalankan Tonic gRPC server serentak via tokio::select!
    // tokio::select! {
    //     _ = axum::serve(listener, app)    => {},
    //     _ = grpc::serve(auth_client_grpc) => {},
    // }
}
