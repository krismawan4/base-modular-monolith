use super::*;

#[tokio::test]
async fn test_in_memory_blacklist() {
    let bl = InMemoryTokenBlacklist::new();
    let token = "sample_jwt_token_123";

    assert!(!bl.is_blacklisted(token).await.unwrap());

    bl.blacklist_token(token, 10).await.unwrap();

    assert!(bl.is_blacklisted(token).await.unwrap());
    assert!(!bl.is_blacklisted("other_token").await.unwrap());
}

