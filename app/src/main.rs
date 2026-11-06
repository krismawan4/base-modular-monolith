use std::net::SocketAddr;
use std::sync::Arc;

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
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod graphql;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Modular Monolith API",
        version = "0.1.0",
        description = "Modular Monolith Rust REST API with Axum, RBAC Auth, and Users Management"
    )
)]
struct ApiDoc;

async fn root() -> Json<Value> {
    Json(json!({
        "status": "success",
        "data": {
            "service": "modular-monolith-api",
            "version": env!("CARGO_PKG_VERSION")
        }
    }))
}

pub fn create_app(
    auth_client:     auth::DynAuthClient,
    users_client:    users::DynUsersClient,
    jwt_secret:      Arc<String>,
    token_blacklist: auth::DynTokenBlacklist,
) -> Router {
    let mut api_doc = ApiDoc::openapi();
    api_doc.merge(auth::AuthApiDoc::openapi());
    api_doc.merge(users::UsersApiDoc::openapi());

    let components = api_doc.components.get_or_insert_with(Default::default);
    components.add_security_scheme(
        "bearer_auth",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .description(Some("Masukkan JWT Bearer Token (contoh: eyJhbGciOi...)"))
                .build(),
        ),
    );

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_credentials(true);

    Router::new()
        .route("/", get(root))
        .merge(auth::auth_routes(auth_client, jwt_secret, token_blacklist))
        .merge(users::users_routes(users_client.clone()))
        .merge(graphql::graphql_routes(users_client))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api_doc))
        .layer(cors)
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http())
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

    let users_client = if let Ok(grpc_url) = std::env::var("USERS_GRPC_URL") {
        tracing::info!("Menggunakan Users Microservice via gRPC di {grpc_url}");
        users::build_grpc_client(grpc_url)
            .await
            .expect("Gagal terhubung ke Users gRPC Microservice")
    } else {
        local_users_client
    };


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

    let app = create_app(auth_client, users_client, jwt_secret, token_blacklist);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("Format HOST:PORT tidak valid");
    tracing::info!("HTTP server listening on http://{addr}");
    tracing::info!("Swagger UI documentation available at http://{addr}/swagger-ui/");
    tracing::info!("GraphQL Playground available at http://{addr}/graphql");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;

