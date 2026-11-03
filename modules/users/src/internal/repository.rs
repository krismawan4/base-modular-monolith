//! # Users Repository (PostgreSQL & In-Memory)
//!
//! Abstraksi penyimpanan data entitas `User`.
//! Mendukung backend PostgreSQL (via `sqlx`) dan In-Memory (via `HashMap` + `RwLock`).

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) id:            String,
    pub(crate) name:          String,
    pub(crate) email:         String,
    pub(crate) password_hash: Option<String>,
    pub(crate) role_name:     String,
    pub(crate) token:         String,
    pub(crate) created_at:    DateTime<Utc>,
}

#[derive(Debug, Default)]
pub(crate) struct Store {
    pub(crate) users:       HashMap<String, User>,
    pub(crate) email_index: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) enum UsersRepository {
    Postgres(PgPool),
    InMemory(Arc<RwLock<Store>>),
}

#[derive(Debug)]
pub(crate) enum RepoError {
    DuplicateEmail,
    RoleNotFound,
    Database(String),
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoError::DuplicateEmail => write!(f, "user with this email already exists"),
            RepoError::RoleNotFound   => write!(f, "role not found"),
            RepoError::Database(msg)  => write!(f, "database error: {msg}"),
        }
    }
}

async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS roles (
            name VARCHAR(50) PRIMARY KEY
        );
        "#,
        r#"
        INSERT INTO roles (name) VALUES ('admin'), ('member') ON CONFLICT DO NOTHING;
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id VARCHAR(36) PRIMARY KEY,
            name VARCHAR(255) NOT NULL DEFAULT 'User',
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255),
            role_name VARCHAR(50) NOT NULL REFERENCES roles(name),
            token VARCHAR(255) NOT NULL DEFAULT '',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
        r#"
        ALTER TABLE users ADD COLUMN IF NOT EXISTS name VARCHAR(255) NOT NULL DEFAULT 'User';
        "#,
        r#"
        ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255);
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }

    Ok(())
}

impl UsersRepository {
    pub(crate) async fn from_pool(pool: PgPool) -> Result<Self, sqlx::Error> {
        init_db(&pool).await?;
        Ok(Self::Postgres(pool))
    }

    pub(crate) fn new_in_memory() -> Self {
        let mut store = Store::default();
        let admin_id = "admin-dev-001".to_string();
        let admin_hash = bcrypt::hash("password123", 4).unwrap_or_default();
        let admin = User {
            id: admin_id.clone(),
            name: "Admin Developer".to_string(),
            email: "admin@example.com".to_string(),
            password_hash: Some(admin_hash),
            role_name: "admin".to_string(),
            token: "admin-token".to_string(),
            created_at: Utc::now(),
        };
        store.email_index.insert(admin.email.clone(), admin_id.clone());
        store.users.insert(admin_id, admin);

        Self::InMemory(Arc::new(RwLock::new(store)))
    }

    pub(crate) async fn create_user(&self, user: User) -> Result<User, RepoError> {
        match self {
            Self::Postgres(pool) => {
                let role_exists = sqlx::query("SELECT 1 FROM roles WHERE name = $1")
                    .bind(&user.role_name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| RepoError::Database(e.to_string()))?;

                if role_exists.is_none() {
                    return Err(RepoError::RoleNotFound);
                }

                let row = sqlx::query(
                    r#"
                    INSERT INTO users (id, name, email, password_hash, role_name, token, created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    RETURNING id, name, email, password_hash, role_name, token, created_at
                    "#,
                )
                .bind(&user.id)
                .bind(&user.name)
                .bind(&user.email)
                .bind(&user.password_hash)
                .bind(&user.role_name)
                .bind(&user.token)
                .bind(user.created_at)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return RepoError::DuplicateEmail;
                        }
                    }
                    RepoError::Database(e.to_string())
                })?;

