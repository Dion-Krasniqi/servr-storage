use rust_worker::setup::setup;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), aws_sdk_s3::Error>{
    
    let app = if let Ok(a) = setup().await {
        a
    } else {
        return Ok(())
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Error: {:?}", e);
    }

    Ok(())
}
