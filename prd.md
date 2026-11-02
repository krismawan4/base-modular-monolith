# Product Requirement Document (PRD)
## Project Name: base-modular-monolith (Rust Blueprint)

---

## 1. Executive Summary
Dokumen ini mendefinisikan kebutuhan teknis dan fungsional untuk membangun sebuah **Base Template/Boilerplate Proyek berbasis Modular Monolith** menggunakan bahasa pemrograman **Rust**. 

Proyek ini bertujuan menjadi fondasi arsitektur kelas *enterprise* bagi sistem e-commerce/marketplace masa depan. Proyek ini menggabungkan performa tinggi dan keamanan memori dari Rust dengan fleksibilitas batasan modul (*strict decoupling*) yang setara dengan *Microservices*, namun dideploy dalam satu unit monolit tunggal guna memangkas kompleksitas infrastruktur di awal bisnis.

---

## 2. Architectural Objectives & Constraints

### 2.1. Strict Decoupling (Database-per-Service Paradigm)
* **No Cross-Module Database JOINs:** Setiap komponen bisnis wajib mengisolasi databasenya sendiri. Tidak ada fungsi `JOIN` SQL antar-tabel dari modul yang berbeda.
* **No Cross-Module Direct Calls:** Modul tidak boleh memanggil fungsi internal atau implementasi privat dari modul lain secara langsung. Komunikasi wajib melalui **Trait (Kontrak Publik)** yang diekspos di level *root crate* komponen.
* **Compile-Time Boundary Checking:** Menggunakan fitur **Cargo Workspaces** untuk mengisolasi setiap modul menjadi *crate* independen. Jika terjadi pelanggaran impor (*circular dependency* atau impor modul privat), proses kompilasi Rust wajib gagal otomatis (*build failure*).

### 2.2. Dual-Protocol Communication (REST & gRPC)
* **Northbound (Client-to-Server):** Menggunakan **Axum HTTP REST API** untuk melayani *request* dari aplikasi mobile/web frontend.
* **East-West / Future Microservice (Service-to-Service):** Menggunakan **Tonic gRPC Server** yang berjalan serentak. Jika kelak suatu modul harus di-*scale-out* menjadi *Microservice* mandiri, komunikasi antar-modul bisa langsung dialihkan ke gRPC via network tanpa perlu merombak logika bisnis pada modul pemanggil (*nol coding change*).

---

## 3. Core Feature Requirements (The Base RBAC Component)

Modul fondasi pertama yang wajib ada dalam proyek ini adalah Manajemen Pengguna berbasis Peran (**RBAC - Role-Based Access Control**).

### 3.1. User CRUD & Management
* Sistem mampu membuat pengguna (*User*), mengasosiasikannya dengan *Role* tertentu, serta mencantumkan daftar *Permission* yang valid.
* Modul bisnis lain (seperti *Order* atau *Product*) tidak boleh tahu struktur tabel user ini di database, melainkan wajib meminta validasi token melalui kontrak yang disediakan modul Auth.

### 3.2. Authorization Guard / Middleware
* Sistem wajib memiliki *Middleware Extractor* global di layer HTTP (Axum) yang bertugas:
  1. Mengekstrak Token/JWT dari HTTP Header (`Authorization`).
  2. Melakukan *lookup/validation* ke modul Auth via Trait.
  3. Menyuntikkan informasi pengguna dan *permissions*-nya ke fungsi *Controller/Handler* berikutnya jika valid.
  4. Mengembalikan HTTP Status `401 Unauthorized` atau `403 Forbidden` jika token tidak valid atau *permission* tidak mencukupi.

### 3.3. Concurrent Non-Blocking Engine
* Aplikasi utama (`app`) wajib mampu menjalankan HTTP Web Server (Axum) dan gRPC Server (Tonic) secara bersamaan menggunakan *async concurrency* (`tokio::select!`) dalam satu proses *runtime* OS tanpa saling memblokir (*non-blocking*).

---

## 4. Non-Functional Requirements

| Parameter | Kebutuhan Teknis |
| :--- | :--- |
| **Language & Runtime** | Rust (Edition 2021), Tokio Async Runtime |
| **HTTP Framework** | Axum v0.7+ |
| **gRPC Framework** | Tonic v0.12+ / Prost v0.13+ |
| **Serialization** | Serde (JSON derivation) |
| **Isolation Mechanism**| Cargo Workspaces (Multi-crate architecture) |
| **Performance Target** | Baseline RAM usage < 30MB (Idle), Response time < 5ms untuk fungsi in-memory |

---

## 5. Future Scalability Roadmap (CQRS Readiness)
Boilerplate ini harus mendukung penerapan pola **CQRS (Command Query Responsibility Segregation)**. Di masa depan, saat modul `product` atau `order` ditambahkan, jalur penulisan data (*Write DB*) dan jalur pembacaan data (*Read DB*) akan dipisah total, dan sinkronisasi data antar-database tersebut akan dimediasi oleh *Message Broker* seperti Azure Service Bus atau Apache Kafka.
