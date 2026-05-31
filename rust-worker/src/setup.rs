use axum::{routing::post,
           routing::get, 
           Router};

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use aws_sdk_s3 as s3;
use moka::future::Cache;

use uuid::Uuid;
use std::env;
use std::sync::Arc;
use std::collections::HashMap;

use crate::methods::{get_files, 
                     create_folder, 
                     upload_file, 
                     delete_file, 
                     rename_file,
                     download_file,
                     create_bucket};
use crate::auth_methods::{login_user};
use crate::models::{AppState, AuthState, FileResponse};

async fn hello_world() -> &'static str {
    println!("Hello");
    "Hello"
}

pub async fn file_setup(pool: PgPool) -> Result<Router, s3::Error> {

    println!("File Listener On");
    
    /* Database Connection Setup
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => { eprintln!("Error: {}", e);
                    "".to_string()
        },
    };
    
    let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await
    .expect("Failed to create pool");
    */
    /*
    // R2 API Setup
    let account_id = match env::var("ACCOUNT_ID"){
        Ok(url) => url,
        Err(e) => { eprintln!("Error: {}", e);
                    "".to_string()
        },
    };

    let access_key_id = match env::var("ACCESS_KEY_ID"){
        Ok(url) => url,
        Err(e) => { eprintln!("Error: {}", e);
                    "".to_string()
        },
    };

    let secret_access_key = match env::var("SECRET_ACCESS_KEY"){
        Ok(url) => url, 
        Err(e) => { eprintln!("Error: {}", e);
                    "".to_string()
        },
    };
    
    let r2_url = format!("https://{}.r2.cloudflarestorage.com",
        account_id);

    let r2_credentials = aws_sdk_s3::config::Credentials::new(
        access_key_id,
        secret_access_key,
        None,
        None,
        "R2",
    ); */

    let minio_url = match env::var("MINIO_ENDPOINT") {
        Ok(url) => { 
            println!("Minio: {}",url);
            url
        },
        Err(e) => {
                   eprintln!("Error {:?}", e);
                   "".to_string()
        },
    };
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest()) 
        //was from_env(), but default naming in the env file
        .endpoint_url(minio_url)
        //.credentials_provider(r2_credentials)
        .region(aws_config::meta
            ::region::RegionProviderChain::default_provider()
            .or_else("eu-west-2"))
        .load()
        .await;

    let s3_config = s3::config::Builder::from(&config)
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(s3_config);
    

    // Cache Setup
    const NUM_THREADS: u64 = 100;
    
    let cache: Cache<Uuid, Arc<HashMap<Uuid, FileResponse>>> = 
        Cache::new(NUM_THREADS);

    let state = AppState {pool, client, cache};
    

    //Axum HTTP Server Setup
    let app = Router::new()
        .route("/get-files", post(get_files))
        .route("/upload-file", post(upload_file))
        .route("/delete-file", post(delete_file))
        .route("/create-bucket", post(create_bucket))
        .route("/rename-file", post(rename_file))
        .route("/create-folder", post(create_folder))
        .route("/download-file", post(download_file))
        .route("/", get(hello_world))
        .with_state(state);
 
    Ok(app)
}
pub async fn auth_setup(pool: PgPool) -> Result<Router, s3::Error> {
    println!("Auth Listener On");
    
    let state = AuthState {pool};

    let app = Router::new()
        .route("/sign-in", post(login_user)) 
        .route("/", get(hello_world))
            .with_state(state); 
    Ok(app)
}
