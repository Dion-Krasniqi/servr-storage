use tokio::net::TcpListener;
use rust_worker::setup::setup;

pub struct TestApp {
    pub base_url: String,
    pub client: reqwest::Client,
}

pub async fn spawn_app() -> TestApp {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(e) => { eprintln!("Error: {}", e);
                    "".to_string()
        },
    };
    
    let pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(5)
    .connect(&database_url)
    .await
    .expect("Failed to create pool");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let app = setup(pool).await.unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestApp {
        base_url: format!("http://127.0.0.1:{}", port),
        client: reqwest::Client::new(),
    }
}


