# Modular Monolith & Microservices in Rust

Arsitektur aplikasi backend modern berbasis **Rust** yang mengkombinasikan kecepatan dan kesederhanaan **Modular Monolith** dengan fleksibilitas transisi ke **Microservices (via gRPC & Tonic)**.

---

## 📂 Struktur Proyek

```text
modular_monolith/
├── Cargo.toml                  # Workspace Configuration
├── .env                        # Konfigurasi Environment Variable
├── proto/                      # Kontrak Protobuf (gRPC East-West Communication)
│   ├── users.proto             # Kontrak gRPC modul Users (CRUD)
│   ├── auth.proto              # Kontrak gRPC modul Auth (RBAC)
│   └── order.proto             # Kontrak gRPC modul Order
├── app/                        # AGGREGATOR (API Gateway, HTTP Router & GraphQL)
│   └── src/
│       ├── main.rs             # Inisialisasi (wiring) modul & konfigurasi server
│       └── graphql.rs          # Schema & Resolver GraphQL (Async-GraphQL)
└── modules/                    # Domain / Crate Komponen Bisnis
    ├── auth/                   # Modul Autentikasi, JWT, HttpOnly Cookie, RBAC & Redis Blacklist
    └── users/                  # Modul Manajemen User (PostgreSQL, In-Memory, & gRPC Service)
        └── src/
            ├── bin/
            │   └── users_service.rs  # Standalone gRPC Microservice Daemon (:8001)
            └── internal/
                ├── grpc.rs           # Implementasi gRPC Server & Client
                ├── service.rs        # Business Logic Layer
                └── repository.rs     # Data Access Layer (PostgreSQL & In-Memory fallback)
```

---

## ⚙️ Persyaratan Sistem

- **Rust** (1.75+ disarankan, edisi 2021)
- **Protobuf Compiler (`protoc`)**: Diperlukan untuk kompilasi `.proto` tonic. (Otomatis terinstal via winget / package manager).
- *(Opsional)* **PostgreSQL & Redis**: Jika offline, aplikasi otomatis berjalan dalam mode **in-memory fallback**.

---

## 🔧 Konfigurasi Environment (`.env`)

Aplikasi secara otomatis membaca file `.env` di root direktori berkat integrasi `dotenvy`.

```env
# PostgreSQL Database URL
DATABASE_URL=postgres://postgres:password@127.0.0.1:5432/modular_monolith

# Server Aggregator HTTP & GraphQL Port
HOST=127.0.0.1
PORT=8000
RUST_LOG=info

# JWT Secret
JWT_SECRET=super_secret_jwt_key_that_is_at_least_32_characters_long!

# Redis Cache / Token Blacklist
REDIS_URL=redis://127.0.0.1:6379

# --- Microservice Configuration ---
USERS_GRPC_PORT=8001

# Aktifkan baris di bawah jika ingin Aggregator (app) terhubung ke Users Microservice via gRPC:
# USERS_GRPC_URL=http://127.0.0.1:8001
```

---

## 🚀 Cara Menjalankan Aplikasi

Aplikasi ini dapat dijalankan dalam **2 mode arsitektur**:

### Mode 1: Full Modular Monolith (1 Proses)

Dalam mode ini, seluruh modul (`auth` dan `users`) berjalan dalam satu proses memori yang sama.

1. Pastikan baris `USERS_GRPC_URL` di file `.env` dalam keadaan **dinonaktifkan (diberi tanda `#`)**:
   ```env
   # USERS_GRPC_URL=http://127.0.0.1:8001
   ```
2. Jalankan Aggregator dari root workspace:
   ```bash
   cargo run -p app
   ```
   *Server akan berjalan di http://127.0.0.1:8000.*

---

### Mode 2: Microservice Architecture (Dual Process via gRPC)

Dalam mode ini, modul `users` dijalankan sebagai **Microservice terpisah** di port `8001` dengan protokol gRPC, dan Aggregator memanggilnya lewat jaringan HTTP/2 gRPC.

1. **Jalankan Users gRPC Microservice (Terminal 1):**
   ```bash
   cargo run --bin users_service
   ```
   *Output:*
   ```text
   🚀 Users gRPC Microservice listening on http://0.0.0.0:8001 (port 8001)
   ```

2. **Aktifkan gRPC Client di `.env`:**
   Buka file `.env` dan aktifkan baris berikut (hilangkan tanda `#`):
   ```env
   USERS_GRPC_URL=http://127.0.0.1:8001
   ```

3. **Jalankan Aggregator (Terminal 2):**
   ```bash
   cargo run -p app
   ```
   *Output:*
   ```text
   INFO Menggunakan Users Microservice via gRPC di http://127.0.0.1:8001
   INFO HTTP server listening on http://127.0.0.1:8000
   ```

---

## 📖 Dokumentasi & Interactive UIs

Setelah Aggregator berjalan di port 8000:

| Layanan | URL | Keterangan |
| :--- | :--- | :--- |
| **REST API Swagger UI** | [http://127.0.0.1:8000/swagger-ui/](http://127.0.0.1:8000/swagger-ui/) | Dokumentasi OpenAPI & Interactive REST tester |
| **GraphQL Playground** | [http://127.0.0.1:8000/graphql](http://127.0.0.1:8000/graphql) | IDE GraphQL untuk query & mutation user |
| **OpenAPI Spec (JSON)** | [http://127.0.0.1:8000/api-docs/openapi.json](http://127.0.0.1:8000/api-docs/openapi.json) | File OpenAPI raw schema |

---

## 🧪 Menjalankan Automated Tests

Seluruh pengujian unit dan integrasi (termasuk gRPC & REST tests) dapat dijalankan dengan:

```bash
# Menjalankan seluruh test di semua crate/modul
cargo test --workspace

# Menjalankan test modul users saja (termasuk gRPC test)
cargo test -p users

# Menjalankan test modul auth saja
cargo test -p auth

# Menjalankan test integrasi app (Swagger & GraphQL)
cargo test -p app
```