                Ok(User {
                    id:            row.get("id"),
                    name:          row.get("name"),
                    email:         row.get("email"),
                    password_hash: row.get("password_hash"),
                    role_name:     row.get("role_name"),
                    token:         row.get("token"),
                    created_at:    row.get("created_at"),
                })
            }
            Self::InMemory(store) => {
                let mut guard = store.write().unwrap();
                if guard.email_index.contains_key(&user.email) {
                    return Err(RepoError::DuplicateEmail);
                }

                guard.email_index.insert(user.email.clone(), user.id.clone());
                guard.users.insert(user.id.clone(), user.clone());
                Ok(user)
            }
        }
    }

    pub(crate) async fn find_by_id(&self, id: &str) -> Result<Option<User>, RepoError> {
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, email, password_hash, role_name, token, created_at FROM users WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;

                Ok(row.map(|r| User {
                    id:            r.get("id"),
                    name:          r.get("name"),
                    email:         r.get("email"),
                    password_hash: r.get("password_hash"),
                    role_name:     r.get("role_name"),
                    token:         r.get("token"),
                    created_at:    r.get("created_at"),
                }))
            }
            Self::InMemory(store) => {
                let guard = store.read().unwrap();
                Ok(guard.users.get(id).cloned())
            }
        }
    }

    pub(crate) async fn list_users(&self) -> Result<Vec<User>, RepoError> {
        match self {
            Self::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, email, password_hash, role_name, token, created_at FROM users ORDER BY created_at ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| RepoError::Database(e.to_string()))?;

                let users = rows
                    .into_iter()
                    .map(|r| User {
                        id:            r.get("id"),
                        name:          r.get("name"),
                        email:         r.get("email"),
                        password_hash: r.get("password_hash"),
                        role_name:     r.get("role_name"),
                        token:         r.get("token"),
                        created_at:    r.get("created_at"),
                    })
                    .collect();

                Ok(users)
            }
            Self::InMemory(store) => {
                let guard = store.read().unwrap();
                let mut list: Vec<User> = guard.users.values().cloned().collect();
                list.sort_by_key(|u| u.created_at);
                Ok(list)
            }
        }
    }

    pub(crate) async fn update_user(&self, user: User) -> Result<Option<User>, RepoError> {
        match self {
            Self::Postgres(pool) => {
                let role_exists = sqlx::query("SELECT 1 FROM roles WHERE name = $1")
                    .bind(&user.role_name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| RepoError::Database(e.to_string()))?;

                if role_exists.is_none() {
                    return Err(RepoError::RoleNotFound);
                }

                let row = sqlx::query(
                    r#"
                    UPDATE users
                    SET name = $1, email = $2, password_hash = $3, role_name = $4
                    WHERE id = $5
                    RETURNING id, name, email, password_hash, role_name, token, created_at
                    "#,
                )
                .bind(&user.name)
                .bind(&user.email)
                .bind(&user.password_hash)
                .bind(&user.role_name)
                .bind(&user.id)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    if let sqlx::Error::Database(ref db_err) = e {
                        if db_err.is_unique_violation() {
                            return RepoError::DuplicateEmail;
                        }
                    }
                    RepoError::Database(e.to_string())
                })?;

                Ok(row.map(|r| User {
                    id:            r.get("id"),
                    name:          r.get("name"),
                    email:         r.get("email"),
                    password_hash: r.get("password_hash"),
                    role_name:     r.get("role_name"),
                    token:         r.get("token"),
                    created_at:    r.get("created_at"),
                }))
            }
            Self::InMemory(store) => {
                let mut guard = store.write().unwrap();
                let existing = match guard.users.get(&user.id) {
                    Some(u) => u.clone(),
                    None => return Ok(None),
                };

                // Check email conflict if email changed
                if existing.email != user.email {
                    if let Some(owner_id) = guard.email_index.get(&user.email) {
                        if owner_id != &user.id {
                            return Err(RepoError::DuplicateEmail);
                        }
                    }
                    guard.email_index.remove(&existing.email);
                    guard.email_index.insert(user.email.clone(), user.id.clone());
                }

                guard.users.insert(user.id.clone(), user.clone());
                Ok(Some(user))
            }
        }
    }

    pub(crate) async fn delete_user(&self, id: &str) -> Result<bool, RepoError> {
        match self {
            Self::Postgres(pool) => {
                let res = sqlx::query("DELETE FROM users WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| RepoError::Database(e.to_string()))?;

                Ok(res.rows_affected() > 0)
            }
            Self::InMemory(store) => {
                let mut guard = store.write().unwrap();
                if let Some(removed) = guard.users.remove(id) {
                    guard.email_index.remove(&removed.email);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

