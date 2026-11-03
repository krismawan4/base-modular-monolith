//! # Auth Repository (PostgreSQL & In-Memory)
//!
//! Menyimpan `User`, `Role`, dan `Permission` baik di PostgreSQL (via `sqlx`)
//! maupun In-Memory (via `HashMap` + `RwLock` untuk pengujian).
//!
//! Seluruh struct di file ini bersifat `pub(crate)` dan tidak bocor ke luar crate `auth`.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sqlx::{PgPool, Row};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tipe Data Internal (BUKAN DTO publik)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Permission {
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Role {
    pub(crate) name:        String,
    pub(crate) permissions: Vec<Permission>,
}

#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) id:            String,
    pub(crate) name:          String,
    pub(crate) email:         String,
    pub(crate) password_hash: Option<String>,
    pub(crate) role_name:     String,
    #[allow(dead_code)]
    pub(crate) token:         String,
}

// ---------------------------------------------------------------------------
// In-Memory Store
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub(crate) struct Store {
    users:       HashMap<String, User>,
    email_index: HashMap<String, String>,
    token_index: HashMap<String, String>,
    roles:       HashMap<String, Role>,
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum AuthRepository {
    Postgres(PgPool),
    InMemory(Arc<RwLock<Store>>),
}

/// Menyiapkan skema database (tabel dan default seed) jika belum ada.
async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS roles (
            name VARCHAR(50) PRIMARY KEY
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS permissions (
            name VARCHAR(100) PRIMARY KEY
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            role_name VARCHAR(50) NOT NULL REFERENCES roles(name) ON DELETE CASCADE,
            permission_name VARCHAR(100) NOT NULL REFERENCES permissions(name) ON DELETE CASCADE,
            PRIMARY KEY (role_name, permission_name)
        );
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
        r#"
        INSERT INTO roles (name) VALUES ('admin'), ('member') ON CONFLICT DO NOTHING;
        "#,
        r#"
        INSERT INTO permissions (name) VALUES ('user:read'), ('user:write') ON CONFLICT DO NOTHING;
        "#,
        r#"
        INSERT INTO role_permissions (role_name, permission_name) VALUES 
            ('admin', 'user:read'),
            ('admin', 'user:write'),
            ('member', 'user:read')
        ON CONFLICT DO NOTHING;
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }

    let user_exists = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind("admin@example.com")
        .fetch_optional(pool)
        .await?;

    if user_exists.is_none() {
        let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let user_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO users (id, name, email, password_hash, role_name, token) 
             VALUES ($1, $2, $3, $4, 'admin', $5)",
        )
        .bind(&user_id)
        .bind("Admin Developer")
        .bind("admin@example.com")
        .bind(&password_hash)
        .bind(&user_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

impl AuthRepository {
    /// Membuat repository berbasis PostgreSQL.
    pub(crate) async fn from_pool(pool: PgPool) -> Result<Self, sqlx::Error> {
        init_db(&pool).await?;
        Ok(Self::Postgres(pool))
    }

    /// Membuat repository berbasis in-memory (berguna untuk unit test).
    pub(crate) fn new_in_memory() -> Self {
        let mut store = Store::default();

        store.roles.insert(
            "admin".to_string(),
            Role {
                name:        "admin".to_string(),
                permissions: vec![
                    Permission { name: "user:read".to_string() },
                    Permission { name: "user:write".to_string() },
                ],
            },
        );
        store.roles.insert(
            "member".to_string(),
            Role {
                name:        "member".to_string(),
                permissions: vec![Permission { name: "user:read".to_string() }],
            },
        );

        let admin_id = "admin-dev-001".to_string();
        let admin_hash = bcrypt::hash("password123", 4).unwrap_or_default();
        let admin_user = User {
            id: admin_id.clone(),
            name: "Admin Developer".to_string(),
            email: "admin@example.com".to_string(),
            password_hash: Some(admin_hash),
            role_name: "admin".to_string(),
            token: "admin-token".to_string(),
        };
        store.users.insert(admin_id.clone(), admin_user);
        store.email_index.insert("admin@example.com".to_string(), admin_id.clone());
        store.token_index.insert("admin-token".to_string(), admin_id);

        Self::InMemory(Arc::new(RwLock::new(store)))
    }

    /// Menyimpan user baru.
    pub(crate) async fn create_user(
        &self,
        email:     &str,
        role_name: &str,
    ) -> Result<User, CreateUserError> {
        match self {
            Self::Postgres(pool) => {
                let role_exists = sqlx::query("SELECT 1 FROM roles WHERE name = $1")
                    .bind(role_name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| CreateUserError::Database(e.to_string()))?;

                if role_exists.is_none() {
                    return Err(CreateUserError::RoleNotFound);
                }

                let id    = Uuid::new_v4().to_string();
                let token = Uuid::new_v4().to_string();
                let name  = "User".to_string();

                let res = sqlx::query(
                    "INSERT INTO users (id, name, email, role_name, token) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(&id)
                .bind(&name)
                .bind(email)
                .bind(role_name)
                .bind(&token)
                .execute(pool)
                .await;

                match res {
                    Ok(_) => Ok(User {
                        id,
                        name,
                        email: email.to_string(),
                        password_hash: None,
                        role_name: role_name.to_string(),
                        token,
                    }),
                    Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                        Err(CreateUserError::DuplicateEmail)
                    }
                    Err(e) => Err(CreateUserError::Database(e.to_string())),
                }
            }
            Self::InMemory(store) => {
                let mut store = store.write().expect("auth store RwLock poisoned");

                if !store.roles.contains_key(role_name) {
                    return Err(CreateUserError::RoleNotFound);
                }
                if store.email_index.contains_key(email) {
                    return Err(CreateUserError::DuplicateEmail);
                }

                let id    = Uuid::new_v4().to_string();
                let token = Uuid::new_v4().to_string();
                let name  = "User".to_string();

                let user = User {
                    id:            id.clone(),
                    name,
                    email:         email.to_string(),
                    password_hash: None,
                    role_name:     role_name.to_string(),
                    token:         token.clone(),
                };

                store.users.insert(id.clone(), user.clone());
                store.email_index.insert(email.to_string(), id.clone());
                store.token_index.insert(token, id);

                Ok(user)
            }
        }
    }

    /// Mengambil user berdasarkan ID.
    pub(crate) async fn find_user_by_id(&self, user_id: &str) -> Result<Option<User>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query("SELECT id, COALESCE(name, 'User') as name, email, password_hash, role_name, token FROM users WHERE id = $1")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;

                Ok(row.map(|r| User {
                    id:            r.get("id"),
                    name:          r.get("name"),
                    email:         r.get("email"),
                    password_hash: r.get("password_hash"),
                    role_name:     r.get("role_name"),
                    token:         r.get("token"),
                }))
            }
            Self::InMemory(store) => {
                let store = store.read().expect("auth store RwLock poisoned");
                Ok(store.users.get(user_id).cloned())
            }
        }
    }

    /// Mengambil user berdasarkan email (untuk login).
    pub(crate) async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query("SELECT id, COALESCE(name, 'User') as name, email, password_hash, role_name, token FROM users WHERE email = $1")
                    .bind(email)
                    .fetch_optional(pool)
                    .await?;

                Ok(row.map(|r| User {
                    id:            r.get("id"),
                    name:          r.get("name"),
                    email:         r.get("email"),
                    password_hash: r.get("password_hash"),
                    role_name:     r.get("role_name"),
                    token:         r.get("token"),
                }))
            }
            Self::InMemory(store) => {
                let store = store.read().expect("auth store RwLock poisoned");
                let Some(user_id) = store.email_index.get(email) else {
                    return Ok(None);
                };
                Ok(store.users.get(user_id).cloned())
            }
        }
    }

    /// Menemukan user berdasarkan token.
    pub(crate) async fn find_user_by_token(&self, token: &str) -> Result<Option<User>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query("SELECT id, COALESCE(name, 'User') as name, email, password_hash, role_name, token FROM users WHERE token = $1")
                    .bind(token)
                    .fetch_optional(pool)
                    .await?;

                Ok(row.map(|r| User {
                    id:            r.get("id"),
                    name:          r.get("name"),
                    email:         r.get("email"),
                    password_hash: r.get("password_hash"),
                    role_name:     r.get("role_name"),
                    token:         r.get("token"),
                }))
            }
            Self::InMemory(store) => {
                let store = store.read().expect("auth store RwLock poisoned");
                let Some(user_id) = store.token_index.get(token) else {
                    return Ok(None);
                };
                Ok(store.users.get(user_id).cloned())
            }
        }
    }

    /// Mengambil role berdasarkan nama beserta permissions-nya.
    pub(crate) async fn find_role_by_name(&self, role_name: &str) -> Result<Option<Role>, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let role_row = sqlx::query("SELECT name FROM roles WHERE name = $1")
                    .bind(role_name)
                    .fetch_optional(pool)
                    .await?;

                if role_row.is_none() {
                    return Ok(None);
                }

                let perm_rows = sqlx::query(
                    "SELECT permission_name FROM role_permissions WHERE role_name = $1",
                )
                .bind(role_name)
                .fetch_all(pool)
                .await?;

                let permissions = perm_rows
                    .into_iter()
                    .map(|r| Permission {
                        name: r.get("permission_name"),
                    })
                    .collect();

                Ok(Some(Role {
                    name: role_name.to_string(),
                    permissions,
                }))
            }
            Self::InMemory(store) => {
                let store = store.read().expect("auth store RwLock poisoned");
                Ok(store.roles.get(role_name).cloned())
            }
        }
    }

    /// Menambahkan permission ke role. Idempotent.
    pub(crate) async fn add_permission_to_role(
        &self,
        role_name:       &str,
        permission_name: &str,
    ) -> Result<bool, sqlx::Error> {
        match self {
            Self::Postgres(pool) => {
                let role_exists = sqlx::query("SELECT 1 FROM roles WHERE name = $1")
                    .bind(role_name)
                    .fetch_optional(pool)
                    .await?;

                if role_exists.is_none() {
                    return Ok(false);
                }

                sqlx::query("INSERT INTO permissions (name) VALUES ($1) ON CONFLICT DO NOTHING")
                    .bind(permission_name)
                    .execute(pool)
                    .await?;

                sqlx::query(
                    "INSERT INTO role_permissions (role_name, permission_name) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                )
                .bind(role_name)
                .bind(permission_name)
                .execute(pool)
                .await?;

                Ok(true)
            }
            Self::InMemory(store) => {
                let mut store = store.write().expect("auth store RwLock poisoned");
                let Some(role) = store.roles.get_mut(role_name) else {
                    return Ok(false);
                };

                let already_exists = role.permissions.iter().any(|p| p.name == permission_name);
                if !already_exists {
                    role.permissions.push(Permission {
                        name: permission_name.to_string(),
                    });
                }
                Ok(true)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Error Types (crate-internal)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) enum CreateUserError {
    RoleNotFound,
    DuplicateEmail,
    Database(String),
}
