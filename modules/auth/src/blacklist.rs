//! # Token Blacklist (Redis & In-Memory)
//!
//! Mengelola pembatalan token (revocation) dengan Redis sebagai backend utama
//! dan In-Memory HashMap sebagai fallback untuk pengujian atau saat Redis offline.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[async_trait]
pub trait TokenBlacklist: Send + Sync {
    /// Menandai token sebagai revoked dengan TTL (detik) hingga masa berlaku token habis di Redis.
    async fn blacklist_token(&self, token: &str, ttl_secs: u64) -> Result<(), String>;

    /// Memeriksa apakah token ada di blacklist.
    async fn is_blacklisted(&self, token: &str) -> Result<bool, String>;
}

pub type DynTokenBlacklist = Arc<dyn TokenBlacklist>;

// ---------------------------------------------------------------------------
// Redis Implementation
// ---------------------------------------------------------------------------

pub struct RedisTokenBlacklist {
    manager: redis::aio::ConnectionManager,
    key_prefix: String,
}

impl RedisTokenBlacklist {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let manager = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            manager,
            key_prefix: "jwt_blacklist:".to_string(),
        })
    }
}

#[async_trait]
impl TokenBlacklist for RedisTokenBlacklist {
    async fn blacklist_token(&self, token: &str, ttl_secs: u64) -> Result<(), String> {
        if ttl_secs == 0 {
            return Ok(());
        }
        let key = format!("{}{}", self.key_prefix, hash_token(token));
        let mut conn = self.manager.clone();
        redis::cmd("SETEX")
            .arg(&key)
            .arg(ttl_secs)
            .arg("1")
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Redis error: {e}"))
    }

    async fn is_blacklisted(&self, token: &str) -> Result<bool, String> {
        let key = format!("{}{}", self.key_prefix, hash_token(token));
        let mut conn = self.manager.clone();
        let exists: bool = redis::cmd("EXISTS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Redis error: {e}"))?;
        Ok(exists)
    }
}

// ---------------------------------------------------------------------------
// In-Memory Implementation (Untuk unit test & fallback saat Redis offline)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct InMemoryTokenBlacklist {
    store: RwLock<HashMap<String, Instant>>,
}

impl InMemoryTokenBlacklist {
    pub fn new() -> Self {
        Self::default()
    }

    fn clean_expired(&self) {
        let now = Instant::now();
        let mut guard = self.store.write().unwrap();
        guard.retain(|_, expires_at| *expires_at > now);
    }
}

#[async_trait]
impl TokenBlacklist for InMemoryTokenBlacklist {
    async fn blacklist_token(&self, token: &str, ttl_secs: u64) -> Result<(), String> {
        if ttl_secs == 0 {
            return Ok(());
        }
        self.clean_expired();
        let hash = hash_token(token);
        let expires_at = Instant::now() + Duration::from_secs(ttl_secs);
        let mut guard = self.store.write().unwrap();
        guard.insert(hash, expires_at);
        Ok(())
    }

    async fn is_blacklisted(&self, token: &str) -> Result<bool, String> {
        self.clean_expired();
        let hash = hash_token(token);
        let guard = self.store.read().unwrap();
        if let Some(expires_at) = guard.get(&hash) {
            Ok(*expires_at > Instant::now())
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
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
}

