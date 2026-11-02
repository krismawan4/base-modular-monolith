# Base Modular Monolith Marketplace (Rust)

Proyek ini adalah *boilerplate* dasar untuk arsitektur *Modular Monolith* menggunakan **Rust (Axum & Tonic gRPC)** dengan pendekatan *Database-per-Service*.

## 📂 Project Structure

```text
base-modular-monolith/
├── Cargo.toml                # Konfigurasi Workspace Utama (Mendaftarkan semua modul)
├── prd.md                    # Dokumentasi Product Requirement Document
├── gemini.md                 # Panduan Konteks AI (AI Context Window Guide)
├── proto/
│   ├── auth.proto            # Kontrak gRPC modul otorisasi
│   └── order.proto           # Kontrak gRPC modul order jika dibutuhkan
├── app/                      # AGGREGATOR: Gerbang HTTP Axum Router & gRPC Server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs           # Tempat menyatukan (wiring) semua modul komponen
│       └── middleware.rs     # Middleware Global Guard (RBAC Extractor)
└── components/               # Domain / Kumpulan Modul Bisnis Marketplace
    ├── auth/                 # Modul User, Role, Permission (RBAC)
    ├── product/              # Modul Produk (Katalog, Kategori, Stok)
    ├── order/                # Modul Order (Transaksi, Checkout)
    └── payment/              # Modul Payment (Invoicing, Gateway Integration)
```

## 🛠️ Tech Stack & Requirements
* Rust Edition 2021
* Tokio Async Runtime
* Axum (HTTP REST)
* Tonic (gRPC Server/Client)
