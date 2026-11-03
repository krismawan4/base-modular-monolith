//! # Users HTTP Handlers
//!
//! Handler Axum untuk endpoint REST CRUD modul Users:
//! - POST   /api/users
//! - GET    /api/users
//! - GET    /api/users/:id
//! - PUT    /api/users/:id
//! - DELETE /api/users/:id

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{CreateUserInput, DynUsersClient, UpdateUserInput, UserDto, UserError};

fn user_error_to_response(e: UserError) -> (StatusCode, Json<Value>) {
    let status = match &e {
        UserError::NotFound             => StatusCode::NOT_FOUND,
        UserError::DuplicateEmail        => StatusCode::CONFLICT,
        UserError::ValidationError(_)   => StatusCode::BAD_REQUEST,
        UserError::Internal(_)          => StatusCode::INTERNAL_SERVER_ERROR,
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
// Request Payloads
// ---------------------------------------------------------------------------

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateUserPayload {
    pub name:     Option<String>,
    pub email:    String,
    pub password: Option<String>,
    pub role:     Option<String>,
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
pub struct UpdateUserPayload {
    pub name:     Option<String>,
    pub email:    Option<String>,
    pub password: Option<String>,
    pub role:     Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/users
#[utoipa::path(
    post,
    path = "/api/users",
    request_body = CreateUserPayload,
    responses(
        (status = 201, description = "User created successfully", body = UserDto),
        (status = 400, description = "Invalid request payload or validation error"),
        (status = 409, description = "User with this email already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn create_user_handler(
    State(client): State<DynUsersClient>,
    payload:       Result<Json<CreateUserPayload>, JsonRejection>,
) -> impl IntoResponse {
    let Json(payload) = match payload {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": "Invalid request payload",
                    "details": e.body_text(),
                    "data": null
                })),
            )
                .into_response();
        }
    };

    let input = CreateUserInput {
        name:     payload.name.unwrap_or_default(),
        email:    payload.email,
        password: payload.password,
        role:     payload.role,
    };

    match client.create_user(input).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(json!({
                "status": "success",
                "message": "User created successfully",
                "data": user
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = user_error_to_response(e);
            (status, body).into_response()
        }
    }
}

/// GET /api/users
#[utoipa::path(
    get,
    path = "/api/users",
    responses(
        (status = 200, description = "List of users", body = Vec<UserDto>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn list_users_handler(State(client): State<DynUsersClient>) -> impl IntoResponse {
    match client.list_users().await {
        Ok(users) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "data": users
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = user_error_to_response(e);
            (status, body).into_response()
        }
    }
}

/// GET /api/users/:id
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    responses(
        (status = 200, description = "User details", body = UserDto),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn get_user_handler(
    State(client): State<DynUsersClient>,
    Path(id):      Path<String>,
) -> impl IntoResponse {
    match client.get_user_by_id(&id).await {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "data": user
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = user_error_to_response(e);
            (status, body).into_response()
        }
    }
}

/// PUT /api/users/:id
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    request_body = UpdateUserPayload,
    responses(
        (status = 200, description = "User updated successfully", body = UserDto),
        (status = 400, description = "Invalid request payload or validation error"),
        (status = 404, description = "User not found"),
        (status = 409, description = "Duplicate email"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn update_user_handler(
    State(client): State<DynUsersClient>,
    Path(id):      Path<String>,
    payload:       Result<Json<UpdateUserPayload>, JsonRejection>,
) -> impl IntoResponse {
    let Json(payload) = match payload {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": "Invalid request payload",
                    "details": e.body_text(),
                    "data": null
                })),
            )
                .into_response();
        }
    };

    let input = UpdateUserInput {
        name:     payload.name,
        email:    payload.email,
        password: payload.password,
        role:     payload.role,
    };

    match client.update_user(&id, input).await {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "User updated successfully",
                "data": user
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = user_error_to_response(e);
            (status, body).into_response()
        }
    }
}

