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
                     create_bucket,};
use crate::auth_methods::{login_user, create_user, read_me, logout_user};
use crate::models::{AppState, AuthState, FileResponse};

async fn hello_world() -> &'static str {
    println!("Hello");
    "Hello"
}

pub async fn setup(pool: PgPool) -> Result<Router, s3::Error> {

    println!("Listener On");
    
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
   
    let key: String = match std::env::var("SECRET_KEY") {
    Ok(k) => k,
    Err(e) => {
        "".to_string()
        },
    };

    let state = AppState {pool, client, cache, key};
    

    //Axum HTTP Server Setup
    let app = Router::new()
        .route("/get-files", post(get_files))
        .route("/upload-file", post(upload_file))
        .route("/delete-file", post(delete_file))
        .route("/create-bucket", post(create_bucket))
        .route("/rename-file", post(rename_file))
        .route("/create-folder", post(create_folder))
        .route("/download-file", post(download_file))
        // auth
        .route("/sign-in", post(login_user)) 
        .route("/sign-up", post(create_user))
        .route("/sign-out", post(logout_user)) 
        .route("/me", get(read_me)) 
        .route("/", get(hello_world))
        .with_state(state);
 
    Ok(app)
}
/*
pub async fn auth_setup(pool: PgPool) -> Result<Router, s3::Error> {
    println!("Auth Listener On");
        let mode: usize = match std::env::var("MODE").unwrap().as_str() {
        "DEV" => 0,
        _ =>  1,
    };
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

    let state = AuthState {pool, key: SECRET_KEY, client};
    if mode == 0 {
        println!("Wrong");
        let app = Router::new()
            .route("/sign-in", post(login_test)) 
            .route("/sign-up", post(create_user))
            .route("/sign-out", post(logout_user)) 
            .route("/me", get(get_current_test)) 
            .route("/", get(hello_world))
                .with_state(state); 
        return Ok(app);
    }
    let app = Router::new()
                .route("/", get(hello_world))
            .with_state(state); 
    Ok(app)
}
*/
