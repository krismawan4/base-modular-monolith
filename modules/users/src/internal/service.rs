//! # Users Service
//!
//! Mengimplementasikan trait publik `UsersClient` menggunakan `UsersRepository`.
//! Mengubah model internal dan entity error menjadi DTO publik dan UserError.

use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{CreateUserInput, DynUsersClient, UpdateUserInput, UserDto, UserError, UsersClient};

use super::repository::{RepoError, User, UsersRepository};

fn user_to_dto(user: User) -> UserDto {
    UserDto {
        id:         user.id,
        name:       user.name,
        email:      user.email,
        role:       user.role_name,
        created_at: user.created_at.to_rfc3339(),
    }
}

pub(crate) struct UsersService {
    repo: UsersRepository,
}

impl UsersService {
    pub(crate) async fn new_pg(pool: sqlx::PgPool) -> Result<DynUsersClient, sqlx::Error> {
        let repo = UsersRepository::from_pool(pool).await?;
        Ok(Arc::new(Self { repo }))
    }

    pub(crate) fn new_in_memory() -> DynUsersClient {
        Arc::new(Self {
            repo: UsersRepository::new_in_memory(),
        })
    }
}

#[async_trait]
impl UsersClient for UsersService {
    async fn create_user(&self, input: CreateUserInput) -> Result<UserDto, UserError> {
        let email = input.email.trim();
        if email.is_empty() || !email.contains('@') {
            return Err(UserError::ValidationError("invalid email format".to_string()));
        }

        let name = if input.name.trim().is_empty() {
            "User".to_string()
        } else {
            input.name.trim().to_string()
        };

        let role_name = input
            .role
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "member".to_string());

        let password_hash = match input.password {
            Some(ref p) if !p.is_empty() => {
                let hash = bcrypt::hash(p, bcrypt::DEFAULT_COST)
                    .map_err(|e| UserError::Internal(e.to_string()))?;
                Some(hash)
            }
            _ => None,
        };

        let user_id = Uuid::new_v4().to_string();
        let user = User {
            id:            user_id.clone(),
            name,
            email:         email.to_string(),
            password_hash,
            role_name,
            token:         user_id,
            created_at:    Utc::now(),
        };

        let created = self.repo.create_user(user).await.map_err(|e| match e {
            RepoError::DuplicateEmail => UserError::DuplicateEmail,
            RepoError::RoleNotFound   => UserError::ValidationError("specified role does not exist".to_string()),
            RepoError::Database(msg)  => UserError::Internal(msg),
        })?;

        Ok(user_to_dto(created))
    }

    async fn get_user_by_id(&self, id: &str) -> Result<UserDto, UserError> {
        let user = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?
            .ok_or(UserError::NotFound)?;

        Ok(user_to_dto(user))
    }

    async fn list_users(&self) -> Result<Vec<UserDto>, UserError> {
        let users = self
            .repo
            .list_users()
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        Ok(users.into_iter().map(user_to_dto).collect())
    }

    async fn update_user(&self, id: &str, input: UpdateUserInput) -> Result<UserDto, UserError> {
        let mut user = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?
            .ok_or(UserError::NotFound)?;

        if let Some(name) = input.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                user.name = trimmed.to_string();
            }
        }

        if let Some(email) = input.email {
            let trimmed = email.trim();
            if trimmed.is_empty() || !trimmed.contains('@') {
                return Err(UserError::ValidationError("invalid email format".to_string()));
            }
            user.email = trimmed.to_string();
        }

        if let Some(role) = input.role {
            let trimmed = role.trim();
            if !trimmed.is_empty() {
                user.role_name = trimmed.to_string();
            }
        }

        if let Some(ref pass) = input.password {
            if !pass.is_empty() {
                let hash = bcrypt::hash(pass, bcrypt::DEFAULT_COST)
                    .map_err(|e| UserError::Internal(e.to_string()))?;
                user.password_hash = Some(hash);
            }
        }

        let updated = self
            .repo
            .update_user(user)
            .await
            .map_err(|e| match e {
                RepoError::DuplicateEmail => UserError::DuplicateEmail,
                RepoError::RoleNotFound   => UserError::ValidationError("specified role does not exist".to_string()),
                RepoError::Database(msg)  => UserError::Internal(msg),
            })?
            .ok_or(UserError::NotFound)?;

        Ok(user_to_dto(updated))
    }

    async fn delete_user(&self, id: &str) -> Result<(), UserError> {
        let deleted = self
            .repo
            .delete_user(id)
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        if deleted {
            Ok(())
        } else {
            Err(UserError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
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
}

