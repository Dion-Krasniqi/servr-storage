use axum::{ routing::post, Router };
use sqlx::postgres::PgPoolOptions;
#[macro_use]
extern crate dotenv_codegen;


mod files;
mod models;
use files::methods::{get_files, create_folder, upload_file, delete_file};


#[tokio::main]
async fn main() {
    let DATABASE_URL = dotenv!("DATABASE_URL");
    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&DATABASE_URL)
    .await
    .expect("Failed to create pool");
    let app = Router::new().route("/create-folder",post(create_folder))
        .route("/get-files", post(get_files))
        .route("/upload-file", post(upload_file))
        .route("/delete-file", post(delete_file))
        .with_state(pool.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app)
        .await.
        unwrap();
}
