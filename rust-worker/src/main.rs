use axum::{ routing::post, Router };
use sqlx::postgres::PgPoolOptions;
use aws_sdk_s3 as s3;

#[macro_use]
extern crate dotenv_codegen;


mod files;
mod models;
use files::methods::{get_files, create_folder, upload_file, delete_file};
use crate::models::{AppState};


#[tokio::main]
async fn main() -> Result<(), s3::Error> {
    //let DATABASE_URL = dotenv!("DATABASE_URL");
    let DATABASE_URL = "postgresql://postgres:dinqja123@localhost/servr_db";
    
    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&DATABASE_URL)
    .await
    .expect("Failed to create pool");
    let account_id = dotenv!("ACCOUNT_ID");
    let access_key_id = dotenv!("ACCESS_KEY_ID");
    let secret_access_key = dotenv!("SECRET_ACCESS_KEY");

    let config = aws_config::from_env()
        .endpoint_url(format!("https://{}.r2.cloudflarestorage.com" , account_id))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
                access_key_id,
                secret_access_key,
                None,
                None,
                "R2",
        )).region("auto")
        .load()
        .await;

    let client = s3::Client::new(&config);
    
    let state = AppState {pool, client};
    let app = Router::new()//.route("/create-folder",post(create_folder))
        .route("/get-files", post(get_files))
        .route("/upload-file", post(upload_file))
        //.route("/delete-file", post(delete_file))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app)
        .await.
        unwrap();
    
    Ok(())
}
