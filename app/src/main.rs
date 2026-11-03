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
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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
    auth_client: auth::DynAuthClient,
    users_client: users::DynUsersClient,
    jwt_secret: Arc<String>,
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
        .merge(auth::auth_routes(auth_client, jwt_secret))
        .merge(users::users_routes(users_client))
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

    let (auth_client, users_client) = match PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Terhubung ke database PostgreSQL di {database_url}");
            let auth = build_client(pool.clone())
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
            (build_in_memory_client(), users::build_in_memory_client())
        }
    };

    let jwt_secret = Arc::new(std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "super_secret_jwt_key_that_is_at_least_32_characters_long!".to_string()
    }));

    let app = create_app(auth_client, users_client, jwt_secret);

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

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_swagger_ui_and_openapi_json() {
        let auth_client = auth::build_in_memory_client();
        let users_client = users::build_in_memory_client();
        let jwt_secret = Arc::new("test_secret_key_that_is_at_least_32_characters_long!".to_string());

        let app = create_app(auth_client, users_client, jwt_secret);

        // 1. Check openapi.json
        let req = Request::builder()
            .method("GET")
            .uri("/api-docs/openapi.json")
            .body(Body::empty())
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["info"]["title"], "Modular Monolith API");

        // Verify paths from auth and users modules are present
        let paths = json["paths"].as_object().unwrap();
        assert!(paths.contains_key("/api/auth/login"), "paths should contain /api/auth/login");
        assert!(paths.contains_key("/api/auth/logout"), "paths should contain /api/auth/logout");
        assert!(paths.contains_key("/api/users"), "paths should contain /api/users");
        assert!(paths.contains_key("/api/users/{id}"), "paths should contain /api/users/{{id}}");

        // Verify security scheme
        let schemes = json["components"]["securitySchemes"].as_object().unwrap();
        assert!(schemes.contains_key("bearer_auth"), "securitySchemes should contain bearer_auth");
        assert_eq!(schemes["bearer_auth"]["type"], "http");
        assert_eq!(schemes["bearer_auth"]["scheme"], "bearer");

        // 2. Check swagger-ui
        let req_ui = Request::builder()
            .method("GET")
            .uri("/swagger-ui/")
            .body(Body::empty())
            .unwrap();

        let res_ui = app.oneshot(req_ui).await.unwrap();
        assert_eq!(res_ui.status(), StatusCode::OK);
    }
}
