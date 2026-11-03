use std::net::SocketAddr;
use std::sync::Arc;

use auth::{build_client, build_in_memory_client};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_cookies::{cookie::SameSite, Cookie, CookieManagerLayer, Cookies};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod middleware;
use middleware::{create_jwt, AuthSource, AuthUser};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<String>,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device_name: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct UserRecord {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
}

async fn root() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "modular-monolith-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn login_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    payload: Result<Json<LoginPayload>, JsonRejection>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let Json(payload) = payload.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid request payload",
                "details": e.body_text()
            })),
        )
    })?;
    let user = sqlx::query_as::<_, UserRecord>(
        "SELECT id, COALESCE(name, 'User') as name, email, password_hash FROM users WHERE email = $1",
    )
    .bind(&payload.email)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Database error", "details": e.to_string() })),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid email or password" })),
        )
    })?;

    let hash = user.password_hash.as_deref().unwrap_or("");
    let is_valid = bcrypt::verify(&payload.password, hash).unwrap_or(false);
    if !is_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid email or password" })),
        ));
    }

    let token = create_jwt(&user.id, &user.email, &state.jwt_secret).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Token creation error", "details": e.to_string() })),
        )
    })?;

    if let Some(device) = payload.device_name.filter(|d| !d.trim().is_empty()) {
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "token_type": "Bearer",
                "token": token,
                "device_name": device,
                "user": {
                    "id": user.id,
                    "name": user.name,
                    "email": user.email
                }
            })),
        )
            .into_response())
    } else {
        let mut auth_cookie = Cookie::new("auth_token", token);
        auth_cookie.set_http_only(true);
        auth_cookie.set_same_site(SameSite::Lax);
        auth_cookie.set_path("/");
        auth_cookie.set_secure(false);

        cookies.add(auth_cookie);

        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Authenticated via HttpOnly Cookie",
                "user": {
                    "id": user.id,
                    "name": user.name,
                    "email": user.email
                }
            })),
        )
            .into_response())
    }
}

async fn logout_handler(cookies: Cookies) -> Json<Value> {
    let mut auth_cookie = Cookie::from("auth_token");
    auth_cookie.set_path("/");
    cookies.remove(auth_cookie);

    Json(json!({
        "status": "success",
        "message": "Logged out successfully"
    }))
}

async fn dashboard_handler(
    auth_user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user = sqlx::query_as::<_, UserProfile>(
        "SELECT id, COALESCE(name, 'User') as name, email FROM users WHERE id = $1",
    )
    .bind(&auth_user.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Database error", "details": e.to_string() })),
        )
    })?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "User profile not found" })),
        )
    })?;

    Ok(Json(json!({
        "status": "success",
        "client_channel": match auth_user.auth_source {
            AuthSource::Cookie => "Web SPA (Cookie)",
            AuthSource::Bearer => "Mobile (Bearer Token)",
        },
        "user": user,
        "dashboard_stats": {
            "unread_notifications": 3,
            "account_health": "Optimal"
        }
    })))
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id VARCHAR(36) PRIMARY KEY,
            name VARCHAR(255) NOT NULL DEFAULT 'User',
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255),
            role_name VARCHAR(50) DEFAULT 'member',
            token VARCHAR(255) DEFAULT '',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS name VARCHAR(255) NOT NULL DEFAULT 'User'")
        .execute(pool)
        .await?;

    sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255)")
        .execute(pool)
        .await?;

    let user_exists = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind("admin@example.com")
        .fetch_optional(pool)
        .await?;

    if user_exists.is_none() {
        let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let user_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO users (id, name, email, password_hash, role_name, token) 
             VALUES ($1, $2, $3, $4, 'admin', $5)",
        )
        .bind(&user_id)
        .bind("Admin Developer")
        .bind("admin@example.com")
        .bind(&password_hash)
        .bind(&user_id)
        .execute(pool)
        .await?;

        tracing::info!(
            "Seeded default user: admin@example.com / password123 (id: {})",
            user_id
        );
    }

    Ok(())
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

    let (pool, auth_client) = match PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            tracing::info!("Terhubung ke database PostgreSQL di {database_url}");
            if let Err(e) = init_db(&pool).await {
                tracing::warn!("Gagal inisialisasi tabel users: {e}");
            }
            let client = build_client(pool.clone())
                .await
                .expect("Gagal menginisialisasi skema database Auth");
            (pool, client)
        }
        Err(err) => {
            tracing::warn!(
                "Tidak dapat terhubung ke PostgreSQL ({err}). Menjalankan dengan in-memory store sebagai fallback..."
            );
            tracing::info!("Tip: Jalankan `docker compose up -d postgres` untuk mengaktifkan database.");
            let fallback_pool = PgPoolOptions::new()
                .connect_lazy(&database_url)
                .expect("Failed to create lazy pool");
            (fallback_pool, build_in_memory_client())
        }
    };

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        "super_secret_jwt_key_that_is_at_least_32_characters_long!".to_string()
    });

    let state = AppState {
        pool,
        jwt_secret: Arc::new(jwt_secret),
    };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
        ])
        .allow_credentials(true);

    let api_routes = Router::new()
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/api/dashboard", get(dashboard_handler))
        .with_state(state);

    let app = Router::new()
        .route("/", get(root))
        .merge(api_routes)
        .merge(auth::http::auth_routes(auth_client))
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
