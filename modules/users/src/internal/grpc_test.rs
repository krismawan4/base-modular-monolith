use super::*;
use std::net::SocketAddr;

#[tokio::test]
async fn test_grpc_client_and_server_crud() {
    let in_memory_service = crate::build_in_memory_client();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    drop(listener);

    let server_handle = tokio::spawn(async move {
        let service = UsersGrpcServer::new(in_memory_service);
        let grpc_service = proto::users_grpc_service_server::UsersGrpcServiceServer::new(service);

        let _ = tonic::transport::Server::builder()
            .add_service(grpc_service)
            .serve(addr)
            .await;
    });

    // Tunggu server bind
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let client = UsersGrpcClient::connect(format!("http://{addr}"))
        .await
        .expect("Client should connect to gRPC server");

    // 1. List users
    let initial = client.list_users().await.unwrap();
    assert!(!initial.is_empty());

    // 2. Create user via gRPC
    let created = client
        .create_user(CreateUserInput {
            name: "gRPC User".to_string(),
            email: "grpc@example.com".to_string(),
            password: None,
            role: Some("member".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(created.name, "gRPC User");
    assert_eq!(created.email, "grpc@example.com");

    // 3. Get user by ID via gRPC
    let fetched = client.get_user_by_id(&created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.email, "grpc@example.com");

    // 4. Update user via gRPC
    let updated = client
        .update_user(
            &created.id,
            UpdateUserInput {
                name: Some("Updated gRPC User".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.name, "Updated gRPC User");

    // 5. Delete user via gRPC
    client.delete_user(&created.id).await.unwrap();

    // 6. Get after delete should return NotFound
    let err = client.get_user_by_id(&created.id).await.unwrap_err();
    assert_eq!(err, UserError::NotFound);

    server_handle.abort();
}

