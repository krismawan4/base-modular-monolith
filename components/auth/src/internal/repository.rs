//! # In-Memory Repository
//!
//! Menyimpan `User`, `Role`, dan `Permission` di dalam HashMap yang dilindungi
//! `RwLock`. Penggunaan `RwLock` (bukan `Mutex`) memungkinkan banyak pembaca
//! bersamaan sambil tetap menjamin eksklusivitas saat ada penulis — cocok untuk
//! workload baca-dominan seperti validasi token.
//!
//! **Catatan:** Seluruh struct di file ini bersifat `pub(crate)`.
//! Mereka tidak boleh bocor ke luar batas crate `auth`.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tipe Data Internal (BUKAN DTO publik)
// ---------------------------------------------------------------------------

/// Representasi Permission di dalam store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Permission {
    pub(crate) name: String,
}

/// Representasi Role di dalam store.
#[derive(Debug, Clone)]
pub(crate) struct Role {
    pub(crate) name:        String,
    pub(crate) permissions: Vec<Permission>,
}

/// Representasi User di dalam store.
#[derive(Debug, Clone)]
pub(crate) struct User {
    pub(crate) id:        String,
    pub(crate) email:     String,
    pub(crate) role_name: String,
    /// Token simulasi — dalam produksi diganti JWT yang sesungguhnya.
    pub(crate) token:     String,
}

// ---------------------------------------------------------------------------
// Store (data container di balik RwLock)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Store {
    /// Kunci: user ID.
    users: HashMap<String, User>,
    /// Kunci: email (untuk deteksi duplikat).
    email_index: HashMap<String, String>,
    /// Kunci: token string (untuk validasi cepat O(1)).
    token_index: HashMap<String, String>,
    /// Kunci: role name.
    roles: HashMap<String, Role>,
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

/// Repository tunggal yang memegang semua data in-memory.
///
/// Dibungkus `Arc` agar bisa di-share ke banyak thread (Tokio worker threads).
#[derive(Debug, Clone)]
pub(crate) struct AuthRepository {
    store: Arc<RwLock<Store>>,
}

impl AuthRepository {
    /// Membuat instance baru dengan seed data role default.
    pub(crate) fn new() -> Self {
        let mut store = Store::default();

        // Seed: dua role dasar agar sistem langsung bisa diuji.
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

        Self {
            store: Arc::new(RwLock::new(store)),
        }
    }

    // -----------------------------------------------------------------------
    // User CRUD
    // -----------------------------------------------------------------------

    /// Menyimpan user baru. Mengembalikan `Err` jika email sudah ada atau
    /// role tidak dikenal.
    pub(crate) fn create_user(
        &self,
        email:     &str,
        role_name: &str,
    ) -> Result<User, CreateUserError> {
        let mut store = self
            .store
            .write()
            .expect("auth store RwLock poisoned");

        if !store.roles.contains_key(role_name) {
            return Err(CreateUserError::RoleNotFound);
        }
        if store.email_index.contains_key(email) {
            return Err(CreateUserError::DuplicateEmail);
        }

        let id    = Uuid::new_v4().to_string();
        let token = Uuid::new_v4().to_string(); // simulasi opaque token

        let user = User {
            id:        id.clone(),
            email:     email.to_string(),
            role_name: role_name.to_string(),
            token:     token.clone(),
        };

        store.users.insert(id.clone(), user.clone());
        store.email_index.insert(email.to_string(), id.clone());
        store.token_index.insert(token, id);

        Ok(user)
    }

    /// Mengambil user berdasarkan ID. Mengembalikan `None` jika tidak ada.
    pub(crate) fn find_user_by_id(&self, user_id: &str) -> Option<User> {
        let store = self
            .store
            .read()
            .expect("auth store RwLock poisoned");
        store.users.get(user_id).cloned()
    }

    /// Menemukan user berdasarkan token. Mengembalikan `None` jika tidak valid.
    pub(crate) fn find_user_by_token(&self, token: &str) -> Option<User> {
        let store = self
            .store
            .read()
            .expect("auth store RwLock poisoned");
        let user_id = store.token_index.get(token)?;
        store.users.get(user_id).cloned()
    }

    // -----------------------------------------------------------------------
    // Role & Permission
    // -----------------------------------------------------------------------

    /// Mengambil role berdasarkan nama. Mengembalikan `None` jika tidak ada.
    pub(crate) fn find_role_by_name(&self, role_name: &str) -> Option<Role> {
        let store = self
            .store
            .read()
            .expect("auth store RwLock poisoned");
        store.roles.get(role_name).cloned()
    }

    /// Menambahkan permission ke role. Idempotent.
    /// Mengembalikan `false` jika role tidak ditemukan.
    pub(crate) fn add_permission_to_role(
        &self,
        role_name:       &str,
        permission_name: &str,
    ) -> bool {
        let mut store = self
            .store
            .write()
            .expect("auth store RwLock poisoned");

        let Some(role) = store.roles.get_mut(role_name) else {
            return false;
        };

        let already_exists = role
            .permissions
            .iter()
            .any(|p| p.name == permission_name);

        if !already_exists {
            role.permissions
                .push(Permission { name: permission_name.to_string() });
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Error Types (crate-internal)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) enum CreateUserError {
    RoleNotFound,
    DuplicateEmail,
}

