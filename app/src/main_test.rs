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
    let blacklist = auth::build_in_memory_blacklist();

    let app = create_app(auth_client, users_client, jwt_secret, blacklist);

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

#[tokio::test]
async fn test_graphql_playground_and_operations() {
    let auth_client = auth::build_in_memory_client();
    let users_client = users::build_in_memory_client();
    let jwt_secret = Arc::new("test_secret_key_that_is_at_least_32_characters_long!".to_string());
    let blacklist = auth::build_in_memory_blacklist();

    let app = create_app(auth_client, users_client, jwt_secret, blacklist);

    // 1. GET /graphql (Playground UI)
    let req_playground = Request::builder()
        .method("GET")
        .uri("/graphql")
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req_playground).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. POST /graphql (Query hello)
    let query_hello = serde_json::to_vec(&json!({
        "query": "{ hello }"
    })).unwrap();

    let req_hello = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query_hello))
        .unwrap();

    let res = app.clone().oneshot(req_hello).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["data"]["hello"], "Hello from Axum GraphQL!");

    // 3. POST /graphql (Mutation createUser)
    let mutation_create = serde_json::to_vec(&json!({
        "query": "mutation { createUser(input: { name: \"John Doe\", email: \"john@example.com\" }) { id name email } }"
    })).unwrap();

    let req_create = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(mutation_create))
        .unwrap();

    let res = app.clone().oneshot(req_create).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["data"]["createUser"]["name"], "John Doe");
    assert_eq!(val["data"]["createUser"]["email"], "john@example.com");

    // 4. POST /graphql (Query users)
    let query_users = serde_json::to_vec(&json!({
        "query": "{ users { name email } }"
    })).unwrap();

    let req_users = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query_users))
        .unwrap();

    let res = app.clone().oneshot(req_users).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let users = val["data"]["users"].as_array().unwrap();
    assert!(users.iter().any(|u| u["email"] == "john@example.com"));
}

