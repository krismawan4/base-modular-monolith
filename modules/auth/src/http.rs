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

#[derive(Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device_name: Option<String>,
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

/// POST /api/auth/login
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
async fn logout_handler(cookies: Cookies) -> Json<Value> {
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
async fn assign_permission(
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
async fn validate_token(
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
mod tests {
    use super::*;
    use crate::build_in_memory_client;
    use axum::{
        body::Body,
        http::{header, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use tower_cookies::CookieManagerLayer;

    fn test_app() -> Router {
        let client = build_in_memory_client();
        let jwt_secret = Arc::new("test_secret_key_that_is_at_least_32_characters_long!".to_string());
        auth_routes(client, jwt_secret).layer(CookieManagerLayer::new())
    }

    #[tokio::test]
    async fn test_login_cookie_response_has_data_property() {
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "admin@example.com",
                    "password": "password123"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "success");
        assert_eq!(json["message"], "Authenticated via HttpOnly Cookie");
        assert!(json.get("data").is_some());
        assert_eq!(json["data"]["user"]["email"], "admin@example.com");
        assert_eq!(json["data"]["user"]["name"], "Admin Developer");
    }

    #[tokio::test]
    async fn test_login_bearer_response_has_data_property() {
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "admin@example.com",
                    "password": "password123",
                    "device_name": "iPhone 15"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "success");
        assert!(json.get("data").is_some());
        assert_eq!(json["data"]["token_type"], "Bearer");
        assert_eq!(json["data"]["device_name"], "iPhone 15");
        assert!(json["data"]["token"].is_string());
        assert_eq!(json["data"]["user"]["email"], "admin@example.com");
    }

    #[tokio::test]
    async fn test_logout_response_has_data_property() {
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/logout")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "success");
        assert_eq!(json["message"], "Logged out successfully");
        assert!(json.get("data").is_some());
        assert!(json["data"].is_null());
    }

    #[tokio::test]
    async fn test_auth_users_route_is_removed_from_auth_module() {
        let app = test_app();

        let create_req = Request::builder()
            .method("POST")
            .uri("/api/auth/users")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "newuser@example.com",
                    "role_name": "admin"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(create_req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_error_response_has_data_property() {
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "admin@example.com",
                    "password": "wrong_password"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "Invalid email or password");
        assert!(json.get("data").is_some());
        assert!(json["data"].is_null());
    }

    #[tokio::test]
    async fn test_dashboard_with_bearer_token() {
        let app = test_app();

        // 1. Login with device_name to get Bearer token
        let login_req = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "admin@example.com",
                    "password": "password123",
                    "device_name": "Pixel 8"
                })
                .to_string(),
            ))
            .unwrap();

        let login_res = app.clone().oneshot(login_req).await.unwrap();
        assert_eq!(login_res.status(), StatusCode::OK);

        let body = login_res.into_body().collect().await.unwrap().to_bytes();
        let login_json: Value = serde_json::from_slice(&body).unwrap();
        let token = login_json["data"]["token"].as_str().unwrap();

        // 2. Call dashboard with Authorization: Bearer <token>
        let dash_req = Request::builder()
            .method("GET")
            .uri("/api/auth/dashboard")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let dash_res = app.oneshot(dash_req).await.unwrap();
        assert_eq!(dash_res.status(), StatusCode::OK);

        let dash_body = dash_res.into_body().collect().await.unwrap().to_bytes();
        let dash_json: Value = serde_json::from_slice(&dash_body).unwrap();

        assert_eq!(dash_json["status"], "success");
        assert_eq!(dash_json["data"]["client_channel"], "Mobile (Bearer Token)");
        assert_eq!(dash_json["data"]["user"]["email"], "admin@example.com");
    }

    #[tokio::test]
    async fn test_old_api_login_path_returns_404() {
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/api/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "email": "admin@example.com",
                    "password": "password123"
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
