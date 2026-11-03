//! # Users Component — Public Contract
//!
//! Crate ini mengelola data entitas User (CRUD) dan mengekspos
//! DTO serta Trait publik yang aman digunakan oleh modul lain.
//! Seluruh detail persistensi tersembunyi dalam modul `internal`.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserDto {
    pub id:         String,
    pub name:       String,
    pub email:      String,
    pub role:       String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput {
    pub name:     String,
    pub email:    String,
    pub password: Option<String>,
    pub role:     Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateUserInput {
    pub name:     Option<String>,
    pub email:    Option<String>,
    pub password: Option<String>,
    pub role:     Option<String>,
}

// ---------------------------------------------------------------------------
// Public Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    NotFound,
    DuplicateEmail,
    ValidationError(String),
    Internal(String),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::NotFound               => write!(f, "user not found"),
            UserError::DuplicateEmail          => write!(f, "user with this email already exists"),
            UserError::ValidationError(msg)   => write!(f, "validation error: {msg}"),
            UserError::Internal(msg)          => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for UserError {}

// ---------------------------------------------------------------------------
// Public Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait UsersClient: Send + Sync {
    async fn create_user(&self, input: CreateUserInput) -> Result<UserDto, UserError>;
    async fn get_user_by_id(&self, id: &str) -> Result<UserDto, UserError>;
    async fn list_users(&self) -> Result<Vec<UserDto>, UserError>;
    async fn update_user(&self, id: &str, input: UpdateUserInput) -> Result<UserDto, UserError>;
    async fn delete_user(&self, id: &str) -> Result<(), UserError>;
}

pub type DynUsersClient = Arc<dyn UsersClient>;

// ---------------------------------------------------------------------------
// Factory Functions
// ---------------------------------------------------------------------------

pub async fn build_client(pool: sqlx::PgPool) -> Result<DynUsersClient, UserError> {
    internal::service::UsersService::new_pg(pool)
        .await
        .map_err(|e| UserError::Internal(e.to_string()))
}

pub fn build_in_memory_client() -> DynUsersClient {
    internal::service::UsersService::new_in_memory()
}

// ---------------------------------------------------------------------------
// HTTP Router
// ---------------------------------------------------------------------------

pub mod http;
pub use http::{users_routes, UsersApiDoc};

// ---------------------------------------------------------------------------
// Internal Details
// ---------------------------------------------------------------------------

mod internal;

