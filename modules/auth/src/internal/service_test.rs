use super::*;

async fn make_client() -> DynAuthClient {
    AuthService::new_in_memory()
}

#[tokio::test]
async fn create_user_ok() {
    let client = make_client().await;
    let user = client.create_user("alice@example.com", "admin").await.unwrap();

    assert_eq!(user.email, "alice@example.com");
    assert_eq!(user.role.name, "admin");
    assert_eq!(user.role.permissions.len(), 2);
}

#[tokio::test]
async fn create_user_unknown_role_fails() {
    let client = make_client().await;
    let err = client.create_user("bob@example.com", "superadmin").await.unwrap_err();
    assert_eq!(err, AuthError::RoleNotFound);
}

#[tokio::test]
async fn create_duplicate_email_fails() {
    let client = make_client().await;
    client.create_user("charlie@example.com", "member").await.unwrap();

    let err = client.create_user("charlie@example.com", "member").await.unwrap_err();
    assert_eq!(err, AuthError::DuplicateUser);
}

#[tokio::test]
async fn get_user_by_id_ok() {
    let client = make_client().await;
    let created = client.create_user("dave@example.com", "member").await.unwrap();
    let fetched = client.get_user_by_id(&created.id).await.unwrap();

    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.email, "dave@example.com");
}

#[tokio::test]
async fn get_user_not_found() {
    let client = make_client().await;
    let err = client.get_user_by_id("non-existent-id").await.unwrap_err();
    assert_eq!(err, AuthError::UserNotFound);
}

#[tokio::test]
async fn assign_permission_and_validate_token() {
    let client = make_client().await;
    let created = client.create_user("eve@example.com", "member").await.unwrap();

    client
        .assign_permission_to_role("member", "order:create")
        .await
        .unwrap();

    let updated = client.get_user_by_id(&created.id).await.unwrap();
    let has_perm = updated.role.permissions.iter().any(|p| p.name == "order:create");
    assert!(has_perm, "permission order:create harus ada di role member");
}

#[tokio::test]
async fn assign_permission_idempotent() {
    let client = make_client().await;
    client
        .assign_permission_to_role("admin", "product:read")
        .await
        .unwrap();
    client
        .assign_permission_to_role("admin", "product:read")
        .await
        .unwrap();

    let user = client.create_user("frank@example.com", "admin").await.unwrap();
    let count = user
        .role
        .permissions
        .iter()
        .filter(|p| p.name == "product:read")
        .count();
    assert_eq!(count, 1, "permission tidak boleh terduplikasi");
}

#[tokio::test]
async fn assign_permission_to_unknown_role_fails() {
    let client = make_client().await;
    let err = client
        .assign_permission_to_role("ghost_role", "any:perm")
        .await
        .unwrap_err();
    assert_eq!(err, AuthError::RoleNotFound);
}

#[tokio::test]
async fn validate_invalid_token_fails() {
    let client = make_client().await;
    let err = client.validate_token("invalid-tok").await.unwrap_err();
    assert_eq!(err, AuthError::TokenInvalid);
}

#[tokio::test]
async fn authenticate_seeded_admin_ok() {
    let client = make_client().await;
    let user = client
        .authenticate_user("admin@example.com", "password123")
        .await
        .unwrap();
    assert_eq!(user.email, "admin@example.com");
    assert_eq!(user.role.name, "admin");
}

#[tokio::test]
async fn authenticate_wrong_password_fails() {
    let client = make_client().await;
    let err = client
        .authenticate_user("admin@example.com", "wrong-password")
        .await
        .unwrap_err();
    assert_eq!(err, AuthError::InvalidCredentials);
}

