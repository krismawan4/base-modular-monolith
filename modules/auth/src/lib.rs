//! # Auth Component — Public Contract
//!
//! Crate ini mengekspos **hanya** DTO dan Trait publik yang boleh digunakan
//! oleh crate lain (misalnya `app`). Seluruh detail implementasi disembunyikan
//! di dalam modul `internal` yang bersifat privat.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public DTOs
// ---------------------------------------------------------------------------

/// Data Transfer Object untuk Permission.
/// Merepresentasikan satu hak akses yang bersifat read-only bagi konsumen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDto {
    pub name: String,
}

/// Data Transfer Object untuk Role.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDto {
    pub name:        String,
    pub permissions: Vec<PermissionDto>,
}

/// Data Transfer Object untuk User.
/// Satu-satunya representasi User yang boleh beredar di luar crate `auth`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id:    String,
    pub name:  String,
    pub email: String,
    pub role:  RoleDto,
}

// ---------------------------------------------------------------------------
// Public Errors
// ---------------------------------------------------------------------------

/// Tipe error domain untuk seluruh operasi Auth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    UserNotFound,
    RoleNotFound,
    TokenInvalid,
    DuplicateUser,
    InvalidCredentials,
    Internal(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::UserNotFound       => write!(f, "user not found"),
            AuthError::RoleNotFound       => write!(f, "role not found"),
            AuthError::TokenInvalid       => write!(f, "token is invalid or expired"),
            AuthError::DuplicateUser      => write!(f, "user with this email already exists"),
            AuthError::InvalidCredentials => write!(f, "invalid email or password"),
            AuthError::Internal(msg)      => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

// ---------------------------------------------------------------------------
// Public Trait (Kontrak)
// ---------------------------------------------------------------------------

/// Kontrak publik crate `auth`.
///
/// Setiap crate lain yang perlu berinteraksi dengan sistem Auth **wajib**
/// menggunakan trait ini, bukan mengakses implementasi secara langsung.
#[async_trait]
pub trait AuthClient: Send + Sync {
    /// Membuat pengguna baru dan mengasosiasikannya dengan `role_name`.
    /// Mengembalikan `UserDto` yang baru dibuat.
    async fn create_user(&self, email: &str, role_name: &str) -> Result<UserDto, AuthError>;

    /// Mengambil data pengguna berdasarkan ID.
    async fn get_user_by_id(&self, user_id: &str) -> Result<UserDto, AuthError>;

    /// Menambahkan permission baru ke dalam sebuah role secara dinamis.
    /// Idempotent: permission yang sudah ada tidak akan diduplikasi.
    async fn assign_permission_to_role(
        &self,
        role_name:       &str,
        permission_name: &str,
    ) -> Result<(), AuthError>;

    /// Memvalidasi token (simulasi JWT) dan mengembalikan `UserDto` pemiliknya
    /// beserta seluruh permission yang dimiliki.
    async fn validate_token(&self, token: &str) -> Result<UserDto, AuthError>;

    /// Mengautentikasi pengguna berdasarkan email dan password.
    async fn authenticate_user(&self, email: &str, password: &str) -> Result<UserDto, AuthError>;
}

/// Type alias `Arc<dyn AuthClient>` agar mudah digunakan sebagai Axum State
/// atau parameter fungsi tanpa harus menulis full generic setiap saat.
pub type DynAuthClient = Arc<dyn AuthClient>;

// ---------------------------------------------------------------------------
// Factory Functions (pintu masuk ke implementasi internal)
// ---------------------------------------------------------------------------

/// Membangun instance `AuthClient` menggunakan PostgreSQL connection pool.
///
/// Dipanggil di `app/src/main.rs` saat bootstrap aplikasi dengan `sqlx::PgPool`.
pub async fn build_client(pool: sqlx::PgPool) -> Result<DynAuthClient, AuthError> {
    internal::service::AuthService::new_pg(pool)
        .await
        .map_err(|e| AuthError::Internal(e.to_string()))
}

/// Membangun instance `AuthClient` in-memory (berguna untuk testing atau fallback dev).
pub fn build_in_memory_client() -> DynAuthClient {
    internal::service::AuthService::new_in_memory()
}

/// Membangun instance `TokenBlacklist` berbasis Redis.
pub async fn build_redis_blacklist(redis_url: &str) -> Result<DynTokenBlacklist, redis::RedisError> {
    let bl = blacklist::RedisTokenBlacklist::new(redis_url).await?;
    Ok(Arc::new(bl))
}

/// Membangun instance `TokenBlacklist` in-memory (berguna untuk testing & fallback dev).
pub fn build_in_memory_blacklist() -> DynTokenBlacklist {
    Arc::new(blacklist::InMemoryTokenBlacklist::new())
}

// ---------------------------------------------------------------------------
// Middleware & Blacklist (publik agar bisa digunakan di app)
// ---------------------------------------------------------------------------

pub mod blacklist;
pub mod middleware;

pub use blacklist::{DynTokenBlacklist, TokenBlacklist};
pub use middleware::{create_jwt, decode_jwt, AuthSource, AuthUser};

// ---------------------------------------------------------------------------
// Internal (semua detail implementasi tersembunyi di sini)
// ---------------------------------------------------------------------------

mod internal;