/// DELETE /api/users/:id
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    params(
        ("id" = String, Path, description = "User ID (UUID)")
    ),
    responses(
        (status = 200, description = "User deleted successfully"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Users"
)]
pub async fn delete_user_handler(
    State(client): State<DynUsersClient>,
    Path(id):      Path<String>,
) -> impl IntoResponse {
    match client.delete_user(&id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "User deleted successfully",
                "data": null
            })),
        )
            .into_response(),
        Err(e) => {
            let (status, body) = user_error_to_response(e);
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
        create_user_handler,
        list_users_handler,
        get_user_handler,
        update_user_handler,
        delete_user_handler,
    ),
    components(schemas(
        crate::UserDto,
        CreateUserPayload,
        UpdateUserPayload,
    )),
    tags(
        (name = "Users", description = "User Management CRUD API")
    )
)]
pub struct UsersApiDoc;

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn users_routes(client: DynUsersClient) -> Router {
    Router::new()
        .route("/api/users", post(create_user_handler).get(list_users_handler))
        .route(
            "/api/users/:id",
            get(get_user_handler)
                .put(update_user_handler)
                .delete(delete_user_handler),
        )
        .with_state(client)
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

    fn test_app() -> Router {
        let client = build_in_memory_client();
        users_routes(client)
    }

    #[tokio::test]
    async fn test_crud_lifecycle() {
        let app = test_app();

        // 1. CREATE USER
        let create_req = Request::builder()
            .method("POST")
            .uri("/api/users")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "name": "Budi Santoso",
                    "email": "budi@example.com",
                    "role": "member"
                })
                .to_string(),
            ))
            .unwrap();

        let create_res = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_res.status(), StatusCode::CREATED);

        let body = create_res.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "success");
        let user_id = json["data"]["id"].as_str().unwrap().to_string();
        assert_eq!(json["data"]["name"], "Budi Santoso");
        assert_eq!(json["data"]["email"], "budi@example.com");

        // 2. GET USER BY ID
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/api/users/{user_id}"))
            .body(Body::empty())
            .unwrap();

        let get_res = app.clone().oneshot(get_req).await.unwrap();
        assert_eq!(get_res.status(), StatusCode::OK);

        // 3. LIST USERS
        let list_req = Request::builder()
            .method("GET")
            .uri("/api/users")
            .body(Body::empty())
            .unwrap();

        let list_res = app.clone().oneshot(list_req).await.unwrap();
        assert_eq!(list_res.status(), StatusCode::OK);
        let list_body = list_res.into_body().collect().await.unwrap().to_bytes();
        let list_json: Value = serde_json::from_slice(&list_body).unwrap();
        assert!(list_json["data"].as_array().unwrap().len() >= 2);

        // 4. UPDATE USER
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/api/users/{user_id}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "name": "Budi S. Updated"
                })
                .to_string(),
            ))
            .unwrap();

        let update_res = app.clone().oneshot(update_req).await.unwrap();
        assert_eq!(update_res.status(), StatusCode::OK);
        let update_body = update_res.into_body().collect().await.unwrap().to_bytes();
        let update_json: Value = serde_json::from_slice(&update_body).unwrap();
        assert_eq!(update_json["data"]["name"], "Budi S. Updated");

        // 5. DELETE USER
        let delete_req = Request::builder()
            .method("DELETE")
            .uri(format!("/api/users/{user_id}"))
            .body(Body::empty())
            .unwrap();

        let delete_res = app.clone().oneshot(delete_req).await.unwrap();
        assert_eq!(delete_res.status(), StatusCode::OK);

        // 6. VERIFY NOT FOUND AFTER DELETE
        let verify_req = Request::builder()
            .method("GET")
            .uri(format!("/api/users/{user_id}"))
            .body(Body::empty())
            .unwrap();

        let verify_res = app.oneshot(verify_req).await.unwrap();
        assert_eq!(verify_res.status(), StatusCode::NOT_FOUND);
    }
}
