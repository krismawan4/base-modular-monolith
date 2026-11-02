//! # Auth HTTP Handlers
//!
//! Handler Axum untuk endpoint REST modul Auth.
//! Fungsi `auth_routes()` dipanggil sekali di `app/src/main.rs` dan
//! di-merge ke router utama.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{AuthError, DynAuthClient, UserDto};

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
    email: String,
    role:  RoleResponse,
}

impl From<UserDto> for UserResponse {
    fn from(u: UserDto) -> Self {
        UserResponse {
            id:    u.id,
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
        AuthError::UserNotFound  => StatusCode::NOT_FOUND,
        AuthError::RoleNotFound  => StatusCode::UNPROCESSABLE_ENTITY,
        AuthError::TokenInvalid  => StatusCode::UNAUTHORIZED,
        AuthError::DuplicateUser => StatusCode::CONFLICT,
        AuthError::Internal(_)   => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(ErrorResponse { error: e.to_string() }))
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

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
/// ```rust
/// // di app/src/main.rs
/// let app = Router::new()
///     .merge(auth::auth_routes(auth_client));
/// ```
pub fn auth_routes(client: DynAuthClient) -> Router {
    Router::new()
        .route("/api/auth/users",                         post(create_user))
        .route("/api/auth/users/:id",                     get(get_user_by_id))
        .route("/api/auth/roles/:role_name/permissions",  post(assign_permission))
        .route("/api/auth/validate-token",                post(validate_token))
        .with_state(client)
}

