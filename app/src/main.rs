use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{header, HeaderValue, Method},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

mod graphql;

async fn root(Extension(system_info): Extension<Arc<graphql::SystemInfo>>) -> Json<Value> {
    Json(json!({
        "status": "success",
        "data": {
            "service": "modular-monolith-api",
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": system_info.architecture,
            "users_mode": system_info.users_mode,
            "graphql_playground": "/graphql"
        }
    }))
}

pub fn create_app(
    auth_client:      auth::DynAuthClient,
    users_client:     users::DynUsersClient,
    jwt_secret:       Arc<String>,
    _token_blacklist: auth::DynTokenBlacklist,
    system_info:      Arc<graphql::SystemInfo>,
) -> Router {
    let cors_origin = std::env::var("CORS_ALLOWED_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".to_string());
    let origin_header = cors_origin
        .parse::<HeaderValue>()
        .unwrap_or_else(|_| HeaderValue::from_static("http://localhost:5173"));

    let cors = CorsLayer::new()
        .allow_origin(origin_header)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_credentials(true);

    let x_request_id = axum::http::HeaderName::from_static("x-request-id");
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        );

    Router::new()
        .route("/", get(root))
        .merge(graphql::graphql_routes(users_client, auth_client, jwt_secret, system_info.clone()))
        .layer(Extension(system_info))
        .layer(cors)
        .layer(trace_layer)
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    if log_format.eq_ignore_ascii_case("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            )
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            )
            .init();
    }

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL wajib disetel di dalam file .env");

    let (auth_client, local_users_client) = match PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Terhubung ke database PostgreSQL di {database_url}");
            let auth = auth::build_client(pool.clone())
                .await
                .expect("Gagal menginisialisasi skema database Auth");
            let users = users::build_client(pool)
                .await
                .expect("Gagal menginisialisasi skema database Users");
            (auth, users)
        }
        Err(err) => {
            tracing::warn!(
                "Tidak dapat terhubung ke PostgreSQL ({err}). Menjalankan dengan in-memory store sebagai fallback..."
            );
            tracing::info!("Tip: Jalankan `docker compose up -d postgres` untuk mengaktifkan database.");
            (auth::build_in_memory_client(), users::build_in_memory_client())
        }
    };

    let (users_client, users_mode, architecture) = if let Ok(grpc_url) = std::env::var("USERS_GRPC_URL") {
        tracing::info!("Menggunakan Users Microservice via gRPC di {grpc_url}");
        let client = users::build_grpc_client(grpc_url.clone())
            .await
            .expect("Gagal terhubung ke Users gRPC Microservice");
        (client, format!("Microservice via gRPC ({grpc_url})"), "Microservice (Hybrid)".to_string())
    } else {
        (local_users_client, "In-Process (Modular Monolith)".to_string(), "Modular Monolith".to_string())
    };

    let system_info = Arc::new(graphql::SystemInfo {
        architecture,
        users_mode,
    });

    let jwt_secret = Arc::new(std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "super_secret_jwt_key_that_is_at_least_32_characters_long!".to_string()
    }));

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        "redis://127.0.0.1:6379".to_string()
    });

    let token_blacklist = match auth::build_redis_blacklist(&redis_url).await {
        Ok(bl) => {
            tracing::info!("Terhubung ke Redis di {redis_url}");
            bl
        }
        Err(err) => {
            tracing::warn!(
                "Tidak dapat terhubung ke Redis ({err}). Menjalankan dengan in-memory blacklist sebagai fallback..."
            );
            tracing::info!("Tip: Jalankan `docker compose up -d redis` untuk mengaktifkan Redis server.");
            auth::build_in_memory_blacklist()
        }
    };

    let app = create_app(auth_client, users_client, jwt_secret, token_blacklist, system_info);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Format HOST:PORT tidak valid");
    tracing::info!("HTTP server listening on http://{addr}");
    tracing::info!("GraphQL Playground available at http://{addr}/graphql");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;
