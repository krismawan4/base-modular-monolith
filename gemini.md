# Gemini Context Guide
## Technical Instructions for AI Assistant

Dokumen ini berfungsi sebagai peta konteks (*Context Window Guide*) bagi AI (seperti Gemini, ChatGPT, Claude) saat diminta untuk membantu mengembangkan, menulis kode, membuat modul baru, atau melakukan *refactoring* pada proyek **`base-modular-monolith`** ini.

---

## 1. System Prompt Constraints
Saat berinteraksi mengenai proyek ini, AI wajib mematuhi aturan berikut:
1. **Language Constaint:** Seluruh contoh kode wajib ditulis menggunakan bahasa **Rust** versi modern (Async/Await, Tokio *runtime*). **Dilarang** menyarankan, menganalogikan, atau menuliskan potongan kode menggunakan PHP, Java, Go, atau Node.js kecuali diminta secara eksplisit.
2. **Architecture Enforcement:** Selalu patuhi pola **Cargo Workspaces**. Jika diminta membuat modul baru (misal: `inventory`), kode wajib dipecah ke dalam struktur `components/inventory/src/lib.rs` (untuk *public trait/contracts*) dan `components/inventory/src/internal/` (untuk logika privat).
3. **No Direct Shared States:** Jangan pernah menyarankan pembuatan fungsi atau *global state* yang memungkinkan sebuah modul membaca struct internal modul lain tanpa melalui gerbang `trait` publiknya.

---

## 2. Directory Mapping & Reference Code

AI harus mengasumsikan bahwa struktur proyek dasar adalah sebagai berikut:

```text
base-modular-monolith/
├── Cargo.toml                # Root Workspace
├── proto/
│   └── auth.proto            # Protobuf file
├── app/                      # Axum Router & Tonic gRPC Server (Aggregator)
└── components/
    └── auth/                 # Crate Utama Otorisasi (RBAC)
```

### 2.1. Template Membuat Modul Baru (Crate Komponen)
Jika user meminta: *"Buatkan saya modul baru untuk [Nama Modul]!"*, generate struktur `lib.rs` publik dengan pola berikut:

```rust
// components/[nama_modul]/src/lib.rs
use std::sync::Arc;
use async_trait::async_trait;

// 1. Ekspos hanya DTO Bersih yang aman dibaca modul lain
pub struct [NamaModul]Dto {
    pub id: String,
    // field publik lainnya...
}

// 2. Ekspos Kontrak/Interface berupa Trait Async
#[async_trait]
pub trait [NamaModul]Client: Send + Sync {
    async fn execute_business_logic(&self, id: &str) -> Result<[NamaModul]Dto, String>;
}

pub type Dyn[NamaModul]Client = Arc<dyn [NamaModul]Client>;

// 3. Modul internal disembunyikan secara absolut
mod internal;
```

### 2.2. Template Integrasi Router HTTP Axum Lintas Crate
Ingatkan user bahwa *Handler* HTTP didefinisikan secara lokal di dalam crate masing-masing, namun registrasinya dilakukan dengan menggabungkan router (*merge routing*) di level `app/src/main.rs`:

```rust
// Pola penggabungan rute di app/main.rs
let http_app = Router::new()
    .merge(auth::auth_routes())
    .merge(product::product_routes()) // Contoh modul baru
    .with_state(global_state);
```

---

## 3. Troubleshooting & Common Pitfalls in Rust Workspaces

Jika terjadi error kompilasi saat AI atau developer menuliskan kode, cek daftar panduan solusi di bawah ini:

### 3.1. Error: `the trait ... is not implemented for ...` pada Async Trait
* **Penyebab:** Di Rust, fungsi di dalam *Trait* secara bawaan belum mendukung kata kunci `async` di versi stabil tanpa bantuan makro.
* **Solusi:** Pastikan macro `#[async_trait]` dari *crate* `async-trait` dipasang baik pada deklarasi *Trait* (di `lib.rs`) maupun pada implementasi *Struct*-nya (di `internal/service.rs`).

### 3.2. Error: `cannot find module internal in this scope`
* **Penyebab:** Lupa mendeklarasikan folder internal di file utama modul.
* **Solusi:** Pastikan di akhir file `lib.rs` komponen terdapat deklarasi `mod internal;`, dan di dalam folder `internal/` terdapat file `mod.rs` yang mendaftarkan seluruh sub-file di dalamnya (`pub mod service; pub mod repository;`).

### 3.3. Error: `cyclic dependency detected` saat kargo kompilasi
* **Penyebab:** Modul A mengimport Modul B, dan Modul B mengimport kembali Modul A. Ini adalah pelanggaran arsitektur berat.
* **Solusi:** Alihkan komunikasi salah satu modul menggunakan arsitektur **Event-Driven / gRPC Event via Tokio Channel** di background, atau satukan konteks bisnis jika kedua domain tersebut ternyata sangat terikat erat (*tightly coupled*).


# Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.