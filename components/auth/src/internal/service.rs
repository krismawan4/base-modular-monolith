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
            email: user.email,
            role:  role_to_dto(&role),
        })
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
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
}
