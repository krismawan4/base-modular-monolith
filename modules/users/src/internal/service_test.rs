use super::*;

#[tokio::test]
async fn test_create_and_get_user() {
    let service = UsersService::new_in_memory();
    let input = CreateUserInput {
        name:     "John Doe".to_string(),
        email:    "john@example.com".to_string(),
        password: Some("secret123".to_string()),
        role:     Some("member".to_string()),
    };

    let created = service.create_user(input).await.unwrap();
    assert_eq!(created.name, "John Doe");
    assert_eq!(created.email, "john@example.com");
    assert_eq!(created.role, "member");

    let fetched = service.get_user_by_id(&created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.email, "john@example.com");
}

#[tokio::test]
async fn test_create_duplicate_email_fails() {
    let service = UsersService::new_in_memory();
    let input1 = CreateUserInput {
        name:     "User One".to_string(),
        email:    "dup@example.com".to_string(),
        password: None,
        role:     None,
    };
    service.create_user(input1).await.unwrap();

    let input2 = CreateUserInput {
        name:     "User Two".to_string(),
        email:    "dup@example.com".to_string(),
        password: None,
        role:     None,
    };
    let err = service.create_user(input2).await.unwrap_err();
    assert_eq!(err, UserError::DuplicateEmail);
}

#[tokio::test]
async fn test_list_and_update_and_delete_user() {
    let service = UsersService::new_in_memory();

    // Admin default exists in in-memory
    let initial_users = service.list_users().await.unwrap();
    assert!(!initial_users.is_empty());

    let created = service
        .create_user(CreateUserInput {
            name:     "Alice".to_string(),
            email:    "alice@example.com".to_string(),
            password: None,
            role:     Some("member".to_string()),
        })
        .await
        .unwrap();

    // Update
    let updated = service
        .update_user(
            &created.id,
            UpdateUserInput {
                name:  Some("Alice Smith".to_string()),
                email: None,
                role:  Some("admin".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "Alice Smith");
    assert_eq!(updated.role, "admin");

    // Delete
    service.delete_user(&created.id).await.unwrap();

    // Get after delete should fail
    let res = service.get_user_by_id(&created.id).await;
    assert_eq!(res.unwrap_err(), UserError::NotFound);
}

#[tokio::test]
async fn test_list_users_order_and_limit() {
    let service = UsersService::new_in_memory();

    let u1 = service
        .create_user(CreateUserInput {
            name:     "User 1".to_string(),
            email:    "u1@example.com".to_string(),
            password: None,
            role:     None,
        })
        .await
        .unwrap();

    let u2 = service
        .create_user(CreateUserInput {
            name:     "User 2".to_string(),
            email:    "u2@example.com".to_string(),
            password: None,
            role:     None,
        })
        .await
        .unwrap();

    let list = service.list_users().await.unwrap();
    assert!(list.len() >= 2);
    // User 2 was created after User 1, so User 2 should appear before User 1 (DESC order)
    let pos_u2 = list.iter().position(|u| u.id == u2.id).unwrap();
    let pos_u1 = list.iter().position(|u| u.id == u1.id).unwrap();
    assert!(pos_u2 < pos_u1);
}


