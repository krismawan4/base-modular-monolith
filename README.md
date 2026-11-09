# Modular Monolith & Microservices in Rust

Arsitektur aplikasi backend modern berbasis **Rust** yang mengkombinasikan kecepatan dan kesederhanaan **Modular Monolith** dengan fleksibilitas transisi ke **Microservices (via gRPC & Tonic)**.

> [!NOTE]
> **Branch `main`** kini berfokus pada **GraphQL API** (menggunakan [Async-GraphQL](https://async-graphql.github.io/)) untuk komunikasi client-to-server dan **gRPC** untuk komunikasi antar-layanan (East-West traffic). Implementasi REST API terdahulu beserta Swagger UI diarsipkan dan tersimpan rapi di branch terpisah: [`archive/rest-api`](https://github.com/krismawan4/base-modular-monolith/tree/archive/rest-api).

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

# CORS Configuration (Origin frontend yang diizinkan)
CORS_ALLOWED_ORIGIN=http://localhost:5173

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

### 💻 Mode Development Lokal (`cargo run` & `npm run dev`)

Jika Anda sedang mendevelop kode secara lokal dan ingin fitur *Hot Module Replacement* (HMR) frontend serta kompilasi cepat Rust:

#### 1. Aktifkan Container Docker Pendukung (Database & Cache)
Hanya aktifkan container **infrastruktur** agar data tersimpan persisten:
```bash
docker compose up -d postgres redis
```

> [!WARNING]
> **JANGAN** menyalakan container `backend` dan `frontend` di Docker saat mode lokal, agar tidak terjadi bentrok port (*port conflict*) dengan `cargo run` (:8000) dan dev server frontend (:4321 / :5173).

*(Opsional: Jika ingin melihat visualisasi log realtime di Grafana, jalankan: `docker compose up -d postgres redis loki alloy grafana`)*.

#### 2. Jalankan Backend (Rust Axum)
Cukup jalankan:
```bash
cargo run
```
*Backend otomatis mengompilasi kode dan berjalan di `http://127.0.0.1:8000`.*

> [!TIP]
> Proyek ini memiliki 2 binary (`app` dan `users_service`). Berkat konfigurasi `default-members = ["app"]` di root `Cargo.toml`, Anda cukup mengetik `cargo run`.
> Jika ingin menjalankan binary secara spesifik:
> - **Aggregator Utama (Port 8000):** `cargo run` (atau `cargo run -p app` / `cargo run --bin app`)
> - **Users Microservice gRPC (Port 8001):** `cargo run --bin users_service`

#### 3. Jalankan Frontend (Astro & React)
```bash
npm --prefix frontend run dev
```
*Frontend berjalan dengan fitur instant HMR tanpa perlu menjalankan `npm run build` setiap ada perubahan kode.*

---

### 📋 Standarisasi `getData` (Top 1000 & Client-Side Processing)

Untuk menghindari lambatnya paginasi konvensional di database (*OFFSET scanning & COUNT(*) overhead*), proyek ini menerapkan standarisasi `getData`:

1. **Backend (Database):**
   - Query distandarkan selalu mengambil data terbaru:
     ```sql
     SELECT ... FROM users ORDER BY created_at DESC LIMIT 1000
     ```
   - Sangat cepat dan efisien karena memanfaatkan *Index Scan* pada kolom `created_at`.

2. **Frontend (In-Memory Processing via `useDataTable`):**
   - **Pencarian Global:** Filter teks yang otomatis mencari ke semua kolom.
   - **Pencarian Spesifik Kolom:** Filter independen per kolom (Nama, Email, Role, ID).
   - **Sorting Tiap Kolom:** Pengurutan instan (ASC / DESC / Reset) untuk String, Date, maupun Angka.
   - **Paginasi Fleksibel:** Pengguna dapat memilih `10`, `15`, `30`, `50`, atau `100` data per halaman dengan navigasi halaman cerdas (*smart pagination controls*).

---

### Mode Arsitektur Backend

Backend dapat dijalankan dalam **2 pilihan arsitektur**:

#### Mode 1: Full Modular Monolith (1 Proses)

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

#### Mode 2: Microservice Architecture (Dual Process via gRPC)

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
| **Frontend Web App** | [http://127.0.0.1:5173/](http://127.0.0.1:5173/) | Dashboard Admin & Login GraphQL |
| **GraphQL Playground** | [http://127.0.0.1:8000/graphql](http://127.0.0.1:8000/graphql) | IDE GraphQL interaktif untuk query & mutation |
| **Root Health/Status** | [http://127.0.0.1:8000/](http://127.0.0.1:8000/) | Status service & versi aplikasi |

---

## 🧪 Menjalankan Automated Tests

Seluruh pengujian unit dan integrasi (termasuk gRPC & GraphQL tests) dapat dijalankan dengan:

```bash
# Menjalankan seluruh test di semua crate/modul
cargo test --workspace

# Menjalankan test modul users saja (termasuk gRPC test)
cargo test -p users

# Menjalankan test modul auth saja
cargo test -p auth

# Menjalankan test integrasi app (GraphQL & Service Health)
cargo test -p app
```

---

## 🧹 Maintenance & Pembersihan Disk

### 1. Membersihkan Folder `target/` Rust (`cargo clean`)
Selama proses development dan kompilasi berulang kali, ukuran folder `target/` dapat membengkak hingga puluhan gigabyte:

* **Reset Total (Menghapus seluruh isi folder `target/` ke 0 byte):**
  ```bash
  cargo clean
  ```
* **Hanya Menghapus Build Release:**
  ```bash
  cargo clean --release
  ```
* **Hanya Menghapus Build Crate Tertentu:**
  ```bash
  cargo clean -p app
  cargo clean -p users
  ```
* **Pembersihan Cerdas via `cargo-sweep` (Hanya Hapus Cache Usang):**
  Jika ingin menghemat ruang disk tanpa memperlambat kompilasi harian:
  ```bash
  cargo install cargo-sweep
  cargo sweep --time 14      # Hapus artefak yang tidak disentuh > 14 hari
  cargo sweep --installed    # Hapus file compiler versi lama
  ```
* **Manual via Terminal (PowerShell):**
  ```powershell
  Remove-Item -Recurse -Force target
  ```

---

## 🚀 Panduan Deployment ke Production

### 1. Menggunakan Docker Compose (Direkomendasikan)
Cara termudah dan paling aman untuk menjalankan seluruh stack di server production (VPS / Cloud VM):

```bash
# 1. Pastikan file .env sudah disetel dengan password/secret production yang aman:
cp .env .env.production
# Edit nilai DATABASE_URL, JWT_SECRET, REDIS_URL, CORS_ALLOWED_ORIGIN

# 2. Build dan jalankan seluruh container (Postgres, Redis, Rust Backend, Frontend Nginx):
docker compose up -d --build

# 3. Cek status logs:
docker compose logs -f backend
```

### 2. Pembersihan Docker (Prune Images & Build Cache)
Jika disk penuh atau ingin membersihkan cache kompilasi Docker dan image usang:

```bash
# Menghapus build cache BuildKit saja (sangat menghemat ruang disk):
docker builder prune -a -f

# Menghapus seluruh image yang sedang tidak dipakai:
docker image prune -a -f

# Pembersihan menyeluruh (Image + Cache + Container mati):
docker system prune -a -f
```

> [!WARNING]
> **Hati-hati dengan flag `--volumes`:**  
> Jika menjalankan `docker system prune -a --volumes -f`, volume data PostgreSQL dan Redis yang sedang tidak berjalan juga akan ikut terhapus. Gunakan `docker system prune -a -f` (tanpa `--volumes`) agar volume data database tetap aman.

| Kebutuhan | Perintah |
| :--- | :--- |
| **Hapus Build Cache Saja** | `docker builder prune -a -f` |
| **Hapus Semua Unused Images** | `docker image prune -a -f` |
| **Pembersihan Bersih (Aman Volume Data)** | `docker system prune -a -f` |
| **Pembersihan Total (+ Volume Data)** | `docker system prune -a --volumes -f` |

---

### 3. Menggunakan Binary Linux Native (Systemd di VPS)
Jika ingin performa maksimal (*bare-metal / near-zero overhead*):

1. **Compile binary Release:**
   ```bash
   cargo build --release -p app
   # Binary hasil build berada di: target/release/app
   ```

2. **Buat file service systemd (`/etc/systemd/system/modular-monolith.service`):**
   ```ini
   [Unit]
   Description=Modular Monolith Axum Backend
   After=network.target postgresql.service redis.service

   [Service]
   Type=simple
   User=www-data
   WorkingDirectory=/opt/modular_monolith
   ExecStart=/opt/modular_monolith/app
   EnvironmentFile=/opt/modular_monolith/.env
   Restart=always
   RestartSec=5

   [Install]
   WantedBy=multi-user.target
   ```

3. **Aktifkan dan Jalankan Service:**
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable modular-monolith
   sudo systemctl start modular-monolith
   ```

4. **Konfigurasi Reverse Proxy (Nginx) dengan HTTPS:**
   ```nginx
   server {
       server_name api.domainanda.com;

       location / {
           proxy_pass http://127.0.0.1:8000;
           proxy_http_version 1.1;
           proxy_set_header Upgrade $http_upgrade;
           proxy_set_header Connection "upgrade";
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
       }
   }
   ```



---

## 📊 Centralized Logging (Grafana Alloy + Loki + Grafana)

Proyek ini dilengkapi stack pemantauan log terpusat modern:

```text
[ Docker Containers ]
  ├── modular_backend (Axum JSON / Text Logs)
  ├── modular_postgres
  ├── modular_redis
  └── modular_frontend
             │
             ▼ (Docker Socket Scraper)
     [ Grafana Alloy ]  (:12345)
             │
             ▼ (HTTP Push)
     [ Grafana Loki ]   (:3100)
             │
             ▼ (Loki Datasource)
     [ Grafana UI ]     (:3000)
```

### 1. Menjalankan Stack Logging
```bash
# Jalankan seluruh service beserta Loki, Alloy, dan Grafana:
docker compose up -d

# Atau jika hanya ingin menjalankan stack logging:
docker compose up -d loki alloy grafana
```

### 2. Mengakses Grafana Dashboard
- **URL**: [http://localhost:3000](http://localhost:3000)
- **Username**: `admin`
- **Password**: `admin`

Datasource **Loki** dan Dashboard **"Centralized Container Logs"** telah di-provisioning otomatis:
1. Buka menu **Dashboards** > folder **Logging** > buka dashboard **Centralized Container Logs**.
2. Anda dapat memfilter logs per-kontainer (`modular_backend`, `modular_postgres`, dsb.) dan melihat stream log secara real-time.
3. Atau buka menu **Explore** dan masukkan LogQL:
   ```logql
   {container_name="modular_backend"}
   ```
   Untuk query log dengan format JSON:
   ```logql
   {container_name="modular_backend"} | json
   ```

### 3. Format Log Backend (`LOG_FORMAT`)
Backend Axum mendukung 2 mode logging via environment variable:
- `LOG_FORMAT=text`: Format standar human-readable (cocok untuk local development di terminal).
- `LOG_FORMAT=json`: Format JSON terstruktur (aktif secara default di Docker untuk parsing otomatis di Loki).
