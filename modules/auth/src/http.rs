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
    blacklist::DynTokenBlacklist,
    middleware::{create_jwt, decode_jwt, AuthSource, AuthUser},
    AuthError, DynAuthClient, UserDto,
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AuthState {
    pub client:     DynAuthClient,
    pub jwt_secret: Arc<String>,
    pub blacklist:  DynTokenBlacklist,
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

fn auth_error_to_response(e: AuthError) -> (StatusCode, Json<Value>) {
    let status = match &e {
        AuthError::UserNotFound       => StatusCode::NOT_FOUND,
        AuthError::RoleNotFound       => StatusCode::UNPROCESSABLE_ENTITY,
        AuthError::TokenInvalid       => StatusCode::UNAUTHORIZED,
        AuthError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthError::DuplicateUser      => StatusCode::CONFLICT,
        AuthError::Internal(_)        => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "status": "error",
            "message": e.to_string(),
            "data": null
        })),
    )
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device_name: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AssignPermissionBody {
    pub permission_name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ValidateTokenBody {
    pub token: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/auth/login
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginPayload,
    responses(
        (status = 200, description = "Login successful (Cookie or Bearer Token)"),
        (status = 400, description = "Invalid request payload"),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn login_handler(
    State(state): State<AuthState>,
    cookies: Cookies,
    payload: Result<Json<LoginPayload>, JsonRejection>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let Json(payload) = payload.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": "Invalid request payload",
                "details": e.body_text(),
                "data": null
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
                Json(json!({
                    "status": "error",
                    "message": "Invalid email or password",
                    "data": null
                })),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": "Authentication error",
                    "details": e.to_string(),
                    "data": null
                })),
            ),
        })?;

    let token = create_jwt(&user.id, &user.email, &state.jwt_secret).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Token creation error",
                "details": e.to_string(),
                "data": null
            })),
        )
    })?;

    if let Some(device) = payload.device_name.filter(|d| !d.trim().is_empty()) {
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Authenticated via Bearer Token",
                "data": {
                    "token_type": "Bearer",
                    "token": token,
                    "device_name": device,
                    "user": {
                        "id": user.id,
                        "name": user.name,
                        "email": user.email
                    }
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
                "data": {
                    "user": {
                        "id": user.id,
                        "name": user.name,
                        "email": user.email
                    }
                }
            })),
        )
            .into_response())
    }
}

/// POST /api/auth/logout
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth"
)]
pub async fn logout_handler(
    State(state): State<AuthState>,
    cookies: Cookies,
    headers: axum::http::HeaderMap,
) -> Json<Value> {
    // 1. Ekstrak token dari header Authorization: Bearer <token>
    let token_from_bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .and_then(|val| {
            if val.starts_with("Bearer ") {
                Some(val[7..].trim().to_string())
            } else {
                None
            }
        });

    // 2. Jika tidak ada di header, cek cookie auth_token
    let token = token_from_bearer.or_else(|| {
        cookies.get("auth_token").map(|c| c.value().to_string())
    });

    // 3. Masukkan ke blacklist (Redis / Memory) dengan sisa masa berlaku token (TTL)
    if let Some(tok) = token {
        if let Ok(claims) = decode_jwt(&tok, &state.jwt_secret) {
            let now = chrono::Utc::now().timestamp() as usize;
            if claims.exp > now {
                let ttl_secs = (claims.exp - now) as u64;
                let _ = state.blacklist.blacklist_token(&tok, ttl_secs).await;
            }
        }
    }

    // 4. Hapus Cookie HttpOnly
    let mut auth_cookie = Cookie::from("auth_token");
    auth_cookie.set_path("/");
    cookies.remove(auth_cookie);

    Json(json!({
        "status": "success",
        "message": "Logged out successfully",
        "data": null
    }))
}

/// GET /api/auth/dashboard
#[utoipa::path(
    get,
    path = "/api/auth/dashboard",
    responses(
        (status = 200, description = "Dashboard profile and stats retrieved"),
        (status = 401, description = "Unauthorized (missing or invalid token)"),
        (status = 404, description = "User profile not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth"
)]
pub async fn dashboard_handler(
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
                Json(json!({
                    "status": "error",
                    "message": "User profile not found",
                    "data": null
                })),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": "Database error",
                    "details": e.to_string(),
                    "data": null
                })),
            ),
        })?;

    Ok(Json(json!({
        "status": "success",
        "data": {
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
        }
    })))
}

/// POST /api/auth/roles/:role_name/permissions
/// Body: `{ "permission_name": "order:read" }`
#[utoipa::path(
    post,
    path = "/api/auth/roles/{role_name}/permissions",
    params(
        ("role_name" = String, Path, description = "Role name (e.g. admin, member)")
    ),
    request_body = AssignPermissionBody,
    responses(
        (status = 200, description = "Permission assigned to role successfully"),
        (status = 422, description = "Role not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn assign_permission(
    State(client):     State<DynAuthClient>,
    Path(role_name):   Path<String>,
    Json(body):        Json<AssignPermissionBody>,
) -> impl IntoResponse {
    match client
        .assign_permission_to_role(&role_name, &body.permission_name)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": format!("Permission '{}' assigned to role '{}'", body.permission_name, role_name),
                "data": null
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, body).into_response()
        }
    }
}

/// POST /api/auth/validate-token
/// Body: `{ "token": "..." }`
#[utoipa::path(
    post,
    path = "/api/auth/validate-token",
    request_body = ValidateTokenBody,
    responses(
        (status = 200, description = "Token is valid, user details returned", body = UserDto),
        (status = 401, description = "Token is invalid or expired"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn validate_token(
    State(client): State<DynAuthClient>,
    Json(body):    Json<ValidateTokenBody>,
) -> impl IntoResponse {
    match client.validate_token(&body.token).await {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "data": UserResponse::from(user)
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = auth_error_to_response(e);
            (status, body).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAPI Documentation
// ---------------------------------------------------------------------------

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        login_handler,
        logout_handler,
        dashboard_handler,
        assign_permission,
        validate_token,
    ),
    components(schemas(
        crate::UserDto,
        crate::RoleDto,
        crate::PermissionDto,
        LoginPayload,
        AssignPermissionBody,
        ValidateTokenBody,
    )),
    tags(
        (name = "Auth", description = "Authentication & Authorization API")
    )
)]
pub struct AuthApiDoc;

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
pub fn auth_routes(
    client:     DynAuthClient,
    jwt_secret: Arc<String>,
    blacklist:  DynTokenBlacklist,
) -> Router {
    let state = AuthState {
        client,
        jwt_secret,
        blacklist,
    };

    Router::new()
        // Auth session flows
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/logout", post(logout_handler))
        .route("/api/auth/dashboard", get(dashboard_handler))
        // Role & token verification routes
        .route("/api/auth/roles/:role_name/permissions", post(assign_permission))
        .route("/api/auth/validate-token", post(validate_token))
        .with_state(state)
}

#[cfg(test)]
#[path = "http_test.rs"]
mod tests;

