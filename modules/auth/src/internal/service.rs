//! # Auth Service
//!
//! Mengimplementasikan trait publik `AuthClient` menggunakan `AuthRepository`.
//! File ini adalah satu-satunya titik yang menjembatani dunia internal
//! (repository + domain model) dengan dunia luar (DTO publik + trait).

use std::sync::Arc;

use async_trait::async_trait;

use crate::{AuthClient, AuthError, DynAuthClient, PermissionDto, RoleDto, UserDto};

use super::repository::{AuthRepository, CreateUserError};

// ---------------------------------------------------------------------------
// Konversi Internal → DTO Publik
// ---------------------------------------------------------------------------

fn role_to_dto(role: &super::repository::Role) -> RoleDto {
    RoleDto {
        name: role.name.clone(),
        permissions: role
            .permissions
            .iter()
            .map(|p| PermissionDto { name: p.name.clone() })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub(crate) struct AuthService {
    repo: AuthRepository,
}

impl AuthService {
    /// Constructor dengan PostgreSQL pool.
    pub(crate) async fn new_pg(pool: sqlx::PgPool) -> Result<DynAuthClient, sqlx::Error> {
        let repo = AuthRepository::from_pool(pool).await?;
        Ok(Arc::new(Self { repo }))
    }

    /// Constructor dengan in-memory store (untuk testing & dev fallback).
    pub(crate) fn new_in_memory() -> DynAuthClient {
        Arc::new(Self {
            repo: AuthRepository::new_in_memory(),
        })
    }
}

// ---------------------------------------------------------------------------
// Implementasi Trait Publik
// ---------------------------------------------------------------------------

#[async_trait]
impl AuthClient for AuthService {
    async fn create_user(&self, email: &str, role_name: &str) -> Result<UserDto, AuthError> {
        let user = self
            .repo
            .create_user(email, role_name)
            .await
            .map_err(|e| match e {
                CreateUserError::RoleNotFound   => AuthError::RoleNotFound,
                CreateUserError::DuplicateEmail => AuthError::DuplicateUser,
                CreateUserError::Database(msg)  => AuthError::Internal(msg),
            })?;

        let role = self
            .repo
            .find_role_by_name(&user.role_name)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or_else(|| AuthError::Internal("role disappeared after user creation".to_string()))?;

        Ok(UserDto {
            id:    user.id,
            name:  user.name,
            email: user.email,
            role:  role_to_dto(&role),
        })
    }

    async fn get_user_by_id(&self, user_id: &str) -> Result<UserDto, AuthError> {
        let user = self
            .repo
            .find_user_by_id(user_id)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::UserNotFound)?;

        let role = self
            .repo
            .find_role_by_name(&user.role_name)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or_else(|| AuthError::Internal("role data corrupted".to_string()))?;

        Ok(UserDto {
            id:    user.id,
            name:  user.name,
            email: user.email,
            role:  role_to_dto(&role),
        })
    }

    async fn assign_permission_to_role(
        &self,
        role_name:       &str,
        permission_name: &str,
    ) -> Result<(), AuthError> {
        let found = self
            .repo
            .add_permission_to_role(role_name, permission_name)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?;

        if found {
            Ok(())
        } else {
            Err(AuthError::RoleNotFound)
        }
    }

    async fn validate_token(&self, token: &str) -> Result<UserDto, AuthError> {
        let user = self
            .repo
            .find_user_by_token(token)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::TokenInvalid)?;

        let role = self
            .repo
            .find_role_by_name(&user.role_name)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or_else(|| AuthError::Internal("role data corrupted".to_string()))?;

        Ok(UserDto {
            id:    user.id,
            name:  user.name,
            email: user.email,
            role:  role_to_dto(&role),
        })
    }

    async fn authenticate_user(&self, email: &str, password: &str) -> Result<UserDto, AuthError> {
        let user = self
            .repo
            .find_user_by_email(email)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or(AuthError::InvalidCredentials)?;

        let hash = user.password_hash.as_deref().unwrap_or("");
        let is_valid = bcrypt::verify(password, hash).unwrap_or(false);
        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        let role = self
            .repo
            .find_role_by_name(&user.role_name)
            .await
            .map_err(|e| AuthError::Internal(e.to_string()))?
            .ok_or_else(|| AuthError::Internal("role data corrupted".to_string()))?;

        Ok(UserDto {
            id:    user.id,
            name:  user.name,
            email: user.email,
            role:  role_to_dto(&role),
        })
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "service_test.rs"]
mod tests;

