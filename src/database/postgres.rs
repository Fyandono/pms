use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use urlencoding::encode;

pub async fn get_postgres_client() -> PgPool {
    let username = env::var("DATABASE_USER").expect("DATABASE_USER must be set");
    let password = env::var("DATABASE_PASSWORD").expect("DATABASE_PASSWORD must be set");
    let host = env::var("DATABASE_HOST").expect("DATABASE_HOST must be set");
    let port = env::var("DATABASE_PORT").expect("DATABASE_PORT must be set");
    let db = env::var("DATABASE_NAME").expect("DATABASE_NAME must be set");

    // Construct the connection URL
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        username, encode(&password), host, port, db
    );

    // Build and return the connection pool
    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Error building a database connection pool")
}
