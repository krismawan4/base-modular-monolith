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
    let blacklist = crate::build_in_memory_blacklist();
    auth_routes(client, jwt_secret, blacklist).layer(CookieManagerLayer::new())
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

#[tokio::test]
async fn test_bearer_token_cannot_access_dashboard_after_logout() {
    let app = test_app();

    // 1. Login with Bearer token
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "admin@example.com",
                "password": "password123",
                "device_name": "Swagger UI"
            })
            .to_string(),
        ))
        .unwrap();

    let login_res = app.clone().oneshot(login_req).await.unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let body = login_res.into_body().collect().await.unwrap().to_bytes();
    let login_json: Value = serde_json::from_slice(&body).unwrap();
    let token = login_json["data"]["token"].as_str().unwrap();

    // 2. Akses dashboard sebelum logout -> 200 OK
    let dash_req = Request::builder()
        .method("GET")
        .uri("/api/auth/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let dash_res = app.clone().oneshot(dash_req).await.unwrap();
    assert_eq!(dash_res.status(), StatusCode::OK);

    // 3. Logout dengan mengirim token Bearer
    let logout_req = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let logout_res = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

    // 4. Akses dashboard setelah logout -> 401 UNAUTHORIZED
    let dash_after_logout = Request::builder()
        .method("GET")
        .uri("/api/auth/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let final_res = app.oneshot(dash_after_logout).await.unwrap();
    assert_eq!(final_res.status(), StatusCode::UNAUTHORIZED);

    let final_body = final_res.into_body().collect().await.unwrap().to_bytes();
    let final_json: Value = serde_json::from_slice(&final_body).unwrap();
    assert_eq!(final_json["status"], "error");
    assert_eq!(final_json["message"], "Token has been revoked or logged out");
}

#[tokio::test]
async fn test_login_logout_login_again_can_access_dashboard() {
    let app = test_app();

    // 1. Login pertama -> Token 1
    let login_req1 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "admin@example.com",
                "password": "password123",
                "device_name": "Device 1"
            })
            .to_string(),
        ))
        .unwrap();

    let res1 = app.clone().oneshot(login_req1).await.unwrap();
    assert_eq!(res1.status(), StatusCode::OK);
    let body1 = res1.into_body().collect().await.unwrap().to_bytes();
    let json1: Value = serde_json::from_slice(&body1).unwrap();
    let token1 = json1["data"]["token"].as_str().unwrap().to_string();

    // 2. Logout dengan Token 1
    let logout_req = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(header::AUTHORIZATION, format!("Bearer {token1}"))
        .body(Body::empty())
        .unwrap();
    let logout_res = app.clone().oneshot(logout_req).await.unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

    // 3. Verifikasi Token 1 tidak bisa akses dashboard
    let dash_fail = Request::builder()
        .method("GET")
        .uri("/api/auth/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {token1}"))
        .body(Body::empty())
        .unwrap();
    let fail_res = app.clone().oneshot(dash_fail).await.unwrap();
    assert_eq!(fail_res.status(), StatusCode::UNAUTHORIZED);

    // 4. Login kembali -> Token 2
    let login_req2 = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "email": "admin@example.com",
                "password": "password123",
                "device_name": "Device 2"
            })
            .to_string(),
        ))
        .unwrap();

    let res2 = app.clone().oneshot(login_req2).await.unwrap();
    assert_eq!(res2.status(), StatusCode::OK);
    let body2 = res2.into_body().collect().await.unwrap().to_bytes();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    let token2 = json2["data"]["token"].as_str().unwrap();

    // Token 2 harus berbeda dari Token 1
    assert_ne!(&token1, token2);

    // 5. Akses dashboard dengan Token 2 -> HARUS BERHASIL 200 OK!
    let dash_success = Request::builder()
        .method("GET")
        .uri("/api/auth/dashboard")
        .header(header::AUTHORIZATION, format!("Bearer {token2}"))
        .body(Body::empty())
        .unwrap();
    let success_res = app.oneshot(dash_success).await.unwrap();
    assert_eq!(success_res.status(), StatusCode::OK);

    let success_body = success_res.into_body().collect().await.unwrap().to_bytes();
    let success_json: Value = serde_json::from_slice(&success_body).unwrap();
    assert_eq!(success_json["status"], "success");
    assert_eq!(success_json["data"]["user"]["email"], "admin@example.com");
}

