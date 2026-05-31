use rust_worker::setup::{file_setup, auth_setup};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), aws_sdk_s3::Error>{
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
    let pool_c = pool.clone();
    let file_l = tokio::task::spawn(async move { file_layer(pool).await }); 
    let auth_l = tokio::task::spawn(async move { auth_layer(pool_c).await });
    let _ = tokio::join!(file_l, auth_l);
    Ok(())
}
async fn file_layer(pool: PgPool) {
    let app = file_setup(pool).await.unwrap(); 
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Error: {:?}", e);
    }
}
async fn auth_layer(pool: PgPool) {
    let app = auth_setup(pool).await.unwrap(); 
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Error: {:?}", e);
    }
}
