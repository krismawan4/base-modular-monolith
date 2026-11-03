//! # Auth HTTP Handlers
//!
//! Handler Axum untuk endpoint REST modul Auth.
//! Fungsi `auth_routes()` dipanggil di `app/src/main.rs` dan
//! di-merge ke router utama.

use std::sync::Arc;

use axum::{
    extract::{rejection::JsonRejection, FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_cookies::{cookie::SameSite, Cookie, Cookies};

use crate::{
    middleware::{create_jwt, AuthSource, AuthUser},
    AuthError, DynAuthClient, UserDto,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AuthState {
    pub client: DynAuthClient,
    pub jwt_secret: Arc<String>,
}

impl FromRef<AuthState> for DynAuthClient {
    fn from_ref(state: &AuthState) -> Self {
        state.client.clone()
    }
}

// ---------------------------------------------------------------------------
// Response shapes (JSON-serializable wrappers atas DTO publik)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PermissionResponse {
    name: String,
}

#[derive(Serialize)]
struct RoleResponse {
    name:        String,
    permissions: Vec<PermissionResponse>,
}

#[derive(Serialize)]
struct UserResponse {
    id:    String,
    name:  String,
    email: String,
    role:  RoleResponse,
}

impl From<UserDto> for UserResponse {
    fn from(u: UserDto) -> Self {
        UserResponse {
            id:    u.id,
            name:  u.name,
            email: u.email,
            role:  RoleResponse {
                name: u.role.name,
                permissions: u
                    .role
                    .permissions
                    .into_iter()
                    .map(|p| PermissionResponse { name: p.name })
                    .collect(),
            },
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn auth_error_to_response(e: AuthError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match &e {
        AuthError::UserNotFound       => StatusCode::NOT_FOUND,
        AuthError::RoleNotFound       => StatusCode::UNPROCESSABLE_ENTITY,
        AuthError::TokenInvalid       => StatusCode::UNAUTHORIZED,
        AuthError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthError::DuplicateUser      => StatusCode::CONFLICT,
        AuthError::Internal(_)        => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ErrorResponse { error: e.to_string() }))
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device_name: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub email:     String,
    pub role_name: String,
}

#[derive(Deserialize)]
pub struct AssignPermissionBody {
    pub permission_name: String,
}

#[derive(Deserialize)]
pub struct ValidateTokenBody {
    pub token: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/login and /api/auth/login
async fn login_handler(
    State(state): State<AuthState>,
    cookies: Cookies,
    payload: Result<Json<LoginPayload>, JsonRejection>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let Json(payload) = payload.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "error": "Invalid request payload",
                "details": e.body_text()
            })),
        )
    })?;

    let user = state
        .client
        .authenticate_user(&payload.email, &payload.password)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid email or password" })),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Authentication error", "details": e.to_string() })),
            ),
        })?;

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

/// POST /api/logout and /api/auth/logout
async fn logout_handler(cookies: Cookies) -> Json<Value> {
    let mut auth_cookie = Cookie::from("auth_token");
    auth_cookie.set_path("/");
    cookies.remove(auth_cookie);

    Json(json!({
        "status": "success",
        "message": "Logged out successfully"
    }))
}

/// GET /api/dashboard and /api/auth/dashboard
async fn dashboard_handler(
    auth_user: AuthUser,
    State(state): State<AuthState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let user = state
        .client
        .get_user_by_id(&auth_user.user_id)
        .await
        .map_err(|e| match e {
            AuthError::UserNotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "User profile not found" })),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database error", "details": e.to_string() })),
            ),
        })?;

    Ok(Json(json!({
        "status": "success",
        "client_channel": match auth_user.auth_source {
            AuthSource::Cookie => "Web SPA (Cookie)",
            AuthSource::Bearer => "Mobile (Bearer Token)",
        },
        "user": {
            "id": user.id,
            "name": user.name,
            "email": user.email
        },
        "dashboard_stats": {
            "unread_notifications": 3,
            "account_health": "Optimal"
        }
    })))
}

/// POST /api/auth/users
/// Body: `{ "email": "...", "role_name": "admin" }`
async fn create_user(
    State(client): State<DynAuthClient>,
    Json(body):    Json<CreateUserBody>,
) -> impl IntoResponse {
    match client.create_user(&body.email, &body.role_name).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(UserResponse::from(user)).unwrap()),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, Json(serde_json::to_value(body.0).unwrap())).into_response()
        }
    }
}

/// GET /api/auth/users/:id
async fn get_user_by_id(
    State(client): State<DynAuthClient>,
    Path(user_id): Path<String>,
) -> impl IntoResponse {
    match client.get_user_by_id(&user_id).await {
        Ok(user) => (
            StatusCode::OK,
            Json(serde_json::to_value(UserResponse::from(user)).unwrap()),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, Json(serde_json::to_value(body.0).unwrap())).into_response()
        }
    }
}

/// POST /api/auth/roles/:role_name/permissions
/// Body: `{ "permission_name": "order:read" }`
async fn assign_permission(
    State(client):     State<DynAuthClient>,
    Path(role_name):   Path<String>,
    Json(body):        Json<AssignPermissionBody>,
) -> impl IntoResponse {
    match client
        .assign_permission_to_role(&role_name, &body.permission_name)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, Json(serde_json::to_value(body.0).unwrap())).into_response()
        }
    }
}

/// POST /api/auth/validate-token
/// Body: `{ "token": "..." }`
async fn validate_token(
    State(client): State<DynAuthClient>,
    Json(body):    Json<ValidateTokenBody>,
) -> impl IntoResponse {
    match client.validate_token(&body.token).await {
        Ok(user) => (
            StatusCode::OK,
            Json(serde_json::to_value(UserResponse::from(user)).unwrap()),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, Json(serde_json::to_value(body.0).unwrap())).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Mengembalikan sub-router Auth yang siap di-merge ke router utama Axum.
///
/// ```rust,ignore
/// // di app/src/main.rs
/// let app = Router::new()
///     .merge(auth::auth_routes(auth_client, jwt_secret));
/// ```
pub fn auth_routes(client: DynAuthClient, jwt_secret: Arc<String>) -> Router {
    let state = AuthState { client, jwt_secret };

    Router::new()
        // Auth session flows (mendukung path /api/* dan /api/auth/*)
        .route("/api/login", post(login_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/dashboard", get(dashboard_handler))
        .route("/api/auth/dashboard", get(dashboard_handler))
        // Management routes
        .route("/api/auth/users", post(create_user))
        .route("/api/auth/users/:id", get(get_user_by_id))
        .route("/api/auth/roles/:role_name/permissions", post(assign_permission))
        .route("/api/auth/validate-token", post(validate_token))
        .with_state(state)
}
