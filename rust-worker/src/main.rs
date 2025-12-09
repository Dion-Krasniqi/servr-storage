use axum::{ routing::post, Router };
use sqlx::postgres::PgPoolOptions;
mod files;
mod models;
use files::methods::{get_files, create_folder};


#[tokio::main]
async fn main() {
    let DATABASE_URL = "postgresql://postgres:dinqja123@localhost/servr_db";
    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&DATABASE_URL)
    .await
    .expect("Failed to create pool");

    let app = Router::new().route("/get-files", post(create_folder)).with_state(pool.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app)
        .await.
        unwrap();
}
