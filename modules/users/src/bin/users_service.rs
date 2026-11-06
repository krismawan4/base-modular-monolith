use std::net::SocketAddr;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL wajib disetel di dalam file .env");

    let users_client = match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅ Users Microservice terhubung ke PostgreSQL di {database_url}");
            users::build_client(pool)
                .await
                .expect("Gagal menginisialisasi skema database Users")
        }
        Err(err) => {
            eprintln!("⚠️ Tidak dapat terhubung ke PostgreSQL ({err}). Berjalan dengan in-memory store sebagai fallback...");
            users::build_in_memory_client()
        }
    };

    let port: u16 = std::env::var("USERS_GRPC_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8001);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    println!("🚀 Users gRPC Microservice listening on http://{addr} (port {port})");
    users::run_grpc_server(users_client, addr).await?;

    Ok(())
}

