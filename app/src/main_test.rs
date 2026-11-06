use super::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn test_root_endpoint() {
    let auth_client = auth::build_in_memory_client();
    let users_client = users::build_in_memory_client();
    let jwt_secret = Arc::new("test_secret_key_that_is_at_least_32_characters_long!".to_string());
    let blacklist = auth::build_in_memory_blacklist();

    let app = create_app(auth_client, users_client, jwt_secret, blacklist);

    let req = Request::builder()
        .method("GET")
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["service"], "modular-monolith-api");
    assert_eq!(json["data"]["graphql_playground"], "/graphql");
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

    // 5. POST /graphql (Mutation login)
    let mutation_login = serde_json::to_vec(&json!({
        "query": "mutation { login(input: { email: \"admin@example.com\", password: \"password123\" }) { token tokenType user { email role } } }"
    })).unwrap();

    let req_login = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .body(Body::from(mutation_login))
        .unwrap();

    let res = app.clone().oneshot(req_login).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    let login_data = &val["data"]["login"];
    assert_eq!(login_data["tokenType"], "Bearer");
    assert_eq!(login_data["user"]["email"], "admin@example.com");
    let jwt_token = login_data["token"].as_str().unwrap();

    // 6. POST /graphql (Query me with Authorization Bearer header)
    let query_me = serde_json::to_vec(&json!({
        "query": "{ me { email role } }"
    })).unwrap();

    let req_me = Request::builder()
        .method("POST")
        .uri("/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {jwt_token}"))
        .body(Body::from(query_me))
        .unwrap();

    let res = app.oneshot(req_me).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["data"]["me"]["email"], "admin@example.com");
}
