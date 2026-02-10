use axum::{routing::post,routing::get, Router, };
use sqlx::postgres::PgPoolOptions;
use aws_sdk_s3 as s3;
use uuid::Uuid;

use dotenv::dotenv;
use std::env;

mod files;
mod models;
use files::methods::{get_files, create_folder, upload_file, delete_file, rename_file,
                     create_bucket};
use crate::models::{AppState, FileResponse};

async fn hello_world() -> &'static str {
    println!("Hello");
    "Hello"
}

use moka::future::Cache;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), s3::Error> {

    println!("On");
    let DATABASE_URL = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => "".to_string(),
    };
    //let DATABASE_URL = "postgresql://postgres:dinqja123@localhost/servr_db";
    
    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&DATABASE_URL)
    .await
    .expect("Failed to create pool");
    let account_id = match env::var("ACCOUNT_ID"){
        Ok(url) => url,
        Err(e) => "".to_string(),
    };
    let access_key_id = match env::var("ACCESS_KEY_ID"){
        Ok(url) => url,
        Err(e) => "".to_string(),
    };
    let secret_access_key = match env::var("SECRET_ACCESS_KEY"){
        Ok(url) => url,
        Err(e) => "".to_string(),
    };

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
    
    //let state = AppState {pool, client};
    // cache setup
    const NUM_THREADS: u64 = 100;
    let cache: Cache<Uuid, Vec<FileResponse>> = Cache::new(100);


    let alt_state = AppState {pool, client, cache};
    let app = Router::new()
        .route("/get-files", post(get_files))
        .route("/upload-file", post(upload_file))
        .route("/delete-file", post(delete_file))
        .route("/create-bucket", post(create_bucket))
        .route("/rename-file", post(rename_file))
        .route("/create-folder", post(create_folder))
        .route("/", get(hello_world))
        .with_state(alt_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    
    if let Err(e) = axum::serve(listener, app)
        .await{
            eprintln!("Error : {:?}", e);
        }
    
    Ok(())
}
