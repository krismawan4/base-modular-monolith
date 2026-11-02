//! Titik masuk aplikasi utama.
//!
//! Di sinilah Axum HTTP server dan Tonic gRPC server akan dijalankan
//! secara bersamaan menggunakan `tokio::select!`.
//!
//! Untuk saat ini file ini hanya mendemonstrasikan cara menggunakan
//! `DynAuthClient` yang dibangun dari crate `auth`.

use auth::{DynAuthClient, build_client};

#[tokio::main]
async fn main() {
    let auth_client: DynAuthClient = build_client();

    // Demonstrasi: buat user dan validasi
    let user = auth_client
        .create_user("admin@example.com", "admin")
        .await
        .expect("create_user failed");

    println!("Created user: {:?}", user);

    // TODO: pasang Axum Router + Tonic gRPC server di sini
    //
    // let http_app = Router::new()
    //     .merge(auth_routes())
    //     .with_state(auth_client.clone());
    //
    // tokio::select! {
    //     _ = axum_serve(http_app)  => {},
    //     _ = tonic_serve(auth_client) => {},
    // }
}

